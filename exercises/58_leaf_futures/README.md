# Module 3 · Leaf Futures: External Wakeups, Timers and Cooperative Yielding

> `28_futures` and `29_async_runtime` built the executor half of async:
> `Future`, `Poll`, `Waker` and a hand-written runtime. Every leaf future there
> woke itself. This module writes the leaves that real code awaits: a channel
> woken by its sender, a sleep woken by a timer thread, and a yield that lets
> the other tasks run. All **std**, **100% safe**, **stable** Rust, edition
> 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the failing test and
> the requirements, but **not** the fix. Read what the test says first. Press
> `h` when you want the full answer.

## Core Ideas

A **leaf future** is one that is not built out of other futures, so it is
where `Pending` comes from in the first place. `async` blocks and combinators
only pass a `Pending` up from the leaves they await.

- **`Pending` is a promise.** A leaf may return `Pending` only after it has
  arranged for the task to be woken. Either it hands `cx.waker()` to whoever
  will cause the event (the other half of a channel, a timer, an I/O driver),
  or it wakes the task itself because it only wants to be polled again later.
  A `Pending` with no wake-up arranged is a task that never runs again.
- **The latest waker wins.** The `Future::poll` docs: "on multiple calls to
  `poll`, only the `Waker` from the `Context` passed to the most recent call
  should be scheduled to receive a wakeup." The future may have moved to
  another task, or a combinator may give each child its own waker. Every poll
  that returns `Pending` replaces the stored waker. `Waker::will_wake` (and
  `Waker::clone_from`, which uses it) skips the clone when nothing changed.
- **Check and register atomically.** "Is the value there?" and "store my
  waker" must happen under the same lock (or the same atomic protocol) that
  the waking side uses. Otherwise the event can happen in between: the waker
  finds nobody to wake, and the task sleeps forever. That is a **lost
  wakeup**. Waking itself is done *after* releasing the lock.
- **Never block in `poll`.** `poll` runs on the executor's thread. A
  `std::thread::sleep`, a blocking `recv` or a long CPU loop inside it stops
  every other task on that thread. Blocking belongs on a thread of its own: a
  timer thread, or `spawn_blocking`.
- **Tasks are cooperative.** The executor gets its thread back only when a
  task's `poll` returns, and that happens only when an awaited leaf returns
  `Pending`. `.await` on a future that is already `Ready` does not yield.

## Who Wakes the Task?

| Leaf                   | Returns `Pending` until                  | Who calls `wake()`                     | Exercise   |
| ---------------------- | ---------------------------------------- | -------------------------------------- | ---------- |
| oneshot `Receiver`     | a value arrives or the sender is dropped | `Sender::send`, or the sender's `Drop` | `oneshot1` |
| `Sleep`                | the timer has fired                      | the timer thread                       | `timer1`   |
| `YieldNow`             | it has been polled once                  | itself, before returning `Pending`     | `yield1`   |
| a socket read in tokio | the OS reports the socket readable       | the I/O driver (epoll, kqueue, IOCP)   | none here  |
| `Mutex::lock` in tokio | the lock is released                     | the task that unlocks it               | none here  |

## Blocking, Yielding and Offloading

| You have                                    | Do this                                                                                    |
| ------------------------------------------- | ------------------------------------------------------------------------------------------ |
| a pause                                     | `tokio::time::sleep(d).await`, never `std::thread::sleep`                                  |
| a blocking call (file, DNS, a sync client)  | `tokio::task::spawn_blocking`, then `.await` its `JoinHandle`                              |
| heavy CPU work                              | `spawn_blocking` for one-offs, a pool like rayon for a lot of it                           |
| a long loop that must stay on the runtime   | `tokio::task::yield_now().await` once per batch of iterations                              |
| blocking code you cannot move out of a task | `tokio::task::block_in_place` (multi-threaded runtime only; it panics on `current_thread`) |

tokio also has an automatic **coop budget**: a task gets 128 operations per
poll, after which tokio's channels, sockets and timers return `Pending` on
purpose. A pure CPU loop never touches them, which is why it still needs an
explicit yield. `yield_now` gives no hard guarantee either: the runtime may
poll the same task again, and a `select!` whose other branch completes in the
same poll swallows the yield.

## Exercise Path

1. **oneshot1** — A oneshot channel over `Arc<Mutex<Shared>>` whose
   `Receiver::poll` returns `Pending` without storing a waker, so the second
   poll's waker B is woken 0 times after `send` and the `block_on` tests report
   a lost wake-up. Store or replace the waker of every `Pending` poll under the
   same lock as the value check, and answer `Err(Canceled)` once the sender is
   gone (a value sent before the drop still wins).
2. **timer1** — `Sleep::poll` calls `thread::sleep` and returns `Ready`, so a
   sleeping task blocks the executor: the event order is `[a:start, a:done,
   b:ran]` instead of `[a:start, b:ran, a:done]`, and overlapping sleeps end in
   spawn order. Register once with the given timer thread, return `Pending`,
   keep the latest waker in the timer entry, and finish when the entry has
   fired. A zero duration is `Ready` on the first poll.
3. **yield1** — `async fn yield_now() {}` awaits nothing, so it never yields
   and two tasks run back to back (`[a0, a1, a2, b0, b1, b2]`). Write a
   `YieldNow` leaf that wakes its own task and returns `Pending` exactly once
   (you met it as `YieldOnce` in `28_futures/futures2`). Then make a CPU-bound
   checksum loop yield once per batch of `every` items, so that a heartbeat
   task keeps ticking while it runs.

## Related Modules and Labs

- `29_async_runtime/runtime2` and `runtime3` are the `block_on` and the
  single-threaded executor that the tests here inline, with checks added so
  that a lost wakeup fails the test at once instead of hanging it.
- `57_async_combinators` (`join1`, `select1`, `cancel1`) polls several leaves
  from one future. A `Sleep` that loses a `select` is dropped: tokio removes
  its timer entry, while the timer in `timer1` simply fires into the void.
- `deep-dive/src/raw_waker.rs` builds a `Waker` by hand from a
  `RawWakerVTable`, the `unsafe` layer under `std::task::Wake`.
- The `blocking_in_async` part of the `backend-lab/` crate repeats
  `yield1`'s starvation on a real tokio `current_thread` runtime and fixes it
  with `spawn_blocking`.

## Further Reading

- [`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll): the "most recent call" rule for wakers
- [`std::task::Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html): `wake`, `wake_by_ref`, `will_wake`, `clone_from`
- [`std::task::Wake`](https://doc.rust-lang.org/std/task/trait.Wake.html): building a `Waker` from an `Arc` in safe code
- [Asynchronous Programming in Rust: Task Wakeups with `Waker`](https://rust-lang.github.io/async-book/02_execution/03_wakeups.html): the async book's timer future, one thread per timer
- [Alice Ryhl: Async: What is blocking?](https://ryhl.io/blog/async-what-is-blocking/)
- [Reducing tail latencies with automatic cooperative task yielding](https://tokio.rs/blog/2020-04-preemption) (the tokio coop budget)
- [`tokio::task::yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html), [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) and [`tokio::task::coop`](https://docs.rs/tokio/latest/tokio/task/coop/index.html)
- [`tokio::time::sleep`](https://docs.rs/tokio/latest/tokio/time/fn.sleep.html) and [the tokio timer wheel](https://tokio.rs/blog/2018-03-timers)
- [`tokio::sync::oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html) and [`futures::channel::oneshot`](https://docs.rs/futures/latest/futures/channel/oneshot/index.html)
- [`futures::task::AtomicWaker`](https://docs.rs/futures/latest/futures/task/struct.AtomicWaker.html): the lock-free "store the latest waker" primitive real leaves use
