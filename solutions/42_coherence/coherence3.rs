// Traits & Abstraction · Coherence — part 3: extension traits, methods on types you don't own (E0118, E0390, E0599).
//
// "How does itertools add methods to every iterator?" The first thing people
// try is an inherent impl: `impl str { .. }`, or `impl<I: Iterator> I { .. }`
// for "every iterator". Inherent impls are even stricter than the orphan
// rule: one may only be written in the crate that defines the type. A
// foreign struct gets E0116 "cannot define inherent `impl` for a type outside
// of the crate where the type is defined". A primitive such as `str` gets
// E0390 "cannot define inherent `impl` for primitive types" (core and alloc
// write theirs with internal attributes). And "every `I: Iterator`" is not
// a type at all, so it gets E0118 "no nominal type found for inherent
// implementation". The strictness has a reason. Inherent methods need no
// import and win over trait methods of the same name, so if two crates could
// both add a `pairwise` to `Chars`, nothing could tell them apart.
//
// The way out is the one rustc's help suggests: an EXTENSION TRAIT. It is a
// trait of your own, so the orphan rule is satisfied whatever type you
// implement it for. `35_error_design/err4` already built one for `Result`
// (`Context`). This part adds the two shapes you meet most in real code.
//
// Part A, one trait for every iterator. Declare the methods as PROVIDED
// methods (with default bodies) on a trait whose supertrait is `Iterator`,
// so the bodies can use `Self::Item`. Then one blanket impl over every
// `I: Iterator` with an empty body switches them on everywhere, including on
// the adapters they return, so calls chain. `itertools::Itertools` is built
// exactly like that. Two details decide whether it compiles and how widely
// it applies:
//
//   - A trait's `Self` may be unsized (`dyn Iterator`), but these methods
//     take `self` BY VALUE and store it in a struct field. So the methods
//     need `Self: Sized`, either as a supertrait (`Iterator + Sized`) or as a
//     `where Self: Sized` on each method.
//   - A bound that only one method needs (`Item: Clone` for `pairwise`)
//     belongs on THAT method. Put it on the blanket impl instead and every
//     method disappears for iterators whose items aren't `Clone`.
//
// Part B, one impl for every string. Implement the trait for `str`, not for
// `String` or `&str`. Method calls auto-deref the receiver (`String` ->
// `str`, `Box<str>` -> `str`, `&&str` -> `&str` -> `str`) and then auto-ref
// it, so a `&self` method on `str` is reachable from every string type. An
// impl for `String` would miss string literals, and one for `&str` would have
// `&&str` receivers that a `String` never reaches.
//
// What extension traits cost: the trait must be IN SCOPE where you call it.
// Forget the `use` in another module and you get E0599, with a help line
// "trait `IterExt` which provides `pairwise` is implemented but not in
// scope". And names can collide. If another trait in scope has a method of
// the same name (even `Iterator` itself: call yours `count` and see), the
// call is E0034 "multiple applicable items in scope", and you have to write
// `IterExt::pairwise(iter)`. That is why the Cargo SemVer guide calls even
// adding a DEFAULTED method to a trait "possibly-breaking": it can make some
// downstream call ambiguous.
//
// How interviewers probe this: "How do you add a method to `str`, or to every
// iterator?", "Why `Iterator + Sized`?", "Why implement it for `str` and not
// for `String`?", "What happens when std later adds a method with the same
// name as yours?".

use std::iter::Peekable;

// ---- Given: two lazy adapters, complete -------------------------------------

// Overlapping neighbor pairs: `1, 2, 3` -> `(1, 2), (2, 3)`. This is the
// adapter from `34_iterators/iter2`, which could only be reached through a
// free function there.
struct Pairwise<I: Iterator> {
    inner: I,
    prev: Option<I::Item>,
}

impl<I> Iterator for Pairwise<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let first = match self.prev.take() {
            Some(prev) => prev,
            None => self.inner.next()?,
        };
        let second = self.inner.next()?;
        self.prev = Some(second.clone());
        Some((first, second))
    }
}

// Collapses each run of equal neighbors into one item, like `Vec::dedup` or
// Unix `uniq`: "aaabccd" -> "abcd". It yields the first item of a run and
// drops the rest, so it needs `PartialEq` and nothing else.
struct DedupAdjacent<I: Iterator> {
    inner: Peekable<I>,
}

impl<I> Iterator for DedupAdjacent<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let first = self.inner.next()?;
        while self.inner.next_if(|next| *next == first).is_some() {}
        Some(first)
    }
}

// ---- Part A — methods for every iterator ----------------------------------

// A local trait, so the orphan rule is satisfied for any type it is
// implemented for. `Iterator` as a supertrait lets the default bodies name
// `Self::Item`; `Sized` is needed because both methods take `self` by value
// and move it into a struct field. Each bound that only one method needs
// sits on that method, so `pairwise` never asks for `PartialEq` and
// `dedup_adjacent` never asks for `Clone`.
trait IterExt: Iterator + Sized {
    fn pairwise(self) -> Pairwise<Self>
    where
        Self::Item: Clone,
    {
        Pairwise {
            inner: self,
            prev: None,
        }
    }

    fn dedup_adjacent(self) -> DedupAdjacent<Self>
    where
        Self::Item: PartialEq,
    {
        DedupAdjacent {
            inner: self.peekable(),
        }
    }
}

// One blanket impl with an empty body: every iterator gets the provided
// methods, including iterators defined after this line (the tests'
// `Countdown`) and the two adapters above, which is what lets calls chain.
impl<I: Iterator> IterExt for I {}

// ---- Part B — methods for every string ------------------------------------

// Declared once, implemented once, for `str` itself. Method calls auto-deref
// the receiver down to `str` (`String`, `&String`, `Box<str>`, `Rc<str>`)
// and then auto-ref it to the `&str` that `&self` expects, so every string
// type reaches this impl, and `StrExt::is_blank("")` finds `Self = str`.
trait StrExt {
    fn is_blank(&self) -> bool;
    fn truncate_ellipsis(&self, max_chars: usize) -> String;
}

impl StrExt for str {
    // True for "" and for text made only of whitespace (Unicode whitespace
    // included).
    fn is_blank(&self) -> bool {
        self.chars().all(char::is_whitespace)
    }

    // Keeps the first `max_chars` characters (chars, not bytes, so the cut
    // always lands on a char boundary) and marks the cut with "…". Text that
    // already fits comes back unchanged.
    fn truncate_ellipsis(&self, max_chars: usize) -> String {
        match self.char_indices().nth(max_chars) {
            Some((cut, _)) => format!("{}…", &self[..cut]),
            None => self.to_string(),
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::rc::Rc;

    // Deliberately neither `Clone` nor `Copy`.
    #[derive(Debug, PartialEq)]
    struct Token(u32);

    // The other way around: `Clone`, but deliberately not `PartialEq`.
    #[derive(Clone)]
    struct Frame(u8);

    // An iterator type that the trait and its impl have never seen.
    struct Countdown(u32);

    impl Iterator for Countdown {
        type Item = u32;

        fn next(&mut self) -> Option<u32> {
            let current = self.0;
            self.0 = current.checked_sub(1)?;
            Some(current)
        }
    }

    // ---- Part A ----

    #[test]
    fn pairwise_on_a_range() {
        let pairs: Vec<_> = (1..=4).pairwise().collect();
        assert_eq!(pairs, [(1, 2), (2, 3), (3, 4)]);
    }

    #[test]
    fn pairwise_needs_at_least_two_items() {
        assert_eq!(std::iter::empty::<u8>().pairwise().next(), None);
        assert_eq!([1].into_iter().pairwise().next(), None);
    }

    #[test]
    fn pairwise_borrows_items_that_are_not_clone() {
        let tokens = [Token(1), Token(2), Token(3)];
        // `iter()` yields `&Token`, and a shared reference is `Clone` (it is
        // even `Copy`) when `Token` is not. Only pointers get copied.
        let pairs: Vec<(&Token, &Token)> = tokens.iter().pairwise().collect();
        assert_eq!(pairs.len(), 2);
        assert!(ptr::eq(pairs[0].1, &tokens[1]));
        assert!(ptr::eq(pairs[1].0, &tokens[1]));
        assert_eq!(pairs[1].1, &Token(3));
    }

    #[test]
    fn pairwise_needs_only_clone() {
        // Owned items that can be cloned but not compared: only
        // `dedup_adjacent` needs `PartialEq`.
        let frames = vec![Frame(1), Frame(2), Frame(3)];
        let pairs: Vec<(u8, u8)> = frames
            .into_iter()
            .pairwise()
            .map(|(a, b)| (a.0, b.0))
            .collect();
        assert_eq!(pairs, [(1, 2), (2, 3)]);
    }

    #[test]
    fn dedup_adjacent_collapses_runs() {
        let letters: String = "aaabccd".chars().dedup_adjacent().collect();
        assert_eq!(letters, "abcd");
        // Only NEIGHBORS merge, so a value may come back later.
        let numbers: Vec<_> = [1, 1, 2, 1, 1].into_iter().dedup_adjacent().collect();
        assert_eq!(numbers, [1, 2, 1]);
        assert_eq!(std::iter::empty::<char>().dedup_adjacent().next(), None);
    }

    #[test]
    fn dedup_adjacent_needs_only_partial_eq() {
        // Owned items that can't be cloned...
        let tokens = vec![Token(7), Token(7), Token(8)];
        let deduped: Vec<Token> = tokens.into_iter().dedup_adjacent().collect();
        assert_eq!(deduped, [Token(7), Token(8)]);
        // ...and `f64`, which is `PartialEq` but not `Eq`: NaN != NaN, so two
        // NaNs in a row are not a run.
        let nan = f64::NAN;
        assert_eq!([0.5, 0.5, nan, nan].into_iter().dedup_adjacent().count(), 3);
    }

    #[test]
    fn works_on_an_iterator_defined_elsewhere() {
        let pairs: Vec<_> = Countdown(3).pairwise().collect();
        assert_eq!(pairs, [(3, 2), (2, 1)]);
        let deduped: Vec<_> = Countdown(2).dedup_adjacent().collect();
        assert_eq!(deduped, [2, 1]);
    }

    #[test]
    fn extension_methods_chain() {
        // The adapters are iterators too, so the same impl covers them.
        let pairs: Vec<_> = "aabbbc".chars().dedup_adjacent().pairwise().collect();
        assert_eq!(pairs, [('a', 'b'), ('b', 'c')]);
        let steps: Vec<i32> = [1, 1, 4, 4, 9]
            .into_iter()
            .dedup_adjacent()
            .pairwise()
            .map(|(a, b)| b - a)
            .collect();
        assert_eq!(steps, [3, 5]);
    }

    #[test]
    fn the_adapters_stay_lazy() {
        let mut pulled = 0;
        let first_two: Vec<_> = (1..=1000)
            .inspect(|_| pulled += 1)
            .pairwise()
            .take(2)
            .collect();
        assert_eq!(first_two, [(1, 2), (2, 3)]);
        // Collecting everything up front would have pulled all 1000.
        assert_eq!(pulled, 3);

        let mut pulled = 0;
        let first_two: Vec<_> = (1..=1000)
            .inspect(|_| pulled += 1)
            .dedup_adjacent()
            .take(2)
            .collect();
        assert_eq!(first_two, [1, 2]);
        // `dedup_adjacent` peeks one item ahead to see where a run ends.
        assert_eq!(pulled, 3);
    }

    #[test]
    fn callable_through_the_trait_name() {
        // Fully qualified syntax: how you call an extension method when
        // another trait in scope has a method of the same name (E0034).
        let pairs: Vec<(u8, u8)> = IterExt::pairwise([1, 2].into_iter()).collect();
        assert_eq!(pairs, [(1, 2)]);
        let deduped: Vec<u8> = IterExt::dedup_adjacent([3, 3].into_iter()).collect();
        assert_eq!(deduped, [3]);
        assert!(StrExt::is_blank(""));
        assert_eq!(StrExt::truncate_ellipsis("abc", 1), "a…");
    }

    // ---- Part B ----

    #[test]
    fn is_blank_means_empty_or_whitespace() {
        assert!("".is_blank());
        assert!(" \t\r\n".is_blank());
        // U+3000 IDEOGRAPHIC SPACE is whitespace too.
        assert!("\u{3000}".is_blank());
        assert!(!" x ".is_blank());
        assert!(!"_".is_blank());
    }

    #[test]
    fn works_on_every_string_type() {
        let owned = String::from("  ");
        assert!(owned.is_blank());
        let borrowed: &String = &owned;
        assert!(borrowed.is_blank());
        let boxed: Box<str> = Box::from("nope");
        assert!(!boxed.is_blank());
        let shared: Rc<str> = Rc::from("\n");
        assert!(shared.is_blank());
        assert_eq!(owned.truncate_ellipsis(1), " …");
        assert_eq!(boxed.truncate_ellipsis(2), "no…");
        assert_eq!(shared.truncate_ellipsis(1), "\n");
    }

    #[test]
    fn truncate_counts_chars_not_bytes() {
        assert_eq!("héllo wörld".truncate_ellipsis(2), "hé…");
        assert_eq!("日本語".truncate_ellipsis(1), "日…");
        assert_eq!("héllo".truncate_ellipsis(4), "héll…");
    }

    #[test]
    fn truncate_leaves_short_text_alone() {
        assert_eq!("héllo".truncate_ellipsis(5), "héllo");
        assert_eq!("hi".truncate_ellipsis(10), "hi");
        assert_eq!("".truncate_ellipsis(0), "");
        assert_eq!("abc".truncate_ellipsis(0), "…");
    }
}
