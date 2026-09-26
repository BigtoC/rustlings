// Module 4 · Condition variables — part 2: a bounded queue needs two condvars.
//
// Part 1's queue never says no. When producers outrun consumers it just
// grows, until the process runs out of memory. A BOUNDED queue pushes back
// instead. `push` blocks while the queue is full, so a fast producer is slowed
// to the pace of its consumers: BACKPRESSURE. `try_push` never blocks; when
// the queue is full it hands the item back, and the caller decides what to do
// with it (retry later, drop it, answer "503, busy") instead of losing it.
// `std::sync::mpsc::sync_channel(n)` is this queue with a single consumer:
// `send` blocks, and `try_send` returns `Err(TrySendError::Full(item))`.
// "Implement a bounded blocking queue" is a classic live-coding task.
//
// There are now TWO conditions to wait for. Consumers wait for "not empty",
// producers wait for "not full", so there are two condvars. Why not one? A
// single condvar puts producers and consumers into one pool of sleepers, and
// `notify_one` wakes an arbitrary one of them. A pop that frees a slot may
// wake another CONSUMER, which re-checks, finds nothing it can use and goes
// back to sleep, while the producer that needed the slot is never told. With
// a few threads on each side, every thread can end up asleep with no
// notification left in flight: a deadlock made of lost wakeups. One condvar
// works only with `notify_all`, which wakes everybody on every change (the
// THUNDERING HERD). Two condvars let each change wake one thread that can
// use it: a push wakes one consumer, a pop wakes one producer.
//
// Part 1's loop rule holds on both sides. A producer woken by a pop can find
// the slot already taken by another producer (or a `try_push`) that got the
// lock first, so `push` re-checks the capacity after every wakeup, just as
// `pop` re-checks for an item. The `pop` below uses `wait_while`, the
// packaged form of that loop: it keeps waiting while its closure returns
// `true`.
//
// Interviewers ask: implement it, bounded and multi-producer multi-consumer.
// Why two condvars? When is `notify_all` the right call? (When one change can
// let several waiters proceed, or when waiters for different conditions share
// one condvar.) Why does `try_push` hand the item back instead of dropping
// it? What does a capacity of 0 mean? (A rendezvous, as in
// `sync_channel(0)`: every send waits for a receiver. This queue requires a
// capacity of at least 1.)

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

// A bounded multi-producer, multi-consumer FIFO queue.
struct BoundedQueue<T> {
    items: Mutex<VecDeque<T>>,
    capacity: usize,
    // Consumers sleep here while the queue is empty.
    not_empty: Condvar,
    // Producers sleep here while the queue is full.
    not_full: Condvar,
}

impl<T> BoundedQueue<T> {
    fn new(capacity: usize) -> Self {
        assert!(
            capacity > 0,
            "a bounded queue needs room for at least one item"
        );
        BoundedQueue {
            items: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn len(&self) -> usize {
        self.items.lock().unwrap().len()
    }

    // Adds `item` at the back if there is room. If the queue is full, it
    // hands `item` back as `Err(item)`. Never blocks.
    fn try_push(&self, item: T) -> Result<(), T> {
        let mut items = self.items.lock().unwrap();
        // Check and push under ONE lock. Checking with a separate `len()`
        // call first would be check-then-act: another producer could take
        // the last slot in between. Refusing hands the caller's own value
        // back, so nothing is lost and nothing needs to be `Clone`.
        if items.len() >= self.capacity {
            return Err(item);
        }
        items.push_back(item);
        drop(items);
        self.not_empty.notify_one();
        Ok(())
    }

    // Adds `item` at the back, blocking while the queue is full.
    fn push(&self, item: T) {
        let mut items = self.items.lock().unwrap();
        // Part 1's loop, on the other condvar. Every wakeup re-checks the
        // capacity under the lock, so a spurious wakeup, or a slot that
        // another producer took first, just sends this thread back to sleep.
        while items.len() >= self.capacity {
            items = self.not_full.wait(items).unwrap();
        }
        items.push_back(item);
        drop(items);
        self.not_empty.notify_one();
    }

    // Removes and returns the oldest item, blocking while the queue is empty.
    fn pop(&self) -> T {
        let items = self.items.lock().unwrap();
        let mut items = self
            .not_empty
            .wait_while(items, |items| items.is_empty())
            .unwrap();
        let item = items
            .pop_front()
            .expect("`wait_while` returns only once there is an item");
        // One slot was freed, so wake ONE producer, on `not_full`, where only
        // producers sleep. With a single shared condvar this `notify_one`
        // could wake a consumer instead and the producer would never hear
        // about the slot. EVERY pop notifies, not just one that found the
        // queue full: after two quick pops the second finds it no longer
        // full (the producer woken by the first has not pushed yet), and
        // skipping its notify leaves another producer asleep next to a free
        // slot. Unlock first, as in `push`.
        drop(items);
        self.not_full.notify_one();
        item
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

    // Only a broken queue ever gets near this deadline: a thread that should
    // have returned but is still blocked fails the test instead of hanging it.
    const PATIENCE: Duration = Duration::from_secs(20);

    // How long a test watches a thread that must STAY blocked. A correct
    // queue never lets it finish in this window. A wrong one is caught as
    // long as the thread gets any CPU time during it.
    const SETTLE: Duration = Duration::from_millis(250);

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

    // Keeps checking, for SETTLE, that `handle` is still blocked.
    fn assert_stays_blocked<R>(handle: &JoinHandle<R>, why: &str) {
        let start = Instant::now();
        while start.elapsed() < SETTLE {
            assert!(!handle.is_finished(), "{why}");
            thread::sleep(Duration::from_millis(1));
        }
    }

    // The length, read WITHOUT blocking: `None` while the lock is taken. A
    // `push` that keeps the lock while it waits would otherwise hang the test
    // instead of failing it.
    fn len_now<T>(q: &BoundedQueue<T>) -> Option<usize> {
        q.items.try_lock().ok().map(|items| items.len())
    }

    fn spawn_push<T: Send + 'static>(q: &Arc<BoundedQueue<T>>, item: T) -> JoinHandle<()> {
        let q = Arc::clone(q);
        thread::spawn(move || q.push(item))
    }

    fn spawn_pop<T: Send + 'static>(q: &Arc<BoundedQueue<T>>) -> JoinHandle<T> {
        let q = Arc::clone(q);
        thread::spawn(move || q.pop())
    }

    // Calls that must return at once run on a helper thread, so one that
    // blocks anyway fails the test instead of hanging it.
    fn push_now<T: Send + 'static>(q: &Arc<BoundedQueue<T>>, item: T) {
        join_within(
            spawn_push(q, item),
            "`push` returns at once when there is a free slot",
        );
    }

    fn try_push_now<T: Send + 'static>(q: &Arc<BoundedQueue<T>>, item: T) -> Result<(), T> {
        let q = Arc::clone(q);
        join_within(
            thread::spawn(move || q.try_push(item)),
            "`try_push` returns at once, full or not",
        )
    }

    fn pop_now<T: Send + 'static>(q: &Arc<BoundedQueue<T>>) -> T {
        join_within(spawn_pop(q), "`pop` returns an item that is already queued")
    }

    #[test]
    fn try_push_refuses_when_full_and_hands_the_item_back() {
        let q = Arc::new(BoundedQueue::new(2));
        assert_eq!(try_push_now(&q, 1), Ok(()));
        assert_eq!(try_push_now(&q, 2), Ok(()));
        assert_eq!(
            try_push_now(&q, 3),
            Err(3),
            "a full queue must refuse the item and hand it back"
        );
        assert_eq!(q.len(), 2, "a refused item must not be queued");
        assert_eq!(pop_now(&q), 1);
        assert_eq!(try_push_now(&q, 4), Ok(()), "a pop frees a slot");
        assert_eq!(try_push_now(&q, 5), Err(5));
        assert_eq!(pop_now(&q), 2);
        assert_eq!(pop_now(&q), 4);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn keeps_fifo_order_and_fills_up_to_exactly_the_capacity() {
        let q = Arc::new(BoundedQueue::new(3));
        // The third push fills the queue. It must not block: the queue is
        // full only AFTER it.
        for item in 1..=3 {
            push_now(&q, item);
        }
        assert_eq!(q.len(), 3);
        assert_eq!(try_push_now(&q, 99), Err(99));
        assert_eq!(pop_now(&q), 1);
        push_now(&q, 4);
        assert_eq!(pop_now(&q), 2);
        assert_eq!(try_push_now(&q, 5), Ok(()));
        let rest: Vec<u32> = (0..3).map(|_| pop_now(&q)).collect();
        assert_eq!(rest, [3, 4, 5]);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn push_blocks_while_full_and_one_pop_releases_it() {
        let q = Arc::new(BoundedQueue::new(2));
        push_now(&q, 1);
        push_now(&q, 2);
        let producer = spawn_push(&q, 3);
        assert_stays_blocked(&producer, "push did not block when the queue was full");
        assert_eq!(
            len_now(&q),
            Some(2),
            "a full queue must not grow, and a blocked `push` must not keep the lock"
        );
        assert_eq!(pop_now(&q), 1);
        join_within(
            producer,
            "the pop of 1 wakes the producer blocked in `push`",
        );
        assert_eq!(q.len(), 2);
        assert_eq!(pop_now(&q), 2);
        assert_eq!(pop_now(&q), 3);
    }

    #[test]
    fn a_wakeup_without_a_free_slot_sends_push_back_to_sleep() {
        let q = Arc::new(BoundedQueue::new(1));
        push_now(&q, 1);
        let producer = spawn_push(&q, 2);
        assert_stays_blocked(&producer, "push did not block when the queue was full");
        // No pop. To `push`, this is exactly what a spurious wakeup (or a
        // stolen one) looks like.
        q.not_full.notify_all();
        assert_stays_blocked(
            &producer,
            "`push` returned after a wakeup that freed no slot: check the \
             capacity again after EVERY wakeup",
        );
        assert_eq!(len_now(&q), Some(1), "a full queue must not grow");
        assert_eq!(pop_now(&q), 1);
        join_within(producer, "the pop of 1 wakes the producer");
        assert_eq!(pop_now(&q), 2);
    }

    #[test]
    fn one_pop_lets_in_one_of_two_blocked_producers() {
        let q = Arc::new(BoundedQueue::new(1));
        push_now(&q, 1);
        let mut producers = vec![spawn_push(&q, 2), spawn_push(&q, 3)];
        for p in &producers {
            assert_stays_blocked(p, "push did not block when the queue was full");
        }
        assert_eq!(pop_now(&q), 1);
        wait_until("the pop of 1 wakes one of the two producers", || {
            producers.iter().any(|p| p.is_finished())
        });
        let first = producers.remove(if producers[0].is_finished() { 0 } else { 1 });
        join_within(first, "the first producer finishes");
        assert_stays_blocked(
            &producers[0],
            "one pop freed ONE slot, but both producers pushed",
        );
        assert_eq!(len_now(&q), Some(1), "a full queue must not grow");
        let second_item = pop_now(&q);
        join_within(
            producers.remove(0),
            "the second pop wakes the other producer",
        );
        let mut items = [second_item, pop_now(&q)];
        items.sort_unstable();
        assert_eq!(items, [2, 3]);
    }

    #[test]
    fn two_quick_pops_let_in_both_blocked_producers() {
        let q = Arc::new(BoundedQueue::new(2));
        push_now(&q, 1);
        push_now(&q, 2);
        let producers = [spawn_push(&q, 3), spawn_push(&q, 4)];
        for p in &producers {
            assert_stays_blocked(p, "push did not block when the queue was full");
        }
        // Two pops back to back on one thread. The second one usually runs
        // before the producer woken by the first has refilled the queue, so
        // it frees a slot in a queue that is no longer full. It must still
        // wake the other producer.
        let popper = {
            let q = Arc::clone(&q);
            thread::spawn(move || [q.pop(), q.pop()])
        };
        assert_eq!(
            join_within(popper, "two pops from a full queue return at once"),
            [1, 2]
        );
        for p in producers {
            join_within(
                p,
                "each pop wakes one blocked producer (did a pop skip its \
                 notify because the queue was no longer full?)",
            );
        }
        let mut rest = [pop_now(&q), pop_now(&q)];
        rest.sort_unstable();
        assert_eq!(rest, [3, 4]);
    }

    #[test]
    fn try_push_wakes_a_waiting_consumer() {
        let q = Arc::new(BoundedQueue::new(2));
        let consumer = spawn_pop(&q);
        assert_stays_blocked(&consumer, "`pop` returned from an empty queue");
        assert_eq!(try_push_now(&q, 5), Ok(()));
        assert_eq!(
            join_within(consumer, "the `try_push` of 5 wakes the waiting consumer"),
            5
        );
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn four_producers_and_four_consumers_through_a_single_slot() {
        const PER_THREAD: usize = 500;
        let q = Arc::new(BoundedQueue::new(1));
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
        // Watch the queue while the threads run: it must never hold more
        // than its single slot.
        wait_until(
            "every producer and consumer finishes (a lost wakeup?)",
            || {
                if let Some(len) = len_now(&q) {
                    assert!(len <= 1, "a queue of capacity 1 held {len} items");
                }
                producers.iter().all(|p| p.is_finished())
                    && consumers.iter().all(|c| c.is_finished())
            },
        );
        for p in producers {
            join_within(p, "a producer finishes");
        }
        let mut seen = HashSet::new();
        for c in consumers {
            // The queue is FIFO and every producer pushes in order, so the
            // items one consumer got from one producer are in order too.
            let mut last = [None; 4];
            for (p, i) in join_within(c, "a consumer finishes") {
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
    }
}
