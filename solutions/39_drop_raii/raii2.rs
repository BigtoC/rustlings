// Module 1 · Drop and RAII — part 4: a transaction that rolls back in `Drop` unless committed.
//
// Part 3's guard runs any cleanup. Its most common concrete form, in live
// coding rounds and in production, is a TRANSACTION: make some edits, then
// either `commit()` them or have them undone, automatically, however the
// function is left. `rusqlite::Transaction` and `sqlx::Transaction` both roll
// back in `Drop`, and std's own `Vec::retain` uses a drop guard so that a
// panicking predicate cannot leave the vector with holes in it.
//
// The design below edits the store IN PLACE (so the edits are visible at
// once, through the transaction) and keeps a snapshot of the state from
// `begin`, to go back to. `Drop` is the one place that sees every way out of
// the function: the end of a scope, an early `return`, a `?`, and a panic
// that unwinds. So `Drop` is where the rollback belongs. A `rollback()` method
// that callers have to remember to call is exactly the bug that RAII exists
// to prevent.
//
// The interesting part is `commit`. It takes the transaction BY VALUE, so the
// caller cannot touch it afterwards, but `commit` then owns it, and a value is
// dropped when its owner goes away (part 1): here, when `commit` returns. Only
// leaking the value would skip its `drop`. So `commit` cannot skip `drop`; it
// can only leave `drop` a note that says "keep the edits". Writing a field of
// an owned `self` is allowed in a `Drop` type (only MOVING a field out is not;
// that is E0509 from `24_ownership_model/ownership6`).
//
// Two more things interviewers check. A `drop` must not panic: if it panics
// while another panic is already unwinding, the process aborts. And the
// rollback must run during unwinding too, which the tests check inside
// `catch_unwind`. (Test binaries unwind even though this course's profiles
// set `panic = "abort"`: Cargo ignores that setting for test targets. In
// `main`, a panic would abort, and no destructor would run at all.)
//
// How interviewers probe it: "Write a transaction guard that rolls back unless
// committed", "What happens on `?`? On a panic?", "Why does `commit` take
// `self` by value?", and "Can `drop` fail?" (it cannot return an error, which
// is why types like `BufWriter` also offer an explicit, fallible method: see
// the README).

use std::mem;

// An all-or-nothing batch of edits to a table of rows (just `i32`s here).
struct Tx<'a> {
    store: &'a mut Vec<i32>,
    // The rows as they were when the transaction began.
    snapshot: Vec<i32>,
    committed: bool,
}

impl<'a> Tx<'a> {
    fn begin(store: &'a mut Vec<i32>) -> Self {
        let snapshot = store.clone();
        Tx {
            store,
            snapshot,
            committed: false,
        }
    }

    fn push(&mut self, row: i32) {
        self.store.push(row);
    }

    fn clear(&mut self) {
        self.store.clear();
    }

    // The rows as the transaction sees them, edits included.
    fn rows(&self) -> &[i32] {
        self.store
    }

    // Makes the edits permanent.
    fn commit(mut self) {
        // `drop` runs on `self` when this method returns, so `commit` leaves it
        // a note instead of trying to skip it. ASSIGNING a field of a `Drop`
        // type is fine; only moving one out is E0509. `mut self` changes only
        // the local binding, not the signature callers see.
        self.committed = true;
    }
}

// `drop` runs on every way out of the scope that owns the `Tx`, including a
// `?` and a panic that unwinds, so the rollback cannot be forgotten. The
// snapshot is never needed again, so it is MOVED back instead of copied:
// `mem::swap` exchanges the two `Vec` headers (the store gets the snapshot's
// buffer, and the edited rows are freed with the `Tx`). Nothing in here can
// panic, which matters, because a panic in `drop` during unwinding aborts.
impl Drop for Tx<'_> {
    fn drop(&mut self) {
        if !self.committed {
            mem::swap(self.store, &mut self.snapshot);
        }
    }
}

// Parses every line and appends it to `store`, all or nothing: the first line
// that does not parse returns the error, and the store is left as it was.
fn import(store: &mut Vec<i32>, lines: &[&str]) -> Result<(), std::num::ParseIntError> {
    let mut tx = Tx::begin(store);
    for line in lines {
        tx.push(line.trim().parse()?);
    }
    tx.commit();
    Ok(())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{self, AssertUnwindSafe};

    #[test]
    fn edits_are_visible_inside_the_transaction() {
        let mut store = vec![1];
        let mut tx = Tx::begin(&mut store);
        tx.push(2);
        assert_eq!(tx.rows(), [1, 2]);
        tx.commit();
    }

    #[test]
    fn dropping_without_commit_rolls_back() {
        let mut store = vec![1];
        let mut tx = Tx::begin(&mut store);
        tx.push(2);
        drop(tx);
        assert_eq!(store, [1], "an uncommitted transaction must roll back");
    }

    #[test]
    fn commit_keeps_the_edits() {
        let mut store = vec![1];
        let mut tx = Tx::begin(&mut store);
        tx.push(2);
        tx.push(3);
        tx.commit();
        // Reading `store` here compiles only because `commit` consumed `tx`.
        // A `commit(&mut self)` would leave `tx` alive until the end of the
        // test, and its `drop` would still need the `&mut store` borrow then
        // (E0502).
        assert_eq!(store, [1, 2, 3]);
    }

    #[test]
    fn rollback_restores_rows_that_were_removed() {
        let mut store = vec![1, 2, 3];
        {
            let mut tx = Tx::begin(&mut store);
            tx.clear();
            tx.push(9);
            assert_eq!(tx.rows(), [9]);
        }
        // Undoing the pushes is not enough: the cleared rows come back too.
        assert_eq!(store, [1, 2, 3]);
    }

    #[test]
    fn a_failed_import_leaves_the_store_unchanged() {
        let mut store = vec![1];
        // The `?` on "three" leaves `import` early, before `commit`.
        assert!(import(&mut store, &["2", "three", "4"]).is_err());
        assert_eq!(store, [1]);
    }

    #[test]
    fn a_successful_import_is_committed() {
        let mut store = vec![1];
        assert_eq!(import(&mut store, &["2", " 3 "]), Ok(()));
        assert_eq!(store, [1, 2, 3]);
        assert_eq!(import(&mut store, &[]), Ok(()));
        assert_eq!(store, [1, 2, 3]);
    }

    #[test]
    fn a_panic_rolls_back() {
        let mut store = vec![1];
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut tx = Tx::begin(&mut store);
            tx.push(2);
            panic!("simulated crash in the middle of a transaction");
        }));
        assert!(result.is_err());
        assert_eq!(store, [1]);
    }

    #[test]
    fn each_transaction_rolls_back_to_its_own_start() {
        let mut store = vec![1];
        let mut first = Tx::begin(&mut store);
        first.push(2);
        first.commit();

        let mut second = Tx::begin(&mut store);
        second.push(3);
        drop(second);
        assert_eq!(store, [1, 2]);
    }

    #[test]
    fn an_empty_transaction_changes_nothing() {
        let mut store = vec![4, 5];
        Tx::begin(&mut store).commit();
        assert_eq!(store, [4, 5]);
        drop(Tx::begin(&mut store));
        assert_eq!(store, [4, 5]);
    }
}
