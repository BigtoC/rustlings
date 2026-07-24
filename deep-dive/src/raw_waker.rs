//! Lab · hand-writing a `Waker`: `RawWaker` + `RawWakerVTable`
//!
//! In `29_async_runtime/runtime1` we built a `Waker` using the safe `Wake`
//! trait. Here we drop down to the underlying layer: a `Waker` is essentially
//! just
//!
//! - a `*const ()` data pointer (here it points to the contents of an
//!   `Arc<ThreadState>`), plus
//! - a `&'static RawWakerVTable` — four function pointers:
//!   `clone` / `wake` / `wake_by_ref` / `drop`.
//!
//! In the safe version, `Arc`'s reference counting performs these four
//! operations for you. Writing it out by hand shows you exactly what the `Wake`
//! trait does on your behalf, and why it needs `unsafe`.

use std::future::Future;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread::{self, Thread};

/// The data our waker carries: a handle to the thread to be unparked.
struct ThreadState {
    thread: Thread,
}

/// The single global vtable, shared by every `RawWaker` this module creates.
static VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw);

/// Pack an `Arc<ThreadState>` into a `RawWaker` (consuming one reference count).
fn raw_waker(state: Arc<ThreadState>) -> RawWaker {
    RawWaker::new(Arc::into_raw(state).cast::<()>(), &VTABLE)
}

/// `clone`: cloning a waker == holding one more copy of the data, i.e. refcount +1.
unsafe fn clone_raw(ptr: *const ()) -> RawWaker {
    // SAFETY: `ptr` came from `Arc::into_raw::<ThreadState>` and has not been freed.
    // `increment_strong_count` bumps the strong count by exactly 1 without changing `ptr`.
    unsafe { Arc::increment_strong_count(ptr.cast::<ThreadState>()) };
    RawWaker::new(ptr, &VTABLE)
}

/// `wake`: wake by value, consuming this reference count.
unsafe fn wake_raw(ptr: *const ()) {
    // SAFETY: reclaim ownership with `from_raw`; when the function ends `arc` is dropped, refcount -1.
    let arc = unsafe { Arc::from_raw(ptr.cast::<ThreadState>()) };
    arc.thread.unpark();
}

/// `wake_by_ref`: wake by reference, **without** consuming the reference count.
unsafe fn wake_by_ref_raw(ptr: *const ()) {
    // SAFETY: `ManuallyDrop` wraps the reconstructed `Arc` so it is not dropped when done, leaving the refcount unchanged.
    let arc = unsafe { ManuallyDrop::new(Arc::from_raw(ptr.cast::<ThreadState>())) };
    arc.thread.unpark();
}

/// `drop`: destroying a waker == refcount -1.
unsafe fn drop_raw(ptr: *const ()) {
    // SAFETY: reclaim ownership and drop it immediately, refcount -1.
    drop(unsafe { Arc::from_raw(ptr.cast::<ThreadState>()) });
}

/// Drive a future to completion using our hand-written waker. The logic is
/// identical to `runtime2`; the only difference is that here the `Waker` is
/// built from `Waker::from_raw` plus a hand-written vtable.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let state = Arc::new(ThreadState {
        thread: thread::current(),
    });
    // SAFETY: the `RawWaker` returned by `raw_waker` upholds the contract of `VTABLE`
    // (see the SAFETY notes on each function above), so it is safe to build a `Waker` from it.
    let waker = unsafe { Waker::from_raw(raw_waker(state)) };
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::task::Poll;

    // A future that returns Pending twice (waking itself each time) before completing,
    // purpose-built to stress park/unpark.
    struct YieldTimes(u8);

    impl Future for YieldTimes {
        type Output = u32;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
            if self.0 == 0 {
                Poll::Ready(7)
            } else {
                self.0 -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    #[test]
    fn hand_written_waker_drives_a_future() {
        assert_eq!(block_on(YieldTimes(2)), 7);
    }

    #[test]
    fn clone_and_drop_balance_the_refcount() {
        // Clone a waker then drop it; the refcount should return to its original value,
        // with no leak and no double-free.
        let state = Arc::new(ThreadState {
            thread: thread::current(),
        });
        let waker = unsafe { Waker::from_raw(raw_waker(Arc::clone(&state))) };
        let clone = waker.clone();
        drop(clone);
        drop(waker);
        // At this point only our own `state` remains, so the strong count should be 1.
        assert_eq!(Arc::strong_count(&state), 1);
    }
}
