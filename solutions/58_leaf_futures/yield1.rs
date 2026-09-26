// Module 3 · Leaf futures — part 3: cooperative yielding, and why `async fn yield_now() {}` never yields.
//
// Async tasks are COOPERATIVE: the executor cannot preempt a task, it only
// gets its thread back when `poll` returns. And an unfinished task's `poll`
// returns only when one of the leaf futures it awaits returns `Pending`.
// Everything between two such points runs as one uninterrupted stretch. So
// a long CPU loop inside an async fn starves every other task on that
// thread: heartbeats, timers, sockets waiting to be read. It is the
// CPU-bound twin of the blocking `thread::sleep` from part 2 (`timer1`).
//
// For a loop that has to stay on the executor, the fix is to give the thread
// back now and then: `tokio::task::yield_now().await` every N iterations. You
// built this kind of leaf in `28_futures/futures2` (`YieldOnce`). The new
// questions are why the obvious shortcut fails, and where to yield:
//
//   - `.await` is NOT a yield point by itself. It polls the awaited future and
//     carries on at once if that future is `Ready`. `async fn yield_now() {}`
//     awaits nothing, so its future is `Ready` on the first poll and the
//     caller never leaves `poll`. Only a leaf that returns `Pending` suspends
//     a task.
//   - A leaf that returns `Pending` must make sure it is woken, or its task is
//     never polled again. No channel or timer is involved here, so the leaf
//     wakes its own task. The executor here (like `runtime3`'s) puts the
//     woken task at the BACK of its ready queue, so every other ready task
//     runs first.
//   - `std::thread::yield_now()` is something else: it offers the OS THREAD's
//     time slice to other threads. The other tasks live on this very thread,
//     so they gain nothing.
//   - How often to yield is a trade-off. Every yield is a round trip through
//     the executor's queue, so a loop yields once per batch of work (here,
//     every `every` items), not on every iteration.
//
// std's `Waker::wake` docs spell out the limit: yielding to other tasks is
// not guaranteed, because the executor may choose to poll the same task
// again. tokio's `yield_now` also defers its wake-up, so the task is queued
// again only after the runtime has polled its I/O and timer driver. tokio
// also has an automatic COOP BUDGET: after about 128 operations in one poll,
// its channels, sockets and timers return `Pending` on purpose. A pure CPU
// loop touches none of them, which is why it still needs an explicit yield,
// or better, `spawn_blocking` (or a thread pool like rayon) for heavy
// computation.
//
// Interviewers ask "what happens if you run a CPU-heavy loop in an async
// task?", "why doesn't `async {}` yield?", and "implement `yield_now`". The
// follow-ups: "is it safe to wake a task from inside its own `poll`?" (yes:
// the executor queues it and polls it again after this poll returns; tokio
// sets the running task's NOTIFIED bit and re-queues it once the poll ends),
// and "is returning `Pending` without waking ever OK?" (only when someone
// else holds the waker, like the channel in `oneshot1` or the timer in
// `timer1`).

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

/// Pending once, then ready: the leaf behind `yield_now`.
struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            return Poll::Ready(());
        }
        self.yielded = true;
        // Nobody else will wake this task, so it wakes itself: "poll me
        // again, but after everyone else who is ready". Waking before we
        // return is fine; the executor re-queues the task and polls it only
        // after this `poll` has returned `Pending`.
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Gives the executor's thread back once, letting the other ready tasks run.
fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

/// Sums `data`. It shares the executor's thread with every other task, so it
/// must give the thread back after each batch of `every` items. `progress`
/// counts the items summed so far. `every` must not be 0.
async fn checksum(data: Vec<u64>, every: usize, progress: Arc<AtomicUsize>) -> u64 {
    let mut sum = 0_u64;
    for (i, x) in data.into_iter().enumerate() {
        // Yield before each new batch of `every` items: never more than
        // `every` items per poll, and one executor round trip per batch
        // instead of one per item. (No yield before the first batch: the task
        // has only just been polled.)
        if i > 0 && i % every == 0 {
            yield_now().await;
        }
        sum = sum.wrapping_add(x);
        progress.store(i + 1, Ordering::Relaxed);
    }
    sum
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::{Pin, pin};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::sync::mpsc;
    use std::task::{Context, Poll, Wake, Waker};

    // `runtime3`'s single-threaded executor, with two guards so that a broken
    // `yield_now` fails the test instead of hanging it. Every wake-up here
    // happens on this thread, during a poll, so an empty ready queue while
    // tasks are unfinished means nobody will ever wake them: `run` panics.
    // It also panics after `POLL_BUDGET` polls (a task that never finishes).
    const POLL_BUDGET: usize = 100_000;

    type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

    struct Task {
        id: usize,
        future: Mutex<Option<BoxFuture>>,
        queue: mpsc::Sender<Arc<Task>>,
    }

    impl Wake for Task {
        fn wake(self: Arc<Self>) {
            // Fails only once `run` has returned; nobody is listening then.
            let _ = self.queue.send(Arc::clone(&self));
        }
    }

    /// Runs every future on this thread, one poll at a time, in FIFO order,
    /// until all are done. Returns how often each one was polled.
    fn run(futures: Vec<BoxFuture>) -> Vec<usize> {
        let (queue, ready) = mpsc::channel();
        let mut polls = vec![0; futures.len()];
        let mut unfinished = futures.len();
        for (id, future) in futures.into_iter().enumerate() {
            let task = Arc::new(Task {
                id,
                future: Mutex::new(Some(future)),
                queue: queue.clone(),
            });
            queue.send(task).unwrap();
        }
        let mut total = 0;
        while unfinished > 0 {
            let Ok(task) = ready.try_recv() else {
                panic!(
                    "stuck: {unfinished} task(s) returned Pending and nothing \
                     will ever wake them"
                );
            };
            let mut slot = task.future.lock().unwrap();
            // A wake-up for a task that already finished: nothing to do.
            let Some(future) = slot.as_mut() else {
                continue;
            };
            total += 1;
            assert!(
                total <= POLL_BUDGET,
                "more than {POLL_BUDGET} polls: a task keeps returning Pending \
                 (does `yield_now` ever become Ready?)"
            );
            polls[task.id] += 1;
            let waker = Waker::from(Arc::clone(&task));
            if future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_ready()
            {
                *slot = None;
                unfinished -= 1;
            }
        }
        polls
    }

    type Log = Arc<Mutex<Vec<&'static str>>>;

    fn note(log: &Log, event: &'static str) {
        log.lock().unwrap().push(event);
    }

    fn events(log: &Log) -> Vec<&'static str> {
        log.lock().unwrap().clone()
    }

    // A waker that only counts how often it was woken.
    struct CountingWaker {
        wakes: AtomicUsize,
    }

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn yield_now_is_pending_once_then_ready() {
        let counter = Arc::new(CountingWaker {
            wakes: AtomicUsize::new(0),
        });
        let waker = Waker::from(Arc::clone(&counter));
        let mut cx = Context::from_waker(&waker);
        let mut fut = pin!(yield_now());

        assert_eq!(
            fut.as_mut().poll(&mut cx),
            Poll::Pending,
            "the first poll must give the thread back"
        );
        assert_eq!(
            counter.wakes.load(Ordering::SeqCst),
            1,
            "a yield has to reschedule its own task exactly once, or nothing \
             ever polls it again"
        );
        assert_eq!(
            fut.as_mut().poll(&mut cx),
            Poll::Ready(()),
            "the second poll must finish: a yield is Pending exactly once"
        );
        assert_eq!(counter.wakes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn two_yielding_tasks_interleave() {
        let log = Log::default();
        let l = Arc::clone(&log);
        let a: BoxFuture = Box::pin(async move {
            note(&l, "a0");
            yield_now().await;
            note(&l, "a1");
            yield_now().await;
            note(&l, "a2");
        });
        let l = Arc::clone(&log);
        let b: BoxFuture = Box::pin(async move {
            note(&l, "b0");
            yield_now().await;
            note(&l, "b1");
            yield_now().await;
            note(&l, "b2");
        });
        let polls = run(vec![a, b]);
        assert_eq!(
            events(&log),
            ["a0", "b0", "a1", "b1", "a2", "b2"],
            "every yield must let the other task run before this one continues"
        );
        assert_eq!(polls, [3, 3], "two yields each: three polls per task");
    }

    #[test]
    fn a_task_that_does_not_yield_runs_to_completion() {
        let log = Log::default();
        let l = Arc::clone(&log);
        let a: BoxFuture = Box::pin(async move {
            note(&l, "a0");
            yield_now().await;
            note(&l, "a1");
        });
        let l = Arc::clone(&log);
        let b: BoxFuture = Box::pin(async move {
            note(&l, "b0");
            note(&l, "b1");
        });
        let polls = run(vec![a, b]);
        assert_eq!(events(&log), ["a0", "b0", "b1", "a1"]);
        assert_eq!(polls, [2, 1]);
    }

    #[test]
    fn a_lone_task_that_yields_is_polled_again() {
        // No other task is ready, so the executor goes straight back to this
        // one, but only because the yield woke it.
        let log = Log::default();
        let l = Arc::clone(&log);
        let polls = run(vec![Box::pin(async move {
            for _ in 0..3 {
                yield_now().await;
            }
            note(&l, "done");
        })]);
        assert_eq!(events(&log), ["done"]);
        assert_eq!(polls, [4], "three yields: four polls");
    }

    #[test]
    fn a_cpu_loop_that_yields_lets_a_heartbeat_tick() {
        const ITEMS: usize = 10_000;
        const EVERY: usize = 1_000;
        const MAX_TICKS: usize = 50_000;

        let progress = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicBool::new(false));
        let sum = Arc::new(AtomicU64::new(0));
        let ticks = Arc::new(Mutex::new(Vec::new()));

        let crunch: BoxFuture = {
            let (progress, done, sum) = (progress.clone(), done.clone(), sum.clone());
            Box::pin(async move {
                let data: Vec<u64> = (1..=ITEMS as u64).collect();
                sum.store(checksum(data, EVERY, progress).await, Ordering::SeqCst);
                done.store(true, Ordering::SeqCst);
            })
        };
        // Each tick records how far the checksum has got. The loop is bounded
        // so that a broken `yield_now` cannot spin here forever.
        let heartbeat: BoxFuture = {
            let (progress, done, ticks) = (progress.clone(), done.clone(), ticks.clone());
            Box::pin(async move {
                for _ in 0..MAX_TICKS {
                    ticks.lock().unwrap().push(progress.load(Ordering::SeqCst));
                    if done.load(Ordering::SeqCst) {
                        return;
                    }
                    yield_now().await;
                }
                panic!("the heartbeat ticked {MAX_TICKS} times, but the checksum never finished");
            })
        };
        run(vec![crunch, heartbeat]);

        let n = ITEMS as u64;
        assert_eq!(
            sum.load(Ordering::SeqCst),
            n * (n + 1) / 2,
            "the checksum itself must stay exact"
        );
        let ticks = ticks.lock().unwrap().clone();
        let while_running = ticks.iter().filter(|&&p| p < ITEMS).count();
        assert!(
            while_running >= ITEMS / EVERY - 1,
            "the heartbeat ticked {while_running} time(s) while the checksum ran \
             (progress at each tick: {ticks:?}); a yield every {EVERY} items \
             lets it tick {} times",
            ITEMS / EVERY - 1
        );
        let mut previous = 0;
        for &p in &ticks {
            assert!(
                p - previous <= EVERY,
                "{} items were summed between two heartbeat ticks (progress at \
                 each tick: {ticks:?}), but a batch is at most {EVERY} items",
                p - previous
            );
            previous = p;
        }
        assert!(
            ticks.len() <= ITEMS / EVERY + 2,
            "the heartbeat ticked {} times: yield once per {EVERY} items, not \
             more often (every yield is a trip through the executor)",
            ticks.len()
        );
    }
}
