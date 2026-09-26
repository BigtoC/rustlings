// Traits & Abstraction · Coherence — part 2: a blanket impl claims every type it might ever cover (E0119).
//
// Part 1 was the orphan rule: WHERE an impl may be written. This part is the
// other half of coherence, the overlap check: two impls must never both apply
// to the same type, whether both are in your crate or one of them is in a
// dependency. For two concrete impls that is easy to see. A BLANKET impl,
// one over a bare type parameter such as the given
// `impl<T: Display + ?Sized> Describe for T`, makes it a proof obligation:
// before accepting any other `Describe` impl, rustc has to show that its type
// can never be `Display`.
//
// For `impl Describe for Vec<u8>` that looks easy, since `Vec<u8>` is not
// `Display` today. But rustc is not checking your crate against today's std.
// It checks it against every std you might ever build with, and std is free
// to add `impl Display for Vec<u8>` in any release (implementing a trait for
// a type is a compatible change; part 1's orphan rule is what makes it
// safe). So the answer to "could `Vec<u8>` ever be `Display`?" is yes, and
// you get E0119 "conflicting implementations of trait `Describe` for type
// `Vec<u8>`" with the note "upstream crates may add a new impl of trait
// `std::fmt::Display` for type `std::vec::Vec<u8>` in future versions".
//
// Relying on "type X does NOT implement trait Y" is called negative
// reasoning, and rustc allows it only where the answer cannot change under
// you:
//
//   - A LOCAL type. Only this crate can make it `Display`, so if it isn't,
//     it never will be behind your back. `35_error_design/err4` relied on the
//     same fact: its two `Context` impls could coexist because `Report` is
//     local and not an `Error`.
//   - A fundamental wrapper of a local type. `&T`, `&mut T`, `Box<T>` and
//     `Pin<T>` are FUNDAMENTAL (`Box` and `Pin` carry the `#[fundamental]`
//     attribute): a new blanket impl over one of them, such as
//     `impl<T: Neg> Neg for Box<T>`, counts as a breaking change, so std
//     won't add one. `Box<Bytes>` is therefore `Display` exactly when `Bytes`
//     is, and for a local, non-`Display` type `Bytes`,
//     `impl Describe for Box<Bytes>` would be accepted. But
//     `impl Describe for Box<Vec<u8>>` or `for &Vec<u8>` is the same E0119
//     as above, and so is `for Rc<Bytes>` (`Rc` is not fundamental).
//
// The blanket impl is the expensive side of the deal. It claims every type
// that is `Display` now or becomes `Display` later, forever. A type that is
// `Display` can never get its own `describe` in this crate (that is E0119
// again, without the "upstream" note, because the overlap is real rather than
// possible), and stable Rust has no specialization that would let a "more
// specific" impl win. std's `ToString` has exactly this shape,
// `impl<T: Display + ?Sized> ToString for T`, which is why you implement
// `Display` and never `ToString`. Full specialization (RFC 1210) is still
// unstable because it is unsound when the more specific impl depends on
// lifetimes, which are gone by the time the compiler picks an impl. std uses
// only the restricted `min_specialization` internally, so that
// `str::to_string` can skip the formatting machinery.
//
// The same claim makes adding a blanket impl to a published trait a
// semver-major change: some downstream crate may already implement the trait
// for a type of its own that is `Display`, and that impl would start to
// overlap.
//
// How interviewers probe this: "Why does this conflict when `Vec<u8>` doesn't
// even implement `Display`?", "What does `#[fundamental]` mean?", "Is adding
// a blanket impl a breaking change?", "What would specialization give you,
// and why isn't it stable?".

use std::fmt;

// Describes a value for a log line.
trait Describe {
    fn describe(&self) -> String;
}

// Given, and it stays: every `Display` type, sized or not (`str`, `dyn
// Display`), describes itself as its text in angle brackets. The tests use it
// on a `Display` type of their own.
impl<T: fmt::Display + ?Sized> Describe for T {
    fn describe(&self) -> String {
        format!("<{self}>")
    }
}

// TODO: rustc rejects this impl with E0119 "conflicting implementations of
// trait `Describe` for type `Vec<u8>`", and its note is the lesson: "upstream
// crates may add a new impl of trait `std::fmt::Display` for type
// `std::vec::Vec<u8>` in future versions". It doesn't matter that `Vec<u8>`
// is not `Display` today.
// Requirements: keep the blanket impl above as it is. The tests describe
// byte buffers through a type named `Bytes`, built as `Bytes(vec![..])`,
// whose description is exactly what this body produces ("3 bytes",
// "1 byte", "0 bytes"; not "<3 bytes>"). `describe_all` and `dyn Describe`
// must work for bytes and for `Display` types alike. Don't edit the tests.
// Until the byte impl can no longer overlap the blanket impl, this exercise
// will not compile.
impl Describe for Vec<u8> {
    fn describe(&self) -> String {
        match self.len() {
            1 => "1 byte".to_string(),
            n => format!("{n} bytes"),
        }
    }
}

// Given, complete: generic code over the trait, for any one type.
fn describe_all<T: Describe>(items: &[T]) -> Vec<String> {
    items.iter().map(Describe::describe).collect()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // A `Display` type that `describe` has never heard of. Deliberately not
    // `Clone` or `Copy`.
    struct Celsius(f64);

    impl fmt::Display for Celsius {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}°C", self.0)
        }
    }

    #[test]
    fn display_types_use_the_blanket_impl() {
        assert_eq!(42.describe(), "<42>");
        assert_eq!("hi".describe(), "<hi>");
        assert_eq!(String::from("hello").describe(), "<hello>");
        assert_eq!('x'.describe(), "<x>");
        assert_eq!((-2.5).describe(), "<-2.5>");
    }

    #[test]
    fn the_blanket_impl_covers_types_defined_elsewhere() {
        assert_eq!(Celsius(21.5).describe(), "<21.5°C>");
    }

    #[test]
    fn bytes_describe_their_length() {
        assert_eq!(Bytes(vec![1, 2, 3]).describe(), "3 bytes");
        assert_eq!(Bytes(vec![7]).describe(), "1 byte");
        assert_eq!(Bytes(Vec::new()).describe(), "0 bytes");
        // Method calls auto-deref, so a boxed or borrowed `Bytes` works too.
        let boxed = Box::new(Bytes(vec![0; 5]));
        assert_eq!(boxed.describe(), "5 bytes");
        let borrowed: &&Bytes = &&Bytes(vec![0; 2]);
        assert_eq!(borrowed.describe(), "2 bytes");
    }

    #[test]
    fn generic_code_accepts_both_kinds() {
        assert_eq!(describe_all(&[1, 2]), ["<1>", "<2>"]);
        assert_eq!(
            describe_all(&[String::from("a"), String::from("b")]),
            ["<a>", "<b>"]
        );
        assert_eq!(describe_all(&[Celsius(0.0)]), ["<0°C>"]);
        assert_eq!(
            describe_all(&[Bytes(vec![1]), Bytes(vec![1, 2])]),
            ["1 byte", "2 bytes"]
        );
    }

    #[test]
    fn trait_objects_mix_both_kinds() {
        let items: Vec<Box<dyn Describe>> = vec![
            Box::new(42),
            Box::new("hi"),
            Box::new(Celsius(-3.0)),
            Box::new(Bytes(vec![0; 4])),
        ];
        let described: Vec<String> = items.iter().map(|item| item.describe()).collect();
        assert_eq!(described, ["<42>", "<hi>", "<-3°C>", "4 bytes"]);
    }
}
