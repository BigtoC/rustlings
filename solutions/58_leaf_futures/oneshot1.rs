// Module 3 · Leaf futures — part 1: a oneshot channel whose receiver stores the latest waker.
//
// A LEAF future is one that is not built out of other futures: it is where a
// `Pending` comes from in the first place. Every leaf in `28_futures` and
// `29_async_runtime` woke itself (`cx.waker().wake_by_ref()` right before
// returning `Pending`), which is just a polite busy loop: the executor polls
// the task again at once, whether or not anything changed. Real leaves
// (sockets, timers, channels, locks) return `Pending` and hand the waker to
// SOMEONE ELSE: a reactor, a timer thread, or the other half of a channel.
// That someone calls `wake()` when the event actually happens, and only then
// does the executor poll the task again.
//
// That hand-off has three rules, and this oneshot channel needs all of them:
//
//   1. The latest waker wins. Every `poll` gets a `Context`, and nothing
//      promises that its waker is the one you got last time: the receiver may
//      have been moved into another task (you can send it to another thread
//      and await it there), and combinators such as `FuturesUnordered` give
//      each child a waker of its own. The `Future::poll` docs say it
//      outright: only the waker passed to the most recent call should be
//      woken. A waker saved by an EARLIER poll can wake the wrong task, or
//      one that has already finished. So every poll that returns `Pending`
//      must leave its own waker behind, replacing the old one.
//      `Waker::will_wake` tells you when the stored one already wakes the
//      same task, so you can skip the clone (`Waker::clone_from` does that
//      check for you).
//   2. Check and register atomically. If you look for the value under the lock,
//      release it, and only then store the waker, the sender can deliver in
//      between: it finds no waker, wakes nobody, and your task sleeps forever.
//      That is a LOST WAKEUP, the classic leaf-future bug. Look for the value
//      and store the waker under the SAME lock the sender takes.
//   3. The other side can go away. If the sender is dropped without sending,
//      nobody will ever wake the receiver again, so the sender's `Drop` wakes
//      it one last time and the receiver must answer `Err(Canceled)` instead
//      of `Pending`. A value sent before the sender was dropped still wins.
//
// The sender side below is complete. Notice that it takes the waker out while
// it holds the lock but calls `wake()` only after releasing it: the woken task
// may be polled on another thread at once, and its first move is to take the
// same lock.
//
// `tokio::sync::oneshot` and `futures::channel::oneshot` follow the same three
// rules. They replace the `Mutex` with atomic state, so they cannot check and
// register in one step: they register the waker first and then check for the
// value again, which closes the same window.
//
// Interviewers ask you to write this channel, then ask why `poll` must store
// the waker on EVERY call, what a lost wakeup looks like, and what the
// receiver sees when the sender is dropped (tokio: `RecvError`; futures:
// `Canceled`).

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// The sender was dropped without sending a value.
#[derive(Debug, PartialEq, Eq)]
struct Canceled;

/// The state both halves share. One lock guards all of it.
struct Shared<T> {
    value: Option<T>,
    waker: Option<Waker>,
    sender_alive: bool,
}

struct Sender<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

struct Receiver<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Mutex::new(Shared {
        value: None,
        waker: None,
        sender_alive: true,
    }));
    (
        Sender {
            shared: Arc::clone(&shared),
        },
        Receiver { shared },
    )
}

impl<T> Sender<T> {
    /// Stores the value and wakes the receiver's latest waker. Consumes the
    /// sender: a oneshot sends at most once.
    fn send(self, value: T) {
        let waker = {
            let mut shared = self.shared.lock().unwrap();
            shared.value = Some(value);
            shared.waker.take()
        };
        // Wake with the lock released (see the header).
        if let Some(waker) = waker {
            waker.wake();
        }
        // `self` is dropped here. `Drop` finds no waker left, so it wakes
        // nobody a second time.
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let waker = {
            let mut shared = self.shared.lock().unwrap();
            shared.sender_alive = false;
            shared.waker.take()
        };
        // The receiver may be waiting for a value that will never come now:
        // wake it so it can see that the sender is gone.
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, Canceled>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = self.shared.lock().unwrap();
        if let Some(value) = shared.value.take() {
            return Poll::Ready(Ok(value));
        }
        // The value is checked first, so "send, then drop the sender" is a
        // success. Only with no value AND no sender is the wait hopeless.
        if !shared.sender_alive {
            return Poll::Ready(Err(Canceled));
        }
        // Still under the same guard as the checks above: the sender cannot
        // deliver between our "no value yet" and the registration, so its
        // `take()` is guaranteed to find this waker. The waker of THIS poll
        // replaces any older one; `clone_from` skips the clone when the stored
        // waker already `will_wake` the same task.
        match &mut shared.waker {
            Some(stored) => stored.clone_from(cx.waker()),
            slot => *slot = Some(cx.waker().clone()),
        }
        Poll::Pending
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::task::Wake;
    use std::thread::{self, JoinHandle, Thread};
    use std::time::{Duration, Instant};

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

    fn counting_waker() -> (Arc<CountingWaker>, Waker) {
        let counter = Arc::new(CountingWaker {
            wakes: AtomicUsize::new(0),
        });
        (Arc::clone(&counter), Waker::from(counter))
    }

    fn wakes(counter: &CountingWaker) -> usize {
        counter.wakes.load(Ordering::SeqCst)
    }

    fn poll_with<T>(rx: &mut Receiver<T>, waker: &Waker) -> Poll<Result<T, Canceled>> {
        Pin::new(rx).poll(&mut Context::from_waker(waker))
    }

    // `runtime2`'s `block_on`, changed so that a lost wakeup FAILS the test
    // instead of hanging it. The waker sets a flag before it unparks, and
    // `sender` is the thread that holds the other half of the channel. Both
    // `send` and the sender's `Drop` wake the receiver BEFORE that thread
    // finishes. So once it has finished, a future that is still `Pending`
    // and has not been woken never will be, and we panic at once.
    // `STUCK_AFTER` only bounds a sender thread that never finishes.
    const STUCK_AFTER: Duration = Duration::from_secs(30);

    struct ThreadWaker {
        thread: Thread,
        woken: AtomicBool,
    }

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.woken.store(true, Ordering::SeqCst);
            self.thread.unpark();
        }
    }

    fn block_on<F: Future>(future: F, sender: JoinHandle<()>) -> F::Output {
        let signal = Arc::new(ThreadWaker {
            thread: thread::current(),
            woken: AtomicBool::new(false),
        });
        let waker = Waker::from(Arc::clone(&signal));
        let mut cx = Context::from_waker(&waker);
        let mut future = pin!(future);
        let mut sender = Some(sender);
        let give_up = Instant::now() + STUCK_AFTER;
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                if let Some(sender) = sender {
                    sender.join().unwrap();
                }
                return value;
            }
            // `park_timeout` may return early for no reason, so wait for the
            // flag, not for the unpark.
            while !signal.woken.swap(false, Ordering::SeqCst) {
                let Some(running) = &sender else {
                    panic!(
                        "the receiver is still Pending, but the sender thread \
                         has finished: nothing will ever wake it (a lost \
                         wake-up, or a dropped sender that went unnoticed)"
                    );
                };
                if running.is_finished() {
                    // `join` makes everything that thread did visible here,
                    // including a wake-up, so check the flag once more.
                    sender.take().unwrap().join().unwrap();
                    continue;
                }
                assert!(
                    Instant::now() < give_up,
                    "the sender thread did not finish within {STUCK_AFTER:?}"
                );
                thread::park_timeout(Duration::from_millis(1));
            }
        }
    }

    // Polls the inner future once and only then lets the sender thread go.
    // So the value always arrives AFTER the receiver's first poll returned
    // `Pending`: the cross-thread wake path really runs.
    struct PollThenSignal<F> {
        inner: F,
        go: Option<mpsc::Sender<()>>,
    }

    impl<F: Future + Unpin> Future for PollThenSignal<F> {
        type Output = F::Output;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
            let result = Pin::new(&mut self.inner).poll(cx);
            if let Some(go) = self.go.take() {
                go.send(()).unwrap();
            }
            result
        }
    }

    #[test]
    fn a_value_sent_before_the_first_poll_is_ready_at_once() {
        let (tx, mut rx) = channel();
        tx.send(String::from("early"));
        let (counter, waker) = counting_waker();
        assert_eq!(
            poll_with(&mut rx, &waker),
            Poll::Ready(Ok(String::from("early")))
        );
        assert_eq!(wakes(&counter), 0, "a Ready poll has nothing to wake");
    }

    #[test]
    fn send_wakes_the_latest_waker_and_only_that_one() {
        let (tx, mut rx) = channel();
        let (a, waker_a) = counting_waker();
        let (b, waker_b) = counting_waker();

        // Polled with waker A, then again with waker B (say, the receiver was
        // moved into another task, or a combinator now polls it).
        assert_eq!(poll_with(&mut rx, &waker_a), Poll::Pending);
        assert_eq!(poll_with(&mut rx, &waker_b), Poll::Pending);
        assert_eq!(
            (wakes(&a), wakes(&b)),
            (0, 0),
            "nothing may be woken before there is a value (no busy-polling)"
        );

        tx.send(42);
        assert_eq!(
            wakes(&b),
            1,
            "the send must wake the waker of the LATEST poll exactly once"
        );
        assert_eq!(wakes(&a), 0, "waker A is stale: it must not be woken");
        assert_eq!(poll_with(&mut rx, &waker_b), Poll::Ready(Ok(42)));
    }

    #[test]
    fn repolling_with_the_same_waker_wakes_it_once() {
        let (tx, mut rx) = channel();
        let (a, waker_a) = counting_waker();
        for _ in 0..3 {
            assert_eq!(poll_with(&mut rx, &waker_a), Poll::Pending);
        }
        tx.send('x');
        assert_eq!(
            wakes(&a),
            1,
            "one send, one wake, however often the task was polled before"
        );
        assert_eq!(poll_with(&mut rx, &waker_a), Poll::Ready(Ok('x')));
    }

    #[test]
    fn dropping_the_sender_wakes_the_receiver_with_canceled() {
        let (tx, mut rx) = channel::<String>();
        let (a, waker_a) = counting_waker();
        assert_eq!(poll_with(&mut rx, &waker_a), Poll::Pending);

        drop(tx);
        assert_eq!(
            wakes(&a),
            1,
            "a dropped sender must wake the waiting receiver, or it waits forever"
        );
        assert_eq!(poll_with(&mut rx, &waker_a), Poll::Ready(Err(Canceled)));
    }

    #[test]
    fn a_sender_dropped_before_the_first_poll_is_canceled() {
        let (tx, mut rx) = channel::<u8>();
        drop(tx);
        let (a, waker_a) = counting_waker();
        assert_eq!(poll_with(&mut rx, &waker_a), Poll::Ready(Err(Canceled)));
        assert_eq!(wakes(&a), 0);
    }

    #[test]
    fn a_value_sent_before_the_sender_dropped_still_wins() {
        // `send` consumes the sender, so it is gone by the time we poll. The
        // value must win over `sender_alive == false`.
        let (tx, mut rx) = channel();
        tx.send(vec![1, 2, 3]);
        assert_eq!(
            poll_with(&mut rx, Waker::noop()),
            Poll::Ready(Ok(vec![1, 2, 3]))
        );
    }

    #[test]
    fn a_value_sent_from_another_thread_arrives_through_block_on() {
        let (tx, rx) = channel();
        let (go_tx, go_rx) = mpsc::channel();
        let sender = thread::spawn(move || {
            go_rx.recv().unwrap();
            tx.send(String::from("from another thread"));
        });
        let received = block_on(
            PollThenSignal {
                inner: rx,
                go: Some(go_tx),
            },
            sender,
        );
        assert_eq!(received, Ok(String::from("from another thread")));
    }

    #[test]
    fn a_sender_dropped_on_another_thread_ends_block_on_with_canceled() {
        let (tx, rx) = channel::<String>();
        let (go_tx, go_rx) = mpsc::channel();
        let sender = thread::spawn(move || {
            go_rx.recv().unwrap();
            drop(tx);
        });
        let received = block_on(
            PollThenSignal {
                inner: rx,
                go: Some(go_tx),
            },
            sender,
        );
        assert_eq!(received, Err(Canceled));
    }

    // Raises `go` right before it polls the receiver, so that a sender
    // thread spinning on `go` delivers WHILE the receiver's `poll` runs.
    struct SignalThenPoll<F> {
        inner: F,
        go: Arc<AtomicBool>,
    }

    impl<F: Future + Unpin> Future for SignalThenPoll<F> {
        type Output = F::Output;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
            self.go.store(true, Ordering::SeqCst);
            Pin::new(&mut self.inner).poll(cx)
        }
    }

    // Busy-waits for `flag`. A blocking wait takes far longer to wake up than
    // a `poll` takes to run, so the delivery would never land inside one.
    // After a while it yields the OS thread instead of burning it.
    fn spin_until(flag: &AtomicBool) {
        let mut spins = 0_u32;
        while !flag.load(Ordering::SeqCst) {
            if spins < 100_000 {
                spins += 1;
                std::hint::spin_loop();
            } else {
                thread::yield_now();
            }
        }
    }

    #[test]
    fn many_cross_thread_handoffs_never_lose_a_wakeup() {
        // The sender thread delivers while the receiver's first poll is
        // running, a few spins later from round to round, so the send (or
        // the drop) lands before, inside and after the receiver's checks. A
        // receiver that checks and registers under separate `lock()` calls
        // loses wake-ups here (or reports `Canceled` for a value that was
        // sent), usually dozens of times in 500 rounds. One that holds a
        // single guard never does.
        for i in 0..500_u32 {
            let (tx, rx) = channel();
            let ready = Arc::new(AtomicBool::new(false));
            let go = Arc::new(AtomicBool::new(false));
            let sender = thread::spawn({
                let (ready, go) = (Arc::clone(&ready), Arc::clone(&go));
                move || {
                    ready.store(true, Ordering::SeqCst);
                    spin_until(&go);
                    for _ in 0..(i / 4) % 8 {
                        std::hint::spin_loop();
                    }
                    if i % 4 == 0 {
                        drop(tx);
                    } else {
                        tx.send(i);
                    }
                }
            });
            // Start polling only once the sender thread is up and spinning.
            spin_until(&ready);
            let expected = if i % 4 == 0 { Err(Canceled) } else { Ok(i) };
            let rx = SignalThenPoll { inner: rx, go };
            assert_eq!(block_on(rx, sender), expected, "hand-off {i}");
        }
    }
}
