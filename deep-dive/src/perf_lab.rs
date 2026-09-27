//! Lab · bounds checks in the asm, `black_box`, trustworthy micro-benchmarks
//!
//! "Is this index loop slower than the iterator version?" has two honest
//! answers: *read the assembly*, and *measure it properly*. This lab does both
//! for the same small kernels, so you can see what the optimizer really did
//! before you trust a number.
//!
//! Every `a[i]` is a bounds check: compare `i` with `a.len()`, and call
//! `core::panicking::panic_bounds_check` if it is out of range. The check is
//! what makes indexing safe. What it costs depends on whether LLVM can move it
//! out of the loop or prove that it never fails (bounds-check elimination).
//! The kernels:
//!
//! - [`sum_indexed`], [`sum_zip`] and [`sum_hoisted`] compute the same
//!   wrapping dot product of two `u64` slices three ways: an index loop, `zip`,
//!   and an index loop behind `assert_eq!(a.len(), b.len())`. They agree on
//!   equal lengths and differ on unequal ones; the tests pin each contract.
//! - [`lookup_sum_indexed`] and [`lookup_sum_hoisted`] add up `table[key]` for
//!   byte keys. The index is data, not a loop counter, so no length assert up
//!   front can cover it. Converting the table to `&[u32; 256]` once does,
//!   because a `u8` key can never reach 256.
//!
//! Compare with `65_performance/perf3`: its fixed `dot` asserts the length
//! contract before `zip`, and its notes say the same assert lets LLVM drop
//! the bounds checks of an index loop, to be confirmed in the assembly. This
//! lab is that confirmation, with a twist in the first row of the table
//! below. The allocation half of the performance question (perf1, perf2, and
//! "how do you prove zero allocations?") is the counting allocator in
//! `deep-dive/tests/alloc_count.rs`.
//!
//! # What the asm shows
//!
//! Verified in a release build with rustc 1.96.0 on aarch64-apple-darwin, and
//! cross-checked by emitting x86_64-unknown-linux-gnu asm for the same code
//! with the same compiler. Another target, compiler version or edit can change
//! the picture, which is the point: check, don't assume.
//!
//! | Function | `panic_bounds_check` | Where the check runs |
//! | --- | --- | --- |
//! | `sum_indexed` | 1 call | once, *before* the loop; the loop body has none |
//! | `sum_zip` | none | nowhere: `zip` stops at the shorter slice |
//! | `sum_hoisted` | none | only the `assert_eq!` (a call to `assert_failed`) |
//! | `lookup_sum_indexed` | 1 call | on *every* key, inside the loop |
//! | `lookup_sum_hoisted` | none | only the length test in `try_into` |
//!
//! The first row surprises people. `sum_indexed` is the textbook "index loop
//! with a bounds check per iteration", yet LLVM moved the check out of the
//! loop: it compares `b.len()` with `a.len()` up front and panics right away
//! if `b` is too short. That is allowed because the loop has no side effects:
//! nobody can see the partial sum, and the panic message is the same one the
//! loop would have produced at index `b.len()`. The loop that follows has no
//! branch to the panic, so it is unrolled (and on x86_64 vectorized with SSE2)
//! just like `sum_hoisted`. The check that survives in `lookup_sum_indexed` is
//! the expensive kind: a compare and a branch on every key, and a loop LLVM
//! does not unroll. Both functions contain exactly one call to
//! `panic_bounds_check`, so counting calls is not enough; look at where the
//! branch to it sits.
//!
//! # A micro-benchmark you can trust
//!
//! [`bench`](fn@bench) is a minimal harness, and each part of it answers one
//! way a benchmark lies:
//!
//! - **Release builds only.** A debug build skips inlining and keeps overflow
//!   checks, so iterator code looks far slower than it is. The timing test
//!   warns when it runs in a debug build.
//! - **`black_box` the inputs and the result.** [`std::hint::black_box`] is an
//!   identity function that hints to the compiler to assume the worst about
//!   what it does with its argument. On the inputs, it stops LLVM from
//!   specializing on data it can see (folding a sum of known values, or
//!   hoisting a pure call out of the timing loop). On the result, it stops
//!   LLVM from deleting a call whose value nobody uses. Its docs call it
//!   best-effort: good enough for benchmarks, never a tool for correctness. It
//!   protects only the values it sees: work that the code under test does and
//!   throws away can still be deleted inside that code. [`boxed_round_trip`]
//!   allocates in the source, yet the release build of this crate does not
//!   allocate in it at all (the counting allocator in `tests/alloc_count.rs`
//!   sees 1 allocation in a debug build, 0 in release). A benchmark measures
//!   that optimized code, which is fine as long as you know it.
//! - **Warm up first.** The first calls pay for cold caches, page faults and a
//!   CPU that is still raising its clock. [`bench`](fn@bench) runs one untimed
//!   batch.
//! - **Batch, repeat, keep the minimum.** One call is too short for the clock,
//!   so time a batch of `iters` calls and divide. Repeat the batch `samples`
//!   times. Noise (interrupts, other processes) only ever adds time, so the
//!   fastest batch is the most repeatable estimate of the code's own cost.
//! - **Compare variants in one run, on the same data.** Absolute numbers move
//!   between machines and between runs; the ratios are what you report.
//!
//! criterion adds what this harness lacks: statistics, outlier detection and
//! comparison with a saved baseline. It is an external crate, so this std-only
//! lab leaves it out; in your own crate it goes in `[dev-dependencies]` with a
//! `[[bench]]` target that sets `harness = false`.
//!
//! # Sharpest question
//!
//! *How would you prove that a hot path makes zero allocations, and what makes
//! a Rust micro-benchmark trustworthy?* For allocations: install a counting
//! `#[global_allocator]` in a test binary, warm the hot path up, then run it
//! inside a counting window and assert that the count is 0
//! (`tests/alloc_count.rs` does exactly that). For the benchmark: a release
//! build, `black_box` on the inputs and the result, a warm-up, repeated
//! batches summarized by a robust statistic, and the variants compared in the
//! same run. Then read the asm to confirm what the optimizer did (bounds
//! checks gone or hoisted, loop unrolled or vectorized), because a benchmark
//! measures whatever machine code the compiler emitted, not what the source
//! seems to say.
//!
//! # Invariants
//!
//! - On equal-length inputs the three `sum_*` functions return the same value,
//!   and on a 256-entry table the two `lookup_sum_*` functions do.
//! - Arithmetic wraps, so the only panics are the ones under study: the bounds
//!   checks and the length contracts. (With plain `+` and `*`, a debug build
//!   would panic on overflow where a release build wraps.)
//! - The kernels are `#[inline(never)]`: each one is a symbol of its own in
//!   the asm, and the benchmark calls that function instead of inlining it
//!   into the timing loop.
//!
//! # Run it
//!
//! From `deep-dive/`:
//!
//! ```text
//! # The tests (they double as the contract of each variant):
//! cargo test --lib perf_lab
//! # The timing run (release only; prints ns per call for each variant):
//! cargo test --release --lib perf_lab -- --ignored --nocapture
//! # The asm: emit it, then print each kernel's size in lines and its calls
//! # to panic_bounds_check.
//! cargo rustc --release --lib -- --emit asm
//! S=$(ls -t target/release/deps/deep_dive-*.s | head -n 1)
//! asm() { awk "/perf_lab${#1}${1}(17h[0-9a-f]+E)?:\$/,/\.cfi_endproc/" "$S"; }
//! for f in sum_indexed sum_zip sum_hoisted \
//!          lookup_sum_indexed lookup_sum_hoisted; do
//!   printf '%-20s %4s lines %3s panic_bounds_check\n' "$f" \
//!     "$(asm "$f" | grep -c .)" "$(asm "$f" | grep -c panic_bounds_check)"
//! done
//! ```
//!
//! `asm name` prints one function of this module, from its label to its
//! `.cfi_endproc`: run `asm sum_indexed` to read it. Mangled symbols spell
//! out each name's length (`8perf_lab11sum_indexed`), which is why the
//! pattern uses `${#1}`: it stops `sum_indexed` from also matching
//! `lookup_sum_indexed`. The optional `17h<hash>E` suffix covers the legacy
//! mangling that stable rustc uses for your crate today; the pattern also
//! matches the newer v0 scheme, which has no such suffix. A row that reads 0
//! lines means the symbol was not found (renamed, inlined away or merged into
//! another function), not that it has no check. Compiler Explorer
//! (godbolt.org) shows the same asm without a build.
//!
//! `--emit asm` changes the build slightly: unless `-C codegen-units` is set
//! explicitly, rustc then compiles the crate as a single codegen unit, where
//! a release build normally uses up to 16 (that is why there is one `.s`
//! file). For these self-contained kernels it made no difference here: in the
//! release test binary, disassembled with `objdump -d`, they matched the `.s`
//! file instruction for instruction, apart from one equivalent compare
//! (`len > 3` emitted as `len >= 4`). Code that crosses functions can change,
//! as Try it, 4, shows.
//!
//! # Try it
//!
//! 1. Run the asm loop above and compare it with the table. Then read
//!    `asm sum_indexed` and `asm lookup_sum_indexed`: find the one
//!    compare-and-branch to the panic *before* the loop in the first, and the
//!    compare-and-branch *inside* the loop in the second.
//! 2. Change `assert_eq!` in `sum_hoisted` to `debug_assert_eq!` and emit the
//!    release asm again: the assert is compiled out and `panic_bounds_check`
//!    comes back (hoisted in front of the loop, as in `sum_indexed`), because
//!    the release build no longer knows that the lengths are equal. The tests
//!    tell the same story: `cargo test --lib perf_lab` still passes, because a
//!    debug build keeps the assert, but `cargo test --release --lib perf_lab`
//!    fails both `sum_hoisted_rejects_*` tests. A `debug_assert!` is not a
//!    contract.
//! 3. In `lookup_sum_hoisted`, delete the `try_into` line so that the loop
//!    indexes the slice `table`: the per-key check returns, and
//!    `lookup_sum_hoisted_rejects_a_short_table_before_any_key` fails. Run the
//!    timing test before and after to see what that check costs on your
//!    machine. Then put the length test back but keep indexing the slice
//!    (`let _checked: &[u32; 256] = table.try_into().expect(..);`): with
//!    rustc 1.96 on aarch64 the per-key check is gone again. What LLVM needs
//!    is the fact `table.len() == 256`; the array type is how the source
//!    states it.
//! 4. The timing run ends with two rows timed by a loop that uses no
//!    `black_box` at all. `zip inlined, no black_box` runs the zip loop
//!    inlined into the timing loop; its result is unused, so LLVM deleted the
//!    work and the row reads 0 ns per call. `sum_zip, no black_box` calls the
//!    `#[inline(never)]` kernel, and what it reads depends on something the
//!    source does not show: codegen units. In the default release build the
//!    timing loop and `sum_zip` landed in different units, the caller saw only
//!    a bare declaration of `sum_zip`, all 2000 calls per batch ran, and the
//!    row matched the `sum_zip` row. Now build it as one codegen unit:
//!
//!    ```text
//!    RUSTFLAGS="-C codegen-units=1" \
//!      cargo test --release --lib perf_lab -- --ignored --nocapture
//!    ```
//!
//!    LLVM now sees that `sum_zip` only reads its arguments and never unwinds
//!    (`memory(argmem: read)` and `nounwind` in `--emit llvm-ir`), turns 2000
//!    identical calls into one per batch, and the row reads 0 ns too. It keeps
//!    that one call only because `sum_zip` is not marked `willreturn`.
//!    (aarch64, rustc 1.96.) `black_box` takes that luck out of it.
//! 5. Write `add_indexed(dst: &mut [u32], src: &[u32])`, with
//!    `dst[i] = dst[i].wrapping_add(src[i])` for each `i < dst.len()`, and read
//!    its asm. Stores cannot be moved past a panic, so LLVM cannot panic early
//!    here. On aarch64 with rustc 1.96 it computes up front how far the loop
//!    can safely go, runs a vectorized body with no check, and keeps a
//!    per-element check only in the scalar tail loop that follows.
//! 6. Read [`boxed_round_trip`] with `asm boxed_round_trip`. The source
//!    creates and frees a `Box`, yet with rustc 1.96 the release asm (on
//!    aarch64, and in the x86_64 cross-check) calls neither `__rust_alloc` nor
//!    `__rust_dealloc`: LLVM deleted both. The one call left,
//!    `__rust_no_alloc_shim_is_unstable_v2`, is a marker that std's `alloc`
//!    function makes before it allocates; it allocates nothing. This is why
//!    `tests/alloc_count.rs` counts 0 for this function in release and 1 in a
//!    debug build.

use std::hint::black_box;
use std::time::{Duration, Instant};

/// Dot product of `a` and `b` over the indices of `a`, wrapping on overflow.
///
/// Panics with "index out of bounds" if `b` is shorter than `a`, and ignores
/// the tail of a longer `b`. This is the textbook "bounds check on every
/// iteration" loop; see the module docs for what LLVM makes of it.
#[inline(never)]
// The index loop is the subject of this function; the `zip` form clippy
// suggests is `sum_zip`.
#[allow(clippy::needless_range_loop)]
pub fn sum_indexed(a: &[u64], b: &[u64]) -> u64 {
    let mut total = 0u64;
    for i in 0..a.len() {
        total = total.wrapping_add(a[i].wrapping_mul(b[i]));
    }
    total
}

/// Dot product over the pairs that `zip` yields, wrapping on overflow.
///
/// Never indexes, so it has no bounds check and never panics. It also never
/// reports a length mismatch: it silently stops at the shorter slice.
#[inline(never)]
pub fn sum_zip(a: &[u64], b: &[u64]) -> u64 {
    a.iter()
        .zip(b)
        .fold(0u64, |total, (x, y)| total.wrapping_add(x.wrapping_mul(*y)))
}

/// Dot product of two slices of the same length, wrapping on overflow.
///
/// Panics with "length mismatch" unless `a.len() == b.len()`, whichever one is
/// longer. The assert states the contract once, and in doing so tells LLVM
/// that every `b[i]` with `i < a.len()` is in bounds.
#[inline(never)]
// Same index loop as `sum_indexed`, now behind the assert.
#[allow(clippy::needless_range_loop)]
pub fn sum_hoisted(a: &[u64], b: &[u64]) -> u64 {
    assert_eq!(a.len(), b.len(), "length mismatch");
    let mut total = 0u64;
    for i in 0..a.len() {
        total = total.wrapping_add(a[i].wrapping_mul(b[i]));
    }
    total
}

/// Sum of `table[key]` over the keys, wrapping on overflow.
///
/// Panics with "index out of bounds" at the first key `>= table.len()`, so a
/// short table works as long as every key fits. The index comes from the data,
/// so the check stays inside the loop.
#[inline(never)]
pub fn lookup_sum_indexed(table: &[u32], keys: &[u8]) -> u32 {
    let mut total = 0u32;
    for &key in keys {
        total = total.wrapping_add(table[usize::from(key)]);
    }
    total
}

/// Sum of `table[key]` over the keys, for a table of exactly 256 entries,
/// wrapping on overflow.
///
/// Panics with "table must have 256 entries" otherwise, before it reads a
/// single key. After that one length test the table is a `&[u32; 256]`, and a
/// `u8` key is always in bounds, so the loop has no check at all.
#[inline(never)]
pub fn lookup_sum_hoisted(table: &[u32], keys: &[u8]) -> u32 {
    let table: &[u32; 256] = table.try_into().expect("table must have 256 entries");
    let mut total = 0u32;
    for &key in keys {
        total = total.wrapping_add(table[usize::from(key)]);
    }
    total
}

/// Puts `x` in a `Box`, reads it back and frees the box: one allocation and
/// one free in the source.
///
/// `GlobalAlloc`'s docs allow an optimized build to delete an allocation that
/// nobody observes, and a release build of this crate deletes this one; see
/// Try it, 6, and `tests/alloc_count.rs`. `#[inline(never)]` keeps the
/// function in this crate's machine code, where you can read it.
#[inline(never)]
pub fn boxed_round_trip(x: u64) -> u64 {
    let boxed = Box::new(x);
    *boxed
}

/// Times `f`: one untimed warm-up batch of `iters` calls, then `samples`
/// batches of `iters` calls each. Returns the fastest batch's time per call
/// (whole nanoseconds: dividing a `Duration` rounds down).
///
/// Every result goes through [`black_box`], so no call can be deleted as
/// unused; hiding the *inputs* from the optimizer is the caller's job (see the
/// timing test). Calls `f` exactly `(samples + 1) * iters` times.
///
/// # Panics
///
/// If `samples` or `iters` is 0.
pub fn bench<R>(samples: u32, iters: u32, mut f: impl FnMut() -> R) -> Duration {
    assert!(
        samples > 0 && iters > 0,
        "bench needs samples > 0 and iters > 0"
    );
    // Warm-up: caches, branch predictors and the CPU clock settle here.
    for _ in 0..iters {
        black_box(f());
    }
    let mut best = Duration::MAX;
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..iters {
            black_box(f());
        }
        best = best.min(start.elapsed());
    }
    best / iters
}

#[cfg(test)]
mod tests {
    use super::*;

    type Sum = fn(&[u64], &[u64]) -> u64;
    const SUMS: [(&str, Sum); 3] = [
        ("sum_indexed", sum_indexed),
        ("sum_zip", sum_zip),
        ("sum_hoisted", sum_hoisted),
    ];

    /// Deterministic pseudo-random numbers (a 64-bit LCG), so a failure
    /// reproduces exactly.
    fn data(len: usize, seed: u64) -> Vec<u64> {
        let mut x = seed;
        (0..len)
            .map(|_| {
                x = x
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                x >> 11
            })
            .collect()
    }

    /// Byte keys that cover the whole `u8` range.
    fn keys(len: usize) -> Vec<u8> {
        data(len, 3).iter().map(|&x| x as u8).collect()
    }

    /// A full 256-entry table: `table[i] == 3 * i + 1`.
    fn table() -> Vec<u32> {
        (0..256).map(|i| 3 * i + 1).collect()
    }

    #[test]
    fn the_three_sums_agree_on_equal_lengths() {
        for (name, sum) in SUMS {
            assert_eq!(sum(&[1, 2, 3], &[4, 5, 6]), 32, "{name}");
            assert_eq!(sum(&[], &[]), 0, "{name}");
        }
        for len in [1, 2, 7, 64, 1_000] {
            let (a, b) = (data(len, 1), data(len, 2));
            let expected = sum_indexed(&a, &b);
            assert_eq!(sum_zip(&a, &b), expected, "sum_zip, len {len}");
            assert_eq!(sum_hoisted(&a, &b), expected, "sum_hoisted, len {len}");
        }
    }

    #[test]
    fn arithmetic_wraps_instead_of_panicking() {
        // u64::MAX * 2 wraps to u64::MAX - 1, and 1 * 1 brings it back to
        // u64::MAX. In a debug build a plain `*` would panic here.
        for (name, sum) in SUMS {
            assert_eq!(sum(&[u64::MAX, 1], &[2, 1]), u64::MAX, "{name}");
        }
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn sum_indexed_panics_when_b_is_shorter() {
        sum_indexed(&[1, 2, 3], &[1, 2]);
    }

    #[test]
    fn sum_indexed_ignores_the_tail_of_a_longer_b() {
        assert_eq!(sum_indexed(&[1, 2], &[3, 4, 100]), 11);
    }

    #[test]
    fn sum_zip_hides_a_length_mismatch() {
        // Either way round, zip returns the sum over the shorter prefix: no
        // panic and no error, just a wrong answer.
        assert_eq!(sum_zip(&[1, 2, 3], &[3, 4]), 11);
        assert_eq!(sum_zip(&[1, 2], &[3, 4, 100]), 11);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn sum_hoisted_rejects_a_shorter_b() {
        sum_hoisted(&[1, 2, 3], &[1, 2]);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn sum_hoisted_rejects_a_longer_b() {
        sum_hoisted(&[1, 2], &[1, 2, 3]);
    }

    #[test]
    fn the_two_lookups_agree_on_a_full_table() {
        let table = table();
        assert_eq!(lookup_sum_indexed(&table, &[0, 255]), 1 + 766);
        assert_eq!(lookup_sum_hoisted(&table, &[0, 255]), 1 + 766);
        assert_eq!(lookup_sum_hoisted(&table, &[]), 0);
        let keys = keys(1_000);
        assert_eq!(
            lookup_sum_indexed(&table, &keys),
            lookup_sum_hoisted(&table, &keys)
        );
    }

    #[test]
    fn lookup_sum_indexed_accepts_a_short_table_while_the_keys_fit() {
        assert_eq!(lookup_sum_indexed(&[10, 20, 30], &[2, 0, 2]), 70);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn lookup_sum_indexed_panics_at_the_first_key_past_the_end() {
        lookup_sum_indexed(&[10, 20, 30], &[2, 3]);
    }

    #[test]
    #[should_panic(expected = "table must have 256 entries")]
    fn lookup_sum_hoisted_rejects_a_short_table_before_any_key() {
        // Every key would fit, but the contract is checked once, up front.
        lookup_sum_hoisted(&[10, 20, 30], &[0]);
    }

    #[test]
    fn boxed_round_trip_returns_its_input() {
        for x in [0, 42, u64::MAX] {
            assert_eq!(boxed_round_trip(x), x);
        }
    }

    #[test]
    fn bench_warms_up_once_then_times_every_sample() {
        let mut calls = 0u32;
        bench(3, 5, || {
            calls += 1;
            calls
        });
        assert_eq!(calls, (3 + 1) * 5);
    }

    #[test]
    #[should_panic(expected = "bench needs samples > 0 and iters > 0")]
    fn bench_rejects_an_empty_run() {
        bench(0, 10, || 0);
    }

    /// A timing loop with no `black_box` anywhere: what a naive benchmark
    /// measures. Used only for the comparison row in `timing`.
    fn naive_bench(samples: u32, iters: u32, mut f: impl FnMut() -> u64) -> Duration {
        let mut best = Duration::MAX;
        for _ in 0..samples {
            let start = Instant::now();
            for _ in 0..iters {
                f();
            }
            best = best.min(start.elapsed());
        }
        best / iters
    }

    /// The timing run. It asserts nothing (timings are not deterministic); it
    /// prints ns per call for each kernel on the same data.
    #[test]
    #[ignore = "timing run, release only: \
                cargo test --release --lib perf_lab -- --ignored --nocapture"]
    fn timing() {
        if cfg!(debug_assertions) {
            println!("warning: debug build, these numbers mean nothing; rerun with --release");
        }
        const LEN: usize = 4_096;
        let (samples, iters) = (7, 2_000);
        let (a, b) = (data(LEN, 1), data(LEN, 2));
        let (table, keys) = (table(), keys(LEN));
        let rows = [
            (
                "sum_indexed",
                bench(samples, iters, || sum_indexed(black_box(&a), black_box(&b))),
            ),
            (
                "sum_zip",
                bench(samples, iters, || sum_zip(black_box(&a), black_box(&b))),
            ),
            (
                "sum_hoisted",
                bench(samples, iters, || sum_hoisted(black_box(&a), black_box(&b))),
            ),
            (
                "lookup_sum_indexed",
                bench(samples, iters, || {
                    lookup_sum_indexed(black_box(&table), black_box(&keys))
                }),
            ),
            (
                "lookup_sum_hoisted",
                bench(samples, iters, || {
                    lookup_sum_hoisted(black_box(&table), black_box(&keys))
                }),
            ),
            // The two rows below use no `black_box` at all (see Try it, 4).
            (
                "sum_zip, no black_box",
                naive_bench(samples, iters, || sum_zip(&a, &b)),
            ),
            (
                "zip inlined, no black_box",
                naive_bench(samples, iters, || {
                    a.iter()
                        .zip(&b)
                        .fold(0u64, |total, (x, y)| total.wrapping_add(x.wrapping_mul(*y)))
                }),
            ),
        ];
        println!("{LEN} elements, fastest of {samples} batches of {iters} calls:");
        for (name, per_call) in rows {
            println!("  {name:<26} {:>7} ns/call", per_call.as_nanos());
        }
    }
}
