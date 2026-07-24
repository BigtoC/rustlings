// Module 3 · Async runtime — part 4: `Pin`.
//
// `Future::poll` takes `self: Pin<&mut Self>`, not `&mut self`. Why?
//
// An `async` block may hold a reference INTO one of its own local variables
// across an `.await` point — i.e. the generated state machine is a
// *self-referential* struct. If such a future were moved after being polled,
// those internal pointers would dangle. `Pin<P>` is the compile-time promise
// that the pointed-to value will not move again, which makes polling sound.
//
// So to poll a future you need a *pinned* pointer to it. The two safe ways to
// get one are:
//   - `std::pin::pin!(fut)` — pins on the stack, and
//   - `Box::pin(fut)`       — pins on the heap.
//
// (Actually *building* a self-referential future by hand needs `unsafe`; see
//  the `deep-dive/` lab. Here we just consume pinned futures safely.)

use std::future::Future;
use std::task::{Context, Poll, Waker};

// Poll `future` exactly once. If it is already done, return its value; if it
// would still block, return `None`.
fn now_or_never<F: Future>(future: F) -> Option<F::Output> {
    // `Waker::noop()` is std's do-nothing waker: we poll only once, so we never
    // rely on being woken.
    let mut cx = Context::from_waker(Waker::noop());

    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_future_returns_some() {
        // An `async` block is a future; it is ready on the first poll here.
        assert_eq!(now_or_never(async { 1 + 2 }), Some(3));
    }

    #[test]
    fn pending_future_returns_none() {
        // `std::future::pending` never completes, so one poll yields `None`.
        let never = std::future::pending::<i32>();
        assert_eq!(now_or_never(never), None);
    }
}
