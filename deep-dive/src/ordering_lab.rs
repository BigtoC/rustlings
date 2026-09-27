//! Lab · SeqCst vs Acquire/Release: store buffering, IRIW, Peterson's lock
//!
//! `36_atomics/atomics2` (and the `loom_lab` proof of it) shows what
//! `Release`/`Acquire` is for: message passing. The producer writes the data,
//! then the flag with `Release`; a consumer whose `Acquire` load *reads that
//! flag value* sees the data. The edge exists because the load read the
//! store. This lab is about the programs where no load reads the store it
//! needs to be ordered against, which is when `SeqCst`, or a `fence(SeqCst)`,
//! earns its cost:
//!
//! - **Store buffering (SB)**, [`store_buffering`]: each thread stores its own
//!   flag, then loads the other's. Under Release/Acquire both can read the old
//!   value.
//! - **Peterson's lock**, [`Peterson`]: mutual exclusion for two threads from
//!   plain loads and stores, no CAS. Its entry protocol is the SB shape, so
//!   the Release/Acquire version lets both threads into the critical section.
//! - **IRIW** (independent reads of independent writes), [`iriw`]: two readers
//!   watch two unrelated writes. Must they agree on which came first?
//! - **False sharing**, [`CachePadded`] and [`hammer`]: not an ordering bug,
//!   the performance cost of two hot atomics on one cache line.
//!
//! Compare with the graded `36_atomics` exercises: `atomics2` (message
//! passing, which Release/Acquire does handle), `atomics3` (a spinlock: its
//! CAS is a read-modify-write, which is why it does not need Peterson's
//! fences; loom-checked in [`crate::treiber`]) and `atomics5` (the ABA
//! problem, continued with heap nodes in [`crate::treiber`]).
//! [`crate::myarc`] applies the same Release/Acquire reasoning to a reference
//! count.
//!
//! # Sharpest question
//!
//! *Give the store-buffering example where both threads read 0 under
//! Acquire/Release. Why does SeqCst forbid it, and why is memory reclamation
//! the hard part of a lock-free stack?*
//!
//! ```text
//! x = y = 0
//! thread 1: x.store(1, Release); r1 = y.load(Acquire)
//! thread 2: y.store(1, Release); r2 = x.load(Acquire)
//! r1 == 0 && r2 == 0 is allowed
//! ```
//!
//! Release/Acquire creates a happens-before edge only when an Acquire load
//! reads the value a Release store wrote. Here each load reads the initial 0,
//! so there is no edge at all, and nothing orders a thread's store before its
//! own later load of a *different* location. On hardware that is the store
//! buffer: the store waits in the core's buffer while the load is served from
//! the cache. `SeqCst` forbids the outcome: all SeqCst operations form one
//! total order that respects each thread's program order, and a SeqCst load
//! reads the latest store before it in that order. Whichever store comes
//! first, the other thread's load comes after it and reads 1. A `fence(SeqCst)`
//! between each store and load gives the same guarantee to `Relaxed` accesses.
//!
//! Reclamation: in a lock-free stack a CAS makes each push and pop atomic,
//! but a pop must dereference the top node *after* loading the pointer to it.
//! In between, another thread can pop that node and free it, so the read is a
//! use-after-free, and if the allocator reuses the address for a new node the
//! CAS succeeds with a stale link (ABA). A node can only be freed once no
//! thread can still hold a pointer to it, and lock-free code has no lock that
//! would tell it when that is. [`crate::treiber`] makes the bug deterministic
//! and shows the simplest fix.
//!
//! # The IRIW question
//!
//! *Writer 1 stores `x = 1`, writer 2 stores `y = 1`; reader C loads x then
//! y, reader D loads y then x, all loads `Acquire`. Can C see x=1, y=0 while
//! D sees y=1, x=0, that is, can the readers disagree on the order of the
//! writes? Answer for the language, then for the hardware.*
//!
//! - **Language (Rust / C++20):** yes. Release/Acquire only orders threads
//!   pairwise; it gives independent writes no single global order. Making
//!   the four loads `SeqCst`, or putting a `fence(SeqCst)` between each
//!   reader's two loads, forbids it (`seqcst_iriw_readers_always_agree`
//!   makes all six accesses `SeqCst` and asserts 0).
//! - **Hardware, `Acquire` loads:** no on x86_64 and no on ARMv8. x86-TSO is
//!   multi-copy atomic (all cores see stores in one order), and ARMv8 is
//!   "other-multi-copy atomic" with `ldapr`/`ldar` keeping each reader's two
//!   loads in order (architecture manuals; the ARMv8 half was also measured
//!   here, see below). POWER is the classic machine where it does happen,
//!   which is why the language cannot promise it.
//! - **Hardware, `Relaxed` loads:** ARMv8 may reorder a reader's two loads,
//!   and the M4 Pro this lab was written on did, rarely. x86 still never
//!   does.
//!
//! # What each tool can show (verified)
//!
//! Hardware numbers are from an Apple M4 Pro (aarch64-apple-darwin, rustc
//! 1.96, `--release`) running the `#[ignore]`d `*_hardware_stress` tests
//! several times; Miri numbers are from nightly Miri (2026-07-23) running the
//! same tests with their smaller Miri round counts. Counts vary from run to
//! run; "never" means never observed, and is guaranteed only where the
//! Language column says "forbidden".
//!
//! | Outcome | Language | loom 0.7.2 | Miri | M4 Pro |
//! | --- | --- | --- | --- | --- |
//! | SB, Relaxed | allowed | found | 25-30% of rounds | every run, 0.25-11% of rounds |
//! | SB, Release/Acquire | allowed | found | 25-30% of rounds | 0 to 45,000 per 10M rounds, 0 in about half the runs |
//! | SB, SeqCst accesses | forbidden | FALSE ALARM | never | never |
//! | SB, Relaxed + `fence(SeqCst)` | forbidden | verified, exhaustive | never | never |
//! | IRIW, Relaxed loads | allowed | found | about 0.5% of rounds | 0 to 8 per 10M rounds |
//! | IRIW, Acquire loads | allowed | found | about 0.5% of rounds | never in 90M rounds |
//! | IRIW, SeqCst | forbidden | false alarm | never | never |
//! | IRIW, loads + `fence(SeqCst)` | forbidden | verified (bound 2 in CI; unbounded by hand) | not run | not run |
//! | Peterson, Release/Acquire | broken | lost update found | 10-13% of increments lost | 0 to 70 lost per 20M increments |
//! | Peterson, SeqCst accesses | correct | false alarm | exact | exact |
//! | Peterson, fenced | correct | verified, exhaustive | exact | exact |
//!
//! The "found" and "verified" loom entries are the models at the bottom of
//! this file (the Relaxed ones were checked by hand, see "Try it"). The three
//! false alarms were all reproduced with loom 0.7.2; only the SB one is kept
//! as a test, `sb_seqcst_accesses_are_a_loom_false_alarm`.
//!
//! loom's limit is documented in its README ("Unsupported features": SeqCst
//! accesses "are regarded as `AcqRel`", "`fence(SeqCst)` is supported"), and
//! in its source `thread::Set::seq_cst` is an empty function. That is why the
//! verified variants here use fences, and the SeqCst variants are checked by
//! real-thread tests instead. loom also misses executions in the other
//! direction: it does not explore load buffering (same README section).
//! Miri's README makes the matching disclaimer: its weak-memory emulation
//! "is not complete: there are legal behaviors that Miri will never
//! produce".
//!
//! The hardware answer also depends on the *target*, not just the CPU. The
//! same `x.store(1, Release); y.load(Acquire)` compiles to `stlr; ldapr` on
//! aarch64-apple-darwin, and `ldapr` may be satisfied before the earlier
//! `stlr` is visible, hence the SB hits above. On aarch64-unknown-linux-gnu
//! (baseline ARMv8.0, no `ldapr`) an Acquire load is `ldar`, which cannot
//! pass an earlier `stlr`, so that build cannot show Release/Acquire SB on
//! any CPU. On x86_64, loads and Release stores are plain `mov`, a SeqCst
//! store is `xchg`, and `fence(SeqCst)` is `lock or` to the stack; TSO lets a
//! load pass an earlier store to another location, so SB is observable there
//! unless a SeqCst store or fence sits between them. (Assembly checked with
//! `--emit asm`: stable 1.96 for the Apple target, nightly with
//! `-Zbuild-std=core` for the two Linux targets. The x86_64 behavior is the
//! documented TSO model, not run here.) Only the language answer is
//! portable.
//!
//! False sharing, measured with `false_sharing_benchmark` (two threads, 20M
//! `fetch_add`s each): adjacent counters took 130-205 ms, `CachePadded` ones
//! 32-35 ms, about 4-6x faster. Counters only 64 bytes apart took 32-44 ms,
//! so on this machine the penalty is gone, or almost gone, at 64 bytes,
//! although macOS reports 128-byte lines (`sysctl hw.cachelinesize`). 128 is
//! the conservative choice crossbeam also makes, not a measured necessity
//! here.
//!
//! # Invariants
//!
//! - Each round of a hardware harness stores a fresh value (the round
//!   number), so "missed" means "read a value from an earlier round" and
//!   nothing needs resetting between rounds.
//! - Every shared variable is an atomic, including the counter a broken lock
//!   fails to protect. A broken variant therefore loses updates (a logic
//!   race) instead of racing on plain memory (undefined behavior), which is
//!   what lets it run under Miri and be counted.
//! - Only outcomes the language guarantees are asserted: 0 for SeqCst and
//!   fenced SB, 0 for SeqCst IRIW, an exact count for the SeqCst and fenced
//!   Peterson locks. Weaker outcomes are printed by `#[ignore]`d stress tests
//!   or proven reachable by loom.
//! - [`Peterson`] serves exactly two threads, 0 and 1.
//!
//! # Run it
//!
//! ```text
//! # unit tests (the guaranteed outcomes, on real threads)
//! cargo test --manifest-path deep-dive/Cargo.toml ordering_lab
//!
//! # loom models: SB, Peterson and IRIW
//! RUSTFLAGS="--cfg loom" cargo test --manifest-path deep-dive/Cargo.toml \
//!     --lib ordering_lab::loom_tests
//!
//! # hardware stress runs and the false-sharing benchmark (print, never fail)
//! cargo test --release --manifest-path deep-dive/Cargo.toml --lib \
//!     ordering_lab -- --ignored --nocapture
//!
//! # the same stress runs under Miri's weak-memory emulation (about 10 s)
//! cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib \
//!     ordering_lab::tests -- --ignored --nocapture
//! ```
//!
//! # Try it
//!
//! - In `seqcst_store_buffering_never_misses_both`, change
//!   `store_buffering(SeqCst, SeqCst, ..)` to `(Release, Acquire, ..)` and run
//!   the test under Miri: it fails every time (about a quarter of the 100
//!   rounds miss both; 22 to 25 for `-Zmiri-seed` 0, 1 and 2). Natively the
//!   result depends on the machine, the build and the load: on the M4 Pro,
//!   131 of 500 debug runs and 171 of 500 release runs failed in one
//!   sitting, yet a whole 10M-round `sb_hardware_stress` run in the same
//!   sitting showed no Release/Acquire SB at all. That is why no test here
//!   asserts that a weak outcome shows up.
//! - In `Peterson::lock`, delete either `fence(SeqCst)` of the `Fenced` arm,
//!   or make only the `turn` load `Relaxed` (`self.turn.load(Relaxed)` in the
//!   `while`), and rerun the loom models: `peterson_fenced_excludes` fails
//!   with "lost update" in each case. The `Relaxed` turn load is caught only
//!   because thread 1 takes the lock twice in the model (verified: with one
//!   acquisition each it passes). A model only explores the scenario you
//!   wrote.
//! - In `peterson_fenced_excludes`, check `Flavor::SeqCst` instead: loom
//!   reports "Peterson SeqCst: lost update", a false alarm, because it
//!   treats the SeqCst accesses as AcqRel.
//! - In `loom_tests::sb_fence_seqcst_forbids_both_zero`, pass `false` for
//!   `fenced` (plain `Relaxed` SB) and loom finds the both-zero execution.
//!   Likewise `iriw(Relaxed, false)` in `iriw_fence_seqcst_readers_agree`.
//! - Change `#[repr(align(128))]` on [`CachePadded`] to `align(64)`: the
//!   layout test fails (the facts it asserts are the 128-byte ones), and the
//!   benchmark shows how much 64 bytes buys on your machine.
//! - Run the stress tests on another machine, say an x86_64 box, or an
//!   aarch64 Linux build (where an Acquire load is `ldar`), and compare with
//!   the M4 Pro column. The language column is the only one that must match.

use std::ops::Deref;
use std::sync::atomic::Ordering::{self, AcqRel, Acquire, Relaxed, Release, SeqCst};
use std::sync::atomic::{AtomicU64, AtomicUsize, fence};
use std::thread;
use std::time::{Duration, Instant};

/// What [`Peterson`] is built from: std's atomics in a normal build, loom's
/// instrumented copies under `--cfg loom`. The loom models at the bottom of
/// this file therefore check the very lock the real-thread tests run. The
/// hardware harnesses (`store_buffering`, `iriw`, `hammer`) always use std's
/// atomics and [`backoff`]: they exist to run on a real CPU.
mod sync {
    #[cfg(loom)]
    pub use loom::sync::atomic::{AtomicBool, AtomicUsize, fence};
    #[cfg(not(loom))]
    pub use std::sync::atomic::{AtomicBool, AtomicUsize, fence};

    /// Peterson's busy-wait. Under loom every iteration must yield, or the
    /// model would explore an unbounded number of loop iterations.
    #[cfg(loom)]
    pub fn spin_wait(_spins: &mut u32) {
        loom::thread::yield_now();
    }

    /// Peterson's busy-wait: the same polite [`backoff`](super::backoff) as
    /// the hardware harnesses.
    #[cfg(not(loom))]
    pub fn spin_wait(spins: &mut u32) {
        super::backoff(spins);
    }
}

/// Busy-wait politely: spin a little, then yield, so a partner thread that
/// the OS descheduled (a loaded CI runner, `cargo test` running tests in
/// parallel) gets the CPU back instead of us burning our whole time slice.
fn backoff(spins: &mut u32) {
    if *spins < 64 {
        *spins += 1;
        std::hint::spin_loop();
    } else {
        thread::yield_now();
    }
}

/// `thread::Builder::spawn_scoped` with a name, panicking if the OS refuses a
/// thread (the workspace's `clippy.toml` bans the bare `Scope::spawn`).
fn spawn_scoped<'scope, T: Send + 'scope>(
    s: &'scope thread::Scope<'scope, '_>,
    name: &str,
    f: impl FnOnce() -> T + Send + 'scope,
) -> thread::ScopedJoinHandle<'scope, T> {
    thread::Builder::new()
        .name(name.to_owned())
        .spawn_scoped(s, f)
        .expect("failed to spawn a thread")
}

// ---------------------------------------------------------------------------
// Cache-line padding
// ---------------------------------------------------------------------------

/// Pads and aligns a value to 128 bytes, so two `CachePadded` values never
/// share a cache line. 128, not 64: macOS reports 128-byte lines on Apple
/// silicon (`sysctl hw.cachelinesize` prints 128 on the M4 Pro this lab was
/// written on), and on x86_64 Intel's spatial prefetcher pulls 64-byte lines
/// in adjacent pairs. `crossbeam_utils::CachePadded` also picks 128 on
/// x86_64, aarch64 and powerpc64; its source cites that prefetcher, the
/// 128-byte lines of big ARM cores, and powerpc64's 128-byte lines.
#[derive(Debug, Default)]
#[repr(align(128))]
pub struct CachePadded<T>(pub T);

impl<T> CachePadded<T> {
    pub const fn new(value: T) -> Self {
        CachePadded(value)
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Store buffering (SB)
// ---------------------------------------------------------------------------

/// Runs the store-buffering litmus test for `iters` rounds on two real
/// threads and returns in how many rounds BOTH threads missed the other's
/// store (the `r1 == r2 == 0` outcome of the textbook version):
///
/// ```text
/// thread A: x.store(round, store); r1 = y.load(load)
/// thread B: y.store(round, store); r2 = x.load(load)
/// ```
///
/// Each round stores a fresh value (the round number), so nothing needs
/// resetting, and "missed" means "read a value older than this round". With
/// `SeqCst` on every access the answer is 0 on every machine (the language
/// guarantees it). With `Release`/`Acquire` or `Relaxed` the language allows
/// both threads to miss, and whether a given CPU actually shows it is a
/// hardware question.
///
/// # Panics
///
/// Unless `store` is a store ordering (`Relaxed`, `Release`, `SeqCst`) and
/// `load` a load ordering (`Relaxed`, `Acquire`, `SeqCst`).
pub fn store_buffering(store: Ordering, load: Ordering, iters: usize) -> usize {
    check_orderings(store, load);
    sb_rounds(iters, |mine, theirs, round| {
        mine.store(round, store);
        theirs.load(load)
    })
}

/// The store-buffering test with `Relaxed` accesses and a `fence(SeqCst)`
/// between each thread's store and its load. The two SeqCst fences are
/// totally ordered, and that order forbids both threads missing: this also
/// returns 0 on every machine.
pub fn store_buffering_fenced(iters: usize) -> usize {
    sb_rounds(iters, |mine, theirs, round| {
        mine.store(round, Relaxed);
        fence(SeqCst);
        theirs.load(Relaxed)
    })
}

/// Panics unless `store` is valid for a store and `load` for a load.
fn check_orderings(store: Ordering, load: Ordering) {
    assert!(
        matches!(store, Relaxed | Release | SeqCst),
        "{store:?} is not a store ordering: use Relaxed, Release or SeqCst"
    );
    assert!(
        matches!(load, Relaxed | Acquire | SeqCst),
        "{load:?} is not a load ordering: use Relaxed, Acquire or SeqCst"
    );
}

/// Drives both halves of the SB test and counts the rounds both missed.
fn sb_rounds<F>(iters: usize, side: F) -> usize
where
    F: Fn(&AtomicUsize, &AtomicUsize, usize) -> usize + Sync,
{
    // Padded: x and y on different cache lines, as in a real program where
    // the two flags are unrelated.
    let x = CachePadded::new(AtomicUsize::new(0));
    let y = CachePadded::new(AtomicUsize::new(0));
    let (a, b) = thread::scope(|s| {
        let a = spawn_scoped(s, "sb-a", || sb_side(&x, &y, iters, &side));
        let b = spawn_scoped(s, "sb-b", || sb_side(&y, &x, iters, &side));
        (a.join().unwrap(), b.join().unwrap())
    });
    a.iter().zip(&b).filter(|&(&a, &b)| a && b).count()
}

/// One thread's half of the SB test: for each round, did it miss the other
/// thread's store of that round?
fn sb_side<F>(mine: &AtomicUsize, theirs: &AtomicUsize, iters: usize, side: &F) -> Vec<bool>
where
    F: Fn(&AtomicUsize, &AtomicUsize, usize) -> usize,
{
    (1..=iters)
        .map(|round| {
            let missed = side(mine, theirs, round) < round;
            // Lockstep: start the next round only once the other thread has
            // made its store for this one, so the two threads keep running
            // the same round at about the same time. (This load is not part
            // of the litmus test, so `Relaxed` is fine.)
            let mut spins = 0;
            while theirs.load(Relaxed) < round {
                backoff(&mut spins);
            }
            missed
        })
        .collect()
}

// ---------------------------------------------------------------------------
// IRIW: independent reads of independent writes
// ---------------------------------------------------------------------------

/// Runs the IRIW litmus test for `iters` rounds on four real threads and
/// returns in how many rounds the two readers saw the two independent writes
/// in opposite orders:
///
/// ```text
/// writer 1: x.store(round, store)
/// writer 2: y.store(round, store)
/// reader C: c1 = x.load(load); c2 = y.load(load)
/// reader D: d1 = y.load(load); d2 = x.load(load)
/// counted:  C saw x new, y old  AND  D saw y new, x old
/// ```
///
/// The language forbids this when the four loads are `SeqCst`: they then sit
/// in one total order that both readers agree on. (Passing `SeqCst` for both
/// arguments makes all six accesses `SeqCst`.) With `Acquire` loads it is
/// allowed by the language, yet x86_64 and ARMv8 forbid it in hardware; see
/// the module docs.
///
/// # Panics
///
/// On the same invalid orderings as [`store_buffering`]. They are checked
/// before any thread starts: std's atomics would panic inside the writers
/// (or the readers), and the surviving threads would wait for them at the
/// next start line forever.
pub fn iriw(store: Ordering, load: Ordering, iters: usize) -> usize {
    check_orderings(store, load);
    let x = CachePadded::new(AtomicUsize::new(0));
    let y = CachePadded::new(AtomicUsize::new(0));
    // A start line per round: nobody starts round r before all four threads
    // finished round r - 1, so the writes and reads of a round overlap.
    let arrived = CachePadded::new(AtomicUsize::new(0));
    let start_round = |round: usize| {
        arrived.fetch_add(1, AcqRel);
        let mut spins = 0;
        while arrived.load(Acquire) < 4 * round {
            backoff(&mut spins);
        }
    };
    let writer = |var: &AtomicUsize| {
        for round in 1..=iters {
            start_round(round);
            var.store(round, store);
        }
    };
    let reader = |first: &AtomicUsize, second: &AtomicUsize| -> Vec<bool> {
        (1..=iters)
            .map(|round| {
                start_round(round);
                let saw_first = first.load(load) == round;
                let saw_second = second.load(load) == round;
                saw_first && !saw_second
            })
            .collect()
    };
    thread::scope(|s| {
        spawn_scoped(s, "iriw-w1", || writer(&x));
        spawn_scoped(s, "iriw-w2", || writer(&y));
        let c = spawn_scoped(s, "iriw-c", || reader(&x, &y));
        let d = spawn_scoped(s, "iriw-d", || reader(&y, &x));
        let (c, d) = (c.join().unwrap(), d.join().unwrap());
        c.iter().zip(&d).filter(|&(&c, &d)| c && d).count()
    })
}

// ---------------------------------------------------------------------------
// Peterson's lock
// ---------------------------------------------------------------------------

/// Which orderings a [`Peterson`] lock uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// BROKEN: `Release` stores and `Acquire` loads. Each thread's
    /// "store my flag, then load yours" is the store-buffering shape, and
    /// Release/Acquire does not stop both threads reading the old `false`:
    /// both enter the critical section.
    ReleaseAcquire,
    /// Correct: every access `SeqCst`. loom cannot confirm it (loom treats
    /// SeqCst accesses as AcqRel), so real threads check it instead
    /// (`seqcst_peterson_loses_no_update`).
    SeqCst,
    /// Correct: `Relaxed` stores, each followed by a `fence(SeqCst)`, and
    /// `Acquire` loads. loom verifies this one.
    Fenced,
}

/// Peterson's mutual-exclusion lock for exactly two threads, numbered 0 and
/// 1, built from plain loads and stores (no compare-and-swap).
///
/// To enter, a thread raises its flag and gives the turn away, then waits
/// while the other thread's flag is up and it is still the other thread's
/// turn. It needs the store to `flag[me]` to be visible to the other thread
/// before this thread reads `flag[other]`: a store followed by a load of a
/// different location, which is exactly what `Release`/`Acquire` does not
/// order.
pub struct Peterson {
    flag: [sync::AtomicBool; 2],
    turn: sync::AtomicUsize,
    flavor: Flavor,
}

impl Peterson {
    pub fn new(flavor: Flavor) -> Self {
        Peterson {
            flag: [sync::AtomicBool::new(false), sync::AtomicBool::new(false)],
            turn: sync::AtomicUsize::new(0),
            flavor,
        }
    }

    /// Enters the critical section as thread `me` (0 or 1).
    pub fn lock(&self, me: usize) {
        assert!(me < 2, "Peterson's lock is for two threads, 0 and 1");
        let other = 1 - me;
        match self.flavor {
            Flavor::ReleaseAcquire => {
                self.flag[me].store(true, Release);
                self.turn.store(other, Release);
            }
            Flavor::SeqCst => {
                self.flag[me].store(true, SeqCst);
                self.turn.store(other, SeqCst);
            }
            Flavor::Fenced => {
                self.flag[me].store(true, Relaxed);
                // Orders the flag store before the turn store in the single
                // total order of SeqCst fences. One fence after both stores
                // is not enough: nothing would then relate the order of the
                // two threads' turn stores to the fences, and loom finds an
                // execution with both threads inside.
                sync::fence(SeqCst);
                self.turn.store(other, Relaxed);
                // Orders both stores before the loads below: the fix for the
                // store-buffering shape.
                sync::fence(SeqCst);
            }
        }
        // Acquire (or SeqCst) on both loads: whichever one lets us in, it
        // read a value the other thread wrote after its previous critical
        // section (its unlock, or its turn store after a Release or a fence),
        // so that critical section happens-before ours.
        let load = if self.flavor == Flavor::SeqCst {
            SeqCst
        } else {
            Acquire
        };
        let mut spins = 0;
        while self.flag[other].load(load) && self.turn.load(load) == other {
            sync::spin_wait(&mut spins);
        }
    }

    /// Leaves the critical section as thread `me`.
    pub fn unlock(&self, me: usize) {
        let order = if self.flavor == Flavor::SeqCst {
            SeqCst
        } else {
            Release
        };
        self.flag[me].store(false, order);
    }
}

/// Two threads each take the lock `iters` times and increment a shared
/// counter inside it with a separate load and store. Returns the final
/// count: `2 * iters` if the lock excluded, less if both threads were inside
/// at once and one increment overwrote the other (a lost update).
///
/// The counter is an atomic accessed with `Relaxed` load + store, not a plain
/// integer: a broken lock then causes a *logic* race (lost updates) and not
/// a data race, which would be undefined behavior and would make Miri reject
/// the test instead of letting it count.
#[cfg(not(loom))]
pub fn peterson_count(flavor: Flavor, iters: usize) -> usize {
    let lock = Peterson::new(flavor);
    let counter = AtomicUsize::new(0);
    thread::scope(|s| {
        for me in 0..2 {
            let (lock, counter) = (&lock, &counter);
            spawn_scoped(s, "peterson", move || {
                for _ in 0..iters {
                    lock.lock(me);
                    let seen = counter.load(Relaxed);
                    counter.store(seen + 1, Relaxed);
                    lock.unlock(me);
                }
            });
        }
    });
    counter.into_inner()
}

// ---------------------------------------------------------------------------
// False sharing
// ---------------------------------------------------------------------------

/// Two counters side by side: 16 bytes, so they share one cache line.
#[derive(Debug, Default)]
pub struct Adjacent {
    pub a: AtomicU64,
    pub b: AtomicU64,
}

/// The same two counters, each on its own 128-byte line.
#[derive(Debug, Default)]
pub struct Padded {
    pub a: CachePadded<AtomicU64>,
    pub b: CachePadded<AtomicU64>,
}

/// Two threads each increment their OWN counter `iters` times; returns the
/// wall-clock time. The threads share no data, yet if `a` and `b` sit on the
/// same cache line every increment steals the line from the other core
/// (false sharing), and the adjacent layout runs several times slower.
pub fn hammer(a: &AtomicU64, b: &AtomicU64, iters: u64) -> Duration {
    let start = Instant::now();
    thread::scope(|s| {
        for counter in [a, b] {
            spawn_scoped(s, "hammer", move || {
                for _ in 0..iters {
                    counter.fetch_add(1, Relaxed);
                }
            });
        }
    });
    start.elapsed()
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    // Real threads under Miri are slow (every access is interpreted, and the
    // weak-memory emulation keeps a store buffer per location), so the
    // guaranteed-outcome tests use fewer rounds there. Miri is still the
    // stronger check: it produces the forbidden-under-SeqCst outcomes for
    // the weaker orderings in a large share of rounds (see the module docs),
    // so an ordering mistake in these harnesses would show up.
    const SB_ROUNDS: usize = if cfg!(miri) { 100 } else { 2_000 };
    const IRIW_ROUNDS: usize = if cfg!(miri) { 50 } else { 500 };
    const PETERSON_ITERS: usize = if cfg!(miri) { 50 } else { 1_000 };

    #[test]
    fn seqcst_store_buffering_never_misses_both() {
        assert_eq!(store_buffering(SeqCst, SeqCst, SB_ROUNDS), 0);
    }

    #[test]
    fn fenced_store_buffering_never_misses_both() {
        assert_eq!(store_buffering_fenced(SB_ROUNDS), 0);
    }

    #[test]
    fn seqcst_iriw_readers_always_agree() {
        assert_eq!(iriw(SeqCst, SeqCst, IRIW_ROUNDS), 0);
    }

    #[test]
    fn seqcst_peterson_loses_no_update() {
        let count = peterson_count(Flavor::SeqCst, PETERSON_ITERS);
        assert_eq!(count, 2 * PETERSON_ITERS);
    }

    #[test]
    fn fenced_peterson_loses_no_update() {
        let count = peterson_count(Flavor::Fenced, PETERSON_ITERS);
        assert_eq!(count, 2 * PETERSON_ITERS);
    }

    #[test]
    #[should_panic(expected = "Acquire is not a store ordering")]
    fn iriw_rejects_an_acquire_store_before_spawning() {
        // Without the up-front check this hangs: the writers panic inside
        // their threads, and the readers wait for them at round 2 forever.
        iriw(Acquire, Acquire, 2);
    }

    #[test]
    #[should_panic(expected = "Release is not a load ordering")]
    fn store_buffering_rejects_a_release_load() {
        store_buffering(Release, Release, 1);
    }

    #[test]
    #[should_panic(expected = "two threads, 0 and 1")]
    fn peterson_rejects_a_third_thread() {
        Peterson::new(Flavor::SeqCst).lock(2);
    }

    #[test]
    fn cache_padded_layout_facts() {
        // Two unpadded atomics pack into 16 bytes: one cache line.
        assert_eq!(size_of::<(AtomicU64, AtomicU64)>(), 16);
        assert_eq!(size_of::<Adjacent>(), 16);
        // Padded: each value starts a fresh 128-byte block.
        assert_eq!(align_of::<CachePadded<AtomicU64>>(), 128);
        assert_eq!(size_of::<CachePadded<AtomicU64>>(), 128);
        assert_eq!(size_of::<[CachePadded<AtomicU64>; 2]>(), 256);
        assert_eq!(size_of::<Padded>(), 256);
        let padded = Padded::default();
        let a = &padded.a as *const CachePadded<AtomicU64>;
        let b = &padded.b as *const CachePadded<AtomicU64>;
        assert_eq!(a.addr() % 128, 0);
        assert_eq!(b.addr().abs_diff(a.addr()), 128);
    }

    #[test]
    fn hammer_counts_every_increment() {
        let iters = if cfg!(miri) { 50 } else { 10_000 };
        let adjacent = Adjacent::default();
        hammer(&adjacent.a, &adjacent.b, iters);
        assert_eq!(adjacent.a.load(Relaxed), iters);
        assert_eq!(adjacent.b.load(Relaxed), iters);
        let padded = Padded::default();
        hammer(&padded.a, &padded.b, iters);
        assert_eq!(padded.a.load(Relaxed), iters);
        assert_eq!(padded.b.load(Relaxed), iters);
    }

    /// Hardware stress run: how often does each ordering let both SB threads
    /// miss? Prints counts, asserts nothing. Run it with (or under `cargo
    /// +nightly miri test` to see Miri's weak-memory emulation instead):
    ///
    /// ```text
    /// cargo test --release --manifest-path deep-dive/Cargo.toml --lib \
    ///     ordering_lab::tests::sb_hardware_stress -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "hardware stress run; prints counts, run by hand"]
    fn sb_hardware_stress() {
        let iters = if cfg!(miri) { 300 } else { 10_000_000 };
        for (name, store, load) in [
            ("Relaxed", Relaxed, Relaxed),
            ("Release/Acquire", Release, Acquire),
            ("SeqCst", SeqCst, SeqCst),
        ] {
            let both = store_buffering(store, load, iters);
            println!("SB {name:>15}: both missed in {both:>8} of {iters} rounds");
        }
        let both = store_buffering_fenced(iters);
        println!(
            "SB {:>15}: both missed in {both:>8} of {iters} rounds",
            "fence(SeqCst)"
        );
    }

    /// Hardware stress run for IRIW. Prints counts, asserts nothing. Under
    /// Miri it uses 2,000 rounds per ordering, enough to show the Acquire
    /// outcome (about 0.5% of rounds). Run it with:
    ///
    /// ```text
    /// cargo test --release --manifest-path deep-dive/Cargo.toml --lib \
    ///     ordering_lab::tests::iriw_hardware_stress -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "hardware stress run; prints counts, run by hand"]
    fn iriw_hardware_stress() {
        let iters = if cfg!(miri) { 2_000 } else { 10_000_000 };
        for (name, store, load) in [
            ("Relaxed", Relaxed, Relaxed),
            ("Release/Acquire", Release, Acquire),
            ("SeqCst", SeqCst, SeqCst),
        ] {
            let disagree = iriw(store, load, iters);
            println!("IRIW {name:>15}: readers disagreed in {disagree:>8} of {iters} rounds");
        }
    }

    /// Hardware stress run for the broken Peterson lock: prints lost updates.
    ///
    /// ```text
    /// cargo test --release --manifest-path deep-dive/Cargo.toml --lib \
    ///     ordering_lab::tests::peterson_hardware_stress \
    ///     -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "hardware stress run; prints counts, run by hand"]
    fn peterson_hardware_stress() {
        let iters = if cfg!(miri) { 300 } else { 10_000_000 };
        for flavor in [Flavor::ReleaseAcquire, Flavor::SeqCst, Flavor::Fenced] {
            let lost = 2 * iters - peterson_count(flavor, iters);
            // A derived `Debug` ignores width, so pad a formatted `String`.
            let name = format!("{flavor:?}");
            println!(
                "Peterson {name:>14}: {lost:>8} lost updates of {}",
                2 * iters
            );
        }
    }

    /// False-sharing benchmark: adjacent counters vs `CachePadded` ones.
    ///
    /// ```text
    /// cargo test --release --manifest-path deep-dive/Cargo.toml --lib \
    ///     ordering_lab::tests::false_sharing_benchmark \
    ///     -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "timing benchmark; prints durations, run by hand in --release"]
    fn false_sharing_benchmark() {
        // Timings under Miri mean nothing; it just runs a token amount.
        let iters = if cfg!(miri) { 100 } else { 20_000_000 };
        let adjacent = Adjacent::default();
        let padded = Padded::default();
        let slow = hammer(&adjacent.a, &adjacent.b, iters);
        let fast = hammer(&padded.a, &padded.b, iters);
        println!("adjacent counters (one line): {slow:?}");
        println!("CachePadded counters:         {fast:?}");
        println!("ratio: {:.1}x", slow.as_secs_f64() / fast.as_secs_f64());
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use super::{Flavor, Peterson};
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicUsize, fence};
    use loom::thread;
    use std::sync::atomic::Ordering::{self, Acquire, Relaxed, Release, SeqCst};

    /// One SB execution per interleaving: thread A stores x and loads y,
    /// thread B stores y and loads x, optionally with `fence(SeqCst)` between.
    fn store_buffering(store: Ordering, load: Ordering, fenced: bool) {
        loom::model(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let (x2, y2) = (x.clone(), y.clone());
            let b = thread::spawn(move || {
                y2.store(1, store);
                if fenced {
                    fence(SeqCst);
                }
                x2.load(load)
            });
            x.store(1, store);
            if fenced {
                fence(SeqCst);
            }
            let r1 = y.load(load);
            let r2 = b.join().unwrap();
            assert!(r1 == 1 || r2 == 1, "store buffering: both threads read 0");
        });
    }

    #[test]
    #[should_panic(expected = "both threads read 0")]
    fn sb_release_acquire_lets_both_threads_read_zero() {
        store_buffering(Release, Acquire, false);
    }

    #[test]
    fn sb_fence_seqcst_forbids_both_zero() {
        store_buffering(Relaxed, Relaxed, true);
    }

    /// A loom FALSE ALARM, kept as a demonstration of the tool's limit: the
    /// language forbids this outcome when every access is SeqCst, but loom
    /// 0.7 models SeqCst accesses as AcqRel ("Unsupported features" in its
    /// README), so it reports the Release/Acquire result. Only
    /// `fence(SeqCst)` is modeled exactly, which is why the verified
    /// variants in this file use fences.
    #[test]
    #[should_panic(expected = "both threads read 0")]
    fn sb_seqcst_accesses_are_a_loom_false_alarm() {
        store_buffering(SeqCst, SeqCst, false);
    }

    /// Both threads take the lock once and bump a counter with a separate
    /// load and store; thread 1 then takes it a second time, so loom also
    /// checks the hand-over from one critical section to the next.
    fn peterson(flavor: Flavor) {
        loom::model(move || {
            let lock = Arc::new(Peterson::new(flavor));
            let counter = Arc::new(AtomicUsize::new(0));
            let bump = |lock: &Peterson, counter: &AtomicUsize, me: usize| {
                lock.lock(me);
                let seen = counter.load(Relaxed);
                counter.store(seen + 1, Relaxed);
                lock.unlock(me);
            };
            let (lock1, counter1) = (lock.clone(), counter.clone());
            let other = thread::spawn(move || {
                bump(&lock1, &counter1, 1);
                bump(&lock1, &counter1, 1);
            });
            bump(&lock, &counter, 0);
            other.join().unwrap();
            assert_eq!(counter.load(Relaxed), 3, "Peterson {flavor:?}: lost update");
        });
    }

    #[test]
    #[should_panic(expected = "lost update")]
    fn peterson_release_acquire_loses_an_update() {
        peterson(Flavor::ReleaseAcquire);
    }

    #[test]
    fn peterson_fenced_excludes() {
        peterson(Flavor::Fenced);
    }

    /// IRIW under loom: two writers, two readers reading in opposite orders.
    /// Four threads make the full state space large: unbounded, loom took
    /// about 2 minutes to find the Acquire counterexample and 8 minutes to
    /// verify the fenced model (debug build, on a busy machine; both run by
    /// hand, the fenced one passed). So these models bound the preemptions
    /// loom explores to 2, which makes the passing model a bounded check in
    /// CI; the counterexample already shows up with a bound of 1.
    /// `LOOM_MAX_PREEMPTIONS` overrides the bound (at most 255, and a large
    /// bound explores more slowly than none); to search unbounded, delete
    /// the `if` that sets it.
    fn iriw(load: Ordering, fenced: bool) {
        let mut builder = loom::model::Builder::new();
        if builder.preemption_bound.is_none() {
            builder.preemption_bound = Some(2);
        }
        builder.check(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let reader = move |first: Arc<AtomicUsize>, second: Arc<AtomicUsize>| {
                let saw_first = first.load(load);
                if fenced {
                    fence(SeqCst);
                }
                let saw_second = second.load(load);
                saw_first == 1 && saw_second == 0
            };
            let x1 = x.clone();
            let w1 = thread::spawn(move || x1.store(1, Release));
            let y1 = y.clone();
            let w2 = thread::spawn(move || y1.store(1, Release));
            let (x2, y2) = (x.clone(), y.clone());
            let c = thread::spawn(move || reader(x2, y2));
            let d = reader(y, x);
            let c = c.join().unwrap();
            w1.join().unwrap();
            w2.join().unwrap();
            assert!(
                !(c && d),
                "IRIW: the readers disagree on the order of the writes"
            );
        });
    }

    #[test]
    #[should_panic(expected = "readers disagree")]
    fn iriw_acquire_readers_can_disagree() {
        iriw(Acquire, false);
    }

    #[test]
    fn iriw_fence_seqcst_readers_agree() {
        iriw(Acquire, true);
    }
}
