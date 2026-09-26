// Module 3 · Async combinators — part 2: a hand-written `select`, where dropping the loser cancels it.
//
// `select` races two futures: it resolves to the output of whichever one
// finishes FIRST and throws the other one away. `tokio::select!`,
// `futures::future::select` and every timeout work this way
// (`tokio::time::timeout(d, fut)` is a race between `fut` and a sleep).
//
// "Throw away" means DROP, and in Rust dropping a future is the ONLY way to
// cancel it from the outside. There is no built-in cancellation signal and no
// exception. A future does work only while it is polled, so once nobody polls
// it, it simply stops, frozen at the `.await` where it last returned
// `Pending`. Dropping it runs the destructors of everything it holds at that
// point: the `async` block's live locals, a lock guard, a socket, a channel
// permit, a timer registration. That is why the loser must be dropped as soon
// as the race is decided, not whenever the caller gets around to dropping the
// `select`: until then it keeps its resources (a held lock blocks every other
// task, a connection stays open). And because `Drop` is synchronous, a
// canceled future cannot `.await` anything on its way out: stable Rust has no
// async drop.
//
// Two more rules:
//
//   1. Poll `a` first, and poll `b` only while `a` is still `Pending`. Once
//      `a` is `Ready` the race is over, and polling `b` anyway is a bug, not
//      a harmless extra poll: if `b` completes too, its output (say, a
//      message it just took out of a channel) has nowhere to go and is
//      dropped with it. The message is lost.
//   2. Checking `a` first is BIASED: if `a` is ready on every poll, `b` is
//      never polled at all (starvation). That is why `tokio::select!` picks
//      a random branch to check first unless you write `biased;`. A fixed
//      order keeps these tests deterministic.
//
// Like `join1`'s `Join`, this `Select` accepts only `Unpin` children
// (`Box::pin` an `async` block first); `futures::future::select` has the same
// `Unpin` bounds. The children sit in `Option`s so that each one can be
// dropped on its own, by emptying its slot, long before the `Select` itself
// is dropped.
//
// How interviewers probe this: "How do you cancel a future in Rust?", "What
// happens to the losing branch of a `select!`?", "Why does `tokio::select!`
// poll its branches in random order?", and "Can a canceled future run async
// cleanup?".

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// The result of a race: `Left` if `a` won, `Right` if `b` won.
#[derive(Debug, PartialEq)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

// Races `a` against `b`. A slot is `Some` while its child is still in the
// race.
struct Select<A, B> {
    a: Option<A>,
    b: Option<B>,
}

fn select<A, B>(a: A, b: B) -> Select<A, B> {
    Select {
        a: Some(a),
        b: Some(b),
    }
}

impl<A: Future + Unpin, B: Future + Unpin> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `Option<A>` and `Option<B>` are `Unpin` because `A` and `B` are, so
        // `Select` is `Unpin` too. That gives you a plain `&mut Select`, and
        // `Pin::new(child).poll(cx)` polls a child borrowed out of its slot
        // with `as_mut()`.
        let this = self.get_mut();

        // TODO: The tests fail, most of them with "not Ready after 100
        // polls": this `poll` never polls a child, so the race never ends.
        // Requirements:
        //   - poll `a` first, with the caller's `cx`, and poll `b` only while
        //     `a` is still `Pending`;
        //   - return the first `Ready` output as `Either::Left` or
        //     `Either::Right`, and `Pending` while both children are
        //     `Pending`;
        //   - drop the loser on the very poll that decides the race: the
        //     tests keep the `Select` alive after it returned `Ready` and
        //     check a drop flag inside the loser;
        //   - no `mem::forget`, no `unsafe`, don't change the tests.
        // Until the first child to finish wins and the loser is dropped at
        // once, the tests will fail.
        Poll::Pending
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::future::{pending, poll_fn, ready};
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::task::{Wake, Waker};

    // Sets its flag when it is dropped. A test that hands one to a future
    // sees the exact moment that future is dropped.
    struct DropFlag(Rc<Cell<bool>>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    // What a test can observe about a `Leaf` from outside.
    struct Probe {
        polls: Rc<Cell<u32>>,
        dropped: Rc<Cell<bool>>,
    }

    // A leaf future that returns `Pending` `pendings` times and then
    // `Ready(value)`, so it is `Ready` on its (pendings + 1)-th poll. It
    // counts its polls and carries a `DropFlag`.
    struct Leaf<T> {
        pendings: u32,
        value: Option<T>,
        polls: Rc<Cell<u32>>,
        _flag: DropFlag,
    }

    fn leaf<T>(pendings: u32, value: T) -> (Leaf<T>, Probe) {
        let polls = Rc::new(Cell::new(0));
        let dropped = Rc::new(Cell::new(false));
        let leaf = Leaf {
            pendings,
            value: Some(value),
            polls: Rc::clone(&polls),
            _flag: DropFlag(Rc::clone(&dropped)),
        };
        (leaf, Probe { polls, dropped })
    }

    impl<T: Unpin> Future for Leaf<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
            let this = &mut *self;
            this.polls.set(this.polls.get() + 1);
            if this.pendings > 0 {
                this.pendings -= 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let value = this.value.take();
            Poll::Ready(value.expect("a leaf was polled again after it returned Ready"))
        }
    }

    // The executor of these tests: polls `future` with `waker` until it is
    // `Ready` and returns the output together with the number of polls it
    // took. It borrows the future, so the test can inspect the `Select`
    // after it finished. Still `Pending` after 100 polls means stuck.
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

    #[test]
    fn the_faster_left_child_wins() {
        // `a` is `Ready` on its 2nd poll; `b` would need 4.
        let (a, a_probe) = leaf(1, 5);
        let (b, b_probe) = leaf(3, "slow");
        let mut race = select(a, b);
        let (output, polls) = run(&mut race);
        assert_eq!(output, Either::Left(5));
        assert_eq!(polls, 2);
        // `race` is still alive, but the loser must be gone already.
        assert!(
            b_probe.dropped.get(),
            "the loser `b` must be dropped on the poll that returned Ready"
        );
        // `b` was polled on poll 1 only: on poll 2, `a` won before `b`'s turn.
        assert_eq!(b_probe.polls.get(), 1);
        assert_eq!(a_probe.polls.get(), 2);
    }

    #[test]
    fn the_faster_right_child_wins() {
        // `b` is `Ready` on its 3rd poll; `a` would need 5.
        let (a, a_probe) = leaf(4, 1_u8);
        let (b, b_probe) = leaf(2, 'b');
        let mut race = select(a, b);
        let (output, polls) = run(&mut race);
        assert_eq!(output, Either::Right('b'));
        assert_eq!(polls, 3);
        assert!(
            a_probe.dropped.get(),
            "the loser `a` must be dropped on the poll that returned Ready"
        );
        assert_eq!(
            a_probe.polls.get(),
            3,
            "`a` is polled first on every poll, the last one included"
        );
        assert_eq!(b_probe.polls.get(), 3);
    }

    #[test]
    fn nothing_is_dropped_before_the_race_is_decided() {
        let (a, a_probe) = leaf(2, 'a');
        let (b, b_probe) = leaf(2, 'b');
        let mut race = select(a, b);
        let mut cx = Context::from_waker(Waker::noop());
        for poll in 1..=2 {
            assert!(Pin::new(&mut race).poll(&mut cx).is_pending());
            assert_eq!(
                (a_probe.polls.get(), b_probe.polls.get()),
                (poll, poll),
                "both children are polled while neither is Ready"
            );
            assert!(
                !a_probe.dropped.get() && !b_probe.dropped.get(),
                "no child may be dropped while the race is still open"
            );
        }
        // Poll 3: both would be `Ready` now. `a` is checked first, so it wins
        // and `b` is not polled again.
        assert_eq!(
            Pin::new(&mut race).poll(&mut cx),
            Poll::Ready(Either::Left('a'))
        );
        assert_eq!(
            b_probe.polls.get(),
            2,
            "`b` must not be polled once `a` won"
        );
        assert!(
            b_probe.dropped.get(),
            "the loser `b` must be dropped on the poll that returned Ready"
        );
    }

    #[test]
    fn a_decided_race_does_not_poll_the_loser() {
        // `select(shutdown, recv)`: `recv` takes the next message out of a
        // queue the moment it is polled. Shutdown is already signaled, so it
        // wins on the first poll. If `Select` polls `recv` anyway, the message
        // moves out of the queue into `recv`'s output, which is dropped with
        // the loser: the message is lost for good.
        let queue = RefCell::new(VecDeque::from(["order #1".to_string()]));
        let recv = poll_fn(|cx| match queue.borrow_mut().pop_front() {
            Some(message) => Poll::Ready(message),
            None => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        });
        let (output, polls) = run(&mut select(ready("shutdown"), recv));
        assert_eq!(output, Either::Left("shutdown"));
        assert_eq!(polls, 1);
        assert_eq!(
            queue.borrow().len(),
            1,
            "`recv` was polled after shutdown had won, and its message was lost"
        );
    }

    #[test]
    fn a_child_that_never_finishes_simply_loses() {
        // `std::future::pending()` never completes and is `Unpin`.
        let (output, polls) = run(&mut select(pending::<()>(), ready(7)));
        assert_eq!(output, Either::Right(7));
        assert_eq!(polls, 1);
        let (output, polls) = run(&mut select(ready("now"), pending::<u64>()));
        assert_eq!(output, Either::Left("now"));
        assert_eq!(polls, 1);
    }

    #[test]
    fn a_canceled_async_block_drops_its_locals_at_its_await() {
        // A canceled `async` block stops at the `.await` it is suspended on,
        // and dropping it drops the locals that are alive there. `_guard`
        // stands in for a `MutexGuard` or a connection. The rest of the
        // block never runs.
        let guard_dropped = Rc::new(Cell::new(false));
        let reached_the_end = Rc::new(Cell::new(false));
        let guard = DropFlag(Rc::clone(&guard_dropped));
        let end = Rc::clone(&reached_the_end);
        let slow = Box::pin(async move {
            let _guard = guard;
            pending::<()>().await;
            end.set(true);
        });
        let (timeout, _) = leaf(2, "timed out");
        let mut race = select(slow, timeout);
        let (output, polls) = run(&mut race);
        assert_eq!(output, Either::Right("timed out"));
        assert_eq!(polls, 3);
        assert!(
            guard_dropped.get(),
            "the canceled `async` block still holds its guard"
        );
        assert!(!reached_the_end.get());
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
        // `a` returns `Pending` twice and wins on poll 3; `b` returns
        // `Pending` on polls 1 and 2 and is not polled on poll 3. Each
        // `Pending` wakes the waker it was handed once: 2 + 2 = 4 wakeups.
        let counter = Arc::new(CountingWaker(AtomicU32::new(0)));
        let waker = Waker::from(Arc::clone(&counter));
        let (a, _) = leaf(2, 'a');
        let (b, _) = leaf(9, 'b');
        let (output, polls) = run_with(&mut select(a, b), &waker);
        assert_eq!(output, Either::Left('a'));
        assert_eq!(polls, 3);
        assert_eq!(
            counter.0.load(Ordering::SeqCst),
            4,
            "the children must be polled with the caller's `Context`"
        );
    }
}
