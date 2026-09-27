# Module 4 · Condition Variables: Blocking Queues and a Semaphore

> The waiting half of the concurrency track. `30_send_sync` put shared state
> behind `Arc<Mutex<T>>`; this module makes a thread **sleep until that state
> changes**, without spinning, with `std::sync::Condvar`. You build the three
> classic live-coding answers: a blocking queue, a bounded queue, and a
> semaphore. All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements, but **not** the fix, as in an interview.
> Press `h` when you want the full answer.

## Core Ideas

- **The condition lives in the mutex, not in the condvar.** A condvar is only
  a place to sleep until another thread says "the state changed, look again".
  It has no memory: a notify sent while nobody sleeps simply disappears. That
  is harmless, because a waiter checks the state (under the lock) before it
  goes to sleep.
- **`wait(guard)` unlocks and sleeps in one atomic step**, and locks again
  before it returns. A notify sent after you checked cannot slip in before you
  are asleep. It takes the guard **by value** because it has to unlock the
  mutex, so a reference into the data that you tried to keep across the call
  is E0505 "cannot move out of `guard` because it is borrowed".
- **Always wait in a loop.** Returning from `wait` means "look again", never
  "your condition holds". Wakeups can be spurious (the docs say `wait` "is
  susceptible to spurious wakeups"), stolen (another thread got the lock
  first and took the item), or one of many (`notify_all`). `wait_while` is the
  same loop, packaged.
- **Change the state under the lock, then notify.** Notifying while still
  holding the lock and notifying just after unlocking are both correct with
  std. Notifying *before* the state change, outside the lock, can lose the
  wakeup: the woken thread looks too early and goes back to sleep.
- **One condvar per condition.** A bounded queue has two conditions ("not
  empty" for consumers, "not full" for producers), so it has two condvars, and
  every change wakes one thread that can use it. Use `notify_all` when one
  change can let several waiters proceed, or when waiters for different
  conditions share one condvar. One condvar belongs to one mutex: using it
  with a second one "may result in a runtime panic".
- **Whatever must be given back is a guard.** A permit, like a lock, is
  returned in `Drop`, so an early `return`, a `?` or a panic cannot leak it.

## The Protocol

```rust
// The waiting side: check under the lock, sleep while the answer is "not yet".
let mut state = mutex.lock().unwrap();
while !ready(&state) {
    state = cv.wait(state).unwrap();
}
// The lock is held again here, and `ready(&state)` is true.

// The changing side: change the state under the lock, then notify.
let mut state = mutex.lock().unwrap();
change(&mut state);
drop(state); // optional: unlock first, so the woken thread can run at once
cv.notify_one(); // or `notify_all()`
```

A bounded wait (`wait_timeout`) needs the same loop, and it has to recompute
the time left on every pass. `wait_timeout_while` does both for you.

## Wakeup Bugs

| Bug                                                                                       | What goes wrong                                                                                 | How the tests notice                                                      |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `if` instead of `while`                                                                   | a wakeup with the condition still false goes ahead: pops an empty queue, overfills, over-admits | a bare `notify_all()` fakes a spurious wakeup                             |
| lost wakeup: a sleeper nobody counted, a state change outside the lock                    | a thread sleeps forever while its condition holds                                               | the watchdog: "gave up after 20s waiting until ..."                       |
| a pop that notifies only if the queue was full (a drop only if the count was 0)           | two slots or permits come back in quick succession, but only one waiter is woken                | two slots (or permits) are handed back in quick succession to two waiters |
| a notify before the state change, or a check and a wait in two separate critical sections | the wakeup falls into the gap and is lost                                                       | a race of a few microseconds that the tests rarely hit: follow the rule   |
| `notify_one` on one condvar shared by producers and consumers                             | it wakes the wrong kind of thread, and eventually every thread sleeps                           | the capacity-1 stress test in `condvar2`                                  |
| sleeping or spinning while still holding the lock                                         | nobody else can ever change the state: a deadlock                                               | state is read with `try_lock`, so the test fails, not hangs               |
| busy-waiting or `sleep`-polling instead of `wait`                                         | it works, but burns CPU and adds latency                                                        | not detectable without flaky timing; the TODOs forbid it                  |

## How These Tests Stay Deterministic

- **No test needs luck to pass.** A thread that must return is joined with a
  20-second watchdog that only a broken solution ever reaches (rustlings runs
  tests without a timeout, so a hang would hang forever).
- **"Must stay blocked" is watched for 250 ms.** A correct solution never
  finishes in that window. A wrong one is caught as long as its thread gets any
  CPU time during it. Under heavy load a wrong solution can slip through, but
  a correct one can never fail.
- **Two things come back at once.** One test pops twice in a row, another
  drops two permits back to back, each with two threads waiting. A solution
  that skips a notify is caught whenever the second pop or drop runs before
  the first woken thread has used its wakeup, which is nearly always. A
  correct solution passes however the threads are scheduled.
- **Spurious wakeups are faked.** A real one cannot be summoned on demand, so
  the tests call `notify_all()` without changing anything. From inside `wait`
  the two look exactly the same.
- **`thread::spawn` + `Arc`, not `thread::scope`.** A scope joins every thread
  before it returns (see `51_scoped_threads`), so one thread stuck in a broken
  `pop` would hang the test instead of failing it.

## Exercise Path

1. **condvar1** — `BlockingQueue::pop` has an empty body (E0308). Implement
   it: while the queue is empty, wait on `not_empty` in a loop (or
   `wait_while`), and count yourself in `waiters` while you sleep, because
   `push` notifies only when someone is counted. A test fakes a spurious
   wakeup with a bare `notify_all`, which an `if` does not survive, and a
   4-producer, 4-consumer stress test checks that every item is delivered
   once, in order.
2. **condvar2** — A `BoundedQueue` whose `try_push` never refuses and whose
   `push` ignores the capacity: the tests see `Ok(())` instead of `Err(3)`, and
   "push did not block when the queue was full". Make `try_push` hand the item
   back when full, make `push` sleep on `not_full` in a loop, and make every
   `pop` wake one producer on `not_full`, the condvar only producers sleep on.
3. **condvar3** — A `Semaphore` whose `acquire` hands out a permit even when
   none is free, and whose `Permit` has no `Drop`, so a permit never comes
   back. Wait in `acquire` while the count is 0, and implement `Drop` for
   `Permit` to return the permit and wake one waiter, on every drop. That also
   returns the permit of a holder that panics. Eight threads on three permits
   must never exceed three at once.

## Related Modules

- `30_send_sync/send_sync2` — `Arc<Mutex<T>>`, the lock every condvar pairs
  with.
- `29_async_runtime/runtime2` — `block_on` sleeps with `thread::park`, which
  may also return spuriously, so its loop polls again after every wakeup.
- `39_drop_raii` — `Drop` guards; `condvar3`'s `Permit` is one more.
- `51_scoped_threads` — `thread::scope`, and why these tests do not use it.
- `53_lock_hazards` (`deadlock1`, `rwlock1`) — deadlocks from lock order, and
  `RwLock`.
- `54_channels` (`channel1..3`) — std's own bounded queue: `sync_channel`,
  `try_send` backpressure, and shutdown when every sender is gone (a condvar
  queue needs an explicit "closed" flag in its state for that).
- `36_atomics/atomics4` — a lock-free bounded counter (`try_inc`): the
  non-blocking half of a semaphore with no lock at all, but no way to wait.
- `31_debugging/debugging8` — a `MutexGuard` kept alive by a `match`
  scrutinee deadlocks its own thread, caught with the same `try_lock` trick
  these tests use.
- `deep-dive/src/loom_lab.rs` — `loom` explores every interleaving of a small
  concurrent test. It also ships a mock `Condvar`, so these queues could be
  model-checked the same way.
- Lab crate `backend-lab/` (`backpressure_http`) — backpressure at the HTTP
  edge: a handler `try_send`s into a bounded tokio channel and answers 503
  when it is full.

## Further Reading

- [`std::sync::Condvar`](https://doc.rust-lang.org/std/sync/struct.Condvar.html), including [`wait_while`](https://doc.rust-lang.org/std/sync/struct.Condvar.html#method.wait_while) and [`wait_timeout_while`](https://doc.rust-lang.org/std/sync/struct.Condvar.html#method.wait_timeout_while)
- [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html), including poisoning
- [`std::sync::mpsc::sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) and [`std::thread::park`](https://doc.rust-lang.org/std/thread/fn.park.html)
- [Shared-State Concurrency (The Book)](https://doc.rust-lang.org/book/ch16-03-shared-state.html)
- [Rust Atomics and Locks, ch. 1: Condition Variables](https://mara.nl/atomics/basics.html#condvar) and [ch. 9: building a condition variable](https://mara.nl/atomics/building-locks.html#condition-variable), by Mara Bos (its "Avoiding Syscalls" part counts waiters, as `condvar1` does)
- [POSIX `pthread_cond_wait`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/pthread_cond_wait.html): "Spurious wakeups ... may occur"
- [`parking_lot::Condvar`](https://docs.rs/parking_lot/latest/parking_lot/struct.Condvar.html), which promises no spurious wakeups (the loop is still needed for stolen ones)
- [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html): async, fair (FIFO), with owned permits
- [`loom::sync::Condvar`](https://docs.rs/loom/latest/loom/sync/struct.Condvar.html)
