//! Lab · proving *why* the `atomics2` handoff needs `Release`/`Acquire`
//!
//! The graded exercise `atomics2` only checks that the handoff is *functionally*
//! correct — and on x86/ARM a wrong ordering usually still appears to work, so a
//! runtime test can never catch the bug. `loom` closes that gap: it is a
//! model checker for the C11 memory model that runs a small concurrent snippet
//! under **every** legal thread interleaving *and* every allowed memory
//! reordering, then fails if any of them violates an assertion.
//!
//! This module is compiled only under `--cfg loom` (see `deep-dive/Cargo.toml`),
//! so a normal `cargo test` never touches it. Run it with:
//!
//! ```text
//! RUSTFLAGS="--cfg loom" cargo test --manifest-path deep-dive/Cargo.toml loom_lab
//! ```
//!
//! Note the imports below: under loom we use `loom::sync::*` / `loom::thread`,
//! which are instrumented shadows of the `std` versions. That is the whole trick
//! — the code shape is identical to `atomics2`, only the atomics are
//! model-checked.

#[cfg(test)]
mod tests {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use loom::thread;

    /// The CORRECT handoff. The producer publishes the payload with a `Relaxed`
    /// store and then flips the flag with a `Release` store; the consumer reads
    /// the flag with an `Acquire` load. Across every interleaving loom explores,
    /// whenever the consumer *observes* the flag as `true` the `Release`/`Acquire`
    /// pair guarantees the payload write happened-before the read, so it can only
    /// ever see 42.
    #[test]
    fn release_acquire_publishes_the_payload() {
        loom::model(|| {
            let flag = Arc::new(AtomicBool::new(false));
            let data = Arc::new(AtomicU64::new(0));

            let (f_prod, d_prod) = (flag.clone(), data.clone());
            let producer = thread::spawn(move || {
                d_prod.store(42, Ordering::Relaxed);
                f_prod.store(true, Ordering::Release);
            });

            // If we have observed the flag, the data MUST be published. (We do not
            // spin — loom itself schedules the consumer both before and after the
            // producer's stores, covering the "not yet ready" case too.)
            if flag.load(Ordering::Acquire) {
                assert_eq!(data.load(Ordering::Relaxed), 42);
            }

            producer.join().unwrap();
        });
    }

    /// The BROKEN handoff. With `Relaxed` on BOTH the flag store and the flag load
    /// there is no happens-before edge, so loom is allowed to schedule an
    /// execution in which the consumer sees `flag == true` while still reading the
    /// stale `data == 0`. loom finds exactly that interleaving and the assertion
    /// fires — which is why this test is expected to panic. (Comment out
    /// `#[should_panic]` and run it to see loom print the failing schedule.)
    #[test]
    #[should_panic]
    fn relaxed_relaxed_can_observe_a_stale_payload() {
        loom::model(|| {
            let flag = Arc::new(AtomicBool::new(false));
            let data = Arc::new(AtomicU64::new(0));

            let (f_prod, d_prod) = (flag.clone(), data.clone());
            let producer = thread::spawn(move || {
                d_prod.store(42, Ordering::Relaxed);
                f_prod.store(true, Ordering::Relaxed);
            });

            if flag.load(Ordering::Relaxed) {
                // loom will find a schedule where this reads 0, panicking the model.
                assert_eq!(data.load(Ordering::Relaxed), 42);
            }

            producer.join().unwrap();
        });
    }
}
