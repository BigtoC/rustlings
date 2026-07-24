// Module 4 · Send / Sync — part 2: shared MUTABLE state with `Arc<Mutex<T>>`.
//
// `Arc<T>` lets many threads share a value, but only gives out `&T` — you
// cannot mutate through it. To mutate shared state safely you need interior
// mutability guarded by a lock: `Mutex<T>`. Call `.lock()` to block until you
// hold the lock; it returns a `Result` whose `Ok` is a guard that derefs to
// `&mut T`. When the guard drops, the lock is released.
//
// The idiomatic "shared counter" type is therefore `Arc<Mutex<T>>`:
//   - `Arc`   — many owners across threads (`Send + Sync`),
//   - `Mutex` — exclusive, race-free mutation of the inner value.

use std::sync::{Arc, Mutex};
use std::thread;

// Spawn `threads` workers, each incrementing the shared counter `per_thread`
// times. After joining them all, the counter equals `threads * per_thread`.
fn count(threads: usize, per_thread: usize) -> usize {
    let counter = Arc::new(Mutex::new(0usize));

    let mut handles = Vec::new();
    for _ in 0..threads {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..per_thread {
                *counter.lock().unwrap() += 1;
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    *counter.lock().unwrap()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_increments_are_counted() {
        assert_eq!(count(8, 1000), 8000);
    }

    #[test]
    fn zero_threads_stay_zero() {
        assert_eq!(count(0, 1000), 0);
    }
}
