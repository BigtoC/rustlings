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

Two of the labs point at extra tooling that makes the danger visible:

```bash
# Check the raw-pointer labs (raw_vec, myarc, unsafe_list) for undefined
# behaviour by running their tests under Miri:
cargo +nightly miri test --manifest-path deep-dive/Cargo.toml

# Model-check the atomics handoff across every thread interleaving with loom
# (the loom_lab module + its dependency are compiled only under this cfg):
RUSTFLAGS="--cfg loom" cargo test --manifest-path deep-dive/Cargo.toml loom_lab
```

## Lab index

| File                      | Course module      | What it reveals                                                                                                                                                                                                                                         |
|---------------------------|--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `src/unsafe_list.rs`      | M2 data structures | A **doubly** linked list built on `NonNull` raw pointers (impossible in the safe version), just like the standard library's `std::collections::LinkedList`. Compare with the safe singly linked list in `27_data_structures/linkedlist1`.               |
| `src/raw_vec.rs`          | M2 data structures | `Vec` from scratch: `Layout::array` + `alloc`/`realloc` growth, `ptr::write`/`ptr::read` to move elements, `Deref` to a slice, and a `Drop` that drops only the initialized elements then frees exactly once. Run it under Miri.                        |
| `src/raw_waker.rs`        | M3 async           | The real memory layout of a `Waker`: one data pointer plus a `RawWakerVTable` (the four function pointers `clone`/`wake`/`wake_by_ref`/`drop`). Compare with the safe version in `29_async_runtime/runtime1` to see what the `Wake` trait does for you. |
| `src/self_referential.rs` | M3 async           | Why a self-referential struct cannot be moved, and exactly what `Pin` / `!Unpin` / `PhantomPinned` are protecting. This is precisely the shape of the state machine that an `async` block generates.                                                    |
| `src/vtable_lab.rs`       | Traits & dispatch  | A `&dyn Trait` fat pointer built by hand: a data pointer paired with a `&'static` table of function pointers. Dispatch is one pointer load plus one indirect call. Compare with `32_dispatch`. Uses `PhantomData` to keep the borrow honest.            |
| `src/myarc.rs`            | Concurrency        | `Arc` from scratch over an `AtomicUsize` strong count: why `clone` can be `Relaxed` but the final `drop` needs `Release` + an `Acquire` fence before it frees. Compare with `30_send_sync` / `36_atomics`.                                              |
| `src/loom_lab.rs`         | Concurrency        | Proof (under `loom`, compiled only with `--cfg loom`) that the `36_atomics` handoff genuinely needs `Release`/`Acquire`: the `Relaxed` version can observe a stale payload across some interleaving.                                                    |

## Safety notes

Every `unsafe` site carries a `// SAFETY:` comment explaining the invariant it
relies on. Reading those comments is itself an exercise: try asking "what would
happen if this invariant were broken?" — for example, replacing the
`Pin<Box<_>>` in `self_referential` with a bare value and moving it, or
deliberately mismatching the `clone`/`drop` reference counts in `raw_waker`.
