// Module 4 · Lock hazards — part 1: two threads lock two mutexes in opposite orders and deadlock.
//
// "Fearless concurrency" is a promise about DATA RACES: `Send`, `Sync` and
// the borrow checker make sure that no two threads touch the same memory at
// the same time with one of them writing. It is not a promise about
// DEADLOCKS. In a deadlock every thread involved waits for a lock that
// another one holds, so nothing ever happens again, and no memory is misused
// along the way. The Rustonomicon says it plainly: it is considered "safe"
// for Rust to get deadlocked. The compiler could not catch it anyway. It
// checks one function at a time, and lock order is a property of the whole
// program: which locks can a thread already hold when it asks for this one?
//
// The classic case is a bank. `transfer(from, to, amount)` locks `from`, then
// `to`. Two transfers in opposite directions run at the same time:
//
//   T1: transfer(0, 1, ..) locks account 0, then waits for account 1.
//   T2: transfer(1, 0, ..) locks account 1, then waits for account 0.
//
// Neither ever gets its second lock. All four of Coffman's conditions for a
// deadlock hold: a lock has one holder at a time (mutual exclusion), each
// thread keeps one lock while it waits for another (hold and wait), nobody
// can take a lock away from its holder (no preemption), and the waits form a
// cycle (circular wait). Remove any one and the deadlock cannot happen. The
// standard fix removes the cycle: give the locks one GLOBAL ORDER, and make
// every piece of code that holds two of them at once take them in that
// order. A thread that holds a lock then only ever waits for a "later" one,
// so no chain of waits can lead back to it. Any fixed order works (an
// account number, a unique id, the address of a lock that never moves), as
// long as EVERY code path uses the same one. Here `total` already locks
// every account in ascending index order, so that is the order.
//
// The other ways out, and what they cost:
//   - one lock for the whole bank: no deadlock, but every transfer waits for
//     every other one, even when they share no account;
//   - lock one account, `try_lock` the other, and on failure let go of the
//     first and start over (no more "hold and wait"): it works, but it burns
//     CPU under contention and can livelock, two threads backing off in step;
//   - release `from` before locking `to`: no thread ever holds two locks, so
//     no deadlock, but the transfer is no longer ATOMIC. For a moment the
//     money is in neither account, and an auditor who sums the bank right
//     then finds it missing. That trades a deadlock for a corrupted total.
//
// There is a second, smaller trap. `transfer(2, 2, ..)` locks the SAME mutex
// twice on one thread. `std::sync::Mutex` is not reentrant: its docs say the
// second `lock()` "will not return" (it might panic or deadlock). Two ids
// that happen to be equal are enough to get there. (`31_debugging/debugging8`
// meets this self-deadlock again, caused by a guard that lives longer than it
// looks.)
//
// How the tests catch a deadlock without hanging: every transfer that might
// deadlock runs on a helper thread, and the test waits for it with
// `recv_timeout`. A stuck helper is left behind and dies when the test binary
// exits. The opposite-order deadlock depends on timing. Two threads doing
// 100_000 opposite transfers nearly always hit it within milliseconds, but
// not always, which is exactly how lock-order bugs survive testing in real
// code. So two tests probe the order directly: the test itself holds one
// account, as a transfer on another thread would, and uses `try_lock` to
// watch what a transfer does with the other one.
//
// Interviewers ask "can safe Rust deadlock?" (yes), then how you would fix
// this bank and why the fix works, then how you would find a deadlock in a
// running service. Good answers: take a thread dump (attach a debugger and
// print every thread's backtrace; each stuck thread sits in a lock call, and
// its callers tell you which locks it took on the way there), enable
// `parking_lot`'s optional `deadlock_detection` feature, or use a lock-order
// checker like the Linux kernel's lockdep.

use std::sync::Mutex;

/// Why a transfer was refused. A refused transfer changes no balance.
#[derive(Debug, PartialEq, Eq)]
enum TransferError {
    /// `from` and `to` are the same account.
    SameAccount,
    /// `from` holds only `balance`, which is less than `amount`.
    InsufficientFunds { balance: u64, amount: u64 },
}

/// A bank with one lock per account, so that transfers between different
/// pairs of accounts run in parallel.
struct Bank {
    accounts: Vec<Mutex<u64>>,
}

impl Bank {
    fn new(balances: &[u64]) -> Self {
        Bank {
            accounts: balances.iter().map(|&b| Mutex::new(b)).collect(),
        }
    }

    /// The sum of all balances, as one consistent snapshot: it locks every
    /// account, in ascending index order, and holds them all while it adds.
    fn total(&self) -> u64 {
        let guards: Vec<_> = self
            .accounts
            .iter()
            .map(|account| account.lock().unwrap())
            .collect();
        guards.iter().map(|balance| **balance).sum()
    }

    /// Moves `amount` from account `from` to account `to`, all or nothing.
    /// Panics if an id is out of range.
    fn transfer(&self, from: usize, to: usize, amount: u64) -> Result<(), TransferError> {
        // Equal ids would lock one `Mutex` twice on this thread, and the
        // second `lock()` never returns. Reject them before locking anything.
        if from == to {
            return Err(TransferError::SameAccount);
        }
        // Lock in the one global order, ascending index, whichever way the
        // money goes: `total` uses the same order. A thread holding `low`
        // only ever waits for a higher account, so no cycle of waits can
        // form. Both guards live until we return, so the check, the debit
        // and the credit are a single step for every other thread.
        let (low, high) = (from.min(to), from.max(to));
        let mut low_guard = self.accounts[low].lock().unwrap();
        let mut high_guard = self.accounts[high].lock().unwrap();
        // Map the guards back to their roles.
        let (src, dst) = if from == low {
            (&mut *low_guard, &mut *high_guard)
        } else {
            (&mut *high_guard, &mut *low_guard)
        };
        if *src < amount {
            return Err(TransferError::InsufficientFunds {
                balance: *src,
                amount,
            });
        }
        *src -= amount;
        *dst += amount;
        Ok(())
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::sync::{Arc, TryLockError};
    use std::thread;
    use std::time::{Duration, Instant};

    // The time limit for anything that might deadlock. It is far beyond what
    // a correct transfer needs, even on a machine busy with other work, so
    // only a stuck transfer ever reaches it.
    const WATCHDOG: Duration = Duration::from_secs(10);

    /// Runs `bank.transfer(from, to, amount)` on a helper thread. The result
    /// arrives on the returned channel.
    fn spawn_transfer(
        bank: &Arc<Bank>,
        from: usize,
        to: usize,
        amount: u64,
    ) -> Receiver<Result<(), TransferError>> {
        let (tx, rx) = mpsc::channel();
        let bank = Arc::clone(bank);
        thread::spawn(move || {
            let _ = tx.send(bank.transfer(from, to, amount));
        });
        rx
    }

    /// Waits at most `WATCHDOG` for the result of `spawn_transfer`.
    fn wait_for(
        rx: &Receiver<Result<(), TransferError>>,
        from: usize,
        to: usize,
    ) -> Result<(), TransferError> {
        match rx.recv_timeout(WATCHDOG) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => panic!(
                "deadlock detected: transfer({from}, {to}, ..) did not return within {WATCHDOG:?}"
            ),
            Err(RecvTimeoutError::Disconnected) => {
                panic!("transfer({from}, {to}, ..) panicked (see the message above)")
            }
        }
    }

    /// `bank.transfer(from, to, amount)`, run under the watchdog.
    fn transfer(
        bank: &Arc<Bank>,
        from: usize,
        to: usize,
        amount: u64,
    ) -> Result<(), TransferError> {
        wait_for(&spawn_transfer(bank, from, to, amount), from, to)
    }

    /// Every balance, read with `try_lock`, so that a lock nobody released
    /// fails the test instead of hanging it.
    fn balances(bank: &Bank) -> Vec<u64> {
        bank.accounts
            .iter()
            .enumerate()
            .map(|(id, account)| match account.try_lock() {
                Ok(balance) => *balance,
                Err(TryLockError::WouldBlock) => {
                    panic!("account {id} is still locked after every transfer returned")
                }
                Err(TryLockError::Poisoned(_)) => {
                    panic!("account {id} is poisoned: a thread panicked while holding it")
                }
            })
            .collect()
    }

    #[test]
    fn a_transfer_moves_money_in_the_right_direction() {
        let bank = Arc::new(Bank::new(&[100, 50, 0]));
        assert_eq!(transfer(&bank, 0, 2, 30), Ok(()));
        assert_eq!(balances(&bank), [70, 50, 30]);
        // From a higher to a lower account: the debit and the credit must
        // still land on the right accounts.
        assert_eq!(transfer(&bank, 2, 1, 30), Ok(()));
        assert_eq!(balances(&bank), [70, 80, 0]);
        // The whole balance may go, and so may nothing at all.
        assert_eq!(transfer(&bank, 1, 0, 80), Ok(()));
        assert_eq!(transfer(&bank, 2, 0, 0), Ok(()));
        assert_eq!(balances(&bank), [150, 0, 0]);
        assert_eq!(bank.total(), 150);
    }

    #[test]
    fn insufficient_funds_change_nothing() {
        let bank = Arc::new(Bank::new(&[100, 50]));
        assert_eq!(
            transfer(&bank, 0, 1, 101),
            Err(TransferError::InsufficientFunds {
                balance: 100,
                amount: 101
            })
        );
        // The same from the higher account: the check must read `from`.
        assert_eq!(
            transfer(&bank, 1, 0, 51),
            Err(TransferError::InsufficientFunds {
                balance: 50,
                amount: 51
            })
        );
        assert_eq!(balances(&bank), [100, 50]);
    }

    #[test]
    fn a_transfer_to_the_same_account_is_rejected() {
        let bank = Arc::new(Bank::new(&[100, 50, 25]));
        assert_eq!(transfer(&bank, 2, 2, 10), Err(TransferError::SameAccount));
        // Rejected before the funds are even looked at.
        assert_eq!(
            transfer(&bank, 1, 1, 1_000),
            Err(TransferError::SameAccount)
        );
        assert_eq!(balances(&bank), [100, 50, 25]);
    }

    // How long a probe keeps watching once the transfer holds the account it
    // watches. A correct transfer keeps holding it for as long as the test
    // blocks the other account, so it passes however long the probe watches.
    const HOLD_WINDOW: Duration = Duration::from_millis(50);

    /// Holds account `busy` on this thread (as a transfer on another thread
    /// would) while a helper runs `transfer(from, to, 10)`, and watches the
    /// other account of the pair with `try_lock`. Once the helper is seen
    /// waiting, runs `meanwhile`, then lets the helper finish. Returns what
    /// went wrong, if anything did.
    fn watch_while_busy(
        bank: &Arc<Bank>,
        busy: usize,
        from: usize,
        to: usize,
        meanwhile: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let other = if busy == from { to } else { from };
        let before = balances(bank)[other];
        let busy_guard = bank.accounts[busy].lock().unwrap();
        let rx = spawn_transfer(bank, from, to, 10);
        let deadline = Instant::now() + WATCHDOG;
        let mut held_since: Option<Instant> = None;
        let verdict = loop {
            match bank.accounts[other].try_lock() {
                Err(TryLockError::WouldBlock) => {
                    // The transfer holds `other` and waits for `busy`.
                    let since = *held_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= HOLD_WINDOW {
                        break meanwhile();
                    }
                }
                Ok(balance) if *balance != before => {
                    break Err(format!(
                        "transfer({from}, {to}) changed account {other} while account {busy} \
                         was locked by another thread: it moved money before it held both \
                         accounts, so for a moment the money was in neither"
                    ));
                }
                Ok(_) if held_since.is_some() => {
                    break Err(format!(
                        "transfer({from}, {to}) let go of account {other} while it was still \
                         waiting for account {busy}"
                    ));
                }
                Ok(_) => {}
                Err(TryLockError::Poisoned(_)) => {
                    break Err(format!(
                        "transfer({from}, {to}) panicked while it held account {other}"
                    ));
                }
            }
            if Instant::now() > deadline {
                break Err(format!(
                    "transfer({from}, {to}) never locked account {other} while account {busy} \
                     was busy. It must lock the lower-numbered account first, the order \
                     `total` uses. A transfer that starts with the higher one can deadlock \
                     against a transfer in the other direction, or against `total`"
                ));
            }
            thread::sleep(Duration::from_millis(1));
        };
        // Let the transfer finish before reporting, so that no lock is left
        // poisoned or held.
        drop(busy_guard);
        let result = wait_for(&rx, from, to);
        verdict?;
        assert_eq!(result, Ok(()));
        Ok(())
    }

    #[test]
    fn a_transfer_locks_the_lower_account_first() {
        let bank = Arc::new(Bank::new(&[100, 100, 100, 100]));
        // While transfer(1, 0) waits for account 1, a transfer between two
        // other accounts has nothing to wait for.
        let unrelated = || match spawn_transfer(&bank, 2, 3, 5).recv_timeout(WATCHDOG) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(format!("transfer(2, 3, 5) returned {e:?}")),
            Err(RecvTimeoutError::Timeout) => Err(String::from(
                "transfer(2, 3) had to wait while transfer(1, 0) was waiting for account 1, \
                 though the two share no account: keep one lock per account, not one lock \
                 for the whole bank",
            )),
            Err(RecvTimeoutError::Disconnected) => Err(String::from("transfer(2, 3) panicked")),
        };
        // Another thread holds account 1. A transfer from 1 to 0 must wait
        // for it while already holding account 0.
        if let Err(why) = watch_while_busy(&bank, 1, 1, 0, unrelated) {
            panic!("{why}");
        }
        assert_eq!(balances(&bank), [110, 90, 95, 105]);
    }

    #[test]
    fn a_transfer_holds_both_accounts_until_it_is_done() {
        let bank = Arc::new(Bank::new(&[100, 100]));
        // Another thread holds account 1. A transfer from 0 to 1 must wait
        // for it holding account 0, and must not touch the money before it
        // holds both.
        if let Err(why) = watch_while_busy(&bank, 1, 0, 1, || Ok(())) {
            panic!("{why}");
        }
        assert_eq!(balances(&bank), [90, 110]);
    }

    /// Waits until `workers` results have arrived on `rx`. `progress` counts
    /// finished transfers, and the test fails with "deadlock detected" once
    /// no transfer at all has finished for `WATCHDOG`.
    fn collect<T>(rx: &Receiver<T>, workers: usize, progress: &AtomicU64) -> Vec<T> {
        let mut results = Vec::new();
        let mut seen = progress.load(Ordering::Relaxed);
        let mut last_progress = Instant::now();
        while results.len() < workers {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => results.push(result),
                Err(RecvTimeoutError::Timeout) => {
                    let now = progress.load(Ordering::Relaxed);
                    if now != seen {
                        seen = now;
                        last_progress = Instant::now();
                    } else if last_progress.elapsed() > WATCHDOG {
                        panic!(
                            "deadlock detected: no transfer finished for {WATCHDOG:?} \
                             ({now} transfers had finished)"
                        );
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("a transfer thread panicked (see the message above)")
                }
            }
        }
        results
    }

    #[test]
    fn opposite_transfers_do_not_deadlock() {
        const ROUNDS: u64 = 100_000;
        let bank = Arc::new(Bank::new(&[1_000_000, 1_000_000]));
        let progress = Arc::new(AtomicU64::new(0));
        let (tx, rx) = mpsc::channel();
        for (from, to) in [(0, 1), (1, 0)] {
            let (bank, progress, tx) = (Arc::clone(&bank), Arc::clone(&progress), tx.clone());
            thread::spawn(move || {
                let mut ok = 0;
                for _ in 0..ROUNDS {
                    if bank.transfer(from, to, 1).is_ok() {
                        ok += 1;
                    }
                    progress.fetch_add(1, Ordering::Relaxed);
                }
                let _ = tx.send(ok);
            });
        }
        drop(tx);
        let ok = collect(&rx, 2, &progress);
        assert_eq!(ok, [ROUNDS, ROUNDS], "every transfer had enough funds");
        assert_eq!(balances(&bank), [1_000_000, 1_000_000]);
    }

    #[test]
    fn an_audit_during_transfers_always_balances() {
        const ACCOUNTS: usize = 4;
        const TOTAL: u64 = 4_000;
        const TRANSFERS: u32 = 20_000;
        let bank = Arc::new(Bank::new(&[TOTAL / ACCOUNTS as u64; ACCOUNTS]));
        let progress = Arc::new(AtomicU64::new(0));
        let done = Arc::new(AtomicBool::new(false));

        // The auditor sums the bank again and again while the transfers run,
        // and reports the first total that is off.
        let (audit_tx, audit_rx) = mpsc::channel();
        {
            let (bank, done) = (Arc::clone(&bank), Arc::clone(&done));
            thread::spawn(move || {
                let mut audits = 0u64;
                let verdict = loop {
                    let total = bank.total();
                    if total != TOTAL {
                        break Err(total);
                    }
                    audits += 1;
                    if done.load(Ordering::Relaxed) {
                        break Ok(audits);
                    }
                };
                let _ = audit_tx.send(verdict);
            });
        }

        // Three workers transfer between pseudo-random pairs of accounts, in
        // both directions, and some of the amounts are too large.
        let (tx, rx) = mpsc::channel();
        for worker in 0..3u32 {
            let (bank, progress, tx) = (Arc::clone(&bank), Arc::clone(&progress), tx.clone());
            thread::spawn(move || {
                let mut seed = 0x9e37_79b9_u32.wrapping_mul(worker + 1);
                let mut below = |n: usize| {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (seed >> 16) as usize % n
                };
                let (mut ok, mut refused) = (0u32, 0u32);
                for _ in 0..TRANSFERS {
                    let from = below(ACCOUNTS);
                    let to = (from + 1 + below(ACCOUNTS - 1)) % ACCOUNTS;
                    match bank.transfer(from, to, below(1_500) as u64) {
                        Ok(()) => ok += 1,
                        Err(TransferError::InsufficientFunds { .. }) => refused += 1,
                        Err(TransferError::SameAccount) => panic!("{from} and {to} differ"),
                    }
                    progress.fetch_add(1, Ordering::Relaxed);
                }
                let _ = tx.send((ok, refused));
            });
        }
        drop(tx);
        let results = collect(&rx, 3, &progress);
        done.store(true, Ordering::Relaxed);
        for (ok, refused) in results {
            assert_eq!(ok + refused, TRANSFERS);
            assert!(
                ok > 0 && refused > 0,
                "{ok} transfers went through, {refused} were refused"
            );
        }
        match audit_rx.recv_timeout(WATCHDOG) {
            Ok(Ok(audits)) => assert!(audits > 0),
            Ok(Err(total)) => panic!(
                "`total()` saw {total} instead of {TOTAL} while transfers were running: a \
                 transfer let other threads see money that had left one account but not yet \
                 reached the other"
            ),
            Err(RecvTimeoutError::Timeout) => {
                panic!("deadlock detected: `total()` did not return within {WATCHDOG:?}")
            }
            Err(RecvTimeoutError::Disconnected) => panic!("the auditor thread panicked"),
        }
        assert_eq!(balances(&bank).iter().sum::<u64>(), TOTAL);
    }
}
