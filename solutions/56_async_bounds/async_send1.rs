// Module 4 · Async bounds — part 1: a std `MutexGuard` held across `.await` makes a spawned future `!Send`.
//
// `tokio::spawn` (like `smol::spawn` and friends) asks of a future exactly what
// `thread::spawn` asks of a closure:
//
//     pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
//     where
//         F: Future + Send + 'static,
//         F::Output: Send + 'static,
//
// A multi-threaded runtime may poll a task on one worker thread, suspend it at
// an `.await`, and resume it on another (work stealing), so the whole future
// must be `Send`. And an `async fn`'s future is the state machine from
// `28_futures/futures3`: at every `.await` it stores each local that is still
// alive there. `Send` is an auto trait (`30_send_sync`), so the future is
// `Send` exactly when everything it keeps across an `.await` is.
//
// `std::sync::MutexGuard` is `!Send`. With POSIX threads a mutex must be
// unlocked by the thread that locked it, and a guard unlocks when it drops,
// so it must not be dropped on another thread. A guard still alive at an
// `.await` therefore makes the whole future `!Send`, and `spawn` says so:
// "future cannot be sent between threads safely ... has type
// `std::sync::MutexGuard<'_, Stats>` which is not `Send` ... await occurs
// here, with `mut guard` maybe used later". There is no E-code: it is
// rustc's future-specific wording of an unmet `Send` bound.
//
// The part interviewers dig into: `drop(guard)` right before the `.await` is
// often NOT enough (see `flush` below). Moving the guard out releases the
// lock at run time, but the analysis that decides what the state machine
// stores is conservative: a local that was ever borrowed might still be
// pointed to by some reference, so it keeps its slot in the future until the
// end of its SCOPE, moved out or not. And `guard.field` borrows `guard`, since
// every field access goes through `Deref` or `DerefMut`. So the type check
// still sees a `MutexGuard` stored across the `.await`. Only a guard that was
// never borrowed is really gone after `drop`. What always works is ending
// the guard's scope before the `.await`.
//
// Even where it compiles, holding a blocking lock across `.await` invites a
// deadlock. `tokio::spawn` demands `Send` on every runtime flavor, even a
// current-thread one; only `block_on` and `spawn_local` (on a `LocalSet`)
// accept a `!Send` future. There, while the task is suspended, another task
// on the same thread may call `lock()`. That thread already holds the mutex,
// so the call never returns (std's docs: it might deadlock or panic), and the
// suspended task never runs again to release the guard. Clippy's
// `await_holding_lock` lint flags it. When a lock really must stay held
// across an `.await`, use an async mutex such as `tokio::sync::Mutex`, whose
// `lock().await` suspends instead of blocking. For short critical sections
// that never cross an `.await`, tokio's own docs prefer the std `Mutex`: it
// is cheaper.
//
// How interviewers probe this: "Why does `tokio::spawn` reject this future?",
// "Why doesn't `drop(guard)` before the `.await` fix it?", "Would it work on a
// current-thread runtime?", "When do you pick `tokio::sync::Mutex`?".

use std::future::Future;
use std::mem;
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};

// ---- A tiny thread-per-task runtime (given) --------------------------------

// Runs `future` on a new OS thread and returns a handle to join it. The bounds
// are exactly `tokio::spawn`'s: the future moves to another thread (`Send`)
// and may outlive its caller (`'static`), and so does its output.
fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    thread::spawn(move || block_on(future))
}

// Polls `future` on the current thread until it is ready. The leaf future in
// this file wakes itself and is `Pending` for one poll only, so a plain poll
// loop is enough (a real executor sleeps until the waker fires, as in
// `29_async_runtime/runtime2`). The bound turns a future that never finishes
// into a panic instead of a hang.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..1_000 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
    panic!("block_on: the future was still pending after 1000 polls");
}

// Stands in for network I/O: `Pending` on the first poll, `Ready` on the
// second (the `YieldOnce` future of `28_futures/futures2`).
struct Io {
    done: bool,
}

impl Future for Io {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            Poll::Ready(())
        } else {
            self.done = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn io() -> Io {
    Io { done: false }
}

// ---- The server -------------------------------------------------------------

// Request statistics shared by every task of a server.
#[derive(Debug, Default)]
struct Stats {
    started: u32,
    finished: u32,
    // Ids of finished requests that `flush` has not uploaded yet.
    queued: Vec<u32>,
    uploaded: usize,
}

// The request handler: one round trip to a backend.
async fn handle_request() {
    io().await;
}

// Sends a batch of request ids to the metrics collector and returns how many
// it sent.
async fn upload(batch: &[u32]) -> usize {
    io().await;
    batch.len()
}

// Records one request: counts it as started, runs the handler, then counts it
// as finished and queues its id for the next `flush`.
async fn record(stats: Arc<Mutex<Stats>>, id: u32) {
    // A guard that is a temporary dies at the end of its statement, so no
    // `MutexGuard` is alive at the `.await`: the future is `Send`, and other
    // tasks can take the lock while this one waits for the handler.
    stats.lock().unwrap().started += 1;
    handle_request().await;
    let mut guard = stats.lock().unwrap();
    guard.finished += 1;
    guard.queued.push(id);
}

// Uploads every queued id and returns how many were sent.
async fn flush(stats: Arc<Mutex<Stats>>) -> usize {
    // The block ends the guard's scope before the `.await`, so the future
    // stores no `MutexGuard` (a `drop` would not do: `guard` was borrowed).
    // `mem::take` empties the queue while it is locked, so ids queued during
    // the upload stay there for the next flush.
    let batch = {
        let mut guard = stats.lock().unwrap();
        mem::take(&mut guard.queued)
    };
    let sent = upload(&batch).await;
    stats.lock().unwrap().uploaded += sent;
    sent
}

// Serves requests `0..n`, each on a task of its own, waits for all of them,
// then flushes the queued ids on one more task. Returns how many were
// uploaded.
fn serve(stats: &Arc<Mutex<Stats>>, n: u32) -> usize {
    let handles: Vec<_> = (0..n)
        .map(|id| spawn(record(Arc::clone(stats), id)))
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    spawn(flush(Arc::clone(stats))).join().unwrap()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compiles only if `T: Send`: the same check `spawn` makes.
    fn assert_send<T: Send>(_: &T) {}

    fn shared() -> Arc<Mutex<Stats>> {
        Arc::new(Mutex::new(Stats::default()))
    }

    #[test]
    fn record_and_flush_futures_are_send() {
        let stats = shared();
        let record_future = record(Arc::clone(&stats), 1);
        let flush_future = flush(Arc::clone(&stats));
        assert_send(&record_future);
        assert_send(&flush_future);
        // Futures are lazy: creating them ran nothing.
        drop((record_future, flush_future));
        assert_eq!(stats.lock().unwrap().started, 0);
    }

    #[test]
    fn nothing_is_locked_while_record_is_suspended() {
        let stats = shared();
        let mut cx = Context::from_waker(Waker::noop());
        let mut task = pin!(record(Arc::clone(&stats), 7));

        // The first poll stops inside `handle_request().await`.
        assert!(task.as_mut().poll(&mut cx).is_pending());
        {
            let now = stats
                .try_lock()
                .expect("`record` still holds the lock while it is suspended");
            assert_eq!((now.started, now.finished), (1, 0));
        }

        assert!(task.as_mut().poll(&mut cx).is_ready());
        let done = stats.lock().unwrap();
        assert_eq!((done.started, done.finished), (1, 1));
        assert_eq!(done.queued, [7]);
    }

    #[test]
    fn two_tasks_interleave_on_one_thread() {
        // What a current-thread runtime does: poll several tasks on ONE
        // thread. Task 2 can take the lock only because the suspended task 1
        // released it; a blocking `lock()` would otherwise hang the thread.
        let stats = shared();
        let mut cx = Context::from_waker(Waker::noop());
        let mut first = pin!(record(Arc::clone(&stats), 1));
        let mut second = pin!(record(Arc::clone(&stats), 2));

        assert!(first.as_mut().poll(&mut cx).is_pending());
        assert!(stats.try_lock().is_ok(), "task 1 kept the lock");
        assert!(second.as_mut().poll(&mut cx).is_pending());
        assert!(second.as_mut().poll(&mut cx).is_ready());
        assert!(first.as_mut().poll(&mut cx).is_ready());

        let done = stats.lock().unwrap();
        assert_eq!((done.started, done.finished), (2, 2));
        // Task 2 finished first, so its id was queued first.
        assert_eq!(done.queued, [2, 1]);
    }

    #[test]
    fn nothing_is_locked_while_flush_uploads() {
        let stats = shared();
        stats.lock().unwrap().queued = vec![3, 1, 2];
        let mut cx = Context::from_waker(Waker::noop());
        let mut task = pin!(flush(Arc::clone(&stats)));

        // The first poll stops inside `upload(..).await`.
        assert!(task.as_mut().poll(&mut cx).is_pending());
        {
            let now = stats
                .try_lock()
                .expect("`flush` still holds the lock while it uploads");
            assert!(now.queued.is_empty(), "the batch was taken out");
            assert_eq!(now.uploaded, 0);
        }

        assert_eq!(task.as_mut().poll(&mut cx), Poll::Ready(3));
        assert_eq!(stats.lock().unwrap().uploaded, 3);
    }

    #[test]
    fn ids_queued_during_an_upload_wait_for_the_next_flush() {
        let stats = shared();
        stats.lock().unwrap().queued = vec![1];
        let mut cx = Context::from_waker(Waker::noop());
        let mut first_flush = pin!(flush(Arc::clone(&stats)));
        assert!(first_flush.as_mut().poll(&mut cx).is_pending());

        // Request 2 finishes while the first upload is still in flight.
        block_on(record(Arc::clone(&stats), 2));

        assert_eq!(first_flush.as_mut().poll(&mut cx), Poll::Ready(1));
        assert_eq!(stats.lock().unwrap().queued, [2]);
        assert_eq!(block_on(flush(Arc::clone(&stats))), 1);
        let done = stats.lock().unwrap();
        assert!(done.queued.is_empty());
        assert_eq!(done.uploaded, 2);
    }

    #[test]
    fn fifty_spawned_requests_all_finish() {
        let stats = shared();
        assert_eq!(serve(&stats, 50), 50);
        let done = stats.lock().unwrap();
        assert_eq!((done.started, done.finished), (50, 50));
        assert_eq!(done.uploaded, 50);
        assert!(done.queued.is_empty());
    }

    #[test]
    fn spawned_records_queue_every_id_once() {
        let stats = shared();
        let handles: Vec<_> = (0..50)
            .map(|id| spawn(record(Arc::clone(&stats), id)))
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let mut ids = mem::take(&mut stats.lock().unwrap().queued);
        ids.sort_unstable();
        assert_eq!(ids, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn serving_no_requests_uploads_nothing() {
        let stats = shared();
        assert_eq!(serve(&stats, 0), 0);
        let done = stats.lock().unwrap();
        assert_eq!((done.started, done.finished, done.uploaded), (0, 0, 0));
    }
}
