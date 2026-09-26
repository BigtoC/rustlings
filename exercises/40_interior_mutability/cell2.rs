// Module 1 · Interior mutability — part 2: memoizing with `OnceCell`, lending out a plain `&T`.
//
// Part 1's `Cell` never lends out a reference: values only move in and out.
// `RefCell` does lend one, behind a guard (`Ref` / `RefMut`) that counts
// borrows at run time. `OnceCell<T>` (stable since Rust 1.70) sits in
// between: it is written at most ONCE through `&self`, and after that it
// lends out plain `&T` borrows, valid for as long as the cell is borrowed.
// No guard, no borrow flag.
//
// Why is that sound? Once the cell holds a value, no `&self` method can
// change or remove it:
//
//   - `get()` returns `Option<&T>`;
//   - `set(value)` on a full cell fails and gives your value back as
//     `Err(value)`;
//   - `get_or_init(f)` runs `f` only while the cell is empty, stores the
//     result, and returns `&T` either way.
//
// The only ways to change or clear a full cell take `&mut self` (`take`,
// `get_mut`, or assigning a fresh `OnceCell::new()`), and the borrow checker
// already proves that no `&T` lent out by the cell is alive then.
//
// Compare `RefCell<Option<String>>`: a `&self` method that returns `&str`
// from it is E0515 "cannot return value referencing temporary value" (or
// "... local variable" once you name the guard), because the `&str` borrows
// the `Ref` guard, and the guard dies when the method returns. You would have
// to return the guard itself (`Ref<'_, str>`), and any `borrow_mut` made
// while a caller still holds one panics.
//
// That makes `OnceCell` the tool for MEMOIZATION: compute an expensive value
// on first use and cache it inside a type whose methods take `&self`. Three
// details are worth knowing:
//
//   - Order matters. Compute-then-`set` (the bug in `summary` below) does the
//     expensive work on every call, then throws the result away.
//     `get_or_init` checks first.
//   - Re-entrancy. Using OTHER cells inside `f` is fine: `summary` below
//     reuses the cached word count. Filling the SAME cell from inside its
//     own `f` is a bug: if the nested call fills it, the outer `get_or_init`
//     panics with "reentrant init" once `f` returns, and a getter that just
//     calls itself finds the cell still empty every time and recurses until
//     the stack overflows. If `f` panics, the cell stays empty, and the next
//     call tries again.
//   - Invalidation. A cache is only correct while its inputs are unchanged.
//     Here the text only changes through `&mut self`, so clearing the cache
//     needs no interior mutability at all.
//
// Why not `LazyCell` (Rust 1.80), which stores the initializer next to the
// value? Its closure is fixed when the struct is built, so it cannot borrow
// the struct's own `text`. A `get_or_init` closure is created at the call,
// where `&self` is right there. `OnceCell`, like `Cell`, is `!Sync`: part 3
// covers `OnceLock`, its thread-safe twin.
//
// How interviewers probe this: "Memoize an expensive getter without `&mut
// self`", "Why can `OnceCell` return `&T` when `RefCell` needs a guard?",
// "What happens if the initializer calls the getter again?", "How do you
// invalidate the cache?".

use std::cell::{Cell, OnceCell};

struct Doc {
    text: String,
    word_count: OnceCell<usize>,
    summary: OnceCell<String>,
    // Test instrumentation: how many times each expensive step really ran.
    counted: Cell<u32>,
    summarized: Cell<u32>,
}

impl Doc {
    fn new(text: &str) -> Self {
        Doc {
            text: text.to_string(),
            word_count: OnceCell::new(),
            summary: OnceCell::new(),
            counted: Cell::new(0),
            summarized: Cell::new(0),
        }
    }

    // The two expensive steps. Pretend each one takes a while. Don't change
    // them: the tests count how often they run.
    fn count_words(&self) -> usize {
        self.counted.update(|n| n + 1);
        self.text.split_whitespace().count()
    }

    fn build_summary(&self) -> String {
        self.summarized.update(|n| n + 1);
        let title = self.text.lines().next().unwrap_or("").trim();
        format!("{title} ({} words)", self.word_count())
    }

    // ---- Part A — memoize a `Copy` value ------------------------------------

    fn word_count(&self) -> usize {
        // TODO: `word_count_is_computed_once` fails with `left: 3, right: 1`:
        // every call counts the words again, and the `word_count` cell is
        // never used. Count the words on the FIRST call only and cache the
        // result in `self.word_count`. Requirements: keep `&self`; count
        // lazily (nothing up front in `new`); a cached `0` is a valid answer,
        // not "not computed yet"; each `Doc` has its own cache (no `static`,
        // no `thread_local!`); keep calling `count_words` for the real work.
        // Until the words are counted once per text, the tests will fail.
        self.count_words()
    }

    // ---- Part B — memoize a `String` and lend it out ------------------------

    // "<first line> (<N> words)", built once and then lent out as a `&str`.
    fn summary(&self) -> &str {
        // TODO: `summary_is_built_once_and_lent_out` fails: the summary is
        // built on EVERY call, and then `set` refuses it because the cell is
        // already full (compute-then-set). Build it only while the cell is
        // empty, and return a borrow of the `String` cached in
        // `self.summary`. Requirements: keep the signature (a plain `&str`,
        // no guard type); keep calling `build_summary` for the real work; no
        // leaking (`Box::leak`, `String::leak`), no `RefCell`, no `unsafe`.
        // Until the summary is built once per text, the tests will fail.
        let fresh = self.build_summary();
        let _ = self.summary.set(fresh);
        self.summary.get().expect("the cell was just set")
    }

    // ---- Part C — invalidate ------------------------------------------------

    fn set_text(&mut self, text: String) {
        // TODO: once Parts A and B cache, the test
        // `set_text_throws_the_stale_caches_away` fails: the old count and
        // summary survive the new text. Throw BOTH cached values away here.
        // Requirements: stay lazy (compute nothing in `set_text`: the next
        // `word_count()` / `summary()` call does); leave the instrumentation
        // counters alone (no `*self = Doc::new(..)`); you have `&mut self`,
        // so this needs no interior mutability. Until a new text starts with
        // empty caches, the tests will fail.
        self.text = text;
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn nothing_is_computed_up_front() {
        let doc = Doc::new("lazy by default");
        assert_eq!(doc.counted.get(), 0);
        assert_eq!(doc.summarized.get(), 0);
    }

    // ---- Part A ----

    #[test]
    fn word_count_is_computed_once() {
        let doc = Doc::new("the quick brown fox");
        assert_eq!(doc.word_count(), 4);
        assert_eq!(doc.word_count(), 4);
        assert_eq!(doc.word_count(), 4);
        assert_eq!(
            doc.counted.get(),
            1,
            "three calls must count the words once"
        );
    }

    #[test]
    fn a_cached_zero_is_still_cached() {
        // An empty text has 0 words. "0 means not computed yet" would count
        // it again on every call.
        let doc = Doc::new("  \n  ");
        assert_eq!(doc.word_count(), 0);
        assert_eq!(doc.word_count(), 0);
        assert_eq!(doc.counted.get(), 1, "a cached 0 must not be recomputed");
    }

    #[test]
    fn each_doc_has_its_own_cache() {
        let a = Doc::new("one two three");
        let b = Doc::new("four five");
        assert_eq!(a.word_count(), 3);
        assert_eq!(b.word_count(), 2);
        assert_eq!(a.summary(), "one two three (3 words)");
        assert_eq!(b.summary(), "four five (2 words)");
    }

    // ---- Part B ----

    #[test]
    fn summary_is_built_once_and_lent_out() {
        let doc = Doc::new("Interior mutability\nCell, OnceCell and friends");
        let first = doc.summary();
        // `first` is still alive here: the cell lends out plain shared
        // borrows, so any number of them may coexist.
        let second = doc.summary();
        assert_eq!(first, "Interior mutability (6 words)");
        assert_eq!(doc.summarized.get(), 1, "two calls must build it once");
        assert!(
            ptr::eq(first, second),
            "both calls must lend out the same cached string"
        );
        let cached = doc.summary.get().map(String::as_str);
        assert!(
            cached.is_some_and(|cached| ptr::eq(cached, first)),
            "`summary()` must lend out the `String` cached in `self.summary`"
        );
    }

    #[test]
    fn summary_reuses_the_cached_word_count() {
        let doc = Doc::new("count me once");
        assert_eq!(doc.word_count(), 3);
        assert_eq!(doc.summary(), "count me once (3 words)");
        assert_eq!(doc.summary(), "count me once (3 words)");
        assert_eq!(doc.word_count(), 3);
        assert_eq!(doc.counted.get(), 1, "the words were counted again");
        assert_eq!(doc.summarized.get(), 1, "the summary was built again");
    }

    // ---- Part C ----

    #[test]
    fn set_text_throws_the_stale_caches_away() {
        let mut doc = Doc::new("draft one");
        assert_eq!(doc.summary(), "draft one (2 words)");
        assert_eq!(doc.word_count(), 2);
        assert_eq!(doc.counted.get(), 1, "count once, then reuse the cache");

        doc.set_text("Final version\nwith more words".to_string());
        assert_eq!(doc.word_count(), 5, "stale word count after `set_text`");
        assert_eq!(
            doc.summary(),
            "Final version (5 words)",
            "stale summary after `set_text`"
        );
        assert_eq!(doc.word_count(), 5);
        assert_eq!(doc.summary(), "Final version (5 words)");
        assert_eq!(doc.counted.get(), 2, "one count per text");
        assert_eq!(doc.summarized.get(), 2, "one summary per text");
    }

    #[test]
    fn set_text_computes_nothing_by_itself() {
        let mut doc = Doc::new("a b");
        assert_eq!(doc.summary(), "a b (2 words)");
        doc.set_text("a b c".to_string());
        assert_eq!(
            (doc.counted.get(), doc.summarized.get()),
            (1, 1),
            "`set_text` must compute nothing: the next getter call does"
        );
        assert_eq!(doc.word_count(), 3);
        assert_eq!(doc.counted.get(), 2);
    }
}
