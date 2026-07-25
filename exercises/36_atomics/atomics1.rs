// Atomics - atomic operations and memory ordering, part 1: a lock-free counter.
//
// A plain `usize` shared across threads and mutated with `+= 1` is a data race:
// each increment is really "load, add, store", and two threads can interleave
// those steps and lose an update. A mutex would fix it, but it is overkill for a
// single integer. `AtomicUsize::fetch_add` performs the whole read-modify-write
// as ONE indivisible hardware operation, so no update is ever lost - and it does
// this WITHOUT a lock. `Ordering::Relaxed` is enough here: we only need the count
// itself to be atomic, we are not using it to publish any OTHER memory to the
// readers, so we do not need to pay for stronger ordering.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn concurrent_count(threads: usize, per_thread: usize) -> usize {
    let counter = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let c = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..per_thread {
                    // TODO: increment the shared counter atomically. A plain
                    // `let _ = c;` here leaves the loop body empty, so the counter
                    // stays 0 and the test below fails. Perform the whole
                    // read-modify-write in one step:
                    //   c.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    counter.load(Ordering::Relaxed)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_every_increment() {
        assert_eq!(concurrent_count(8, 10_000), 80_000);
    }
}
