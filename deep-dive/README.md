# Deep-dive labs · advanced `unsafe` lab

This is the part of the advanced course that **requires `unsafe`**. The rustlings
exercise directory is locked down with `unsafe_code = "forbid"` (a deliberate
teaching constraint), so these labs live in a separate crate. They are meant as
"read + tinker + run the tests" exercises rather than rustlings-style challenges.

## How to run

```bash
cargo test --manifest-path deep-dive/Cargo.toml
```

Or `cd` into the directory and run `cargo test`. Every module ships with unit
tests that serve both as correctness checks and as usage examples.

Several labs point at extra tooling that makes the danger visible:

```bash
# Check the labs for undefined behavior by running every non-ignored test and
# doctest under Miri. CI (the `deep-dive-miri` job) runs this with strict
# provenance under both aliasing models:
MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test --manifest-path deep-dive/Cargo.toml
MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" cargo +nightly miri test --manifest-path deep-dive/Cargo.toml

# The UB zoo (ub_zoo): its #[ignore]d ub_* tests commit UB on purpose, so the
# two lines above skip them. Run one alone under Miri, or check every one
# against its expected diagnostic under both aliasing models (the last step of
# the deep-dive-miri job runs the script):
cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib -- --ignored --exact ub_zoo::tests::ub_use_after_free
bash deep-dive/scripts/check_ub_zoo.sh

# The FFI lab's UB demonstrations are #[ignore]d too; run each one by name
# under Miri (natively they return early instead of executing UB):
cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib -- --ignored --exact ffi_lab::tests::dangling_as_ptr_is_a_use_after_free

# Stable rustdoc ignores the error code of a compile_fail doctest; nightly
# checks it (so the Miri runs above, which use nightly rustdoc, enforce them):
cargo +nightly test --manifest-path deep-dive/Cargo.toml --doc

# Model-check atomics code across every thread interleaving with loom (the
# loom_lab module, the loom_tests modules of ordering_lab and treiber, and the
# loom dependency are compiled only under this cfg). Run them the way CI does:
RUSTFLAGS="--cfg loom" cargo test --manifest-path deep-dive/Cargo.toml --lib -- loom_lab:: ordering_lab::loom_tests:: treiber::loom_tests::

# Hardware stress runs (store buffering, IRIW, the broken Peterson lock) and
# the false-sharing benchmark print counts or timings and never fail; run
# them in release:
cargo test --release --manifest-path deep-dive/Cargo.toml --lib ordering_lab -- --ignored --nocapture

# The eager-free Treiber stack's use-after-free: Miri reports the UB and the
# run fails, on purpose:
cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib treiber::tests::eager_free_stale_commit_is_a_use_after_free -- --ignored

# alloc_count: a counting #[global_allocator] in its own test binary
# (harness = false). Every checked count holds in debug, release and Miri:
cargo test --manifest-path deep-dive/Cargo.toml --test alloc_count
cargo test --manifest-path deep-dive/Cargo.toml --release --test alloc_count

# perf_lab: the timing run (release only). Bounds-check elimination must be
# confirmed in the asm, not assumed: the "Run it" section of src/perf_lab.rs
# has the loop that counts panic_bounds_check per kernel.
cargo test --manifest-path deep-dive/Cargo.toml --release --lib perf_lab -- --ignored --nocapture

# api_surface: the integration tests plus the property tests (proptest and
# the props module are compiled only under this cfg):
RUSTFLAGS="--cfg proptest" cargo test --manifest-path deep-dive/Cargo.toml --test api_surface
```

Two more lab crates sit next to this one. They depend on crates.io crates, so
each is its own workspace with its own CI job:

- [`backend-lab/`](../backend-lab/) — tokio, axum and tower: `select!` cancel
  safety, graceful shutdown, 503 backpressure, `spawn_blocking`, bounded
  fan-out.
- [`config-lab/`](../config-lab/) — serde at the boundary (validated newtypes,
  `deny_unknown_fields`) and additive Cargo features with a `build.rs`.

## Lab index

| File                                          | Course module            | What it reveals                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
|-----------------------------------------------|--------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `src/unsafe_list.rs`                          | M2 data structures       | A **doubly** linked list built on `NonNull` raw pointers (impossible in the safe version), just like the standard library's `std::collections::LinkedList`. Compare with the safe singly linked list in `27_data_structures/linkedlist1`.                                                                                                                                                                                                                                                                                                                                                                        |
| `src/raw_vec.rs`                              | M2 data structures       | `Vec` from scratch: `Layout::array` + `alloc`/`realloc` growth, `ptr::write`/`ptr::read` to move elements, `Deref` to a slice, and a `Drop` that drops only the initialized elements then frees exactly once. Run it under Miri.                                                                                                                                                                                                                                                                                                                                                                                 |
| `src/raw_waker.rs`                            | M3 async                 | The real memory layout of a `Waker`: one data pointer plus a `RawWakerVTable` (the four function pointers `clone`/`wake`/`wake_by_ref`/`drop`). Compare with the safe version in `29_async_runtime/runtime1` to see what the `Wake` trait does for you.                                                                                                                                                                                                                                                                                                                                                          |
| `src/self_referential.rs`                     | M3 async                 | Why a self-referential struct cannot be moved, and exactly what `Pin` / `!Unpin` / `PhantomPinned` are protecting. This is precisely the shape of the state machine that an `async` block generates.                                                                                                                                                                                                                                                                                                                                                                                                             |
| `src/vtable_lab.rs`                           | Traits & dispatch        | A `&dyn Trait` fat pointer built by hand: a data pointer paired with a `&'static` table of function pointers. Dispatch is one pointer load plus one indirect call. Compare with `32_dispatch`. Uses `PhantomData` to keep the borrow honest.                                                                                                                                                                                                                                                                                                                                                                     |
| `src/myarc.rs`                                | Concurrency              | `Arc` from scratch over an `AtomicUsize` strong count: why `clone` can be `Relaxed` but the final `drop` needs `Release` + an `Acquire` fence before it frees. Compare with `30_send_sync` / `36_atomics`.                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/loom_lab.rs`                             | Concurrency              | Proof (under `loom`, compiled only with `--cfg loom`) that the `36_atomics` handoff genuinely needs `Release`/`Acquire`: the `Relaxed` version can observe a stale payload across some interleaving.                                                                                                                                                                                                                                                                                                                                                                                                             |
| `src/ordering_lab.rs`                         | Concurrency              | When `SeqCst` is needed: store buffering, IRIW and Peterson's lock, where no load reads the store it must be ordered after. Guaranteed outcomes asserted on real threads, weak ones shown by loom, Miri and hardware runs. Plus false sharing.                                                                                                                                                                                                                                                                                                                                                                   |
| `src/treiber.rs`                              | Concurrency              | A lock-free Treiber stack on `AtomicPtr`: a split pop makes the eager-free use-after-free deterministic (Miri rejects it); deferred reclamation fixes it and rules out pointer ABA. Loom-checks the stack and `36_atomics/atomics3`'s spinlock.                                                                                                                                                                                                                                                                                                                                                                  |
| `src/ffi_lab.rs`                              | M1 memory / Concurrency  | Edition-2024 FFI: `unsafe extern` and when a foreign function may honestly be a `safe fn` (`hypot` yes; `abs`/`div` no, since they are UB for `INT_MIN` or a zero divisor, so they get checked wrappers), a `repr(C)` `div_t` returned by value, `qsort` with a Rust comparator, `CString`/`CStr` ownership, a closure passed to C through a generic trampoline that catches its panic (the `extern "C"` abort is checked in a child process), and a C handle that is `Send` but not `Sync`, shared through `Mutex`. Compare with `30_send_sync`, `49_panics`, `56_async_bounds`. Run it under Miri.             |
| `src/ub_zoo.rs`                               | M1 memory model          | Undefined behavior, case by case: `#[ignore]`d `ub_*` tests that mostly pass natively, each with a sound `fixed_*` twin, and each checked against the diagnostic Miri reports: aliasing under Stacked vs Tree Borrows, invalid values, use-after-free, misalignment, a data race (vs a race condition), library vs language UB (including a case Miri cannot see), and a leak (reported, not UB). A deliberately covariant `BadCell` lets safe code read freed memory; making it invariant turns the exploit into E0597. Compare with `38_variance`, `40_interior_mutability`, `41_memory_layout`, `36_atomics`. |
| `src/api_surface.rs` + `tests/api_surface.rs` | Traits & Abstraction     | Testing a library from outside, as a dependent crate sees it: `compile_fail` doctests, each paired with a positive control, for a typestate builder, `#[must_use]`, a sealed trait, `#[non_exhaustive]`, variance and `Send`/`Sync`; integration tests with helpers in `tests/common/mod.rs`; proptest round trips that shrink two planted bugs to minimal counterexamples (`--cfg proptest`); a semver table checked with cargo-semver-checks. Compare with `47_type_level`, `38_variance`, `quizzes/quiz5` and `50_testing_seams`.                                                                             |
| `tests/alloc_count.rs`                        | M5 interview & debugging | A counting `GlobalAlloc` in its own `harness = false` test binary: predict, then count, the allocations of `Rc<str>` vs `Rc<String>`, a `Vec<String>` clone, `collect`, `format!` vs `write!` into a reused buffer, and prove that a warm hot path allocates nothing (`assert_no_alloc`). Checked counts hold in debug, release and under Miri; the rows the optimizer decides are shown, not checked. Compare with `65_performance/perf1`.                                                                                                                                                                      |
| `src/perf_lab.rs`                             | M5 interview & debugging | Bounds checks read from the asm, not assumed: an index loop (LLVM hoists its check), `zip`, a hoisted `assert_eq!`, and a byte-keyed lookup whose per-key check survives until the table becomes `&[u32; 256]`. Plus a `black_box` micro-benchmark harness, a naive timing loop whose result depends on codegen units, and an allocation that release builds delete. Compare with `65_performance/perf3`.                                                                                                                                                                                                        |

## Safety notes

Every `unsafe` site carries a `// SAFETY:` comment explaining the invariant it
relies on. Reading those comments is itself an exercise: try asking "what would
happen if this invariant were broken?" — for example, replacing the
`Pin<Box<_>>` in `self_referential` with a bare value and moving it, or
deliberately mismatching the `clone`/`drop` reference counts in `raw_waker`.
