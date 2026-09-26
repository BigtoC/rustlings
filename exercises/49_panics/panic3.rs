// Traits & Abstraction · Panics — part 3: panic safety: do everything that can panic first, then commit.
//
// Part 2 repaired a broken invariant after the fact. It is better never to
// leave one behind. A method is PANIC SAFE (C++ says "exception safe") when a
// panic in the middle of it leaves the value in a state that its invariants
// allow. C++ names three levels, and interviewers borrow the words:
//   - the basic guarantee: the invariants hold and nothing leaks, but the
//     operation may be partly done;
//   - the strong guarantee: all or nothing. If it panics, the value is
//     exactly as it was before the call;
//   - the no-throw guarantee (in Rust, no-panic): it cannot fail at all.
//
// In safe Rust a panic can never cause undefined behavior, so panic safety is
// about logic: a half-done update becomes visible to someone through
// `catch_unwind` with `AssertUnwindSafe` (part 1), through a poisoned lock
// that is recovered (part 2), or through a `Drop` that reads the value while
// the panic unwinds. In `unsafe` code it is about memory safety, and std's
// own algorithms are written for it: `Vec::retain` keeps a drop guard, so a
// predicate that panics halfway cannot leave the vector with holes or
// duplicated elements in it (the basic guarantee). The Rustonomicon's
// "Exception Safety" chapter walks through the same technique in
// `BinaryHeap::sift_up` (a `Hole` guard).
//
// Panics hide in more places than `panic!`: the caller's closure (here `f`),
// arithmetic in a debug build ("attempt to add with overflow",
// `31_debugging/debugging2`), indexing, `unwrap` and `expect`, a `RefCell`
// that is already borrowed. The recipe for the strong guarantee is simple:
// do all the work that can panic on the side, in locals, and only then
// COMMIT, with steps that cannot panic (moves and assignments). The other
// recipe mutates in place and has a guard put a snapshot back while the
// panic unwinds (`39_drop_raii/raii2`): the same one copy of the entries,
// plus a guard to get right. And one tempting shortcut is worse than the
// bug: taking the vector out with `mem::take` to transform it loses the
// whole thing if the transformation panics.
//
// `apply` below updates the entries in place and keeps a running total. If
// `f` panics on the second entry, the first one is already converted and the
// total covers only that one: after a caught panic, the entries are
// `[10, 2, 3]` and the total is 10.
//
// How interviewers probe it: "Rust has no exceptions, so what is exception
// safety in Rust?", "Can a panic cause undefined behavior in safe code? In
// unsafe code?", "What is the difference between the basic and the strong
// guarantee?", and "How does `Vec::retain` stay valid if the predicate
// panics?"

/// A record of amounts with a cached total.
/// Invariant: `total == entries.iter().sum()`.
#[derive(Debug)]
struct Ledger {
    entries: Vec<i64>,
    total: i64,
}

impl Ledger {
    /// Panics if the total does not fit in an `i64` (in a build with overflow
    /// checks, like these tests; `sum` wraps where they are off).
    fn new(entries: Vec<i64>) -> Self {
        let total = entries.iter().sum();
        Ledger { entries, total }
    }

    fn entries(&self) -> &[i64] {
        &self.entries
    }

    fn total(&self) -> i64 {
        self.total
    }

    fn is_consistent(&self) -> bool {
        self.total == self.entries.iter().sum::<i64>()
    }

    /// Replaces every entry `e` with `f(e)` (a currency conversion, a fee, a
    /// correction), calling `f` once per entry, in order.
    ///
    /// All or nothing: if `f` panics, or if the new total does not fit in an
    /// `i64` (that is a panic too), the panic reaches the caller and the
    /// ledger is exactly as it was before the call.
    fn apply(&mut self, mut f: impl FnMut(i64) -> i64) {
        // TODO: `a_panic_in_f_changes_nothing` fails: after `f` panics on the
        // second entry, the entries are `[10, 2, 3]` and the total is 10, so
        // the ledger is half converted AND its invariant is broken.
        // `an_overflowing_total_changes_nothing` fails the same way. Give
        // `apply` the strong guarantee: while anything can still panic (`f`,
        // the additions), leave `self` untouched; write `self` only once
        // nothing can panic any more, and write both fields then. Call `f`
        // exactly once per entry, in order, and let its panic reach the
        // caller unchanged: no `catch_unwind` in here, no `mem::take` of the
        // entries, no copy of the old entries to roll back to, no wrapping or
        // saturating arithmetic, no `unsafe`, and don't change the tests.
        // Until a panic in `apply` leaves the ledger as it was, the tests will
        // fail.
        self.total = 0;
        for entry in &mut self.entries {
            *entry = f(*entry);
            self.total += *entry;
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::{Mutex, PoisonError};

    // Converts at a fixed rate, but has no rate for the amount 2.
    fn convert(amount: i64) -> i64 {
        if amount == 2 {
            panic!("no rate for {amount}");
        }
        amount * 10
    }

    // Runs `apply` and returns the panic message, if it panicked.
    fn try_apply(ledger: &mut Ledger, f: impl FnMut(i64) -> i64) -> Option<String> {
        let result = panic::catch_unwind(AssertUnwindSafe(|| ledger.apply(f)));
        result
            .err()
            .map(|payload| match payload.downcast::<String>() {
                Ok(message) => *message,
                Err(payload) => match payload.downcast::<&'static str>() {
                    Ok(message) => message.to_string(),
                    Err(_) => String::from("a non-string payload"),
                },
            })
    }

    #[test]
    fn apply_maps_every_entry() {
        let mut ledger = Ledger::new(vec![1, 3, 4]);
        ledger.apply(convert);
        assert_eq!(ledger.entries(), [10, 30, 40]);
        assert_eq!(ledger.total(), 80);
        assert!(ledger.is_consistent());
    }

    #[test]
    fn f_is_called_once_per_entry_in_order() {
        let mut ledger = Ledger::new(vec![5, -1, 7]);
        let mut seen = Vec::new();
        ledger.apply(|amount| {
            seen.push(amount);
            amount + 1
        });
        assert_eq!(seen, [5, -1, 7]);
        assert_eq!(ledger.entries(), [6, 0, 8]);
        assert_eq!(ledger.total(), 14);
    }

    #[test]
    fn a_panic_in_f_changes_nothing() {
        let mut ledger = Ledger::new(vec![1, 2, 3]);
        let message = try_apply(&mut ledger, convert);
        assert!(message.is_some(), "the panic must reach the caller");
        assert_eq!(
            (ledger.entries(), ledger.total()),
            (&[1, 2, 3][..], 6),
            "after a caught panic, the ledger must be exactly as it was"
        );
    }

    #[test]
    fn the_panic_reaches_the_caller_unchanged() {
        let mut ledger = Ledger::new(vec![1, 2, 3]);
        assert_eq!(
            try_apply(&mut ledger, convert),
            Some(String::from("no rate for 2"))
        );
    }

    #[test]
    fn a_panic_on_the_last_entry_changes_nothing() {
        let mut ledger = Ledger::new(vec![4, 5, 2]);
        let mut calls = 0;
        let message = try_apply(&mut ledger, |amount| {
            calls += 1;
            convert(amount)
        });
        assert_eq!(message, Some(String::from("no rate for 2")));
        assert_eq!(calls, 3);
        assert_eq!((ledger.entries(), ledger.total()), (&[4, 5, 2][..], 11));
    }

    #[test]
    fn an_overflowing_total_changes_nothing() {
        // Each converted entry fits in an `i64`; their sum does not.
        let mut ledger = Ledger::new(vec![1, 2]);
        let message = try_apply(&mut ledger, |amount| amount * (i64::MAX / 2));
        assert!(message.is_some(), "a total that overflows must panic");
        assert_eq!(
            (ledger.entries(), ledger.total()),
            (&[1, 2][..], 3),
            "an overflowing total must not leave a half-written ledger either"
        );
    }

    #[test]
    fn an_empty_ledger_stays_empty() {
        let mut ledger = Ledger::new(Vec::new());
        ledger.apply(|_| panic!("f must not be called for an empty ledger"));
        assert!(ledger.entries().is_empty());
        assert_eq!(ledger.total(), 0);
    }

    #[test]
    fn the_ledger_still_works_after_a_failed_apply() {
        let mut ledger = Ledger::new(vec![1, 2, 3]);
        assert!(try_apply(&mut ledger, convert).is_some());
        ledger.apply(|amount| amount + 1);
        assert_eq!(ledger.entries(), [2, 3, 4]);
        assert_eq!(ledger.total(), 9);
    }

    #[test]
    fn a_panic_safe_update_leaves_nothing_to_repair() {
        // The payoff for part 2: a panic inside the lock still poisons the
        // mutex, but a panic-safe `apply` left nothing half done, so the data
        // behind the poison is sound as it is.
        let ledger = Mutex::new(Ledger::new(vec![1, 2, 3]));
        let result = panic::catch_unwind(|| ledger.lock().unwrap().apply(convert));
        assert!(result.is_err());
        assert!(ledger.is_poisoned());
        let guard = ledger.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(
            guard.is_consistent(),
            "behind the poison, the ledger must still be consistent"
        );
        assert_eq!(guard.entries(), [1, 2, 3]);
    }
}
