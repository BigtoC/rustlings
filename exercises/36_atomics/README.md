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
- **A CAS retry loop** makes any "read, decide, write" behave as one atomic
  step: read the value, compute the new one (or give up), CAS it in, and on
  `Err(actual)` decide again from `actual`. "Load, check, then store" (or
  "check, then `fetch_add`") is check-then-act: it loses updates or
  overshoots the limit.
- **A CAS compares bits, not history.** If the value went A → B → A while you
  were paused, your CAS still succeeds. That is the **ABA problem**, and it
  bites when the atomic is an index or a pointer into a linked structure.

## The Spin Loop

When a thread must busy-wait on an atomic, its loop body must call
`std::hint::spin_loop()` rather than spinning on nothing. It hints the CPU that
this is a busy-wait (improving power use and hyperthread yield) and keeps the loop
idiomatic — an empty `loop {}` trips clippy's `empty_loop` / `infinite_loop`
lints.

## CAS Loops and the ABA Problem

`compare_exchange_weak` may fail **spuriously**: it can return `Err(actual)`
with `actual == current`. On x86-64 the weak and strong versions compile to
the same `lock cmpxchg`. On load-linked / store-conditional CPUs (ARMv7,
ARMv8 without LSE, RISC-V, POWER) the store half can fail after an interrupt
or a write to the same cache line, and the strong `compare_exchange` hides
that behind its own retry loop. Inside your own retry loop a spurious failure
costs one more iteration, so use the weak one there, and the strong one for a
single attempt whose failure means something (`try_lock`, a commit step).
`try_update` (stable since Rust 1.95) packages the loop around a closure that
returns `Some(new)` or `None`. std calls it in `Weak::upgrade` to bump a
strong count that must never go from 0 back to 1. Its older name,
`fetch_update`, is marked deprecated from Rust 1.99 onward.

A lock-free stack pops by reading `head` (A) and the node below it (B), then
CAS-ing `head` from A to B. If another thread pops A, pops B and pushes A back
in the meantime, the CAS still sees A and installs B, which is no longer
free. The classic fix is a **version tag**: pack `(tag, index)` into one
`AtomicU64` and write a new tag on every successful CAS, so a stale snapshot
no longer matches. Tags wrap (32 bits: after 2^32 updates), and pointers have
little room for them. For heap nodes on `AtomicPtr` the deeper problem is
**memory reclamation**: T1 may read a node that T2 has already freed. Real
code uses hazard pointers or epoch-based reclamation (`crossbeam-epoch`).
The `unsafe` Treiber-stack follow-up is planned as a deep-dive lab
(`lock-free-ordering-lab` in `deep-dive/ROADMAP.md`).

The tests in parts 4 and 5 never rely on a real race showing up. They force
the bad interleaving on a single thread through a test seam: a
`race_window` closure in part 4, and a pop split into `pop_begin` /
`pop_commit` in part 5. `loom` (see `deep-dive/src/loom_lab.rs`) goes further
and explores every interleaving.

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
4. **atomics4** — A `BoundedCounter` whose `try_inc(max)` hands out tickets
   `0..max`, then `Err(Full)`. The starter loads, checks and stores, so a
   rival increment forced into the `race_window` is lost and two callers get
   ticket 0: the race tests fail. Rewrite it as a `compare_exchange_weak`
   retry loop that re-checks the limit against the value a failed CAS hands
   back (or use `try_update`), and keeps retrying while slots are left: a
   lost race is not `Full`. Check the limit before computing `current + 1`,
   so `u64::MAX` cannot overflow. An 8-thread stress test checks that every
   ticket is handed out exactly once.
5. **atomics5** — The ABA problem in a lock-free free list of arena slots.
   `head` packs `(tag, idx)`, but `next_head` always writes tag 0. A scripted
   A-B-A (pop 0, pop 1, push 0 between `pop_begin` and `pop_commit`) makes the
   stale commit succeed and puts slot 1, still in use, back on the free list.
   Make every successful CAS write the next tag, wrapping past `u32::MAX`, so
   the stale commit returns `Err(Stale)`.

## Further Reading

- [`std::sync::atomic`](https://doc.rust-lang.org/std/sync/atomic/index.html)
- [`enum Ordering`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html)
- [`AtomicUsize` methods such as `fetch_add`](https://doc.rust-lang.org/std/sync/atomic/struct.Atomic.html#impl-Atomic%3Cusize%3E) (`AtomicUsize` is now documented as an alias of `Atomic<usize>`)
- [`AtomicBool` methods such as `compare_exchange`](https://doc.rust-lang.org/std/sync/atomic/struct.Atomic.html#impl-Atomic%3Cbool%3E)
- [`std::hint::spin_loop`](https://doc.rust-lang.org/std/hint/fn.spin_loop.html)
- [`AtomicU64` methods such as `compare_exchange_weak` and `try_update`](https://doc.rust-lang.org/std/sync/atomic/struct.Atomic.html#impl-Atomic%3Cu64%3E)
- [Rust Atomics and Locks, ch. 2: Atomics](https://marabos.nl/atomics/atomics.html) (compare-and-exchange loops)
- [The ABA problem](https://en.wikipedia.org/wiki/ABA_problem) and the [Treiber stack](https://en.wikipedia.org/wiki/Treiber_stack)
- [`crossbeam-epoch`](https://docs.rs/crossbeam-epoch) — epoch-based memory reclamation for lock-free structures
