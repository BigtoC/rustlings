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

## Lab index

| File                      | Course module      | What it reveals                                                                                                                                                                                                                                         |
| ------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/unsafe_list.rs`      | M2 data structures | A **doubly** linked list built on `NonNull` raw pointers (impossible in the safe version), just like the standard library's `std::collections::LinkedList`. Compare with the safe singly linked list in `27_data_structures/linkedlist1`.               |
| `src/raw_waker.rs`        | M3 async           | The real memory layout of a `Waker`: one data pointer plus a `RawWakerVTable` (the four function pointers `clone`/`wake`/`wake_by_ref`/`drop`). Compare with the safe version in `29_async_runtime/runtime1` to see what the `Wake` trait does for you. |
| `src/self_referential.rs` | M3 async           | Why a self-referential struct cannot be moved, and exactly what `Pin` / `!Unpin` / `PhantomPinned` are protecting. This is precisely the shape of the state machine that an `async` block generates.                                                    |

## Safety notes

Every `unsafe` site carries a `// SAFETY:` comment explaining the invariant it
relies on. Reading those comments is itself an exercise: try asking "what would
happen if this invariant were broken?" — for example, replacing the
`Pin<Box<_>>` in `self_referential` with a bare value and moving it, or
deliberately mismatching the `clone`/`drop` reference counts in `raw_waker`.
