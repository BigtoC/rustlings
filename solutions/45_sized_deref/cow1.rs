// Traits & Abstraction · Deref, Borrow and Cow — part 4: returning `Cow<'_, str>`, so clean input is borrowed instead of copied (E0308, E0599).
//
// Most text-cleaning functions leave most of their inputs alone. A
// `fn collapse_spaces(s: &str) -> String` still pays for an allocation and a
// full copy on EVERY call, even when the answer is exactly the input. A `&str`
// return type cannot express the other case: new text needs an owner, and a
// function cannot return a borrow of its own local (E0515, see
// `37_borrowck_errors/borrowck2`, which listed `Cow` as "covered later"; this
// is later).
//
// `std::borrow::Cow` ("clone on write") is the type for "either". In
// `19_smart_pointers/smart_pointers4` you predicted which variant a given
// function returns; here you write such functions yourself:
//
//     enum Cow<'a, B: ?Sized + ToOwned> {
//         Borrowed(&'a B),
//         Owned(<B as ToOwned>::Owned),
//     }
//
// A `Cow<'a, str>` is either a `&'a str` pointing into the caller's data or an
// owned `String`. It derefs to `str` (part 1), so callers read it like any
// other text, and it compares equal to `&str` and `String`. When a caller does
// need a `String`, `into_owned()` moves the `String` out of an `Owned` and
// copies only a `Borrowed`. `to_mut()` copies a `Borrowed` the first time
// someone writes to it, which is the "clone on write" in the name.
//
// The shape of a good `Cow` API: find out cheaply whether anything has to
// change; if not, return `Cow::Borrowed(input)` and allocate nothing at all;
// otherwise build the new text and return `Cow::Owned(new_text)`. Write the
// return type as `Cow<'_, str>`: the `'_` tells the reader the result may
// borrow from the input. (`-> Cow<str>` means the same, but rustc warns about
// it now: "hiding a lifetime that's elided elsewhere is confusing".) `Cow`s
// also compose. A function that borrows a SLICE of its input, such as
// `s.trim()`, can pass that slice on to another `Cow` function and still
// allocate nothing.
//
// The tests below check WHICH variant comes back, so while the functions
// return `String` they fail to compile: E0308 "mismatched types ... expected
// `&Cow<'_, str>`, found `&String`".
//
// How interviewers probe it: "When would you return a `Cow`?", "What does
// `to_mut` do on a `Cow::Borrowed`?", "`Cow<'_, str>`, `String` or `&str` as
// a return type?", and "Why not just return a `String` every time?" (because
// most inputs are already clean, and the allocation is the cost).

use std::borrow::Cow;

// ---- Part A — borrow when nothing changes ----------------------------------

// Collapses every run of ASCII spaces (' ') into a single space. Other
// whitespace, such as tabs and newlines, is kept as it is.
// One cheap scan decides: without a double space there is nothing to
// collapse, so the input itself goes back as `Cow::Borrowed` and nothing is
// allocated. Only when a run exists is a new `String` built, and it is handed
// over as `Cow::Owned`. The `'_` ties the result to `s`, which it may borrow.
fn collapse_spaces(s: &str) -> Cow<'_, str> {
    if !s.contains("  ") {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut previous_was_space = false;
    for c in s.chars() {
        let is_space = c == ' ';
        if !(is_space && previous_was_space) {
            out.push(c);
        }
        previous_was_space = is_space;
    }
    Cow::Owned(out)
}

// ---- Part B — pass the `Cow` along -----------------------------------------

// Trims whitespace from both ends, then collapses runs of spaces.
// `s.trim()` is a borrowed slice of `s`, so whatever `collapse_spaces`
// borrows from it still borrows from `s`, and the `Cow` can be returned as it
// is. Converting it to a `String` here would allocate for every clean input.
fn normalize(s: &str) -> Cow<'_, str> {
    collapse_spaces(s.trim())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::ptr;

    // The borrowed text, or a test failure if the result was allocated.
    fn borrowed<'a>(out: &Cow<'a, str>) -> &'a str {
        match out {
            Cow::Borrowed(text) => text,
            Cow::Owned(text) => {
                panic!("expected `Cow::Borrowed`, got `Cow::Owned({text:?})`")
            }
        }
    }

    // The owned text, or a test failure if the result was borrowed.
    fn owned(out: Cow<'_, str>) -> String {
        match out {
            Cow::Owned(text) => text,
            Cow::Borrowed(text) => {
                panic!("expected `Cow::Owned`, got `Cow::Borrowed({text:?})`")
            }
        }
    }

    // ----- Part A: `collapse_spaces` -----

    #[test]
    fn clean_text_comes_back_borrowed() {
        let input = String::from("one two three");
        let out = collapse_spaces(&input);
        // Not just equal text: the very same bytes of `input`.
        assert!(ptr::eq(borrowed(&out), input.as_str()));
    }

    #[test]
    fn runs_of_spaces_collapse_into_new_text() {
        assert_eq!(owned(collapse_spaces("a   b")), "a b");
        assert_eq!(owned(collapse_spaces("a  b  c")), "a b c");
    }

    #[test]
    fn the_empty_string_is_borrowed() {
        assert_eq!(borrowed(&collapse_spaces("")), "");
    }

    #[test]
    fn single_spaces_are_already_clean() {
        assert_eq!(borrowed(&collapse_spaces(" ")), " ");
        assert_eq!(borrowed(&collapse_spaces(" a b ")), " a b ");
    }

    #[test]
    fn runs_at_the_ends_collapse_too() {
        assert_eq!(owned(collapse_spaces("  a  ")), " a ");
        assert_eq!(owned(collapse_spaces("     ")), " ");
    }

    #[test]
    fn other_whitespace_is_left_alone() {
        assert_eq!(borrowed(&collapse_spaces("a\t\tb\n\nc")), "a\t\tb\n\nc");
        assert_eq!(borrowed(&collapse_spaces("a \t b")), "a \t b");
        assert_eq!(owned(collapse_spaces("a\t  b")), "a\t b");
    }

    #[test]
    fn multibyte_text_survives() {
        assert_eq!(owned(collapse_spaces("héllo   wörld  ✓")), "héllo wörld ✓");
        assert_eq!(borrowed(&collapse_spaces("日本 語")), "日本 語");
    }

    #[test]
    fn a_borrowed_cow_copies_on_the_first_write() {
        let input = String::from("draft");
        let mut out = collapse_spaces(&input);
        assert!(ptr::eq(borrowed(&out), input.as_str()));
        // `to_mut` turns a `Borrowed` into an `Owned` copy, then hands out
        // `&mut String`; the caller's `input` is never touched.
        out.to_mut().push_str(" v2");
        assert_eq!(owned(out), "draft v2");
        assert_eq!(input, "draft");
    }

    #[test]
    fn callers_can_detach_the_result() {
        let detached: String = {
            let temporary = String::from("x  y");
            collapse_spaces(&temporary).into_owned()
        };
        assert_eq!(detached, "x y");
    }

    // ----- Part B: `normalize` -----

    #[test]
    fn normalize_borrows_a_slice_of_its_input() {
        let input = String::from("  hello world \n");
        let out = normalize(&input);
        let text = borrowed(&out);
        assert_eq!(text, "hello world");
        // It starts right after the two leading spaces, inside `input`.
        assert_eq!(text.as_ptr(), input[2..].as_ptr());
    }

    #[test]
    fn normalize_leaves_tidy_input_as_it_is() {
        let input = String::from("tidy");
        let out = normalize(&input);
        assert!(ptr::eq(borrowed(&out), input.as_str()));
    }

    #[test]
    fn normalize_allocates_only_when_it_must() {
        assert_eq!(owned(normalize("  hello    world  ")), "hello world");
        assert_eq!(owned(normalize("\ta  b\n")), "a b");
    }

    #[test]
    fn normalize_of_blank_input_is_empty() {
        assert_eq!(borrowed(&normalize(" \t\n ")), "");
        assert_eq!(borrowed(&normalize("")), "");
    }
}
