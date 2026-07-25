// Atomics - atomic operations and memory ordering, part 3: a spinlock from CAS.
//
// A mutex is not magic - underneath it is an atomic flag plus a compare-and-swap
// (CAS) loop. `compare_exchange(current, new, success, failure)` atomically reads
// the flag and, only if it still equals `current`, writes `new`; it returns
// `Ok` on success and `Err(actual)` on failure. To "take" a lock we try to flip
// the flag from `false` to `true`: exactly one thread can win that race.
//
// The orderings matter: the successful lock uses `Acquire` and `unlock` uses a
// `Release` store, so everything the previous holder did inside the critical
// section is visible to the next holder (the same Release/Acquire publishing you
// saw in part 2). While spinning we call `std::hint::spin_loop()` to hint the CPU
// that we are in a busy-wait. Real spinlocks loop on `compare_exchange_weak`,
// which may fail SPURIOUSLY (report `Err` even when the value matched) but
// compiles to cheaper code on some architectures; retrying in a loop absorbs the
// spurious failures. We use the strong `compare_exchange` for a single try here.

use std::sync::atomic::{AtomicBool, Ordering};

struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    fn new() -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
        }
    }

    fn try_lock(&self) -> bool {
        // One atomic false->true flip; only the winner gets `Ok`. Acquire on
        // success publishes the previous holder's critical section to us.
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    fn lock(&self) {
        while !self.try_lock() {
            std::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::thread;

    #[test]
    fn single_threaded_state_machine() {
        let l = SpinLock::new();
        assert!(l.try_lock()); // free -> we take it
        assert!(!l.try_lock()); // already held -> second try fails
        l.unlock();
        assert!(l.try_lock()); // released -> can take it again
    }

    #[test]
    fn guards_shared_state_across_threads() {
        let lock = Arc::new(SpinLock::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let lock = Arc::clone(&lock);
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    for _ in 0..1000 {
                        lock.lock();
                        // Only one thread is inside the critical section, so a
                        // plain non-atomic style load/store would be safe; we use
                        // the atomic API purely to share the value.
                        let now = counter.load(Ordering::Relaxed);
                        counter.store(now + 1, Ordering::Relaxed);
                        lock.unlock();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::Relaxed), 8000);
    }
}
