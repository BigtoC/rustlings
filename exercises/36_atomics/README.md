# Atomics: Atomic Operations and Memory Ordering

> A deep-dive corner of the concurrency track. These exercises use only **std**
> (`std::sync::atomic`, `Arc`, `std::thread`), **100% safe** code on **stable**
> Rust, edition 2024. The focus is the *why* behind atomics: when a lock-free
> operation is race-free, and what memory ordering actually buys you.

## Core Ideas

A shared `usize` mutated with `+= 1` from several threads is a data race, because
`+= 1` is really three steps (load, add, store) that can interleave and drop
updates. Atomics give you race-free shared state without a mutex:

- **Atomic read-modify-write** (`fetch_add`, `compare_exchange`, ...) performs the
  whole load-add-store as ONE indivisible hardware operation. No update is lost,
  and no lock is taken.
- **Memory ordering** is a *separate* question from atomicity. It controls whether
  the writes a thread did *around* an atomic op become visible to other threads,
  and in what order.
  - **`Relaxed`** guarantees only that the atomic value itself is coherent. It
    publishes no other memory. Perfect for a standalone counter.
  - **`Release` (on a store) paired with `Acquire` (on a load) of the SAME
    atomic** creates a *happens-before* edge: every write the producer did before
    the Release store — even `Relaxed` ones — is visible to any thread that later
    observes the flag with an Acquire load. This is how you *publish* data across
    threads.
- **CAS = compare-and-swap.** `compare_exchange(current, new, success, failure)`
  atomically writes `new` only if the value still equals `current`. It is the
  primitive from which mutexes are built.

## The Spin Loop

When a thread must busy-wait on an atomic, its loop body must call
`std::hint::spin_loop()` rather than spinning on nothing. It hints the CPU that
this is a busy-wait (improving power use and hyperthread yield) and keeps the loop
idiomatic — an empty `loop {}` trips clippy's `empty_loop` / `infinite_loop`
lints.

## Exercise Path

1. **atomics1** — A lock-free counter with `AtomicUsize`. The per-thread loop body
   is empty, so the counter stays `0` and the test fails; call
   `c.fetch_add(1, Ordering::Relaxed)` to make every increment count. `Relaxed` is
   enough because the counter publishes no other memory.
2. **atomics2** — `Release`/`Acquire` publishes data across threads. The consumer
   function is missing its return value, so it will not compile. Busy-wait until
   an **Acquire** load of the flag reads `true`, then return the data — the
   producer's Release store guarantees you see the value it wrote, never the
   initial `0`.
3. **atomics3** — Build a `SpinLock` from `AtomicBool` + CAS. `try_lock` is empty
   and must return a `bool`, so it will not compile. Use `compare_exchange` to
   flip the flag `false -> true` (Acquire on success, Relaxed on failure) and
   return `.is_ok()`; `unlock` stores `false` with `Release` so the next holder
   sees the critical section. Real spinlocks loop on `compare_exchange_weak`,
   which may fail *spuriously*.

## Further Reading

- [`std::sync::atomic`](https://doc.rust-lang.org/std/sync/atomic/index.html)
- [`enum Ordering`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html)
- [`AtomicUsize::fetch_add`](https://doc.rust-lang.org/std/sync/atomic/struct.AtomicUsize.html#method.fetch_add)
- [`AtomicBool::compare_exchange`](https://doc.rust-lang.org/std/sync/atomic/struct.AtomicBool.html#method.compare_exchange)
- [`std::hint::spin_loop`](https://doc.rust-lang.org/std/hint/fn.spin_loop.html)
