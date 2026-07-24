// Module 3 · Async runtime — part 2: the executor's core poll loop.
//
// `block_on` is the smallest possible executor: it drives ONE future to
// completion on the current thread. Every runtime (tokio, async-std, smol) has
// this loop at its heart. The pattern is:
//
//   1. build a `Waker` that unparks THIS thread when called,
//   2. poll the future,
//   3. if `Ready(v)` -> return `v`;
//      if `Pending`  -> park the thread until the waker unparks it, then re-poll.
//
// Parking (instead of busy-looping) is what makes a real executor spend zero
// CPU while waiting.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            // The waker (unparking this thread) is the only thing that ends the
            // park, so we only re-poll when there is a reason to.
            Poll::Pending => thread::park(),
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;

    // A future that yields `Pending` a few times (waking itself each time)
    // before finishing, so `block_on` really exercises the park/unpark path.
    struct YieldTimes(u8);

    impl Future for YieldTimes {
        type Output = u32;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
            if self.0 == 0 {
                Poll::Ready(99)
            } else {
                self.0 -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    #[test]
    fn drives_future_to_completion() {
        assert_eq!(block_on(YieldTimes(3)), 99);
    }
}
