// Atomics - atomic operations and memory ordering, part 2: Release/Acquire.
//
// Atomicity alone is not the whole story. The interesting question in concurrent
// code is: when one thread writes some data and then flips a flag, and another
// thread sees the flag flipped, is it GUARANTEED to also see the data? With
// `Relaxed` on both sides the answer is NO - the CPU and compiler may reorder the
// two writes, so the reader can see `flag == true` yet still read the OLD data.
//
// A `store(.., Ordering::Release)` paired with a `load(.., Ordering::Acquire)` of
// the SAME atomic fixes this. The Release store acts as a one-way barrier: every
// write the producer did BEFORE it (even Relaxed ones) is guaranteed to be
// visible to any thread that later reads the flag with an Acquire load. That
// pairing creates a happens-before edge that "publishes" the data across threads.
// (The loom lab in deep-dive/ proves that Relaxed/Relaxed here can observe 0.)

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;

fn handoff() -> u64 {
    let flag = Arc::new(AtomicBool::new(false));
    let data = Arc::new(AtomicU64::new(0));
    let (f2, d2) = (Arc::clone(&flag), Arc::clone(&data));
    thread::spawn(move || {
        d2.store(42, Ordering::Relaxed);
        f2.store(true, Ordering::Release);
    });

    // The Acquire load pairs with the producer's Release store: once we observe
    // `true`, the prior Relaxed write of `data` is guaranteed to be visible.
    while !flag.load(Ordering::Acquire) {
        std::hint::spin_loop();
    }
    data.load(Ordering::Relaxed)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_sees_released_data() {
        // Repeat many times: the Release/Acquire pairing must publish the write
        // on every single run, never leaving us reading the initial 0.
        for _ in 0..100 {
            assert_eq!(handoff(), 42);
        }
    }
}
