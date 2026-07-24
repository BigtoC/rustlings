// Module 3 · Async runtime — part 1: building a `Waker` safely.
//
// When a future returns `Pending`, it hands the executor a `Waker`. Later, when
// the awaited event happens (I/O ready, timer fired, a lock freed, ...),
// something calls `waker.wake()` to signal "this task can make progress now,
// poll it again".
//
// You do NOT need `unsafe` to build a `Waker`. Implement the `std::task::Wake`
// trait for a type behind an `Arc`, and `Waker::from(arc)` builds the waker for
// you. (The raw `RawWaker` / `RawWakerVTable` version, which *does* need
// `unsafe`, lives in the `deep-dive/` lab at the repo root.)

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Wake;

// A toy waker that just records "I was woken" in an atomic flag. The atomic is
// required because a `Waker` must be `Send + Sync`.
struct FlagWaker {
    woken: AtomicBool,
}

impl Wake for FlagWaker {
    fn wake(self: Arc<Self>) {
        self.woken.store(true, Ordering::SeqCst);
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Waker;

    #[test]
    fn wake_sets_the_flag() {
        let inner = Arc::new(FlagWaker {
            woken: AtomicBool::new(false),
        });
        // Turn our `Arc<FlagWaker>` into a real `Waker`, keeping a clone of the
        // `Arc` so we can inspect the flag afterwards.
        let waker: Waker = inner.clone().into();

        assert!(!inner.woken.load(Ordering::SeqCst));
        waker.wake();
        assert!(inner.woken.load(Ordering::SeqCst));
    }
}
