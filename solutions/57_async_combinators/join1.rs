// Module 3 · Async combinators — part 1: a hand-written `join` that polls both children on every poll.
//
// `28_futures/futures3` ran two futures one AFTER the other, which is what
// `a.await; b.await` compiles to: `b` does not even start until `a` has
// finished. `join!(a, b)` (from `futures` or `tokio`) runs them CONCURRENTLY
// instead. It is still ONE future, polled by one task on one thread, so
// nothing runs in parallel: each time the task is polled, `join` polls every
// child that has not finished yet, and whichever child can make progress
// does. Two requests that each wait 100 ms take about 100 ms when joined and
// 200 ms in sequence.
//
// Three rules make a correct `join`:
//
//   1. Poll EVERY unfinished child on every poll. Returning `Pending` as soon
//      as the first child is `Pending` is the sequential version again, and
//      when the children depend on each other it is worse than slow: a
//      producer that waits for room in a channel and the consumer that would
//      make the room deadlock if the consumer is never polled.
//   2. Never poll a child again after it returned `Ready`. The `Future` docs
//      say that polling a completed future "may panic, block forever, or
//      cause other kinds of problems". An `async` block panics with "`async
//      fn` resumed after completion", `std::future::Ready` with "`Ready`
//      polled after completion". So store each child's output the moment you
//      get it, and let the stored output tell you which child is done.
//   3. Pass the caller's `Context` down. The children register the TASK's
//      waker, so when any of them fires, the whole `join` is polled again,
//      and it polls every unfinished child, woken or not. (So a join that
//      loops over 10_000 children costs O(n) per wakeup. `FuturesUnordered`
//      hands each child a waker of its own that also wakes the task, and
//      polls only the woken children. `futures::future::join_all` switches
//      to `FuturesOrdered`, which is built on it, for large inputs.)
//
// The poll count follows from rule 1. A child that returns `Pending` n times
// is `Ready` on its (n + 1)-th poll, so the join is `Ready` on outer poll
// `max(n_a, n_b) + 1`. The sequential version needs `n_a + 1 + n_b`.
//
// This `Join` accepts only `Unpin` children, so the safe `Pin::new(&mut ..)`
// can poll them. A `!Unpin` future such as an `async` block becomes `Unpin`
// once it is boxed with `Box::pin`. Joining `!Unpin` children in place needs
// pin projection (`pin-project-lite`, or what `futures::join!` does inside),
// which this module sidesteps.
//
// How interviewers probe this: "What is the difference between `join!(a, b)`
// and `a.await; b.await`?", "Does `join!` run things in parallel? When would
// you `spawn` instead?", "What happens if you poll a future after it returned
// `Ready`?", and "Implement `join` by hand."

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Polls `a` and `b` concurrently. Resolves to both outputs, in argument order,
// once BOTH children have finished.
struct Join<A: Future, B: Future> {
    a: A,
    b: B,
    // `Some` once that child has returned `Ready`.
    a_out: Option<A::Output>,
    b_out: Option<B::Output>,
}

fn join<A: Future, B: Future>(a: A, b: B) -> Join<A, B> {
    Join {
        a,
        b,
        a_out: None,
        b_out: None,
    }
}

// `Join` is `Unpin` whenever its children are. Implementing `Unpin` is safe
// code, and it is sound here because `Join` never pins its outputs: it only
// stores them and moves them out (the `std::pin` docs call this choosing
// pinning NOT to be structural for a field). Without this impl, `Join` would
// be `Unpin` only if the OUTPUT types were `Unpin` too, and `self.get_mut()`
// below would not compile for generic outputs.
impl<A: Future + Unpin, B: Future + Unpin> Unpin for Join<A, B> {}

impl<A: Future + Unpin, B: Future + Unpin> Future for Join<A, B> {
    type Output = (A::Output, B::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `Join` is `Unpin`, so a plain `&mut Join` is available, and a child
        // can be polled with `Pin::new(&mut this.a).poll(cx)`.
        let this = self.get_mut();

        // Poll only the children that have not finished: a stored output means
        // that child already returned `Ready` and must not be polled again.
        // Both are polled on every call, with the caller's `cx`, so whichever
        // child's event fires, the task's waker brings us back here and the
        // other child gets its turn too.
        if this.a_out.is_none()
            && let Poll::Ready(output) = Pin::new(&mut this.a).poll(cx)
        {
            this.a_out = Some(output);
        }
        if this.b_out.is_none()
            && let Poll::Ready(output) = Pin::new(&mut this.b).poll(cx)
        {
            this.b_out = Some(output);
        }
        // Done only when both outputs are in. Otherwise put back whatever is
        // there and wait for the next wakeup.
        match (this.a_out.take(), this.b_out.take()) {
            (Some(a), Some(b)) => Poll::Ready((a, b)),
            (a, b) => {
                this.a_out = a;
                this.b_out = b;
                Poll::Pending
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
    use std::cell::Cell;
    use std::future::{poll_fn, ready};
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::task::{Wake, Waker};

    // A leaf future that returns `Pending` `pendings` times and then
    // `Ready("<name> done")`, so it is `Ready` on its (pendings + 1)-th poll.
    // It counts its polls in a shared cell that the test can read from
    // outside, and it panics if it is polled after completion, as many real
    // futures do.
    struct Countdown {
        name: &'static str,
        pendings: u32,
        done: bool,
        polls: Rc<Cell<u32>>,
    }

    fn countdown(name: &'static str, pendings: u32) -> (Countdown, Rc<Cell<u32>>) {
        let polls = Rc::new(Cell::new(0));
        let leaf = Countdown {
            name,
            pendings,
            done: false,
            polls: Rc::clone(&polls),
        };
        (leaf, polls)
    }

    impl Future for Countdown {
        type Output = String;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
            let this = &mut *self;
            this.polls.set(this.polls.get() + 1);
            assert!(
                !this.done,
                "`{}` was polled again after it returned Ready",
                this.name
            );
            if this.pendings > 0 {
                this.pendings -= 1;
                // A real leaf would store the waker and wake it when its event
                // fires; this one can make progress again right away.
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            this.done = true;
            Poll::Ready(format!("{} done", this.name))
        }
    }

    // The executor of these tests: polls `future` with `waker` until it is
    // `Ready` and returns the output together with the number of polls it
    // took. Every leaf here wakes itself before it returns `Pending`, so
    // re-polling at once is what a real executor would do too. A future that
    // is still `Pending` after 100 polls is stuck, and the test fails.
    fn run_with<F: Future + Unpin>(future: &mut F, waker: &Waker) -> (F::Output, u32) {
        let mut cx = Context::from_waker(waker);
        for poll in 1..=100 {
            if let Poll::Ready(output) = Pin::new(&mut *future).poll(&mut cx) {
                return (output, poll);
            }
        }
        panic!("not Ready after 100 polls");
    }

    fn run<F: Future + Unpin>(future: &mut F) -> (F::Output, u32) {
        run_with(future, Waker::noop())
    }

    fn done(name: &str) -> String {
        format!("{name} done")
    }

    #[test]
    fn both_children_are_polled_on_the_first_poll() {
        let (a, a_polls) = countdown("a", 3);
        let (b, b_polls) = countdown("b", 5);
        let mut joined = join(a, b);
        let mut cx = Context::from_waker(Waker::noop());
        assert!(Pin::new(&mut joined).poll(&mut cx).is_pending());
        assert_eq!(a_polls.get(), 1, "`a` must be polled on the first poll");
        assert_eq!(
            b_polls.get(),
            1,
            "`b` must be polled on the first poll too, not only after `a` finished"
        );
    }

    #[test]
    fn finishes_on_the_poll_where_the_slower_child_is_ready() {
        // `a` returns `Pending` 3 times, `b` 5 times. Polled together, `a` is
        // `Ready` on outer poll 4 and `b` on outer poll 6, so the join is
        // `Ready` on poll max(3, 5) + 1 = 6. Run one after the other, it would
        // take 3 + 1 + 5 = 9.
        let (a, a_polls) = countdown("a", 3);
        let (b, b_polls) = countdown("b", 5);
        let (output, polls) = run(&mut join(a, b));
        assert_eq!(output, (done("a"), done("b")));
        assert_eq!(polls, 6, "expected max(3, 5) + 1 = 6 polls");
        // Each child is polled until it is `Ready`, and never after that.
        assert_eq!(a_polls.get(), 4);
        assert_eq!(b_polls.get(), 6);
    }

    #[test]
    fn a_child_that_finished_first_is_not_polled_again() {
        // `a` is `Ready` on the very first poll and `b` on the 5th. `a` must be
        // polled exactly once, and its output kept for 4 more polls.
        let (a, a_polls) = countdown("a", 0);
        let (b, b_polls) = countdown("b", 4);
        let (output, polls) = run(&mut join(a, b));
        assert_eq!(output, (done("a"), done("b")));
        assert_eq!(polls, 5, "expected max(0, 4) + 1 = 5 polls");
        assert_eq!(a_polls.get(), 1);
        assert_eq!(b_polls.get(), 5);
    }

    #[test]
    fn outputs_keep_argument_order_when_b_finishes_first() {
        let (a, a_polls) = countdown("a", 4);
        let (b, b_polls) = countdown("b", 0);
        let (output, polls) = run(&mut join(a, b));
        assert_eq!(output, (done("a"), done("b")));
        assert_eq!(polls, 5, "expected max(4, 0) + 1 = 5 polls");
        assert_eq!(a_polls.get(), 5);
        assert_eq!(b_polls.get(), 1);
    }

    #[test]
    fn two_ready_children_finish_on_the_first_poll() {
        // `std::future::Ready` is `Unpin` and panics with "`Ready` polled after
        // completion" if it is polled twice.
        let (output, polls) = run(&mut join(ready(1_u8), ready("two")));
        assert_eq!(output, (1, "two"));
        assert_eq!(polls, 1);
    }

    #[test]
    fn nested_joins_compose() {
        // A `Join` of `Unpin` children is `Unpin` itself, so it can be a child
        // of another `Join`. All three leaves run concurrently: the result is
        // ready on poll max(2, 6, 4) + 1 = 7.
        let (a, _) = countdown("a", 2);
        let (b, _) = countdown("b", 6);
        let (c, _) = countdown("c", 4);
        let (output, polls) = run(&mut join(join(a, b), c));
        assert_eq!(output, ((done("a"), done("b")), done("c")));
        assert_eq!(polls, 7, "expected max(2, 6, 4) + 1 = 7 polls");
    }

    // A waker that counts how often it is woken.
    struct CountingWaker(AtomicU32);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn children_wake_the_callers_waker() {
        // Every `Pending` of a leaf wakes the waker it was handed once, so the
        // task's waker must be woken 2 + 3 = 5 times. A child polled with an
        // unrelated waker (say, a `Context` built from `Waker::noop()`) wakes
        // nobody, and a real executor would never poll the task again.
        let counter = Arc::new(CountingWaker(AtomicU32::new(0)));
        let waker = Waker::from(Arc::clone(&counter));
        let (a, _) = countdown("a", 2);
        let (b, _) = countdown("b", 3);
        let (output, polls) = run_with(&mut join(a, b), &waker);
        assert_eq!(output, (done("a"), done("b")));
        assert_eq!(polls, 4);
        assert_eq!(
            counter.0.load(Ordering::SeqCst),
            5,
            "the children must be polled with the caller's `Context`"
        );
    }

    // A one-slot mailbox shared by two `async` blocks. `send` waits while the
    // slot is full and `recv` while it is empty, so the producer can never be
    // more than one message ahead of the consumer.
    struct Mailbox {
        slot: Cell<Option<u32>>,
        // How often a `send` or `recv` could not make progress. It only guards
        // against a `poll` that loops forever on one child.
        stalls: Cell<u32>,
    }

    impl Mailbox {
        async fn send(&self, value: u32) {
            poll_fn(|cx| {
                if self.slot.get().is_some() {
                    return self.stall(cx);
                }
                self.slot.set(Some(value));
                Poll::Ready(())
            })
            .await;
        }

        async fn recv(&self) -> u32 {
            poll_fn(|cx| match self.slot.take() {
                Some(value) => Poll::Ready(value),
                None => self.stall(cx),
            })
            .await
        }

        fn stall<T>(&self, cx: &mut Context<'_>) -> Poll<T> {
            self.stalls.set(self.stalls.get() + 1);
            assert!(
                self.stalls.get() < 1_000,
                "the mailbox made no progress in 1000 polls: does `poll` loop on one child?"
            );
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    #[test]
    fn a_rendezvous_needs_both_sides() {
        // The producer can send message 2 only after the consumer has taken
        // message 1, so neither side can finish on its own. A join that runs
        // one child to completion before it polls the other waits forever
        // (here: "not Ready after 100 polls"). `async` blocks are `!Unpin`, so
        // they are boxed.
        let mailbox = Mailbox {
            slot: Cell::new(None),
            stalls: Cell::new(0),
        };
        let producer = Box::pin(async {
            for value in 1..=3 {
                mailbox.send(value).await;
            }
            "sent 3"
        });
        let consumer = Box::pin(async {
            let mut received = Vec::new();
            for _ in 0..3 {
                received.push(mailbox.recv().await);
            }
            received
        });
        let (output, _) = run(&mut join(producer, consumer));
        assert_eq!(output, ("sent 3", vec![1, 2, 3]));
        assert_eq!(mailbox.slot.get(), None, "every message was received");
    }
}
