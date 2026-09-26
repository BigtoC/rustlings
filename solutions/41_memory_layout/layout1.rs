// Module 1 · Memory layout — part 1: `size_of` in words for fat pointers, niches and closures.
//
// "What is `size_of::<Option<Box<T>>>()`, and why?" is a favorite warm-up in
// Rust interviews. You cannot look the answer up in the type's API: it
// follows from how Rust lays values out in memory. This exercise is a quiz.
// Predict the size of nine types, then let the tests grade you.
//
// The answers are in WORDS: `W = size_of::<usize>()`, 8 bytes on a 64-bit
// target and 4 on a 32-bit one. Every type here is built from pointers and
// `usize`s, so its size in words is the same on both.
//
// The rules you need:
//
//   - A pointer to a SIZED type (`&T`, `&mut T`, `Box<T>`) is one word, an
//     address: the compiler already knows how big `T` is. A `fn` pointer is
//     one word too.
//   - A pointer to a dynamically sized type (a DST: `str`, `[T]`,
//     `dyn Trait`) is a FAT pointer: the address plus one word of metadata
//     that only exists at run time. For a slice or a `str` the metadata is
//     the length. For a trait object it is a pointer to the vtable: the
//     concrete type's size, alignment, drop glue and method pointers
//     (`deep-dive/src/vtable_lab.rs` builds one by hand).
//   - An enum must record which variant it holds. Usually that is a separate
//     tag, padded out to the payload's alignment. But if the payload has bit
//     patterns that no valid value ever uses (a NICHE), rustc can encode the
//     other variants in those patterns instead, and the tag costs nothing. A
//     reference is never null, and a `bool` is only ever 0 or 1: both have
//     niches.
//   - A closure is an anonymous struct with one field per captured variable
//     (per captured place, such as `point.x`, since edition 2021). A
//     variable captured by reference is stored as a reference, and one
//     captured by value (with `move`, or because the body consumes it) is
//     stored as the value itself.
//
// `24_ownership_model/ownership4` already showed that a `Vec` is a
// three-word header (pointer, capacity, length). A `String` is a `Vec<u8>`
// inside, so it has the same header.
//
// Some of these sizes are GUARANTEED by the language, and some are only what
// the current compiler does. "Is that guaranteed?" is the follow-up
// interviewers like best, and the hint labels every answer one way or the
// other.
//
// How interviewers probe this: "What is `size_of::<Option<Box<T>>>()`, and
// why?", "What is in the second word of a `&str`, and of a `&dyn Trait`?",
// "Why is `Rc<str>` a better choice than `Rc<String>`?", "How big is a
// closure?". If you can explain every answer below, you can answer those.

/// One machine word: the size of a `usize` (and of any thin pointer) in bytes.
const W: usize = size_of::<usize>();

// Each answer says why, and whether the language GUARANTEES that size or it
// is only what the CURRENT RUSTC does.

// ---- Pointers ----

// Q1: `Option<Box<u64>>` is 1 word. A `Box` is never null, so `None` is stored
// as the null address and no tag is needed (the null pointer optimization).
// GUARANTEED: `Box<U>` is in the `core::option` "Representation" table.
const Q_OPTION_BOX: usize = 1;
// Q2: `&str` is 2 words: the address of the first byte and the length in
// BYTES (not chars). CURRENT RUSTC: the Reference documents two-word fat
// pointers but says not to rely on it.
const Q_STR_REF: usize = 2;
// Q3: `&dyn Display` is 2 words: the address of the value and the address of
// the vtable for its concrete type. CURRENT RUSTC, like Q2.
const Q_DYN_REF: usize = 2;
// Q4: `Rc<str>` is 2 words. `Rc` is a pointer to one allocation holding the
// two counts and the bytes, and a pointer to a DST is fat, so the length
// rides in the handle. `Rc<String>` is 1 word, but it points at a `String`
// header that points at the bytes: two allocations and two hops on every
// read. CURRENT RUSTC (`Rc`'s own layout is not documented).
const Q_RC_STR: usize = 2;

// ---- Niches ----

// Q5: `Option<String>` is 3 words, the same as `String`. The capacity field is
// a `usize` that std restricts to `0..=isize::MAX`, so every larger value is
// a niche, and `None` is one of them. CURRENT RUSTC: not guaranteed, but
// `Option<Vec<T>>` and even `Option<Option<String>>` are also 3 words today.
const Q_OPTION_STRING: usize = 3;
// Q6: `Option<Option<&u8>>` is 2 words. The inner `Option<&u8>` is 1 word
// because `None` took the null address, and null is the only niche a
// reference has: rustc treats every other address as valid. The outer
// `Option` has no spare value left to use, so it adds a tag, and the tag
// takes a whole word because the reference next to it must stay word-aligned.
// CURRENT RUSTC (the inner 1 word is guaranteed; the outer tag is not).
const Q_OPTION_OPTION_REF: usize = 2;

// ---- Closures (the tests build these exact closures) ----

// Q7: 0 words. A closure that captures nothing is an empty struct, a
// zero-sized type: the code is chosen statically by its type, so there is
// nothing to store. Coerce it to `fn(u64) -> u64` and you get a 1-word
// function pointer instead. CURRENT RUSTC: the Reference gives closures no
// layout guarantees.
const Q_CLOSURE_NO_CAPTURE: usize = 0;
// Q8: 3 words. `move` puts the `String` itself (its 3-word header) inside the
// closure; the heap bytes stay where they are. CURRENT RUSTC, like Q7.
const Q_CLOSURE_MOVE_STRING: usize = 3;
// Q9: 2 words. Without `move`, each variable that is only read is captured
// by shared reference, and the closure stores ONE reference PER captured
// variable, not one pointer to the enclosing stack frame. That is not an
// effect of edition 2021's disjoint capture, which only matters when the body
// uses fields. With `move` it would be 6 words (two `String` headers).
// CURRENT RUSTC, like Q7.
const Q_CLOSURE_TWO_REFS: usize = 2;

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Display;
    use std::rc::Rc;

    // ---- Pointers ----

    #[test]
    fn option_of_box() {
        assert!(
            size_of::<Option<Box<u64>>>() == Q_OPTION_BOX * W,
            "Q_OPTION_BOX is wrong: how many words is `Option<Box<u64>>`?"
        );
    }

    #[test]
    fn str_reference() {
        assert!(
            size_of::<&str>() == Q_STR_REF * W,
            "Q_STR_REF is wrong: how many words is `&str`?"
        );
    }

    #[test]
    fn trait_object_reference() {
        assert!(
            size_of::<&dyn Display>() == Q_DYN_REF * W,
            "Q_DYN_REF is wrong: how many words is `&dyn Display`?"
        );
    }

    #[test]
    fn rc_of_str() {
        assert!(
            size_of::<Rc<str>>() == Q_RC_STR * W,
            "Q_RC_STR is wrong: how many words is `Rc<str>`?"
        );
    }

    // ---- Niches ----

    #[test]
    fn option_of_string() {
        assert!(
            size_of::<Option<String>>() == Q_OPTION_STRING * W,
            "Q_OPTION_STRING is wrong: how many words is `Option<String>`?"
        );
    }

    #[test]
    fn option_of_option_of_reference() {
        assert!(
            size_of::<Option<Option<&u8>>>() == Q_OPTION_OPTION_REF * W,
            "Q_OPTION_OPTION_REF is wrong: how many words is `Option<Option<&u8>>`?"
        );
    }

    // ---- Closures ----
    //
    // A closure's type cannot be written down, so these tests build the
    // closure and measure the value with `size_of_val`.

    #[test]
    fn closure_that_captures_nothing() {
        let double = |x: u64| x * 2;
        assert_eq!(double(21), 42);
        assert!(
            size_of_val(&double) == Q_CLOSURE_NO_CAPTURE * W,
            "Q_CLOSURE_NO_CAPTURE is wrong: how many words is `|x: u64| x * 2`?"
        );
    }

    #[test]
    fn move_closure_that_owns_a_string() {
        let name = String::from("Ferris");
        let name_len = move || name.len();
        assert_eq!(name_len(), 6);
        assert!(
            size_of_val(&name_len) == Q_CLOSURE_MOVE_STRING * W,
            "Q_CLOSURE_MOVE_STRING is wrong: how many words is `move || name.len()`?"
        );
    }

    #[test]
    fn closure_that_borrows_two_strings() {
        let first = String::from("Ada");
        let last = String::from("Lovelace");
        let full_len = || first.len() + last.len();
        assert_eq!(full_len(), 11);
        // The closure only borrowed the two strings, so they are still here.
        assert_eq!(first.len() + last.len(), 11);
        assert!(
            size_of_val(&full_len) == Q_CLOSURE_TWO_REFS * W,
            "Q_CLOSURE_TWO_REFS is wrong: how many words is `|| first.len() + last.len()`?"
        );
    }
}
