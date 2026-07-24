// Module 3 · Async — the `Future` trait, part 2: `Poll::Pending` and waking.
//
// Real futures aren't always ready. When a future can't finish yet, it returns
// `Poll::Pending`. Before doing so it MUST arrange to be polled again by using
// the `Waker` inside the `Context` (`cx.waker()`). Otherwise the executor has
// no reason to poll the task again and it would hang forever.
//
// `YieldOnce` returns `Pending` exactly once (handing control back to the
// executor), then `Ready` on the next poll.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready("done")
        } else {
            self.yielded = true;
            // Without this, the executor would never poll us again.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
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

    // A real executor: it parks the thread on `Pending` and the waker unparks
    // it, instead of busy-looping.
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
    fn yields_once_then_finishes() {
        assert_eq!(block_on(YieldOnce { yielded: false }), "done");
    }
}
