//! Part 4 · `blocking_in_async`: a CPU loop starves every task on its thread
//!
//! A tokio task gives its worker thread back only at an `.await` that
//! returns `Pending`. A CPU-heavy loop (hashing, compression, parsing a large
//! document, a big sort) has no such `.await`, so while it runs, no other
//! task on that thread runs: not the heartbeat, not the timers, not the
//! sockets. On a `current_thread` runtime, which is what `#[tokio::test]`
//! uses by default, that is every other task in the program.
//!
//! The work here is [`crunch`]: rounds of a 64-bit hash that, between two
//! rounds, look at the heartbeat counter. It stops once it has seen `want`
//! beats, or when its watchdog (`give_up`) runs out. The watchdog is what
//! keeps the broken variant finite; the fixed variants never reach it.
//!
//! - [`broken::crunch_inline`] runs it inside the async task. The heartbeat
//!   never beats: `beats_seen` is 0 when the watchdog fires. Not even a
//!   `tokio::time::timeout` helps: a timeout is just another future, and it
//!   cannot fire while the task it lives in is stuck in a loop (a test shows
//!   `timeout(50 ms, ...)` returning `Ok` after 200 ms).
//! - [`crunch_in_spawn_blocking`] moves it to tokio's blocking thread pool
//!   with `spawn_blocking` and awaits the `JoinHandle`. The runtime thread is
//!   free, and the heartbeat keeps beating.
//! - [`crunch_cooperatively`] keeps it on the runtime but calls
//!   `tokio::task::yield_now().await` after every round, the fix from
//!   `58_leaf_futures/yield1`. Fine for work that is naturally chunked; it
//!   still occupies the runtime thread for all the CPU time.
//! - `tokio::task::block_in_place` is the fourth option, and it exists only
//!   on the multi-threaded runtime: it panics on `current_thread` (a test
//!   checks the message).
//!
//! For a lot of CPU work, a dedicated pool (rayon, with a oneshot channel to
//! send the answer back) beats `spawn_blocking`, whose pool is sized for
//! threads that mostly wait (512 by default). Code in `spawn_blocking` cannot
//! be canceled: aborting its `JoinHandle` does nothing once it has started.
//!
//! These tests run in real time: under `start_paused`, tokio stops
//! auto-advancing the clock while a `spawn_blocking` task runs, so the
//! heartbeat's timer would never fire. No test races two clocks: the broken
//! variant cannot see a beat however slow or fast the machine is (no other
//! task can run on its thread), and the watchdog only bounds the fixed ones.

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Adds one to `beats` every `period`, forever. Abort the task to stop it.
pub async fn heartbeat(beats: Arc<AtomicU64>, period: Duration) {
    let mut ticker = tokio::time::interval(period);
    loop {
        ticker.tick().await;
        // `Relaxed`: a counter that publishes no other data.
        beats.fetch_add(1, Ordering::Relaxed);
    }
}

/// What a CPU job saw of the heartbeat while it ran.
#[derive(Debug)]
pub struct Probe {
    /// The beat counter when the job stopped.
    pub beats_seen: u64,
    /// True if the watchdog ended the job before it saw `want` beats.
    pub gave_up: bool,
}

/// Hash steps per round, between two looks at the beat counter.
const ROUND: u32 = 10_000;

/// One round of CPU work: `ROUND` steps of a 64-bit LCG. No `.await`.
fn crunch_round(mut acc: u64) -> u64 {
    for _ in 0..ROUND {
        acc = acc
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
    }
    // Keep the optimizer from deleting the work.
    black_box(acc)
}

/// Checks the counter after a round: `Some(probe)` means stop.
fn check(beats: &AtomicU64, want: u64, start: Instant, give_up: Duration) -> Option<Probe> {
    let beats_seen = beats.load(Ordering::Relaxed);
    if beats_seen >= want {
        Some(Probe {
            beats_seen,
            gave_up: false,
        })
    } else if start.elapsed() >= give_up {
        Some(Probe {
            beats_seen,
            gave_up: true,
        })
    } else {
        None
    }
}

/// Synchronous CPU work: crunches until it has seen `want` beats, or until
/// `give_up` has passed.
pub fn crunch(beats: &AtomicU64, want: u64, give_up: Duration) -> Probe {
    let start = Instant::now();
    let mut acc = 0;
    loop {
        acc = crunch_round(acc);
        if let Some(probe) = check(beats, want, start, give_up) {
            return probe;
        }
    }
}

pub mod broken {
    use super::*;

    /// BROKEN: CPU work directly in an async fn. Its first poll runs the
    /// whole loop, and nothing else on this thread runs until it returns.
    pub async fn crunch_inline(beats: Arc<AtomicU64>, want: u64, give_up: Duration) -> Probe {
        crunch(&beats, want, give_up)
    }
}

/// FIX 1: run the work on the blocking pool; the task awaits the result.
pub async fn crunch_in_spawn_blocking(
    beats: Arc<AtomicU64>,
    want: u64,
    give_up: Duration,
) -> Probe {
    tokio::task::spawn_blocking(move || crunch(&beats, want, give_up))
        .await
        .expect("crunch does not panic")
}

/// FIX 2: stay on the runtime, but give the thread back after every round.
pub async fn crunch_cooperatively(beats: Arc<AtomicU64>, want: u64, give_up: Duration) -> Probe {
    let start = Instant::now();
    let mut acc = 0;
    loop {
        acc = crunch_round(acc);
        if let Some(probe) = check(&beats, want, start, give_up) {
            return probe;
        }
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERIOD: Duration = Duration::from_millis(1);
    /// Bounds the fixed variants; they return after about 3 periods.
    const WATCHDOG: Duration = Duration::from_secs(5);
    /// How long the broken variant spins before it gives up.
    const SHORT_WATCHDOG: Duration = Duration::from_millis(200);

    fn start_heartbeat() -> (Arc<AtomicU64>, tokio::task::JoinHandle<()>) {
        let beats = Arc::new(AtomicU64::new(0));
        let task = tokio::spawn(heartbeat(beats.clone(), PERIOD));
        (beats, task)
    }

    // ---- the bug, demonstrated -----------------------------------------

    #[tokio::test]
    async fn broken_inline_crunch_starves_the_heartbeat() {
        let (beats, heartbeat) = start_heartbeat();
        let probe = broken::crunch_inline(beats.clone(), 3, SHORT_WATCHDOG).await;
        // 200 ms at one beat per millisecond, and not a single beat: the
        // heartbeat task never got the thread.
        assert!(probe.gave_up, "the heartbeat got the thread: {probe:?}");
        assert_eq!(probe.beats_seen, 0);
        heartbeat.abort();
    }

    #[tokio::test]
    async fn broken_a_timeout_cannot_interrupt_blocking_code() {
        let (beats, heartbeat) = start_heartbeat();
        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_millis(50),
            broken::crunch_inline(beats, 3, SHORT_WATCHDOG),
        )
        .await;
        // `Timeout` polls the inner future first, and that single poll runs
        // the whole 200 ms loop: the 50 ms timeout "succeeds".
        let probe = outcome.expect("the timer never got a chance to fire");
        assert!(probe.gave_up, "the heartbeat got the thread: {probe:?}");
        assert!(started.elapsed() >= SHORT_WATCHDOG);
        heartbeat.abort();
    }

    // ---- the fixes ------------------------------------------------------

    #[tokio::test]
    async fn spawn_blocking_leaves_the_runtime_free_for_the_heartbeat() {
        let (beats, heartbeat) = start_heartbeat();
        let probe = crunch_in_spawn_blocking(beats, 3, WATCHDOG).await;
        assert!(!probe.gave_up, "too few beats: {probe:?}");
        assert!(probe.beats_seen >= 3);
        heartbeat.abort();
    }

    #[tokio::test]
    async fn yielding_every_round_lets_the_heartbeat_run() {
        let (beats, heartbeat) = start_heartbeat();
        let probe = crunch_cooperatively(beats, 3, WATCHDOG).await;
        assert!(!probe.gave_up, "too few beats: {probe:?}");
        assert!(probe.beats_seen >= 3);
        heartbeat.abort();
    }

    #[tokio::test]
    #[should_panic(expected = "can call blocking only when running on the multi-threaded runtime")]
    async fn block_in_place_panics_on_current_thread() {
        tokio::task::block_in_place(|| ());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn block_in_place_works_on_the_multi_thread_runtime() {
        let (beats, heartbeat) = start_heartbeat();
        // Called from a worker thread: the worker hands its other tasks to
        // a new thread, then runs the closure where it is.
        let probe =
            tokio::spawn(
                async move { tokio::task::block_in_place(|| crunch(&beats, 3, WATCHDOG)) },
            )
            .await
            .unwrap();
        assert!(!probe.gave_up, "too few beats: {probe:?}");
        heartbeat.abort();
    }
}
