//! Lab · a minimal `Arc` from scratch (compare with `std::sync::Arc`)
//!
//! An `Arc<T>` is a thread-safe reference-counted pointer: cloning it hands out
//! another owner of the *same* heap allocation, and the allocation is freed only
//! when the last owner is dropped. To do this by hand we allocate an `ArcInner`
//! that holds an atomic `strong` count next to the data, hand out `NonNull`
//! pointers into it, and adjust the count with atomic operations.
//!
//! The subtle part is the **memory ordering** on the count. Two rules:
//!
//! - `clone` may use `Relaxed`. All that matters is that the increment is
//!   atomic (no two threads lose an update). A clone does not publish or consume
//!   any data through the counter, so no ordering with other memory is needed.
//!
//! - `drop` must use `Release` on the decrement, and an `Acquire` fence before
//!   freeing. The thread that finally frees the box must observe *everything*
//!   every other owner did to the data first — otherwise it could free memory
//!   while another thread's writes are still in flight, a use-after-free. The
//!   `Release`/`Acquire` pair establishes that happens-before edge: every prior
//!   owner's `Release` decrement synchronizes-with the final owner's `Acquire`
//!   fence, so all their accesses are ordered before the deallocation.
//!
//! If `drop` used `Relaxed` on the decrement instead, the count would still
//! reach zero exactly once, but there would be no happens-before edge between
//! the other threads' uses of the data and the free. The CPU/compiler would be
//! free to reorder a still-pending write past the `fetch_sub`, and the freeing
//! thread could reclaim the allocation underneath it. It would pass casual
//! single-threaded tests and corrupt memory under real contention.
//!
//! (Real `Arc` also aborts the process if the strong count would overflow
//! `isize::MAX`, since a leaked count could otherwise wrap and cause a
//! premature free. We omit that guard here to keep the lab small.)

use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering, fence};

/// The heap payload shared by every clone: the atomic count plus the data.
struct ArcInner<T> {
    strong: AtomicUsize,
    data: T,
}

pub struct MyArc<T> {
    ptr: NonNull<ArcInner<T>>,
    // Tell the compiler this type logically owns an `ArcInner<T>` (affects
    // drop-check and variance), just like `std::sync::Arc` does.
    _marker: PhantomData<ArcInner<T>>,
}

impl<T> MyArc<T> {
    pub fn new(data: T) -> Self {
        let boxed = Box::new(ArcInner {
            strong: AtomicUsize::new(1),
            data,
        });
        // Leak the box: ownership is now managed by hand through the count.
        MyArc {
            ptr: NonNull::from(Box::leak(boxed)),
            _marker: PhantomData,
        }
    }

    fn inner(&self) -> &ArcInner<T> {
        // SAFETY: `ptr` came from `Box::leak` and is kept alive as long as this
        // `MyArc` exists (the count is at least 1), so the reference is valid.
        unsafe { self.ptr.as_ref() }
    }

    pub fn strong_count(&self) -> usize {
        self.inner().strong.load(Ordering::Relaxed)
    }
}

impl<T> Clone for MyArc<T> {
    fn clone(&self) -> Self {
        // Relaxed is enough: only the atomicity of the bump matters, we are not
        // publishing or consuming any data through the count here.
        self.inner().strong.fetch_add(1, Ordering::Relaxed);
        MyArc {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T> Deref for MyArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner().data
    }
}

impl<T> Drop for MyArc<T> {
    fn drop(&mut self) {
        // Release: our accesses to the data happen-before any later free.
        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        // We were the last owner. Acquire fence: synchronize-with every other
        // owner's Release decrement, so all their uses of the data are ordered
        // before the deallocation below.
        fence(Ordering::Acquire);
        // SAFETY: the count reached zero, so no other owner exists and no one
        // else can touch this allocation. Reclaiming the `Box` frees it once.
        unsafe {
            drop(Box::from_raw(self.ptr.as_ptr()));
        }
    }
}

// SAFETY: `MyArc<T>` shares `T` across threads and moves `T` between them, so it
// is `Send`/`Sync` only when `T` is both (matching `std::sync::Arc`). The count
// itself is atomic, so the reference-counting machinery is always thread-safe.
unsafe impl<T: Send + Sync> Send for MyArc<T> {}
unsafe impl<T: Send + Sync> Sync for MyArc<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn clones_share_the_same_data() {
        let a = MyArc::new(5);
        let b = a.clone();
        assert_eq!(*a, 5);
        assert_eq!(*b, 5);
        assert_eq!(a.strong_count(), 2);
    }

    #[test]
    fn drop_decrements_the_count() {
        let a = MyArc::new(5);
        let b = a.clone();
        assert_eq!(a.strong_count(), 2);
        drop(b);
        assert_eq!(a.strong_count(), 1);
    }

    #[test]
    fn inner_is_dropped_exactly_once() {
        // A payload that bumps a shared counter when it is dropped. If the
        // allocation were freed twice (or never), the counter would not be 1.
        struct Payload {
            drops: Arc<AtomicUsize>,
        }
        impl Drop for Payload {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
            }
        }

        let drops = Arc::new(AtomicUsize::new(0));
        let a = MyArc::new(Payload {
            drops: Arc::clone(&drops),
        });
        let b = a.clone();
        let c = a.clone();
        assert_eq!(a.strong_count(), 3);

        drop(a);
        drop(b);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(c);
        // Last owner gone: the payload must have been dropped exactly once.
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}
