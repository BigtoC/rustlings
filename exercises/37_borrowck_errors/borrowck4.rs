// Module 1 · Borrow-checker errors — part 4: no downgrade, and reborrowing `&mut` (E0502, E0382).
//
// `24_ownership_model/ownership3` gave you the core rule: at any moment a value
// has EITHER any number of shared borrows (`&T`) OR exactly one exclusive borrow
// (`&mut T`). This exercise is about two consequences of that rule that surprise
// people who "know" it. Interviewers like them because the rule alone does not
// answer them: "Why can't I call `len()` while holding the `&str` that `push`
// gave me?" and "Why can I pass the same `&mut String` to one function twice,
// but not to another?"
//
// Part A: there is no "downgrade". By the elision rules (the `&self` rule from
// `25_lifetimes_deep/lifetimes6`),
//
//     fn push(&mut self, entry: String) -> &str
//
// means `fn push<'a>(&'a mut self, entry: String) -> &'a str`. The result is
// valid only while the EXCLUSIVE borrow `'a` is, so as long as you hold the
// returned `&str`, the whole `Log` stays mutably borrowed. That is true even
// though the `&str` is read-only and `push` has already returned. Any other use
// of the `Log`, even a `&self` method like `len()`, overlaps with it (E0502).
// The borrow checker only reads the SIGNATURE, and the signature cannot say
// "the exclusive part is over now". It has to be that way. `Cell::get_mut`
// takes `&mut self` and gives out a reference that is only sound because the
// cell is exclusively borrowed:
//
//     fn peek(c: &mut Cell<i32>) -> &i32 { c.get_mut() }
//
//     let r = peek(&mut c);
//     c.set(5);        // a `&self` method: E0502, and rightly so, because...
//     println!("{r}"); // ...with a downgrade, `*r` would change under a `&i32`
//
// So this is not a borrow-checker imprecision (like the ones NLL removed), and
// Polonius, which accepts `37_borrowck_errors/borrowck3`, rejects this code
// too. It is what the signature promises. The fix is in the design: a mutating
// method should not hand back a borrow when callers will want to keep reading
// the object. Mutate in one call, then read through `&self` accessors. Their
// shared borrows can all coexist.
//
// Part B: `&mut T` is not `Copy`. A shared `&T` is `Copy`, because copying it
// just makes another shared borrow, which the rule allows. Copying a `&mut T`
// would make TWO live exclusive references to the same place, so `&mut T` is
// neither `Copy` nor `Clone`, and using one by value MOVES it. Then why can you
// call `fn f(x: &mut String)` twice with the same `buf: &mut String`? Because
// the compiler quietly rewrites `f(buf)` into `f(&mut *buf)`. That is a
// REBORROW: a new, shorter-lived exclusive reference derived from `*buf`. While
// the reborrow is alive, `buf` itself is frozen (unusable), so there is still
// only one usable exclusive path at any moment. When the call returns, `buf`
// works again. That is why callers of `f` never notice.
//
// That implicit reborrow is done by coercion, and rustc only inserts it where
// the expected type is already known to be a `&mut` reference: `x: &mut String`,
// and even `x: &mut T` with a generic `T`, qualify. A parameter whose WHOLE type
// is generic, like `out: W`, does not. Its type is an inference variable until
// the compiler picks `W = &mut String`, so there is nothing to coerce to. `buf`
// is moved into the first call, and the second call finds a moved value (E0382).
// The same thing happens with `let a = buf;`, which moves, while
// `let a: &mut String = buf;` reborrows. Taking writers by value (`W: Write`)
// is the Rust API Guidelines convention (C-RW-VALUE), and it works because
// `&mut W` is itself a writer. So callers of such APIs who want to keep their
// reference write the reborrow themselves. `Iterator::by_ref()` and
// `io::Write::by_ref()` exist for exactly this; `fmt::Write` has no `by_ref`.

use std::fmt::{self, Write};

// ---------- Part A: a mutating method that returns a borrow ----------

// Deliberately NOT `Clone`: copying the whole log is not an escape hatch.
struct Log {
    entries: Vec<String>,
}

impl Log {
    fn new() -> Self {
        Log {
            entries: Vec::new(),
        }
    }

    // Stores `entry` and hands back a view of the stored text.
    fn push(&mut self, entry: String) -> &str {
        let index = self.entries.len();
        self.entries.push(entry);
        &self.entries[index]
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    // Read-only view of the newest entry, if there is one.
    fn last(&self) -> Option<&str> {
        self.entries.last().map(String::as_str)
    }
}

// Stores `entry` and reports it together with its 1-based position,
// e.g. "#2: ready".
fn push_and_report(log: &mut Log, entry: &str) -> String {
    // TODO: E0502 "cannot borrow `*log` as immutable because it is also
    // borrowed as mutable". `stored` looks like a harmless shared `&str`, yet
    // `log.len()` is rejected while `stored` is alive. Work out which borrow
    // `stored` is really keeping alive, then fix the DESIGN rather than the
    // call site. `Log::push` is yours to change. When you are done, this
    // function must be able to hold a shared view of the stored entry AND call
    // `log.len()` at the same time.
    // Constraints: don't copy the stored text out of the log (no
    // `.to_string()`, `.to_owned()` or `.clone()` of it, inside `push` or
    // here), no leaking, no `unsafe`, no `RefCell`, keep this function's
    // signature, and don't change the tests.
    // Until you stop the stored entry from pinning the exclusive borrow, this
    // exercise will not compile.
    let stored = log.push(entry.to_string());
    format!("#{}: {stored}", log.len())
}

// ---------- Part B: passing a `&mut` to a generic parameter ----------

// Writes every word to `out`. The writer is taken BY VALUE, as the API
// Guidelines recommend. Any `W: Write` works, and so does `&mut W`, because
// std has `impl<W: Write + ?Sized> Write for &mut W`.
fn append_all<W: Write>(mut out: W, words: &[&str]) -> fmt::Result {
    for word in words {
        out.write_str(word)?;
    }
    Ok(())
}

// Appends "abc" to the caller's buffer, in two batches.
fn build(buf: &mut String) -> fmt::Result {
    // TODO: E0382 "use of moved value: `buf`". rustc's `help:` line even
    // shows the fix. The interview question is WHY it is needed: passing
    // `buf` to a `fn f(x: &mut String)` twice compiles, but passing it to
    // `append_all` twice does not. Once you can explain the difference, fix
    // `build` so that both calls write into the caller's buffer.
    // Constraints: don't change `append_all` (it takes its writer by value
    // on purpose; one of the tests depends on that), keep `build`'s signature
    // and both `append_all` calls, no `.clone()`, no temporary or swapped-out
    // `String` (no `mem::take`), no `unsafe`, and don't change the tests.
    // Until you stop the first call from consuming `buf`, this exercise will
    // not compile.
    append_all(buf, &["a", "b"])?;
    append_all(buf, &["c"])
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Part A -----

    #[test]
    fn reports_each_entry_with_its_position() {
        let mut log = Log::new();
        assert_eq!(push_and_report(&mut log, "boot"), "#1: boot");
        assert_eq!(push_and_report(&mut log, "ready"), "#2: ready");
        assert_eq!(log.len(), 2);
        assert_eq!(log.last(), Some("ready"));
    }

    #[test]
    fn position_counts_entries_pushed_directly_too() {
        // The number in the report is the length AFTER the push, whatever
        // was in the log before.
        let mut log = Log::new();
        log.push("a".to_string());
        log.push("b".to_string());
        assert_eq!(push_and_report(&mut log, "c"), "#3: c");
        assert_eq!(log.last(), Some("c"));
    }

    #[test]
    fn entries_are_stored_and_reported_verbatim() {
        // Empty text and text that looks like a format string are just data.
        let mut log = Log::new();
        assert_eq!(push_and_report(&mut log, ""), "#1: ");
        assert_eq!(push_and_report(&mut log, "{}"), "#2: {}");
        assert_eq!(log.len(), 2);
        assert_eq!(log.last(), Some("{}"));
    }

    #[test]
    fn empty_log_has_no_last_entry() {
        let log = Log::new();
        assert_eq!(log.len(), 0);
        assert_eq!(log.last(), None);
    }

    #[test]
    fn two_shared_borrows_coexist() {
        let mut log = Log::new();
        push_and_report(&mut log, "boot");
        // `last` is a shared borrow of `log` that is still alive while `len()`
        // takes a second shared borrow. Shared + shared is always fine; only
        // an EXCLUSIVE borrow must be alone.
        let last = log.last();
        let len = log.len();
        assert_eq!(last, Some("boot"));
        assert_eq!(len, 1);
    }

    // ----- Part B -----

    // A writer adapter that is passed BY VALUE. It holds its own `&mut String`
    // and upper-cases everything written through it.
    struct Shout<'a>(&'a mut String);

    impl Write for Shout<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.0.push_str(&s.to_uppercase());
            Ok(())
        }
    }

    #[test]
    fn build_writes_every_word() {
        let mut buf = String::new();
        build(&mut buf).unwrap();
        assert_eq!(buf, "abc");
    }

    #[test]
    fn build_appends_and_the_buffer_stays_usable() {
        // `build` only borrowed our `String`. We still own it, and what was
        // there before is kept.
        let mut buf = String::from(">");
        build(&mut buf).unwrap();
        buf.push('!');
        assert_eq!(buf, ">abc!");
    }

    #[test]
    fn callers_of_a_mut_ref_parameter_reborrow_implicitly() {
        let mut buf = String::new();
        let r = &mut buf;
        // `build` takes a `&mut String`, so each call is silently
        // `build(&mut *r)`. `r` is reborrowed, not moved, and works again after
        // each call.
        build(r).unwrap();
        build(r).unwrap();
        r.push('.');
        assert_eq!(buf, "abcabc.");
    }

    #[test]
    fn append_all_takes_any_writer_by_value() {
        let mut buf = String::new();
        // An adapter value, not a `&mut`. This only compiles because
        // `append_all` takes `W` by value.
        append_all(Shout(&mut buf), &["a", "b"]).unwrap();
        // `&mut buf` borrows the OWNED `buf` afresh each time, so passing it
        // twice is fine. Nothing is moved out of `buf`.
        append_all(&mut buf, &["c"]).unwrap();
        append_all(&mut buf, &[]).unwrap();
        assert_eq!(buf, "ABc");
    }
}
