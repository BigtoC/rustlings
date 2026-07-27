// Module 3 · Async runtime — part 3: a multi-task executor.
//
// A real executor juggles many tasks at once. Each spawned future becomes a
// `Task`. Tasks wait on a ready-queue (here: an `mpsc` channel). The executor
// pops a task, polls it once, and moves on. The clever part is the `Waker`:
// waking a task simply pushes it back onto the ready-queue, so a task that
// returned `Pending` gets polled again only once it can make progress.
//
// The other half of that bookkeeping is knowing when a task is DONE: a `Waker`
// may fire after its future has completed, and a completed future must never be
// polled again (an `async` block panics if you try). So the executor drops the
// future as soon as it returns `Ready`.
//
// This is exactly the design of tokio's current-thread runtime, minus the I/O
// reactor and the timer wheel.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::task::{Context, Poll, Wake, Waker};

// A type-erased, heap-pinned future. `Pin<Box<...>>` lets us store futures of
// different concrete types together and move the handle around freely.
type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Task {
    // `None` once the future has completed. A `Waker` is allowed to fire after
    // its task finished, but polling a completed future is NOT allowed — an
    // `async` block panics with "`async fn` resumed after completion". So the
    // executor drops the future the moment it returns `Ready`, and a later
    // wake-up for that task then finds an empty slot and does nothing.
    future: Mutex<Option<BoxFuture>>,
    // Cloning the task and sending it here re-schedules it.
    ready_queue: Sender<Arc<Task>>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        // Re-schedule this task by sending an `Arc` clone back to the executor.
        // (Ignore the error: it only fails if the executor is already gone.)
        let _ = self.ready_queue.send(self.clone());
    }
}

struct Executor {
    ready_queue: Receiver<Arc<Task>>,
    spawner: Sender<Arc<Task>>,
}

impl Executor {
    fn new() -> Self {
        let (spawner, ready_queue) = channel();
        Self {
            ready_queue,
            spawner,
        }
    }

    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task {
            future: Mutex::new(Some(Box::pin(future))),
            ready_queue: self.spawner.clone(),
        });
        let _ = self.spawner.send(task);
    }

    // Run until the ready-queue is empty.
    fn run(&self) {
        while let Ok(task) = self.ready_queue.try_recv() {
            let mut slot = task.future.lock().unwrap();
            // An empty slot means the task already completed and this is a stale
            // wake-up: there is nothing left to poll.
            let Some(future) = slot.as_mut() else {
                continue;
            };
            // The task's own `Waker` re-enqueues it, so polling once per turn is
            // enough: a `Pending` task comes back on its own when ready.
            let waker = Waker::from(task.clone());
            let mut cx = Context::from_waker(&waker);
            if future.as_mut().poll(&mut cx).is_ready() {
                // Finished. Drop the future so a re-queued wake-up cannot resume
                // it, and so its captured state is released promptly.
                *slot = None;
            }
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // A leaf future that yields control exactly once. Spawning it exercises the
    // wake / re-schedule path (poll -> Pending -> re-queued -> poll -> Ready).
    struct YieldNow(bool);

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    #[test]
    fn runs_all_spawned_tasks() {
        let counter = Arc::new(AtomicU32::new(0));
        let executor = Executor::new();

        // Two tasks that finish immediately...
        for _ in 0..2 {
            let counter = counter.clone();
            executor.spawn(async move {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
        // ...and one that yields (Pending) before finishing.
        {
            let counter = counter.clone();
            executor.spawn(async move {
                YieldNow(false).await;
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        executor.run();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    // A future that wakes itself and then finishes in the SAME poll, so the
    // executor is left holding a wake-up for a task that is already done.
    struct WakeThenFinish;

    impl Future for WakeThenFinish {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            cx.waker().wake_by_ref();
            Poll::Ready(())
        }
    }

    #[test]
    fn a_stale_wakeup_does_not_repoll_a_finished_task() {
        let counter = Arc::new(AtomicU32::new(0));
        let executor = Executor::new();

        let counter_clone = counter.clone();
        executor.spawn(async move {
            WakeThenFinish.await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Without the completed-task check in `run`, the re-queued task would be
        // polled a second time and the finished `async` block would panic with
        // "`async fn` resumed after completion".
        executor.run();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
