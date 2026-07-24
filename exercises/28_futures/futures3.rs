// Module 3 · Async — the `Future` trait, part 3: composing futures by hand.
//
// This is the key insight of the whole module. When you write
//
//     let a = first.await;
//     let b = second.await;
//     (a, b)
//
// the compiler generates a hidden struct that implements `Future` and holds an
// enum of "where am I up to" states. Each `.await` is one state. We build that
// state machine by hand here so the sugar has no more secrets.
//
// `Sequence` runs `first` to completion, THEN `second`, and returns both ids.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// A leaf future that becomes `Ready(id)` after being polled `remaining + 1`
// times. It is `Unpin` (only holds plain numbers), which lets us re-poll it
// with the safe `Pin::new`.
struct Countdown {
    remaining: u32,
    id: u8,
}

impl Future for Countdown {
    type Output = u8;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.remaining == 0 {
            Poll::Ready(self.id)
        } else {
            self.remaining -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[derive(Clone, Copy)]
enum State {
    PollingFirst,
    PollingSecond(u8),
    Done,
}

struct Sequence {
    state: State,
    first: Countdown,
    second: Countdown,
}

impl Future for Sequence {
    type Output = (u8, u8);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `Sequence` is `Unpin`, so we can take a plain `&mut Self` and drive the
        // leaves with the safe `Pin::new(&mut ...)`.
        let this = self.get_mut();

        // TODO: Implement the state machine with a `loop { match this.state { .. } }`:
        //   - `State::PollingFirst`: poll `Pin::new(&mut this.first)`. On
        //     `Poll::Ready(a)`, set `this.state = State::PollingSecond(a)` and
        //     loop again. On `Poll::Pending`, `return Poll::Pending`.
        //   - `State::PollingSecond(a)`: poll `Pin::new(&mut this.second)`. On
        //     `Poll::Ready(b)`, set `this.state = State::Done` and
        //     `return Poll::Ready((a, b))`. On `Poll::Pending`, `return Poll::Pending`.
        //   - `State::Done`: `panic!("polled after completion")`.
        // Until you return a `Poll` from every path, this exercise will not
        // compile (the function is missing its return value).
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::task::{Wake, Waker};
    use std::thread::{self, Thread};

    fn block_on<F: Future>(future: F) -> F::Output {
        struct ThreadWaker(Thread);
        impl Wake for ThreadWaker {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }

        let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(value) => return value,
                Poll::Pending => thread::park(),
            }
        }
    }

    #[test]
    fn runs_futures_in_sequence() {
        let seq = Sequence {
            state: State::PollingFirst,
            first: Countdown {
                remaining: 2,
                id: 1,
            },
            second: Countdown {
                remaining: 3,
                id: 2,
            },
        };
        assert_eq!(block_on(seq), (1, 2));
    }
}
