//! # Deep-dive labs (advanced unsafe exercises)
//!
//! These labs are the part of the advanced course that **requires `unsafe`**,
//! so they live outside rustlings' `unsafe_code = "forbid"` constraint, as a
//! "read + tinker" lab rather than a rustlings-style challenge. A few need
//! what a single-file exercise cannot have instead: doctests and integration
//! tests seen from another crate (`api_surface`), a custom global allocator
//! (`tests/alloc_count.rs`) or a release build (`perf_lab`). Every module
//! ships with tests. Run:
//!
//! ```text
//! cargo test --manifest-path deep-dive/Cargo.toml
//! ```
//!
//! | Module | Course module | What it reveals |
//! | --- | --- | --- |
//! | [`unsafe_list`] | M2 data structures | A doubly linked list built on `NonNull` raw pointers, just like the standard library's `LinkedList` |
//! | [`raw_vec`] | M2 data structures | `Vec` from scratch: `Layout`/`alloc`/`realloc`, `ptr::write`/`read`, and a `Drop` that frees exactly once |
//! | [`raw_waker`] | M3 async | The real memory representation of a `Waker`: `RawWaker` + a four-function-pointer vtable |
//! | [`self_referential`] | M3 async | Why a self-referential struct cannot be moved, and what `Pin` is really protecting |
//! | [`vtable_lab`] | Traits & dispatch | A `&dyn Trait` fat pointer, by hand: a data pointer plus a static table of function pointers |
//! | [`myarc`] | Concurrency | `Arc` from scratch: an atomic strong count, and why `clone` can be `Relaxed` but `drop` needs `Release` + an `Acquire` fence |
//! | [`ordering_lab`] | Concurrency | When `SeqCst` is needed: store buffering, IRIW and Peterson's lock checked on real threads, under loom and Miri; `CachePadded` against false sharing |
//! | [`treiber`] | Concurrency | A Treiber stack on `AtomicPtr`: why eager freeing is a use-after-free (Miri) and deferred reclamation is not; `atomics3`'s spinlock under loom |
//! | [`ffi_lab`] | M1 memory / Concurrency | FFI in edition 2024: `unsafe extern` and honest `safe fn`, `repr(C)`, `CString` ownership, a closure through a C callback, a `Send`-but-not-`Sync` C handle |
//! | [`ub_zoo`] | M1 memory model | Undefined behavior case by case under Miri: aliasing, invalid values, use-after-free, data races, library UB, and an unsound covariant cell |
//! | [`api_surface`] | Traits & Abstraction | Testing a library from outside: `compile_fail` doctests with positive controls, sealed traits and `#[non_exhaustive]` as another crate sees them, proptest shrinking, a semver table |
//! | [`perf_lab`] | M5 interview & debugging | Bounds checks found in the asm (`sum_indexed` vs `zip` vs a hoisted assert) and `black_box` micro-benchmarks; its partner `tests/alloc_count.rs` counts allocations with a `GlobalAlloc` |
//!
//! The `loom_lab` module and the `loom_tests` modules of `ordering_lab` and
//! `treiber` are compiled only under `--cfg loom` (they model-check atomics
//! code across every thread interleaving). Run them with:
//!
//! ```text
//! RUSTFLAGS="--cfg loom" cargo test --manifest-path deep-dive/Cargo.toml --lib -- loom_lab:: ordering_lab::loom_tests:: treiber::loom_tests::
//! ```
//!
//! The property tests of `api_surface` (in `tests/api_surface.rs`) are
//! compiled only under `--cfg proptest`, which also pulls in the `proptest`
//! dev-dependency:
//!
//! ```text
//! RUSTFLAGS="--cfg proptest" cargo test --manifest-path deep-dive/Cargo.toml --test api_surface
//! ```
//!
//! To check the labs for undefined behavior under Miri (this runs every
//! non-ignored test and doctest; `ub_zoo`'s `#[ignore]`d `ub_*` cases are
//! checked one by one by `scripts/check_ub_zoo.sh`):
//!
//! ```text
//! cargo +nightly miri test --manifest-path deep-dive/Cargo.toml
//! bash deep-dive/scripts/check_ub_zoo.sh
//! ```

pub mod api_surface;
pub mod ffi_lab;
pub mod myarc;
pub mod ordering_lab;
pub mod perf_lab;
pub mod raw_vec;
pub mod raw_waker;
pub mod self_referential;
pub mod treiber;
pub mod ub_zoo;
pub mod unsafe_list;
pub mod vtable_lab;

#[cfg(loom)]
pub mod loom_lab;
