//! Part 5 · `bounded_fanout`: N requests, at most K in flight
//!
//! "Fetch 10,000 URLs with at most 50 requests at a time" is one of the most
//! common tokio interview prompts. The naive answer
//! ([`broken::fetch_all_unbounded`]) spawns one task per item and has all N
//! requests in flight at once: the upstream sees a burst of 10,000
//! connections, you run out of file descriptors, or a rate limiter bans you.
//! Three bounded answers, each of which returns the first error and leaves
//! nothing running behind it:
//!
//! | Version | Tasks alive | Returns results in | On the first error |
//! | --- | --- | --- | --- |
//! | [`fetch_all_semaphore`] | all N, parked on `acquire` | input order (slots) | `JoinSet::shutdown` aborts all N |
//! | [`fetch_all_window`] | at most K | input order (slots) | `JoinSet::shutdown` aborts at most K; the rest never start |
//! | [`fetch_all_buffered`] | none (polled inside this task) | input order | `try_collect` drops the stream |
//! | [`fetch_all_buffer_unordered`] | none (polled inside this task) | completion order | `try_collect` drops the stream |
//!
//! The `JoinSet` versions receive results in completion order and put each
//! one into its input slot. What differs is when the error is noticed:
//! `buffered(k)` hands results out in input order, so an error at index 1
//! waits behind a slow index 0 (a test shows 1 s instead of 10 ms), while
//! `buffer_unordered(k)` and the `JoinSet` versions see it the moment it
//! happens.
//!
//! Spawned vs in-task: `JoinSet` tasks run in parallel on a multi-threaded
//! runtime, must be `Send + 'static`, and get canceled asynchronously
//! (`abort` is a request: an aborted task stops the next time the runtime
//! polls it, which is why the code waits with `shutdown().await`, and a
//! test shows the gauge still counting aborted tasks right after a plain
//! `drop`). `buffered` / `buffer_unordered` poll all K futures from the one
//! task that awaits the stream: concurrency without parallelism, borrowing
//! allowed, and dropping the stream drops every future in it on the spot.
//! Their catch is that the K futures only make progress while that task
//! polls the stream; if the consumer awaits something slow between two
//! items, the requests in the buffer stall too.
//!
//! `Semaphore::acquire` is not cancel safe in the "lose your place in the
//! queue" sense (tokio's `select!` docs list it next to `Mutex::lock`), so
//! do not race it in a `select!` loop and expect fairness.
//!
//! The streams come from the `futures` crate (`StreamExt::buffered`,
//! `buffer_unordered`, `TryStreamExt::try_collect`); a `Stream` is the async
//! iterator described in `57_async_combinators`, with
//! `poll_next(self: Pin<&mut Self>, cx) -> Poll<Option<Item>>`.
//!
//! Compare with `52_condvar/condvar3` (a counting semaphore built from a
//! `Mutex` and a `Condvar`, whose permit gives itself back in `Drop`, like
//! the `OwnedSemaphorePermit` here) and `57_async_combinators/join1` (a
//! hand-written join; `buffer_unordered` is a join over many futures with a
//! cap on how many are polled at once).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures::stream::{self, StreamExt, TryStreamExt};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;

/// One request to make: it takes `latency`, then succeeds or fails.
#[derive(Debug, Clone)]
pub struct Item {
    pub id: u32,
    pub latency: Duration,
    pub fails: bool,
}

impl Item {
    pub fn ok(id: u32, latency: Duration) -> Self {
        Item {
            id,
            latency,
            fails: false,
        }
    }

    pub fn failing(id: u32, latency: Duration) -> Self {
        Item {
            id,
            latency,
            fails: true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FetchError {
    pub id: u32,
}

/// Counts requests: how many are in flight now, the peak, and how many ever
/// started.
#[derive(Debug, Default)]
pub struct Gauge {
    in_flight: AtomicUsize,
    peak: AtomicUsize,
    started: AtomicUsize,
}

impl Gauge {
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }

    pub fn peak(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    pub fn started(&self) -> usize {
        self.started.load(Ordering::Relaxed)
    }

    fn enter(self: &Arc<Self>) -> InFlight {
        self.started.fetch_add(1, Ordering::Relaxed);
        let now = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
        self.peak.fetch_max(now, Ordering::Relaxed);
        InFlight(self.clone())
    }
}

/// Leaves the gauge when dropped: when the request finishes, or when its
/// future is dropped unfinished (canceled).
struct InFlight(Arc<Gauge>);

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A fake request: in flight for `item.latency` (from its first poll), then
/// `Ok(id * 10)`, or `Err` for a failing item.
pub async fn fetch(item: Item, gauge: Arc<Gauge>) -> Result<u32, FetchError> {
    let _in_flight = gauge.enter();
    sleep(item.latency).await;
    if item.fails {
        Err(FetchError { id: item.id })
    } else {
        Ok(item.id * 10)
    }
}

type Indexed = (usize, Result<u32, FetchError>);

/// With `k == 0` nothing ever starts: the semaphore and stream versions
/// would wait forever, so every bounded version panics instead.
fn check_k(k: usize) {
    assert!(k > 0, "at most 0 in flight never makes progress");
}

/// Results by input index, filled in whatever order they arrive.
struct Slots(Vec<Option<u32>>);

impl Slots {
    fn new(n: usize) -> Self {
        Slots(vec![None; n])
    }

    /// Stores one joined result; returns the error if it was one.
    fn fill(&mut self, joined: Result<Indexed, tokio::task::JoinError>) -> Result<(), FetchError> {
        let (index, result) = joined.expect("fetch never panics and nothing aborts it here");
        self.0[index] = Some(result?);
        Ok(())
    }

    fn into_vec(self) -> Vec<u32> {
        self.0
            .into_iter()
            .map(|slot| slot.expect("every item finished"))
            .collect()
    }
}

/// Joins every task of `set` into input order. On the first error it aborts
/// the others, waits until each has stopped, and returns the error.
async fn join_in_order(mut set: JoinSet<Indexed>, n: usize) -> Result<Vec<u32>, FetchError> {
    let mut slots = Slots::new(n);
    while let Some(joined) = set.join_next().await {
        if let Err(err) = slots.fill(joined) {
            set.shutdown().await;
            return Err(err);
        }
    }
    Ok(slots.into_vec())
}

pub mod broken {
    use super::*;

    /// BROKEN: one task per item, all at once. Every request is in flight
    /// at the same time, however many there are.
    pub async fn fetch_all_unbounded(
        items: Vec<Item>,
        gauge: Arc<Gauge>,
    ) -> Result<Vec<u32>, FetchError> {
        let n = items.len();
        let mut set = JoinSet::new();
        for (index, item) in items.into_iter().enumerate() {
            let gauge = gauge.clone();
            set.spawn(async move { (index, fetch(item, gauge).await) });
        }
        join_in_order(set, n).await
    }
}

/// FIX 1: a `Semaphore` with `k` permits. Every item still gets a task, but
/// each task holds a permit for the whole request, so at most `k` requests
/// are in flight; the other tasks wait in `acquire_owned`.
///
/// Every bounded version panics if `k` is 0.
pub async fn fetch_all_semaphore(
    items: Vec<Item>,
    k: usize,
    gauge: Arc<Gauge>,
) -> Result<Vec<u32>, FetchError> {
    check_k(k);
    let n = items.len();
    let permits = Arc::new(Semaphore::new(k));
    let mut set = JoinSet::new();
    for (index, item) in items.into_iter().enumerate() {
        let permits = permits.clone();
        let gauge = gauge.clone();
        set.spawn(async move {
            // Held until this task ends, successful or not.
            let _permit = permits
                .acquire_owned()
                .await
                .expect("the semaphore is never closed");
            (index, fetch(item, gauge).await)
        });
    }
    join_in_order(set, n).await
}

/// FIX 2: a sliding window. Spawn until `k` tasks are running, then wait for
/// one to finish before spawning the next, so at most `k` tasks exist and an
/// error stops the items that have not started yet from ever starting.
pub async fn fetch_all_window(
    items: Vec<Item>,
    k: usize,
    gauge: Arc<Gauge>,
) -> Result<Vec<u32>, FetchError> {
    check_k(k);
    let mut slots = Slots::new(items.len());
    let mut pending = items.into_iter().enumerate();
    let mut set = JoinSet::new();
    loop {
        while set.len() < k {
            let Some((index, item)) = pending.next() else {
                break;
            };
            let gauge = gauge.clone();
            set.spawn(async move { (index, fetch(item, gauge).await) });
        }
        let Some(joined) = set.join_next().await else {
            // Nothing running and nothing left to start.
            return Ok(slots.into_vec());
        };
        if let Err(err) = slots.fill(joined) {
            set.shutdown().await;
            return Err(err);
        }
    }
}

/// FIX 3a: a stream of request futures, at most `k` polled at a time inside
/// this task. Results come out in input order.
pub async fn fetch_all_buffered(
    items: Vec<Item>,
    k: usize,
    gauge: Arc<Gauge>,
) -> Result<Vec<u32>, FetchError> {
    check_k(k);
    stream::iter(items)
        .map(|item| fetch(item, gauge.clone()))
        .buffered(k)
        .try_collect()
        .await
}

/// FIX 3b: like [`fetch_all_buffered`], but results (and errors) come out in
/// completion order.
pub async fn fetch_all_buffer_unordered(
    items: Vec<Item>,
    k: usize,
    gauge: Arc<Gauge>,
) -> Result<Vec<u32>, FetchError> {
    check_k(k);
    stream::iter(items)
        .map(|item| fetch(item, gauge.clone()))
        .buffer_unordered(k)
        .try_collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    const MS: Duration = Duration::from_millis(1);
    const SEC: Duration = Duration::from_secs(1);
    const N: u32 = 50;
    const K: usize = 5;

    fn items(n: u32, latency: Duration) -> Vec<Item> {
        (0..n).map(|id| Item::ok(id, latency)).collect()
    }

    fn expected(n: u32) -> Vec<u32> {
        (0..n).map(|id| id * 10).collect()
    }

    /// Item 1 fails after 10 ms; every other item takes 1 s.
    fn early_failure(n: u32) -> Vec<Item> {
        (0..n)
            .map(|id| match id {
                1 => Item::failing(id, 10 * MS),
                _ => Item::ok(id, SEC),
            })
            .collect()
    }

    // ---- the bug, demonstrated -----------------------------------------

    #[tokio::test(start_paused = true)]
    async fn broken_unbounded_puts_every_request_in_flight_at_once() {
        let gauge = Arc::new(Gauge::default());
        let out = broken::fetch_all_unbounded(items(N, 100 * MS), gauge.clone()).await;
        assert_eq!(out, Ok(expected(N)));
        assert_eq!(gauge.peak(), N as usize);
    }

    // ---- at most K in flight ------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn every_bounded_version_caps_in_flight_at_k_and_keeps_input_order() {
        let started_at = Instant::now();
        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_semaphore(items(N, 100 * MS), K, gauge.clone()).await;
        assert_eq!(out, Ok(expected(N)));
        assert_eq!(gauge.peak(), K);
        // 50 items, 5 at a time, 100 ms each: 10 waves.
        assert_eq!(started_at.elapsed(), 10 * 100 * MS);

        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_window(items(N, 100 * MS), K, gauge.clone()).await;
        assert_eq!(out, Ok(expected(N)));
        assert_eq!(gauge.peak(), K);

        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_buffered(items(N, 100 * MS), K, gauge.clone()).await;
        assert_eq!(out, Ok(expected(N)));
        assert_eq!(gauge.peak(), K);

        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_buffer_unordered(items(N, 100 * MS), K, gauge.clone()).await;
        // Equal latencies: completion order is input order here.
        assert_eq!(out, Ok(expected(N)));
        assert_eq!(gauge.peak(), K);
    }

    #[tokio::test(start_paused = true)]
    async fn buffered_yields_input_order_buffer_unordered_completion_order() {
        let items = vec![
            Item::ok(0, 30 * MS),
            Item::ok(1, 10 * MS),
            Item::ok(2, 20 * MS),
        ];
        let gauge = Arc::new(Gauge::default());
        let ordered = fetch_all_buffered(items.clone(), 3, gauge.clone()).await;
        assert_eq!(ordered, Ok(vec![0, 10, 20]));
        let unordered = fetch_all_buffer_unordered(items.clone(), 3, gauge.clone()).await;
        assert_eq!(unordered, Ok(vec![10, 20, 0]));
        // The JoinSet versions finish in completion order too, but put each
        // result back into its input slot.
        let window = fetch_all_window(items, 3, gauge).await;
        assert_eq!(window, Ok(vec![0, 10, 20]));
    }

    // ---- short-circuit on the first error -------------------------------

    #[tokio::test(start_paused = true)]
    async fn window_stops_at_the_first_error_and_never_starts_the_rest() {
        let started_at = Instant::now();
        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_window(early_failure(N), 3, gauge.clone()).await;
        assert_eq!(out, Err(FetchError { id: 1 }));
        assert_eq!(started_at.elapsed(), 10 * MS);
        // Only the first window ever started, and `shutdown().await` has
        // waited until the two aborted requests were really dropped.
        assert_eq!(gauge.started(), 3);
        assert_eq!(gauge.in_flight(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn semaphore_stops_at_the_first_error_and_aborts_the_waiters() {
        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_semaphore(early_failure(N), 3, gauge.clone()).await;
        assert_eq!(out, Err(FetchError { id: 1 }));
        // All 50 tasks existed, but only permit holders start a request.
        // The failing task hands its permit to the next waiter as it ends,
        // before the joiner has seen the error, so one more request may
        // start (and is then aborted); the window version cannot do that,
        // because only the joiner starts new work. The rest were aborted
        // while still waiting in `acquire_owned`.
        let started = gauge.started();
        assert!((3..=4).contains(&started), "started: {started}");
        assert_eq!(gauge.in_flight(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn buffer_unordered_sees_the_error_at_once_buffered_waits_its_turn() {
        let started_at = Instant::now();
        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_buffer_unordered(early_failure(N), 3, gauge.clone()).await;
        assert_eq!(out, Err(FetchError { id: 1 }));
        assert_eq!(started_at.elapsed(), 10 * MS);
        // Dropping the stream dropped the in-flight futures on the spot.
        assert_eq!(gauge.in_flight(), 0);

        let started_at = Instant::now();
        let gauge = Arc::new(Gauge::default());
        let out = fetch_all_buffered(early_failure(N), 3, gauge.clone()).await;
        assert_eq!(out, Err(FetchError { id: 1 }));
        // The error of item 1 is handed out only after item 0 (1 s).
        assert_eq!(started_at.elapsed(), SEC);
        assert_eq!(gauge.in_flight(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn dropping_a_joinset_only_requests_the_abort() {
        let gauge = Arc::new(Gauge::default());
        let mut set = JoinSet::new();
        for id in 0..3 {
            set.spawn(fetch(Item::ok(id, SEC), gauge.clone()));
        }
        // Virtual time moves only when every task is idle: all three have
        // started when this returns.
        sleep(MS).await;
        assert_eq!(gauge.in_flight(), 3);
        drop(set);
        // Dropping the set aborted the tasks, but an aborted task stops only
        // when the runtime next polls it: all three futures are still alive.
        assert_eq!(gauge.in_flight(), 3);
        sleep(MS).await;
        assert_eq!(gauge.in_flight(), 0);

        // `shutdown().await` returns only once every task has stopped.
        let mut set = JoinSet::new();
        for id in 0..3 {
            set.spawn(fetch(Item::ok(id, SEC), gauge.clone()));
        }
        sleep(MS).await;
        assert_eq!(gauge.in_flight(), 3);
        set.shutdown().await;
        assert_eq!(gauge.in_flight(), 0);
    }
}
