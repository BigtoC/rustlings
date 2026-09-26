// Traits & Abstraction · Panics — part 2: a poisoned `Mutex`: recover with `into_inner`, repair the invariant, `clear_poison`.
//
// When a thread panics while it holds a `MutexGuard`, the guard's destructor
// still runs during unwinding, so the mutex is unlocked (RAII, as in
// `39_drop_raii`). But the guard notices that its thread is panicking, and it
// marks the mutex POISONED. From then on, every `lock()` returns
// `Err(PoisonError)`, and the usual `lock().unwrap()` turns one crash into
// many: "called `Result::unwrap()` on an `Err` value: PoisonError { .. }".
//
// Why does std do this? A critical section is exactly where an invariant is
// temporarily broken: the entry is pushed, the total is not updated yet. If
// the thread dies in between, the next thread would read a half-done update
// and trust it. Poisoning is the lock saying "the last thread in here did not
// finish". It is also why `&Mutex<T>` is `UnwindSafe` while `&RefCell<T>`
// is not (part 1): the lock reports the problem instead of hiding it.
//
// `lock().unwrap()` is the usual default, and it is usually right: a poisoned
// lock means there is a bug, and propagating the panic makes it visible. But
// a long-running service cannot let one crashed worker take down every thread
// that touches the same data, forever. Recovering is correct when you can
// restore, or check, the invariant from what is there (or when there is no
// invariant: a counter, a cache you can clear). It is wrong when you cannot
// tell what the crashed thread left half-done; then propagate the panic.
//
// The tools: a `PoisonError` still carries the guard, so the lock IS held
// while you look at it (drop the error, or the guard you get out of it,
// before you lock again). `PoisonError::into_inner` hands the guard back, and
// `get_ref` / `get_mut` let you look without consuming the error.
// `Mutex::clear_poison` (stable since Rust 1.77) marks the mutex as
// recovered; without it, every later `lock()` reports the same poison again,
// and `Mutex::is_poisoned` keeps saying `true`.
//
// The `Ledger` below records amounts. `entries` IS the record; `total` is
// only a cached sum, so reading the balance is O(1). A crash can leave them
// disagreeing, and there is no way to know in general which step the crashed
// thread had reached (a real critical section has many steps). So the repair
// is not to guess and undo "the last push", and certainly not to throw the
// ledger away: it is to rebuild what is derived (the total) from what is
// recorded (the entries). Every access goes through `lock_ledger`, so the
// policy lives in exactly one place.
//
// How interviewers probe it: "What is mutex poisoning, and why does std have
// it?", "When is it correct to recover with `into_inner`, and when is it
// wrong?", "Does a panic while holding an `RwLock` READ guard poison it?"
// (no: a reader has only shared access, so std assumes it changed nothing),
// and "What does `parking_lot` do instead?" (its locks do not poison at all).

use std::sync::{Mutex, MutexGuard};

/// An append-only record of amounts, with a cached total.
/// Invariant: `total == entries.iter().sum()`.
#[derive(Debug, Default)]
struct Ledger {
    entries: Vec<i64>,
    total: i64,
}

impl Ledger {
    fn is_consistent(&self) -> bool {
        self.total == self.entries.iter().sum::<i64>()
    }
}

/// Locks the ledger. Every function below goes through here.
fn lock_ledger(ledger: &Mutex<Ledger>) -> MutexGuard<'_, Ledger> {
    // TODO: the tests that crash a worker in the middle of an update fail
    // with "called `Result::unwrap()` on an `Err` value: PoisonError { .. }":
    // the worker died holding the lock, so the mutex is poisoned, and this
    // `unwrap` turns every later access into another panic. Recover instead:
    // get the guard back out of the error, repair the ledger's invariant from
    // its source of truth (keep every entry, including the one the crashed
    // worker recorded; rebuild what is derived from it), and mark the mutex
    // as recovered, so that later `lock()` calls succeed again and
    // `is_poisoned()` is `false`. Keep holding the lock the whole time:
    // unlocking and locking again would let another thread in before the
    // repair. When the mutex is not poisoned, return the guard as it is, with
    // no extra work. Keep the signature, repair in place (don't build a new
    // `Ledger` or copy the entries), leave `post`, `read_total` and `entries`
    // alone, no `unsafe`, and don't change the tests. Until `lock_ledger`
    // recovers from a poisoned mutex, the tests will fail.
    ledger.lock().unwrap()
}

/// Records `amount`.
fn post(ledger: &Mutex<Ledger>, amount: i64) {
    let mut guard = lock_ledger(ledger);
    guard.entries.push(amount);
    guard.total += amount;
}

/// The balance, in O(1).
fn read_total(ledger: &Mutex<Ledger>) -> i64 {
    lock_ledger(ledger).total
}

/// A copy of every recorded amount, oldest first.
fn entries(ledger: &Mutex<Ledger>) -> Vec<i64> {
    lock_ledger(ledger).entries.clone()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    fn ledger_with(amounts: &[i64]) -> Arc<Mutex<Ledger>> {
        let ledger = Arc::new(Mutex::new(Ledger::default()));
        for &amount in amounts {
            post(&ledger, amount);
        }
        ledger
    }

    // A worker that crashes halfway through an update: it records the entry,
    // then panics before it updates the total. The panic unwinds through the
    // guard, which unlocks the mutex and poisons it. `join` waits for all of
    // that, so the mutex is poisoned when this returns, on every run.
    fn crash_mid_post(ledger: &Arc<Mutex<Ledger>>, amount: i64) {
        let ledger = Arc::clone(ledger);
        let crashed = thread::spawn(move || {
            let mut guard = ledger.lock().unwrap();
            guard.entries.push(amount);
            panic!("worker crashed before updating the total");
        })
        .join();
        assert!(crashed.is_err(), "the worker was supposed to panic");
    }

    #[test]
    fn posting_keeps_the_total_up_to_date() {
        let ledger = ledger_with(&[10, 20, -5]);
        assert_eq!(read_total(&ledger), 25);
        assert_eq!(entries(&ledger), [10, 20, -5]);
        assert!(!ledger.is_poisoned());
    }

    #[test]
    fn a_healthy_lock_is_trusted_as_it_is() {
        // A probe, not a real scenario: corrupt the cached total WITHOUT a
        // crash. Only poisoning says "the last update may be half done", so a
        // lock that is not poisoned must hand the ledger back untouched, and
        // `read_total` stays O(1) instead of re-adding every entry.
        let ledger = ledger_with(&[1, 2]);
        ledger.lock().unwrap().total = 100;
        assert_eq!(read_total(&ledger), 100, "repair only a poisoned ledger");
    }

    #[test]
    fn a_crash_mid_update_poisons_the_lock_and_breaks_the_invariant() {
        let ledger = ledger_with(&[10, 20]);
        crash_mid_post(&ledger, 5);
        assert!(ledger.is_poisoned());
        // Look at the damage without going through `lock_ledger`: the error
        // carries the guard, and the guard sees the half-done update.
        let err = ledger.lock().unwrap_err();
        assert_eq!(err.get_ref().entries, [10, 20, 5]);
        assert_eq!(err.get_ref().total, 30);
    }

    #[test]
    fn reading_the_total_repairs_a_poisoned_ledger() {
        let ledger = ledger_with(&[10, 20]);
        crash_mid_post(&ledger, 5);
        assert_eq!(read_total(&ledger), 35);
        assert_eq!(entries(&ledger), [10, 20, 5], "keep every recorded entry");
    }

    #[test]
    fn after_the_repair_the_lock_is_no_longer_poisoned() {
        let ledger = ledger_with(&[10, 20]);
        crash_mid_post(&ledger, 5);
        read_total(&ledger);
        assert!(!ledger.is_poisoned(), "mark the mutex as recovered");
        let guard = ledger.lock().expect("a plain `lock()` works again");
        assert!(
            guard.is_consistent(),
            "the repair must fix the cached total"
        );
        assert_eq!(guard.total, 35);
    }

    #[test]
    fn posting_first_also_repairs() {
        let ledger = ledger_with(&[10, 20]);
        crash_mid_post(&ledger, 5);
        post(&ledger, 1);
        assert_eq!(entries(&ledger), [10, 20, 5, 1]);
        assert_eq!(read_total(&ledger), 36);
    }

    #[test]
    fn reading_the_entries_first_also_repairs() {
        let ledger = ledger_with(&[7]);
        crash_mid_post(&ledger, -2);
        assert_eq!(entries(&ledger), [7, -2]);
        assert!(!ledger.is_poisoned());
        assert!(ledger.lock().unwrap().is_consistent());
    }

    #[test]
    fn the_repair_happens_in_place() {
        // Room for every entry, so no push ever moves the buffer.
        let ledger = Arc::new(Mutex::new(Ledger {
            entries: Vec::with_capacity(8),
            total: 0,
        }));
        let buffer = ledger.lock().unwrap().entries.as_ptr();
        post(&ledger, 10);
        crash_mid_post(&ledger, 5);
        assert_eq!(read_total(&ledger), 15);
        assert!(
            std::ptr::eq(ledger.lock().unwrap().entries.as_ptr(), buffer),
            "repair the ledger you have; don't copy the entries into a new one"
        );
    }

    #[test]
    fn a_crash_after_a_complete_update_loses_nothing() {
        // This worker finishes its update and then panics (say, while
        // logging), still holding the guard: the mutex is poisoned, but the
        // ledger is consistent, and the recovery must keep it as it is.
        let ledger = ledger_with(&[10, 20]);
        let worker = Arc::clone(&ledger);
        let crashed = thread::spawn(move || {
            let mut guard = worker.lock().unwrap();
            guard.entries.push(5);
            guard.total += 5;
            panic!("worker crashed while logging");
        })
        .join();
        assert!(crashed.is_err());
        assert!(ledger.is_poisoned());
        assert_eq!(read_total(&ledger), 35);
        assert_eq!(entries(&ledger), [10, 20, 5]);
        assert!(!ledger.is_poisoned());
    }

    #[test]
    fn a_panic_outside_the_lock_poisons_nothing() {
        // `post` has released the guard before this worker panics.
        let ledger = ledger_with(&[1]);
        let worker = Arc::clone(&ledger);
        let crashed = thread::spawn(move || {
            post(&worker, 2);
            panic!("worker crashed after posting");
        })
        .join();
        assert!(crashed.is_err());
        assert!(!ledger.is_poisoned());
        assert_eq!(read_total(&ledger), 3);
    }

    #[test]
    fn recovers_again_after_a_second_crash() {
        let ledger = ledger_with(&[10]);
        crash_mid_post(&ledger, 5);
        assert_eq!(read_total(&ledger), 15);
        // The second worker can only record its entry if the first poisoning
        // was cleared: its own `lock().unwrap()` would panic otherwise.
        crash_mid_post(&ledger, 100);
        assert!(ledger.is_poisoned());
        assert_eq!(read_total(&ledger), 115);
        assert_eq!(entries(&ledger), [10, 5, 100]);
    }

    #[test]
    fn other_workers_keep_going_after_a_crash() {
        let ledger = ledger_with(&[]);
        crash_mid_post(&ledger, 1_000);
        let workers: Vec<_> = (0..4)
            .map(|w| {
                let ledger = Arc::clone(&ledger);
                thread::spawn(move || {
                    for i in 1..=50 {
                        post(&ledger, w * 100 + i);
                    }
                })
            })
            .collect();
        for worker in workers {
            worker
                .join()
                .expect("one crashed worker must not take the others down");
        }
        let recorded = entries(&ledger);
        assert_eq!(recorded.len(), 201);
        assert_eq!(read_total(&ledger), recorded.iter().sum::<i64>());
        assert_eq!(read_total(&ledger), 1_000 + 4 * (50 * 51 / 2) + 50 * 600);
    }
}
