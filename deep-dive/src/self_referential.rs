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

    /// Expose the numeric address of the self-referential pointer, so tests can
    /// check that "the address stays stable after pinning".
    pub fn ptr_addr(self: Pin<&Self>) -> usize {
        self.data_ptr as usize
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
    fn address_is_stable_after_pinning() {
        let s = SelfRef::new("world".to_string());
        let first = s.as_ref().ptr_addr();
        let second = s.as_ref().ptr_addr();
        assert_ne!(first, 0);
        // The address no longer changes after pinning; that is exactly the
        // precondition for the raw pointer to stay valid.
        assert_eq!(first, second);
    }
}
