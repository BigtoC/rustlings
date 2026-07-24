// Module 3 · Async — the `Future` trait, part 1.
//
// `async` / `await` is syntax sugar. Underneath, every asynchronous computation
// is a plain value that implements the `Future` trait:
//
//     trait Future {
//         type Output;
//         fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
//     }
//
// An executor drives a future by calling `poll` repeatedly. Each call returns
// either `Poll::Ready(value)` (the computation finished) or `Poll::Pending`
// (not done yet, poll me again later).
//
// This first future is the simplest possible one: it is ready immediately.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Immediate(i32);

impl Future for Immediate {
    type Output = i32;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: This future is ready right away. Return the stored `i32`
        // wrapped in `Poll::Ready`. You can read a field through `Pin` with
        // `self.0` because reading does not move the value.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Waker;

    // A minimal executor: poll the future until it is `Ready`. `Waker::noop()`
    // is std's do-nothing waker — fine here because `Immediate` is always ready
    // and never needs to be re-polled.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut cx = Context::from_waker(Waker::noop());
        let mut future = std::pin::pin!(future);
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return value;
            }
        }
    }

    #[test]
    fn immediate_is_ready() {
        assert_eq!(block_on(Immediate(42)), 42);
    }
}
