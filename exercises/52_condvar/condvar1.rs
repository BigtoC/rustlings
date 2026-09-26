// Module 4 · Condition variables — part 1: a blocking queue, and why `wait` goes in a loop.
//
// A `Mutex` answers "may I touch the data now?". It cannot answer "wake me
// when the data looks the way I need". A consumer that finds the queue empty
// must not spin on `lock()` (that burns a core and keeps taking the lock away
// from the producer it is waiting for), and it must not `sleep` and retry
// (that adds latency, and still wakes up for nothing). A CONDITION VARIABLE
// is the missing piece: a place to sleep until another thread says "the state
// changed, look again". The protocol has one shape, and interviewers expect
// it exactly:
//
//   - The condition ("the queue has an item") lives in the data the MUTEX
//     protects. It is only checked or changed with the lock held.
//   - To block, call `cv.wait(guard)`. It unlocks the mutex and puts the
//     thread to sleep in ONE atomic step, so a notify sent after you checked
//     cannot slip in before you are asleep. When `wait` returns, it has
//     locked the mutex again and hands you a new guard.
//   - Whoever changes the condition does so under the lock, then calls
//     `notify_one` (wake one sleeper) or `notify_all` (wake them all).
//
// `wait` takes the guard BY VALUE because it has to unlock the mutex. While
// you sleep there is no guard, so the borrow checker rejects any reference
// into the data that you tried to keep across the call (E0505 "cannot move
// out of `guard` because it is borrowed"). In C you can keep a pointer into
// data that another thread is busy changing. In Rust you can't.
//
// Why a loop? Returning from `wait` means "look again", never "your condition
// is true now":
//
//   - SPURIOUS wakeups. The docs say `wait` "is susceptible to spurious
//     wakeups". POSIX allows `pthread_cond_wait` to return without any
//     notify, and std's futex-based condvar (Linux, Windows) can return in
//     several threads for a single `notify_one`.
//   - STOLEN wakeups. Between the notify and the moment the woken thread has
//     the lock back, another consumer can get the lock first and take the
//     item.
//   - `notify_all` wakes every waiter, perhaps for a single item.
//
// So the check is a `while`, never an `if`. `Condvar::wait_while(guard, cond)`
// is the same loop, packaged.
//
// `waiters` makes the protocol visible. The `push` below skips the notify when
// nobody is waiting (on Linux, std's `notify_one` is a system call every
// time), so `pop` must count itself in before it sleeps. Because that count
// decides whether anyone gets woken, it is part of the condition and must be
// changed under the same lock. A consumer that fell asleep uncounted would
// never be woken: a LOST WAKEUP. A condvar has no memory, and a notify sent
// while nobody sleeps simply disappears. That is why the state in the mutex,
// not the notification, carries the information.
//
// Interviewers ask: why must `wait` be in a loop? What are spurious and lost
// wakeups? Why does `wait` take the guard? `notify_one` or `notify_all`? And
// notify while holding the lock, or after? Both are correct here. Notifying
// after unlocking, as `push` does, spares the woken thread from blocking on
// the mutex straight away.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

struct Inner<T> {
    items: VecDeque<T>,
    // How many threads are inside `pop`, blocked until an item arrives.
    waiters: usize,
}

// An unbounded multi-producer, multi-consumer FIFO queue whose `pop` blocks.
struct BlockingQueue<T> {
    inner: Mutex<Inner<T>>,
    // Notified by `push` when an item arrives and someone is waiting for it.
    not_empty: Condvar,
}

impl<T> BlockingQueue<T> {
    fn new() -> Self {
        BlockingQueue {
            inner: Mutex::new(Inner {
                items: VecDeque::new(),
                waiters: 0,
            }),
            not_empty: Condvar::new(),
        }
    }

    fn len(&self) -> usize {
        self.inner.lock().unwrap().items.len()
    }

    fn waiters(&self) -> usize {
        self.inner.lock().unwrap().waiters
    }

    // Adds `item` at the back and wakes one sleeping consumer, if there is one.
    fn push(&self, item: T) {
        let mut inner = self.inner.lock().unwrap();
        inner.items.push_back(item);
        // Read under the lock. A consumer counts itself in under this same
        // lock before it sleeps, so either it is counted here, or it has not
        // looked at the queue yet and will find `item` without sleeping.
        let someone_is_waiting = inner.waiters > 0;
        // Unlock first, then notify: the woken consumer does not have to wait
        // for this thread to let go of the mutex.
        drop(inner);
        if someone_is_waiting {
            self.not_empty.notify_one();
        }
    }

    // Removes and returns the oldest item, blocking while the queue is empty.
    fn pop(&self) -> T {
        // TODO: `pop` has no body yet, so rustc rejects it with E0308
        // "mismatched types": it must return a `T`, and an empty body is
        // `()`. Implement it:
        //   - return the OLDEST item. While the queue is empty, block on
        //     `not_empty` until there is an item to take: no busy-waiting,
        //     no sleeping in a retry loop;
        //   - a wakeup is only a hint. Whatever woke you (a push, a push
        //     that another consumer beat you to, a `notify_all`, or nothing
        //     at all), take an item only if one is really there; otherwise
        //     go back to sleep;
        //   - while you are blocked, count yourself in `waiters` (`push`
        //     notifies only when it is above 0), and take yourself out again
        //     before you return. The count must be right at every moment the
        //     lock is free: exactly the threads blocked in `pop`;
        //   - FIFO order; keep `push`, the fields and the tests as they are;
        //     no `unsafe`, no extra locks or atomics.
        // Until `pop` returns a `T`, this exercise will not compile.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    // Only a broken queue ever gets near this deadline: a `pop` that should
    // have returned but is still blocked fails the test instead of hanging it.
    const PATIENCE: Duration = Duration::from_secs(20);

    // How long a test watches a consumer that must STAY asleep. A correct
    // `pop` never finishes in this window. A wrong one is caught as long as
    // it gets any CPU time during it.
    const SETTLE: Duration = Duration::from_millis(250);

    // Deliberately NOT `Clone`: the queue moves items, it never copies them.
    #[derive(Debug, PartialEq)]
    struct Job(u32);

    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(
                start.elapsed() < PATIENCE,
                "gave up after {PATIENCE:?} waiting until {what}"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    // Joins `handle` if it finishes within PATIENCE, re-raising its panic.
    fn join_within<R>(handle: JoinHandle<R>, what: &str) -> R {
        wait_until(what, || handle.is_finished());
        match handle.join() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn spawn_pop<T: Send + 'static>(q: &Arc<BlockingQueue<T>>) -> JoinHandle<T> {
        let q = Arc::clone(q);
        thread::spawn(move || q.pop())
    }

    // A `pop` with an item in the queue must return at once. It runs on a
    // helper thread, so a `pop` that blocks anyway fails instead of hanging.
    fn pop_now<T: Send + 'static>(q: &Arc<BlockingQueue<T>>) -> T {
        join_within(spawn_pop(q), "`pop` returns an item that is already queued")
    }

    // The waiter count, read WITHOUT blocking: `None` while the lock is
    // taken. A `pop` that keeps the lock while it waits would otherwise hang
    // the test instead of failing it.
    fn waiters_now<T>(q: &BlockingQueue<T>) -> Option<usize> {
        q.inner.try_lock().ok().map(|inner| inner.waiters)
    }

    // Spawns one consumer and waits until it is asleep in `pop`.
    fn blocked_consumer(q: &Arc<BlockingQueue<u32>>) -> JoinHandle<u32> {
        let consumer = spawn_pop(q);
        wait_until(
            "the consumer sleeps in `pop`: counted in `waiters`, lock released",
            || {
                assert!(
                    !consumer.is_finished(),
                    "`pop` returned from an empty queue"
                );
                waiters_now(q) == Some(1)
            },
        );
        consumer
    }

    // Keeps checking, for SETTLE, that `consumer` is still blocked.
    fn assert_stays_blocked<R>(consumer: &JoinHandle<R>, why: &str) {
        let start = Instant::now();
        while start.elapsed() < SETTLE {
            assert!(!consumer.is_finished(), "{why}");
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn pops_in_fifo_order() {
        let q = Arc::new(BlockingQueue::new());
        for id in 1..=3 {
            q.push(Job(id));
        }
        assert_eq!(q.len(), 3);
        assert_eq!(pop_now(&q), Job(1));
        assert_eq!(pop_now(&q), Job(2));
        q.push(Job(4));
        assert_eq!(pop_now(&q), Job(3));
        assert_eq!(pop_now(&q), Job(4));
        assert_eq!(q.len(), 0);
        assert_eq!(
            q.waiters(),
            0,
            "a `pop` that found an item never waited, so it must not stay counted"
        );
    }

    #[test]
    fn pop_blocks_on_an_empty_queue_until_a_push() {
        let q = Arc::new(BlockingQueue::new());
        let consumer = blocked_consumer(&q);
        q.push(7);
        assert_eq!(join_within(consumer, "the push of 7 wakes the consumer"), 7);
        assert_eq!(
            q.waiters(),
            0,
            "a consumer that got its item is not waiting"
        );
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn a_wakeup_without_an_item_sends_pop_back_to_sleep() {
        let q = Arc::new(BlockingQueue::new());
        let consumer = blocked_consumer(&q);
        // No push. To `pop`, this is exactly what a spurious wakeup (or a
        // stolen one) looks like.
        q.not_empty.notify_all();
        assert_stays_blocked(
            &consumer,
            "`pop` returned (or panicked) after a wakeup that brought no item: \
             check the queue again after EVERY wakeup",
        );
        assert_eq!(
            waiters_now(&q),
            Some(1),
            "the consumer is asleep again, so it must still be counted \
             (otherwise the next `push` will not wake it), and the lock free"
        );
        q.push(7);
        assert_eq!(join_within(consumer, "the push of 7 wakes the consumer"), 7);
        assert_eq!(q.waiters(), 0);
    }

    #[test]
    fn every_sleeping_consumer_is_woken() {
        let q = Arc::new(BlockingQueue::new());
        let consumers: Vec<_> = (0..3).map(|_| spawn_pop(&q)).collect();
        wait_until(
            "all three consumers sleep in `pop` (waiters == 3, lock released)",
            || waiters_now(&q) == Some(3),
        );
        for item in [10, 20, 30] {
            q.push(item);
        }
        let mut got: Vec<u32> = consumers
            .into_iter()
            .map(|c| join_within(c, "each of the three pushes wakes a consumer"))
            .collect();
        got.sort_unstable();
        assert_eq!(got, [10, 20, 30]);
        assert_eq!(q.waiters(), 0);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn four_producers_and_four_consumers_share_every_item() {
        const PER_THREAD: usize = 1_000;
        let q = Arc::new(BlockingQueue::new());
        // Consumers first, so that many of the pops find the queue empty.
        let consumers: Vec<JoinHandle<Vec<(usize, usize)>>> = (0..4)
            .map(|_| {
                let q = Arc::clone(&q);
                thread::spawn(move || (0..PER_THREAD).map(|_| q.pop()).collect())
            })
            .collect();
        let producers: Vec<JoinHandle<()>> = (0..4)
            .map(|p| {
                let q = Arc::clone(&q);
                thread::spawn(move || (0..PER_THREAD).for_each(|i| q.push((p, i))))
            })
            .collect();
        for p in producers {
            join_within(p, "a producer finishes (`push` never blocks)");
        }
        let mut seen = HashSet::new();
        for c in consumers {
            let got = join_within(c, "a consumer gets all of its items (a lost wakeup?)");
            // The queue is FIFO and every producer pushes in order, so the
            // items one consumer got from one producer are in order too.
            let mut last = [None; 4];
            for (p, i) in got {
                assert!(
                    last[p] < Some(i),
                    "item {i} of producer {p} came after item {:?}",
                    last[p]
                );
                last[p] = Some(i);
                assert!(seen.insert((p, i)), "item ({p}, {i}) was popped twice");
            }
        }
        assert_eq!(seen.len(), 4 * PER_THREAD, "every item is popped once");
        assert_eq!(q.len(), 0);
        assert_eq!(q.waiters(), 0);
    }
}
