// Atomics - atomic operations and memory ordering, part 4: a CAS retry loop.
//
// `fetch_add` from part 1 always succeeds, which is exactly what you do NOT
// want from a counter with a limit: a connection cap, a pool of N permits, a
// "reserve the next slot" index into a fixed buffer. There you need "increment
// only IF below `max`", and the obvious code is broken. Reading the value,
// checking it and then storing `current + 1` is CHECK-THEN-ACT. Between the
// load and the store another thread can run, and whatever it did is silently
// overwritten: its increment is lost, and both threads are handed the same
// slot. `if c.load() < max { c.fetch_add(1) }` is the same bug: the check
// and the add are two separate atomic operations.
//
// The fix is the compare-and-swap (CAS) retry loop, the core pattern of
// lock-free programming. Read the current value, decide what the new value
// should be (or give up), then publish it with `compare_exchange` /
// `compare_exchange_weak(current, new, success, failure)`. The CAS succeeds
// only if the value is STILL `current`, so your decision was based on
// up-to-date information. If another thread got there first, the CAS fails
// and returns `Err(actual)`: the value it found. Re-check the limit against
// `actual` and try again. std uses exactly this loop in `Weak::upgrade`,
// because an `Arc`'s strong count must never be bumped from 0 back to 1.
//
// Why `compare_exchange_weak` in a loop? On x86-64 both versions compile to
// the same `lock cmpxchg`. On load-linked / store-conditional machines
// (ARMv7, ARMv8 without the LSE extension, RISC-V, POWER) the hardware store
// can fail SPURIOUSLY, e.g. after an interrupt or when another core writes
// to the same cache line. The strong `compare_exchange` has to hide that behind
// its own internal retry loop; the weak one reports it as an ordinary
// `Err(actual)` with `actual == current`. When you are already in a retry
// loop, a spurious failure just costs one more iteration, so weak saves you
// a loop inside a loop. Use the strong one when a failure has consequences,
// like the single attempt of `try_lock` in part 3.
//
// `Relaxed` is enough here: the counter itself is the only shared data. If
// the ticket indexed into a buffer that the previous owner wrote, you would
// need the Release/Acquire pairing from part 2.
//
// A real race would show up only now and then, which makes a useless test.
// So `try_inc_with` takes a `race_window` closure: a test seam (a
// "failpoint") that runs where other threads could run, after you read and
// before you publish. The tests use it to force a rival increment into that
// window, deterministically, on a single thread. (`loom`, used by the loom
// lab in deep-dive/, explores every interleaving instead of one chosen by
// hand.)
//
// Interviewers ask for exactly this counter. Then they ask why the load/store
// version loses updates, why the weak CAS belongs in a loop, what `Err`
// hands back, and whether the loop is lock-free (yes: a CAS fails only
// because some other thread's CAS succeeded, spurious failures aside) or
// wait-free (no: one unlucky thread can keep losing). Part 5 shows the case
// where comparing values is NOT enough: the ABA problem.

use std::sync::atomic::{AtomicU64, Ordering};

/// Returned when the counter has already reached its limit.
#[derive(Debug, PartialEq, Eq)]
struct Full;

struct BoundedCounter {
    value: AtomicU64,
}

impl BoundedCounter {
    fn new() -> Self {
        Self::starting_at(0)
    }

    fn starting_at(start: u64) -> Self {
        BoundedCounter {
            value: AtomicU64::new(start),
        }
    }

    fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Takes one unit if the counter is below `max`. Returns `Ok(ticket)`,
    /// where the ticket is the value the increment replaced (the first caller
    /// gets 0, so tickets run `0..max`), or `Err(Full)`, changing nothing,
    /// once the counter is at or above `max`.
    fn try_inc(&self, max: u64) -> Result<u64, Full> {
        self.try_inc_with(max, || {})
    }

    /// `try_inc` with a test seam: `race_window` runs where another thread
    /// could run, after you read the value and before you publish the new
    /// one. Production callers pass a no-op.
    fn try_inc_with(&self, max: u64, mut race_window: impl FnMut()) -> Result<u64, Full> {
        // TODO: this body is check-then-act. It reads the value, lets other
        // threads run (`race_window`), then blindly stores `current + 1`,
        // overwriting whatever happened in between. The tests
        // `retries_after_losing_the_race`,
        // `rechecks_the_limit_after_losing_the_race` and
        // `a_thread_that_keeps_losing_retries_until_full` fail with
        // `left: Ok(0)`: the rival increment made inside the window is lost,
        // and both callers are handed ticket 0. Rewrite the body as a CAS
        // retry loop: publish the new value only if the counter still holds
        // the value you checked, and after a lost race re-check the limit
        // against the value the CAS found. Requirements:
        //   - keep both signatures: return the replaced value as the ticket,
        //     and `Err(Full)` without writing anything once the counter is at
        //     or above `max` (even at `u64::MAX`: no overflow panic);
        //   - call `race_window()` once before every compare-and-swap, after
        //     reading the value that CAS will compare against, and keep
        //     retrying for as long as slots are left (no retry cap);
        //   - no `fetch_add` (it cannot refuse, and "add, then undo if too
        //     big" lets other threads see a value above `max`), no `unsafe`,
        //     and no `Mutex` or spinlock: the rival inside `race_window` runs
        //     on this same thread, so a lock held across the window deadlocks
        //     the test instead of failing it. Don't change the tests.
        // Until you replace the load-then-store with a CAS loop, the tests
        // will fail.
        let current = self.value.load(Ordering::Relaxed);
        race_window();
        if current >= max {
            return Err(Full);
        }
        self.value.store(current + 1, Ordering::Relaxed);
        Ok(current)
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn hands_out_tickets_in_order_until_full() {
        let c = BoundedCounter::new();
        assert_eq!(c.try_inc(3), Ok(0));
        assert_eq!(c.try_inc(3), Ok(1));
        assert_eq!(c.try_inc(3), Ok(2));
        assert_eq!(c.try_inc(3), Err(Full));
        assert_eq!(c.try_inc(3), Err(Full));
        assert_eq!(c.get(), 3, "a refused increment must not write");
    }

    #[test]
    fn a_zero_limit_is_always_full() {
        let c = BoundedCounter::new();
        assert_eq!(c.try_inc(0), Err(Full));
        assert_eq!(c.get(), 0);
    }

    #[test]
    fn a_limit_below_the_current_value_is_full() {
        // "At or above" the limit, not just "equal to" it. The value must be
        // left alone: not clamped down to `max`, not rolled back.
        let c = BoundedCounter::starting_at(5);
        assert_eq!(c.try_inc(3), Err(Full));
        assert_eq!(c.get(), 5);
        assert_eq!(c.try_inc(6), Ok(5));
        assert_eq!(c.get(), 6);
    }

    #[test]
    fn works_at_the_top_of_the_u64_range() {
        // Compare `current` against `max` BEFORE computing `current + 1`:
        // tests run in debug mode, so `u64::MAX + 1` would panic.
        let c = BoundedCounter::starting_at(u64::MAX - 1);
        assert_eq!(c.try_inc(u64::MAX), Ok(u64::MAX - 1));
        assert_eq!(c.try_inc(u64::MAX), Err(Full));
        assert_eq!(c.get(), u64::MAX);
    }

    #[test]
    fn retries_after_losing_the_race() {
        // We read 0; inside the race window a rival takes ticket 0 and the
        // value becomes 1. Our publish of 0 -> 1 must now FAIL, and the retry
        // must take ticket 1 from the fresh value.
        let c = BoundedCounter::new();
        let mut calls = 0;
        let mut rival = None;
        let mine = c.try_inc_with(10, || {
            calls += 1;
            if calls == 1 {
                rival = Some(c.try_inc(10));
            }
        });
        assert_eq!(rival, Some(Ok(0)), "the rival should win ticket 0");
        assert_eq!(
            mine,
            Ok(1),
            "after losing the race, retry with the value the CAS found"
        );
        assert_eq!(c.get(), 2, "both increments must survive");
        assert!(
            calls >= 2,
            "race_window ran {calls} time(s): call it before EVERY attempt to \
             publish, and retry after a lost race"
        );
    }

    #[test]
    fn rechecks_the_limit_after_losing_the_race() {
        // One slot left. We read 0, then the rival takes the last slot inside
        // the race window. The retry sees 1 == max and must give up.
        let c = BoundedCounter::new();
        let mut fired = false;
        let mut rival = None;
        let mine = c.try_inc_with(1, || {
            if !fired {
                fired = true;
                rival = Some(c.try_inc(1));
            }
        });
        assert_eq!(rival, Some(Ok(0)), "the rival should win the last slot");
        assert_eq!(
            mine,
            Err(Full),
            "a lost race must re-check the limit: the slot is gone"
        );
        assert_eq!(c.get(), 1, "the counter must never pass its limit");
    }

    #[test]
    fn a_thread_that_keeps_losing_retries_until_full() {
        // Lock-free, not wait-free: here a rival wins EVERY race and takes
        // the 100 slots one by one while we keep retrying. Losing a race is
        // not a reason to give up while slots are left, and every retry must
        // re-check the limit, so the only correct outcome is `Err(Full)`,
        // and only after the rival has taken the last slot.
        let c = BoundedCounter::new();
        let mut calls = 0;
        let mut rival_tickets = Vec::new();
        let mine = c.try_inc_with(100, || {
            calls += 1;
            // The cap only stops a broken solution from retrying forever.
            if calls <= 1_000 {
                rival_tickets.extend(c.try_inc(100).ok());
            }
        });
        assert_eq!(
            mine,
            Err(Full),
            "the rival took every slot while we retried: re-check the limit \
             on every retry"
        );
        assert_eq!(
            c.get(),
            100,
            "`Err(Full)` with slots left: a lost race is not a full counter, \
             so keep retrying"
        );
        assert!(
            rival_tickets.iter().copied().eq(0..100),
            "the rival must get tickets 0..100, each exactly once"
        );
    }

    // Collects every ticket handed out by `threads` threads that each call
    // `try_inc(max)` `attempts` times.
    fn tickets_from_threads(c: &BoundedCounter, threads: u64, attempts: u64, max: u64) -> Vec<u64> {
        thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    s.spawn(move || {
                        (0..attempts)
                            .filter_map(|_| c.try_inc(max).ok())
                            .collect::<Vec<u64>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        })
    }

    #[test]
    fn eight_threads_never_oversubscribe() {
        // 80_000 attempts against 50_000 slots: every slot is handed out
        // exactly once and the rest are refused.
        let c = BoundedCounter::new();
        let mut tickets = tickets_from_threads(&c, 8, 10_000, 50_000);
        assert_eq!(c.get(), 50_000, "final value must be min(attempts, max)");
        assert_eq!(
            tickets.len(),
            50_000,
            "successes must equal the final value"
        );
        tickets.sort_unstable();
        assert!(
            tickets.iter().copied().eq(0..50_000),
            "every ticket in 0..50_000 must be handed out exactly once"
        );
    }

    #[test]
    fn contention_alone_never_refuses() {
        // Far below the limit: losing a race is NOT a reason to report `Full`.
        let c = BoundedCounter::new();
        let tickets = tickets_from_threads(&c, 8, 2_000, u64::MAX);
        assert_eq!(tickets.len(), 16_000, "every attempt must succeed");
        assert_eq!(c.get(), 16_000, "no increment may be lost");
    }
}
