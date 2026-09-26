# Module 4 · Lock Hazards: Deadlocks by Lock Order and RwLock Semantics

> The concurrency track so far made shared state *safe*: `Arc<Mutex<T>>` in
> `30_send_sync`, borrowing across threads in `51_scoped_threads`, waiting on
> a condition in `52_condvar`. This module is about what that safety does not
> cover: threads that wait for each other forever, and a reader-writer lock
> whose rules are subtler than they look. All **std**, **100% safe**,
> **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the failing test
> and the requirements, but **not** the fix. Press `h` when you want the full
> answer. A failing test here usually takes about 10 seconds: that is a
> watchdog giving up on a thread that will never return, not a hang.

## Core Ideas

- **Safe Rust prevents data races, not deadlocks.** `Send`, `Sync` and the
  borrow checker guarantee that no two threads access the same memory at once
  with one of them writing. A deadlock misuses no memory: every thread just
  waits for a lock another one holds. The Rustonomicon calls it "safe" for
  Rust to get deadlocked. The compiler checks one function at a time, and
  lock order is a property of the whole program.
- **A deadlock needs all four Coffman conditions**: mutual exclusion, hold and
  wait, no preemption, circular wait. Remove one and it cannot happen. The
  standard fix removes the cycle: one **global lock order** (account number,
  unique id, address of a lock that never moves) that *every* code path
  follows when it holds two locks at once.
- **`std::sync::Mutex` is not reentrant.** Locking it again on the thread
  that holds it "will not return" (it might panic or deadlock). A transfer
  from an account to itself is enough to get there.
  `31_debugging/debugging8` meets the same self-deadlock through a guard that
  lives longer than it looks.
- **Releasing a lock early is not a fix.** Dropping `from` before locking
  `to` avoids the deadlock and breaks atomicity: for a moment the money is in
  neither account, and a concurrent `total()` sees it missing.
- **`RwLock<T>`: many readers or one writer.** It is the thread-safe,
  blocking version of `RefCell`'s rule: while any reader is inside, a writer
  waits. It pays off when reads are long and frequent and writes are rare.
  Every `read()` still does an atomic read-modify-write on the lock, so for
  short critical sections a `Mutex` is often as fast: measure.
- **There is no upgradable read in std.** Two readers that both wanted to
  upgrade would each wait for the other to leave. A get-or-insert therefore
  drops the read guard, takes `write()` from scratch, and **looks again**,
  because another thread may have inserted in between (double-checked
  locking). A `write()` call while your own read guard is alive deadlocks
  (std's docs allow a panic instead). The other direction exists:
  `RwLockWriteGuard::downgrade` (Rust 1.92) turns a write guard into a read
  guard atomically.
- **`RwLock<T>: Sync` needs `T: Send + Sync`**, while `Mutex<T>: Sync` needs
  only `T: Send`: readers on several threads hold `&T` at the same time. So
  `Mutex<Cell<i32>>` can be shared between threads and `RwLock<Cell<i32>>`
  cannot (E0277).
- **Reader/writer priority is up to the platform.** std's docs only promise
  that a waiting writer "might or might not block concurrent calls to
  `read`". Today std's own lock on Linux, Windows and macOS makes new readers
  wait behind a queued writer, so a thread that takes `read()` twice
  deadlocks if a writer queues up in between: never lock the same `RwLock`
  twice on one thread, not even for reading.

## Deadlock Fixes Compared

| Fix                                           | Removes       | Cost                                                     |
| --------------------------------------------- | ------------- | -------------------------------------------------------- |
| One global lock order (by id or address)      | circular wait | every path that holds two locks must follow it           |
| One lock for everything                       | hold and wait | no parallelism: unrelated operations wait for each other |
| `lock` + `try_lock`, back off and retry       | hold and wait | burns CPU under contention, can livelock                 |
| Release the first lock before taking the next | hold and wait | the operation is no longer atomic                        |

## `Mutex` vs `RwLock`

|                            | `Mutex<T>`                            | `RwLock<T>`                                                                                                 |
| -------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Who can be inside          | one thread                            | many readers, or one writer                                                                                 |
| `Sync` when                | `T: Send`                             | `T: Send + Sync`                                                                                            |
| Poisoned by                | a panic while the lock is held        | a panic while the **write** lock is held                                                                    |
| Same thread locks it again | never returns (may panic or deadlock) | `write()` under your own guard deadlocks (or panics); a second `read()` may deadlock behind a queued writer |
| Upgrade / downgrade        | n/a                                   | no upgrade in std; `RwLockWriteGuard::downgrade` since 1.92                                                 |
| Good fit                   | short critical sections, many writes  | long reads, rare writes                                                                                     |

std's reentrant lock, `ReentrantLock` (what `Stdout` uses inside), is still
unstable. `parking_lot::RwLock` adds `upgradable_read` (only one upgradable
guard at a time, next to plain readers) and `read_recursive`.

## How the Tests Catch a Deadlock

A deadlocked test would hang forever: rustlings runs `cargo test` without a
time limit. So every call that might deadlock runs on a helper thread, and the
test waits for its result with `recv_timeout`. A stuck helper is left behind;
it dies when the test binary exits. The stress tests use a progress watchdog
instead of a fixed deadline: they fail only if **no** operation finished for
10 seconds, however slow the machine is.

The opposite-order deadlock depends on timing, which is how lock-order bugs
survive testing in real code. So `deadlock1` also probes the order directly:
the test holds one account itself and watches with `try_lock` what a transfer
does with the other. `rwlock1` needs no luck either: four readers wait on a
`Barrier` *inside* `with_read`, which a `Mutex` can never let happen. To
explore every interleaving instead of sampling a few, see the `loom` lab
(`deep-dive/src/loom_lab.rs`).

## Exercise Path

1. **deadlock1** — `Bank::transfer` locks `from`, then `to`, so two transfers
   in opposite directions deadlock, and a transfer from an account to itself
   locks one `Mutex` twice. Reject `from == to` first, then lock both accounts
   in the order `total` uses, whichever way the money goes, and keep the check,
   the debit and the credit under both locks. Probes reject a descending
   order, a `try_lock` back-off, an early release and a bank-wide lock.
2. **rwlock1** — A `ConfigStore` behind a `Mutex` lets one reader in at a
   time, and the four readers of the test wait for each other forever. Switch
   to an `RwLock` (readers keep the read lock while `f` runs, so `set` waits
   for them), then make `get_or_insert_with` take only the read lock on a
   hit; on a miss, take the write lock and look again before inserting, so
   that `make` runs at most once per key while four threads race for it.

Next, `54_channels` shares data by sending messages: an actor thread that
owns its data needs no lock at all. (Channels can deadlock too, for example
two threads that each wait to receive from the other.)
`deep-dive/ROADMAP.md` lists two possible extensions of this module under
"Additional topics": lock striping (`shard1`, a `ShardedMap` of
`RwLock<HashMap>` shards) and hot-swapped configuration (`hotswap1`,
`RwLock<Arc<Config>>` that readers clone out).

## Further Reading

- [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html) (what "will not return" means for a second `lock`) and [`std::sync::RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html) (the priority policy and the "potential deadlock example")
- [`RwLockWriteGuard::downgrade`](https://doc.rust-lang.org/std/sync/struct.RwLockWriteGuard.html#method.downgrade)
- [The Book, ch. 16.3: Shared-State Concurrency](https://doc.rust-lang.org/book/ch16-03-shared-state.html) ("`Mutex<T>` comes with the risk of creating deadlocks")
- [The Rustonomicon: Data Races and Race Conditions](https://doc.rust-lang.org/nomicon/races.html)
- [Rust Atomics and Locks, ch. 1: Basics of Rust Concurrency](https://mara.nl/atomics/basics.html) (sections "Lock Poisoning" and "Reader-Writer Lock") and [ch. 9: Building Our Own Locks](https://mara.nl/atomics/building-locks.html) ("Avoiding Writer Starvation")
- [Deadlock and the Coffman conditions](https://en.wikipedia.org/wiki/Deadlock_(computer_science)) and the [dining philosophers problem](https://en.wikipedia.org/wiki/Dining_philosophers_problem) (the resource hierarchy solution is lock ordering)
- [`parking_lot::RwLock`](https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html) (upgradable and recursive reads) and [`parking_lot::deadlock`](https://docs.rs/parking_lot/latest/parking_lot/deadlock/index.html) (run-time deadlock detection)
- [Runtime locking correctness validator (Linux lockdep)](https://docs.kernel.org/locking/lockdep-design.html)
