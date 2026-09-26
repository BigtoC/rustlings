// Module 4 · Condition variables — part 3: a counting semaphore whose permit is an RAII guard.
//
// A SEMAPHORE is a counter of permits: "at most N threads may do this at
// once". It caps open database connections, concurrent downloads, or requests
// in flight to a fragile service. `acquire` takes a permit, waiting while
// none is free, and giving a permit back lets one waiter in. std has no
// semaphore (tokio has an async one), so "build one from `Mutex` + `Condvar`"
// is a common interview task. It is parts 1 and 2 with the queue replaced by
// a number: the condition is "a permit is free", checked in a loop under the
// lock.
//
// The interesting half is giving the permit back. A `release()` method that
// callers must remember to call leaks a permit on every early `return`, every
// `?` and every panic, and after N leaks the semaphore is stuck at zero for
// good. So `acquire` returns a GUARD instead, just as `Mutex::lock` returns a
// `MutexGuard`: a `Permit` whose `Drop` gives the permit back and wakes a
// waiter. `Drop` runs however a scope is left, unwinding included (see
// `39_drop_raii`). `Permit<'_>` borrows the semaphore, so the borrow checker
// guarantees that no permit outlives it. A permit that moves into a
// `thread::spawn`ed thread must own its semaphore instead, through an `Arc`
// (tokio's `acquire_owned` returns such an `OwnedSemaphorePermit`).
//
// A panicking holder poisons nothing here. The semaphore's lock is held for a
// few instructions at a time, never while the permit is in use, and the
// permit's `Drop` takes it again during unwinding to give the permit back.
//
// Later in the course, `36_atomics/atomics4` builds the lock-free half of
// this: a compare-and-swap loop that hands out at most N tickets with no lock
// at all. What it cannot do is WAIT. Sleeping until a permit comes back is
// what the condvar adds.
//
// Interviewers ask: implement a semaphore with `Mutex` + `Condvar`. How do
// you guarantee that every permit comes back? `notify_one` or `notify_all`
// on release? (One permit came back, so one waiter can use it.) How is a
// semaphore with one permit different from a mutex? (A mutex guards data and
// has an owner: std's `MutexGuard` is not `Send`, so only the thread that
// locked it can unlock it. A permit is only a count.) Is it fair? (No. A
// thread that arrives just as a permit comes back can take it before the
// waiter that was woken for it. tokio's semaphore is fair: FIFO.)

use std::sync::{Condvar, Mutex};

struct Semaphore {
    // How many permits are free right now.
    permits: Mutex<usize>,
    // `acquire` sleeps here while no permit is free.
    cv: Condvar,
}

// Proof that you hold one permit. It borrows the semaphore, so it cannot
// outlive it, and it is deliberately not `Clone`: one permit, one holder.
struct Permit<'a> {
    sem: &'a Semaphore,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            cv: Condvar::new(),
        }
    }

    fn available(&self) -> usize {
        *self.permits.lock().unwrap()
    }

    // Takes a permit if one is free right now. Never blocks.
    fn try_acquire(&self) -> Option<Permit<'_>> {
        let mut permits = self.permits.lock().unwrap();
        if *permits == 0 {
            return None;
        }
        *permits -= 1;
        Some(Permit { sem: self })
    }

    // Takes a permit, blocking while none is free.
    fn acquire(&self) -> Permit<'_> {
        let mut permits = self.permits.lock().unwrap();
        // TODO: `a_blocked_acquire_waits_until_a_permit_is_dropped` fails
        // with "acquire handed out a permit although none was free": when
        // the count is 0 this skips the decrement and returns a permit
        // anyway, so more than N threads get in. While no permit is free,
        // sleep on `cv` (no busy-waiting, no sleeping in a retry loop), and
        // take a permit only once one is really free: a wakeup is a hint,
        // not a promise, and another thread may have taken the permit that
        // came back. Until `acquire` waits for a free permit, the tests will
        // fail.
        if *permits > 0 {
            *permits -= 1;
        }
        Permit { sem: self }
    }
}

// TODO: `Permit` has no `Drop`, so a permit that is taken never comes back.
// `dropping_a_permit_gives_it_back` fails with `left: 0`, `right: 1`: the
// permit `b` was dropped, and `available()` did not change. Implement `Drop`
// for `Permit`: give the permit back and wake a thread that may be waiting in
// `acquire`. It must work however the holder's scope ends, a panic included,
// and give back exactly the one permit it holds. Keep the signatures and the
// tests as they are; no `unsafe`, no `mem::forget`. Until dropping a `Permit`
// returns it, the tests will fail.

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, mpsc};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    // Only a broken semaphore ever gets near this deadline: a thread that
    // should have returned but is still blocked fails the test instead of
    // hanging it.
    const PATIENCE: Duration = Duration::from_secs(20);

    // How long a test watches a thread that must STAY blocked. A correct
    // semaphore never lets it finish in this window. A wrong one is caught as
    // long as the thread gets any CPU time during it.
    const SETTLE: Duration = Duration::from_millis(250);

    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(
                start.elapsed() < PATIENCE,
                "gave up after {PATIENCE:?} waiting until {what}"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    // Joins `handle` if it finishes within PATIENCE, re-raising its panic.
    fn join_within<R>(handle: JoinHandle<R>, what: &str) -> R {
        wait_until(what, || handle.is_finished());
        match handle.join() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    // Keeps checking, for SETTLE, that `handle` is still blocked.
    fn assert_stays_blocked<R>(handle: &JoinHandle<R>, why: &str) {
        let start = Instant::now();
        while start.elapsed() < SETTLE {
            assert!(!handle.is_finished(), "{why}");
            thread::sleep(Duration::from_millis(1));
        }
    }

    // Runs a single-threaded test body on a helper thread, so that a `Drop`
    // that deadlocks fails the test instead of hanging it.
    fn on_helper_thread(body: impl FnOnce() + Send + 'static) {
        join_within(thread::spawn(body), "the test body finishes");
    }

    // The free permits, read WITHOUT blocking: `None` while the lock is
    // taken. An `acquire` that keeps the lock while it waits would otherwise
    // hang the test instead of failing it.
    fn available_now(sem: &Semaphore) -> Option<usize> {
        sem.permits.try_lock().ok().map(|permits| *permits)
    }

    #[test]
    fn dropping_a_permit_gives_it_back() {
        on_helper_thread(|| {
            let sem = Semaphore::new(3);
            let a = sem.try_acquire().expect("3 permits are free");
            let b = sem.try_acquire().expect("2 permits are free");
            let c = sem.try_acquire().expect("1 permit is free");
            assert!(sem.try_acquire().is_none(), "all 3 permits are taken");
            assert_eq!(sem.available(), 0);
            drop(b);
            assert_eq!(sem.available(), 1, "dropping a `Permit` must give it back");
            let d = sem.try_acquire();
            assert!(d.is_some(), "the permit that came back can be taken again");
            assert!(sem.try_acquire().is_none(), "it came back ONCE");
            drop((a, c, d));
            assert_eq!(sem.available(), 3);
        });
    }

    #[test]
    fn every_drop_returns_exactly_one_permit() {
        on_helper_thread(|| {
            let sem = Semaphore::new(3);
            let mut held: Vec<Permit<'_>> = (0..3).filter_map(|_| sem.try_acquire()).collect();
            assert_eq!(held.len(), 3);
            // Give them back in a different order than they were taken.
            drop(held.remove(1));
            assert_eq!(sem.available(), 1);
            drop(held.pop());
            assert_eq!(sem.available(), 2);
            {
                let _scoped = sem.try_acquire().expect("2 permits are free");
                assert_eq!(sem.available(), 1);
            }
            assert_eq!(
                sem.available(),
                2,
                "a permit comes back at the end of its scope"
            );
            drop(held);
            assert_eq!(
                sem.available(),
                3,
                "never more permits than the semaphore started with"
            );
            assert!(Semaphore::new(0).try_acquire().is_none());
        });
    }

    #[test]
    fn acquire_takes_a_free_permit_without_waiting() {
        let sem = Arc::new(Semaphore::new(3));
        let taker = {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                let _a = sem.acquire();
                let _b = sem.acquire();
                let _c = sem.acquire();
                sem.available()
            })
        };
        let left = join_within(taker, "`acquire` returns at once while a permit is free");
        assert_eq!(left, 0, "three `acquire` calls take three permits");
        assert_eq!(
            sem.available(),
            3,
            "all three come back when the thread ends"
        );
    }

    #[test]
    fn a_blocked_acquire_waits_until_a_permit_is_dropped() {
        let sem = Arc::new(Semaphore::new(1));
        // A holder thread takes the only permit and keeps it until the test
        // sends `release` (or drops it, by failing first).
        let (release, released) = mpsc::channel::<()>();
        let holder = {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                let _permit = sem.try_acquire().expect("1 permit is free");
                let _ = released.recv();
            })
        };
        wait_until("the holder has taken the only permit", || {
            available_now(&sem) == Some(0)
        });
        let waiter = {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                let _permit = sem.acquire();
            })
        };
        assert_stays_blocked(
            &waiter,
            "acquire handed out a permit although none was free",
        );
        // No permit came back. To `acquire`, this is exactly what a spurious
        // wakeup (or a stolen one) looks like.
        sem.cv.notify_all();
        assert_stays_blocked(
            &waiter,
            "`acquire` returned after a wakeup that brought no permit: check \
             the count again after EVERY wakeup",
        );
        assert_eq!(
            available_now(&sem),
            Some(0),
            "no permit is free, and a blocked `acquire` must not keep the lock"
        );
        release.send(()).unwrap();
        join_within(holder, "the holder drops its permit");
        join_within(
            waiter,
            "dropping the held `Permit` wakes the thread blocked in `acquire`",
        );
        assert_eq!(sem.available(), 1);
    }

    #[test]
    fn two_permits_dropped_back_to_back_wake_two_waiters() {
        let sem = Arc::new(Semaphore::new(2));
        let (release, released) = mpsc::channel::<()>();
        let holder = {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                let first = sem.try_acquire().expect("2 permits are free");
                let second = sem.try_acquire().expect("1 permit is free");
                let _ = released.recv();
                // Back to back: the second permit usually comes back before
                // the waiter woken for the first one has taken it, so the
                // count goes from 1 to 2, not from 0 to 1.
                drop(first);
                drop(second);
            })
        };
        wait_until("the holder has taken both permits", || {
            available_now(&sem) == Some(0)
        });
        let waiters: Vec<JoinHandle<()>> = (0..2)
            .map(|_| {
                let sem = Arc::clone(&sem);
                thread::spawn(move || {
                    let _permit = sem.acquire();
                })
            })
            .collect();
        for w in &waiters {
            assert_stays_blocked(w, "acquire handed out a permit although none was free");
        }
        release.send(()).unwrap();
        join_within(holder, "the holder drops both permits");
        for w in waiters {
            join_within(
                w,
                "each dropped permit wakes one waiter (did a `Drop` skip its \
                 notify because the count was not 0?)",
            );
        }
        assert_eq!(sem.available(), 2);
    }

    #[test]
    fn a_panicking_holder_still_returns_its_permit() {
        let sem = Arc::new(Semaphore::new(2));
        let worker = {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                let _permit = sem.acquire();
                panic!("worker crashed while holding a permit");
            })
        };
        wait_until("the worker finishes", || worker.is_finished());
        assert!(worker.join().is_err(), "the worker panicked");
        assert_eq!(
            sem.available(),
            2,
            "unwinding drops the `Permit`, and its `Drop` gives the permit back"
        );
        assert!(
            !sem.permits.is_poisoned(),
            "no lock was held during the panic"
        );
    }

    #[test]
    fn eight_threads_never_hold_more_than_three_permits() {
        let sem = Arc::new(Semaphore::new(3));
        let inside = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let workers: Vec<JoinHandle<()>> = (0..8)
            .map(|_| {
                let (sem, inside, peak) =
                    (Arc::clone(&sem), Arc::clone(&inside), Arc::clone(&peak));
                thread::spawn(move || {
                    for _ in 0..200 {
                        let _permit = sem.acquire();
                        // `inside` goes up after the permit is taken and down
                        // before it is dropped, so it never exceeds the
                        // number of permits actually held.
                        let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(now, Ordering::SeqCst);
                        thread::yield_now();
                        inside.fetch_sub(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for w in workers {
            join_within(w, "every worker gets its 200 permits (a lost wakeup?)");
        }
        let peak = peak.load(Ordering::SeqCst);
        assert!(
            peak <= 3,
            "{peak} threads held a permit at the same time, but there are only 3"
        );
        assert_eq!(sem.available(), 3, "every permit came back");
    }
}
