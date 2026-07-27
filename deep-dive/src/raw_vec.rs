//! Lab · a minimal growable `Vec` from raw parts (`alloc` + `NonNull` + `ptr`)
//!
//! A `Vec<T>` is not magic. Strip it down and it is really just two pieces:
//!
//! - a `RawVec { ptr, cap }` — a heap allocation big enough for `cap` elements,
//!   plus its capacity, and
//! - a `len` — how many of those `cap` slots are actually initialized.
//!
//! Everything else is bookkeeping. Here we build that from scratch to see it:
//!
//! - **Growth** doubles the capacity and asks the allocator for a fresh block.
//!   We compute the block size with `Layout::array::<T>(cap)` and call
//!   `alloc::alloc` for the first allocation, `alloc::realloc` afterwards.
//! - **`push` / `pop`** *move* elements in and out with `ptr::write` /
//!   `ptr::read`. These do a raw memory copy — no `Clone`, and crucially no
//!   `Drop` of whatever bytes happened to be sitting in the slot before.
//! - **`Drop`** must first drop only the *initialized* region `[0, len)` — one
//!   `ptr::drop_in_place` over the `&mut [T]` slice does it (the slots past `len`
//!   hold uninitialized memory, and dropping them is undefined behavior) — then
//!   hand the block back with `alloc::dealloc` using a `Layout` that matches the
//!   one we allocated with.
//! - **`Deref`/`DerefMut`** hand out a `&[T]` / `&mut [T]` covering exactly the
//!   initialized region, which is how `Vec` gets every slice method for free.
//!
//! Zero-sized types are special-cased *out*: `Layout::array::<T>` would compute
//! a zero-size allocation, which the global allocator forbids. The real `Vec`
//! handles ZSTs by pinning `cap = usize::MAX` and never allocating at all; that
//! extra branch would drown out the lesson, so this teaching `Vec` simply
//! refuses them with an assertion in `new`.
//!
//! This lab is verified leak-free / UB-free under:  cargo +nightly miri test

use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};

pub struct MyVec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
    // Tell the compiler this struct logically owns `T`s (affects drop-check and variance).
    _marker: PhantomData<T>,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        assert!(
            std::mem::size_of::<T>() != 0,
            "this teaching Vec does not support zero-sized types"
        );
        // No allocation yet: `dangling()` is a well-aligned non-null placeholder.
        // We only touch it once `cap > 0`, which is exactly when we allocate.
        MyVec {
            ptr: NonNull::dangling(),
            cap: 0,
            len: 0,
            _marker: PhantomData,
        }
    }

    fn grow(&mut self) {
        // Double the capacity (or start at 1). Real `Vec` also caps the size at
        // `isize::MAX` bytes; `Layout::array` enforces that for us via `unwrap`.
        let new_cap = if self.cap == 0 { 1 } else { self.cap * 2 };
        let new_layout = Layout::array::<T>(new_cap).unwrap();

        let new_ptr = if self.cap == 0 {
            // SAFETY: `new_layout` has non-zero size (T is not a ZST and new_cap >= 1),
            // so calling the global allocator with it is valid.
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            // SAFETY: `self.ptr` was allocated by this same allocator with `old_layout`
            // (the invariant `cap > 0` implies a live allocation of that layout), and
            // `new_layout.size()` is non-zero and does not overflow `isize::MAX` (checked
            // by `Layout::array`). `realloc` preserves the existing initialized bytes.
            unsafe { alloc::realloc(self.ptr.as_ptr() as *mut u8, old_layout, new_layout.size()) }
        };

        // On allocation failure the returned pointer is null; abort per convention.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.grow();
        }
        // SAFETY: after the grow above, `len < cap`, so `ptr + len` is within the
        // allocation and points at an uninitialized slot. `ptr::write` moves `value`
        // there without reading (and thus without dropping) the old contents.
        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.len), value);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: `len` was > 0 and now indexes the last initialized slot, which is
        // within the allocation. `ptr::read` moves the value out by copy; we have
        // just decremented `len` so that slot is now considered uninitialized and
        // will not be dropped again by `Drop`.
        Some(unsafe { ptr::read(self.ptr.as_ptr().add(self.len)) })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> Deref for MyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        // SAFETY: `ptr` is valid and aligned, and the first `len` slots are all
        // initialized (our invariant), so a shared slice over exactly `[0, len)`
        // is sound and cannot outlive `&self`.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: same invariant as `deref`; `&mut self` guarantees exclusive
        // access, so a unique slice over `[0, len)` is sound.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Default for MyVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            // Drop only the initialized region, in ONE call — exactly what the
            // real `Vec` does. `&mut **self` is the `&mut [T]` covering `[0, len)`
            // (see `DerefMut`), so the slots in `[len, cap)` are never touched;
            // dropping those would be undefined behavior.
            // SAFETY: every element of that slice is initialized (our invariant)
            // and `drop_in_place` runs each element's own `Drop` exactly once.
            unsafe {
                ptr::drop_in_place(&mut **self as *mut [T]);
            }
            // Then hand the block back with a layout matching the allocation.
            let layout = Layout::array::<T>(self.cap).unwrap();
            // SAFETY: `ptr` was allocated by this allocator with exactly `layout`
            // (cap unchanged since the last grow), every element has already been
            // dropped above, and we deallocate exactly once (cap != 0 here).
            unsafe {
                alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn push_grows_and_deref_slice_sees_every_element() {
        let mut v = MyVec::new();
        for i in 0..100u64 {
            v.push(i);
        }
        assert_eq!(v.len(), 100);
        assert!(!v.is_empty());
        // Reach through `Deref` to a `&[u64]` and use ordinary slice methods.
        let expected: u64 = (0..100).sum();
        assert_eq!(v.iter().sum::<u64>(), expected);
        assert_eq!(v.first(), Some(&0));
        assert_eq!(v.last(), Some(&99));
        assert_eq!(v[50], 50);
    }

    #[test]
    fn pop_returns_values_lifo_then_none() {
        let mut v = MyVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.pop(), Some(3));
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), None);
        assert!(v.is_empty());
    }

    // A payload that increments a shared counter each time it is dropped, so we
    // can prove every pushed element is dropped exactly once (no leak, no
    // double-free) when the `MyVec` itself drops.
    struct DropCounter(Arc<AtomicUsize>);

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn drop_runs_each_element_destructor_exactly_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        {
            let mut v = MyVec::new();
            for _ in 0..50 {
                v.push(DropCounter(Arc::clone(&drops)));
            }
            // Pop a few by hand: each popped value drops when it leaves scope.
            let _a = v.pop();
            let _b = v.pop();
            drop(_a);
            drop(_b);
            assert_eq!(drops.load(Ordering::SeqCst), 2);
            // The remaining 48 drop when `v` goes out of scope below.
        }
        assert_eq!(drops.load(Ordering::SeqCst), 50);
    }
}
