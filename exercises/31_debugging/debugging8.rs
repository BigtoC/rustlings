// Module 5 · Debugging — part 8: a `MutexGuard` in a `match` scrutinee deadlocks its own thread.
//
// This is `debugging6` with a `Mutex` instead of a `RefCell`, and it is the
// version of the bug that reaches production:
//
//     match cache.lock().unwrap().get(&key).cloned() {
//         Some(value) => value,
//         None => {
//             let value = compute(key);
//             cache.lock().unwrap().insert(key, value.clone()); // never returns
//             value
//         }
//     }
//
// The `MutexGuard` returned by `lock()` is a temporary of the scrutinee, so it
// lives until the whole `match` ends. `.cloned()` does not help. It turns the
// `Option<&V>` into an owned `Option<V>`, which ends the BORROW of the guard,
// so the borrow checker is satisfied. But the guard VALUE is still alive, and
// only its `Drop` unlocks the mutex. In the `None` arm the same thread locks
// again. `std::sync::Mutex` is not reentrant: its docs say that locking a
// mutex the current thread already holds "will not return" (it might panic or
// deadlock). In practice the thread just blocks forever, silently, while it
// holds the lock, and every other thread that touches the cache piles up
// behind it. `RefCell` at least panics; a `Mutex` hangs.
//
// This exercise cannot hang: its second lock is a `try_lock()`, which fails
// with `WouldBlock` instead of blocking. That is a good trick for your own
// debugging, but it is not a lock: `try_lock` also fails whenever ANOTHER
// thread holds the mutex.
//
// The fix is the one from `debugging6`: end the statement that holds the
// guard before doing anything else. It also answers the design question that
// interviewers ask next: don't hold a lock while doing slow work. While
// `compute` runs under the lock, every other lookup waits, and a `compute`
// that consults the cache itself deadlocks. Releasing the lock opens a
// window, though: another thread can miss on the same key and store a value
// while you compute. Decide what happens then. Here the value stored first
// wins, so every caller shares one `Arc`. If even the duplicate work is
// unacceptable, the next step is a per-key "single flight" (for example a
// `OnceLock` per key), which is beyond this exercise.
//
// Part B is a quiz on where the guard dies. Edition 2024 changed two
// temporary scopes: temporaries of an `if let` scrutinee are dropped before
// the `else` block, and temporaries of a block's tail expression are dropped
// before the block's locals. It did NOT change `match`, `while let` or the
// `if let` then-block. Clippy can flag the `match` case for lock guards with
// `clippy::significant_drop_in_scrutinee`, but that lint is in the `nursery`
// group, so it is off by default.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// A memo cache shared by all the request handlers of a server, on many
// threads. That is why the map sits in a `Mutex` and not in a `RefCell` (a
// `RefCell` is not `Sync`). Values are `Arc<str>`, so a hit hands out a cheap
// pointer copy instead of a new `String`.
struct Cache {
    map: Mutex<HashMap<u64, Arc<str>>>,
}

impl Cache {
    fn new() -> Self {
        Cache {
            map: Mutex::new(HashMap::new()),
        }
    }

    // Part A: returns the value cached under `key`. On a miss it calls
    // `compute(key)` and caches the result. `compute` may be slow (think I/O),
    // so it must run WITHOUT the lock held. If another thread stored a value
    // for `key` while `compute` ran, that value is kept and returned instead,
    // so that all callers share one `Arc`.
    fn get_or_compute(&self, key: u64, compute: impl FnOnce(u64) -> String) -> Arc<str> {
        // TODO: Once the Part B constants below exist (until then the tests
        // do not build), every test that misses panics, either inside the
        // test's `compute` ("compute ran while the cache mutex was locked") or
        // at the `expect` below, because `try_lock` returns
        // `Err(WouldBlock)`. The `MutexGuard` from `lock()` is a temporary of
        // the `match` scrutinee, so it is alive in both arms, `.cloned()` or
        // not. With the `lock()` that production code would use, the thread
        // would deadlock on itself. Requirements:
        //   - the lock is released before `compute` runs;
        //   - a hit returns the cached `Arc` (no new `compute` call); a miss
        //     caches the value, but keeps a value that another thread stored
        //     for `key` in the meantime, and returns that one;
        //   - don't copy the map out of the mutex to get rid of the guard
        //     (a test counts the `Arc` handles while `compute` runs);
        //   - once no guard is alive at the second lock, use `lock()` there,
        //     as production code would (switch only when the tests pass: with
        //     `lock()`, a guard you forgot to release hangs the test instead
        //     of failing it);
        //   - keep the `Mutex`, no `unsafe`, don't change the tests.
        // Until the lock is released before `compute` runs, the tests will
        // fail.
        match self.map.lock().unwrap().get(&key).cloned() {
            Some(hit) => hit,
            None => {
                let value: Arc<str> = compute(key).into();
                // Production code would call `self.map.lock()` here and wait
                // forever for itself. `try_lock` fails instead of blocking.
                self.map
                    .try_lock()
                    .expect("the cache is still locked, by this very thread")
                    .insert(key, Arc::clone(&value));
                value
            }
        }
    }
}

// Part B: a quiz. For each position, predict whether the SAME thread can lock
// the mutex again there, while a guard created by `m.lock()` might still be
// alive:
//   - MATCH_ARM_CAN_RELOCK: inside an arm of
//     `match m.lock().unwrap().pop() { .. }`;
//   - IF_LET_THEN_CAN_RELOCK: inside the then-block of
//     `if let Some(job) = m.lock().unwrap().pop() { .. } else { .. }`;
//   - IF_LET_ELSE_CAN_RELOCK_2024: inside the `else` block of that `if let`,
//     in edition 2024;
//   - LET_STMT_CAN_RELOCK: in the statement right after
//     `let job = m.lock().unwrap().pop();`.
//
// TODO: The `quiz_*` tests use four constants that do not exist yet, so the
// tests fail to build with E0425 "cannot find value `MATCH_ARM_CAN_RELOCK` in
// this scope" (and the same for the other three). Declare each one as a
// `const` of type `bool` that holds your prediction: `true` if the mutex can
// be locked again at that point. Each quiz test then checks your prediction
// against what `try_lock` actually reports. Predict first and run second: a
// guess you had to flip teaches you nothing.
// Until the four constants exist, this exercise will not compile.

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    // ---- Part A: `get_or_compute` ----

    #[test]
    fn a_miss_computes_once_and_a_hit_reuses_the_value() {
        let cache = Cache::new();
        let calls = Cell::new(0);
        let compute = |key: u64| {
            calls.set(calls.get() + 1);
            format!("value {key}")
        };
        let first = cache.get_or_compute(7, compute);
        assert_eq!(&*first, "value 7");
        let second = cache.get_or_compute(7, compute);
        // The second call is a hit: no new computation, and the very same
        // `Arc` comes back.
        assert_eq!(calls.get(), 1);
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn keys_are_cached_separately() {
        let cache = Cache::new();
        let calls = Cell::new(0);
        let compute = |key: u64| {
            calls.set(calls.get() + 1);
            format!("v{key}")
        };
        for key in [1, 2, 1, 2, 3] {
            assert_eq!(&*cache.get_or_compute(key, compute), format!("v{key}"));
        }
        assert_eq!(calls.get(), 3);
        assert_eq!(cache.map.lock().unwrap().len(), 3);
    }

    #[test]
    fn compute_runs_without_the_lock_held() {
        let cache = Cache::new();
        let value = cache.get_or_compute(1, |key| {
            // A lookup on another thread would need the lock right now.
            // (`try_lock` never blocks, so a lock that is still held fails
            // this test instead of hanging it.)
            assert!(
                cache.map.try_lock().is_ok(),
                "compute ran while the cache mutex was locked"
            );
            format!("value {key}")
        });
        assert_eq!(&*value, "value 1");
    }

    #[test]
    fn a_miss_does_not_copy_the_map() {
        let cache = Cache::new();
        let first = cache.get_or_compute(1, |key| format!("value {key}"));
        // One handle sits in the map, one is `first`.
        assert_eq!(Arc::strong_count(&first), 2);
        let second = cache.get_or_compute(2, |key| {
            // Copying the whole map out of the mutex also frees the lock, but
            // it copies every entry on every lookup. A copy that is still
            // alive here holds a third handle to `first`.
            assert_eq!(
                Arc::strong_count(&first),
                2,
                "compute ran while a copy of the cached map was alive"
            );
            format!("value {key}")
        });
        assert_eq!(&*second, "value 2");
        assert_eq!(Arc::strong_count(&first), 2);
    }

    #[test]
    fn a_value_stored_while_computing_is_kept() {
        let cache = Cache::new();
        let value = cache.get_or_compute(5, |_| {
            // Play the other thread: it misses on the same key and stores its
            // value while this call is still computing.
            cache
                .map
                .try_lock()
                .expect("compute ran while the cache mutex was locked")
                .insert(5, Arc::from("first"));
            String::from("second")
        });
        // The value that was stored first wins, and this caller gets it too.
        assert_eq!(&*value, "first");
        let again = cache.get_or_compute(5, |_| unreachable!("5 is cached"));
        assert!(Arc::ptr_eq(&value, &again));
    }

    #[test]
    fn the_cache_can_be_shared_between_threads() {
        // Compiles only while `Cache` is `Send + Sync`.
        fn shared<T: Send + Sync>() {}
        shared::<Cache>();
    }

    // ---- Part B: the quiz ----
    //
    // Each test creates a guard in one position and asks `try_lock` whether
    // the mutex is free again at the marked point.

    fn three_jobs() -> Mutex<Vec<u32>> {
        Mutex::new(vec![1, 2, 3])
    }

    #[test]
    fn quiz_match_arm() {
        let jobs = three_jobs();
        let can_relock = match jobs.lock().unwrap().pop() {
            Some(job) => {
                assert_eq!(job, 3);
                jobs.try_lock().is_ok() // <- here
            }
            None => unreachable!("there are three jobs"),
        };
        assert_eq!(
            can_relock, MATCH_ARM_CAN_RELOCK,
            "MATCH_ARM_CAN_RELOCK is wrong: when is the guard in a `match` scrutinee dropped?"
        );
    }

    #[test]
    fn quiz_if_let_then_block() {
        let jobs = three_jobs();
        let can_relock = if let Some(job) = jobs.lock().unwrap().pop() {
            assert_eq!(job, 3);
            jobs.try_lock().is_ok() // <- here
        } else {
            unreachable!("there are three jobs")
        };
        assert_eq!(
            can_relock, IF_LET_THEN_CAN_RELOCK,
            "IF_LET_THEN_CAN_RELOCK is wrong: what did edition 2024 change, and what not?"
        );
    }

    #[test]
    fn quiz_if_let_else_block_2024() {
        let jobs = Mutex::new(Vec::<u32>::new());
        let can_relock = if let Some(job) = jobs.lock().unwrap().pop() {
            unreachable!("there are no jobs, but got {job}")
        } else {
            jobs.try_lock().is_ok() // <- here
        };
        assert_eq!(
            can_relock, IF_LET_ELSE_CAN_RELOCK_2024,
            "IF_LET_ELSE_CAN_RELOCK_2024 is wrong: what did edition 2024 change?"
        );
    }

    #[test]
    fn quiz_after_a_let_statement() {
        let jobs = three_jobs();
        let job = jobs.lock().unwrap().pop();
        let can_relock = jobs.try_lock().is_ok(); // <- here
        assert_eq!(job, Some(3));
        assert_eq!(
            can_relock, LET_STMT_CAN_RELOCK,
            "LET_STMT_CAN_RELOCK is wrong: where does a `let` statement end?"
        );
    }
}
