//! Lab · self-referential structs and what `Pin` is for
//!
//! `SelfRef` holds a `String` along with a **raw pointer to its own `String`**.
//! Once set up, the struct must never be moved — otherwise `data_ptr` would
//! become a dangling pointer.
//!
//! This is exactly the shape of the state machine the compiler generates for an
//! `async` block that holds a local reference across an `.await`: the state
//! machine contains both local variables and references pointing at those
//! locals. `Pin` encodes the constraint "this value will not be moved again" in
//! the type system, which makes that kind of self-reference safe. Here we
//! maintain the constraint by hand with `unsafe`, to see clearly what `Pin` is
//! actually protecting.

use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr;

pub struct SelfRef {
    data: String,
    /// Points at this struct's own `data` field; filled in partway through
    /// construction, initially a null pointer.
    data_ptr: *const String,
    /// Makes `SelfRef` `!Unpin`: once pinned, safe code can no longer obtain a
    /// `&mut Self`, and therefore cannot move it.
    _pin: PhantomPinned,
}

impl SelfRef {
    pub fn new(data: String) -> Pin<Box<Self>> {
        let mut boxed = Box::pin(SelfRef {
            data,
            data_ptr: ptr::null(),
            _pin: PhantomPinned,
        });

        // First compute the stable address of the `data` field (`boxed` has
        // already pinned it on the heap).
        let self_ptr: *const String = &boxed.data;

        // SAFETY: we only write the `data_ptr` field and move no data. `boxed`
        // is pinned, so the address of `data` stays fixed for its entire
        // lifetime, which keeps `self_ptr` valid for the long term.
        unsafe {
            let mut_ref: Pin<&mut Self> = boxed.as_mut();
            Pin::get_unchecked_mut(mut_ref).data_ptr = self_ptr;
        }

        boxed
    }

    /// The ordinary way to read: borrow the `data` field directly.
    pub fn data(self: Pin<&Self>) -> &str {
        &self.get_ref().data
    }

    /// Read `data` through the self-referential raw pointer — only safe as long
    /// as the struct has never been moved.
    pub fn data_via_ptr(self: Pin<&Self>) -> &str {
        // SAFETY: `data_ptr` was set in `new` to point at this struct's `data`;
        // `Pin` guarantees the struct has not been moved, so the pointer is
        // still valid.
        unsafe { &*self.data_ptr }
    }

    /// Expose the numeric address stored in the self-referential pointer (the
    /// value captured back in `new`).
    pub fn ptr_addr(self: Pin<&Self>) -> usize {
        self.data_ptr as usize
    }

    /// The *current* address of the `data` field itself (not its heap buffer —
    /// that would be `data()`). After pinning this never changes, so it must
    /// still equal `ptr_addr`; the gap between the two is exactly what a move
    /// would open up.
    pub fn data_field_addr(self: Pin<&Self>) -> usize {
        &self.get_ref().data as *const String as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_read_paths_agree() {
        let s = SelfRef::new("hello".to_string());
        assert_eq!(s.as_ref().data(), "hello");
        // Reading through the raw pointer must match reading the field directly.
        assert_eq!(s.as_ref().data_via_ptr(), "hello");
    }

    #[test]
    fn self_pointer_still_targets_field_after_pinning() {
        let s = SelfRef::new("world".to_string());
        let stored = s.as_ref().ptr_addr(); // captured during `new`
        let live = s.as_ref().data_field_addr(); // where `data` actually lives now
        assert_ne!(stored, 0);
        // The self-pointer set during construction STILL points exactly at the
        // `data` field: proof the value did not move after being pinned. This is
        // the invariant `data_via_ptr` relies on for soundness — had `new`
        // returned an un-pinned value that was then moved, these would differ.
        assert_eq!(stored, live);
    }
}
