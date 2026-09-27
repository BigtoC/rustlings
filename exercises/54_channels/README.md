# Module 4 · Channels: Backpressure, Disconnect-Driven Shutdown and Actors

> `20_threads/threads3` sent values from two producers through one `mpsc`
> channel. This module covers what interviews ask next: what happens when the
> consumer cannot keep up, how a chain of threads stops, and how a thread
> that owns its state answers questions. All **std**, **100% safe**,
> **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements, but **not** the fix. Read what rustc or
> the test says first. Press `h` when you want the full answer.

## Core Ideas

- **A channel closes from either end.** It has any number of `Sender`s (they
  clone) and one `Receiver`. When every `Sender` is gone, the receiver still
  gets what is queued, and then `recv()` returns `Err(RecvError)`. When the
  `Receiver` is gone, every `send` fails at once and hands the value back in
  `SendError(value)`.
- **Unbounded queues hide overload.** `mpsc::channel()` never pushes back, so
  a slow consumer shows up as growing memory and latency instead of an
  error. `mpsc::sync_channel(n)` holds at most `n` items: `send` blocks while
  it is full, and `try_send` returns `Full(item)` at once. With `n == 0` it
  is a rendezvous.
- **Shutdown is a cascade of drops.** `for x in rx` ends only once every
  `Sender` is gone, so a pipeline stops by each stage returning, and dropping
  its own `Sender`, in turn. One forgotten clone hangs everything behind it,
  silently. In the other direction, a failed `send` is a stage's signal to
  stop, not a reason to panic.
- **An actor owns its state, and everyone else sends it messages.** One
  thread handles the commands in arrival order, so no lock is needed. Answers
  travel back on a reply channel carried inside each request, which also
  tells the caller when the actor has died.

## Picking a Channel

| You need                                            | std                                               | tokio                                   | Behavior                                                        |
| --------------------------------------------------- | ------------------------------------------------- | --------------------------------------- | --------------------------------------------------------------- |
| no limit (the producers are bounded some other way) | `mpsc::channel()`                                 | `mpsc::unbounded_channel()`             | never full; `send` fails only once the receiver is gone         |
| backpressure by waiting                             | `mpsc::sync_channel(n)`, then `send`              | `mpsc::channel(n)`, then `send().await` | the sender waits for space                                      |
| backpressure by refusing (load shedding)            | `sync_channel(n)`, then `try_send`                | `channel(n)`, then `try_send`           | `Full(item)` or `Disconnected(item)` (tokio: `Full` / `Closed`) |
| a hand-off with no buffer                           | `mpsc::sync_channel(0)`                           | none                                    | `send` waits until a receiver takes the item                    |
| exactly one reply                                   | an `mpsc::channel()` used once                    | `oneshot::channel()`                    | `recv` fails if the `Sender` is dropped unanswered              |
| several consumers sharing the work                  | `Arc<Mutex<Receiver<T>>>`, or `crossbeam-channel` | the `async-channel` crate               | each item goes to exactly one consumer                          |

`Receiver` is `Send` but not `Sync`, so only one thread can receive at a
time. `Sender` has been `Sync` since Rust 1.72. Since Rust 1.67, std's `mpsc`
is a port of `crossbeam-channel`. `std::sync::mpmc` and `std::sync::oneshot`
exist, but both are still unstable.

## Shutting Down Cleanly

- **Know who holds every `Sender`.** Move the original into the thread that
  needs it, or `drop` it right after spawning. A clone that stays behind in
  the spawning function keeps the channel open for as long as that function
  runs.
- **Close the queue, then join.** A thread pool drops the `Sender` of its job
  queue first and joins its workers second (the Book's `ThreadPool` does
  this in `Drop`). In the opposite order each worker waits in `recv` for a
  job while you wait for the worker.
- **Treat a failed `send` as "stop".** When the consumer leaves early, each
  stage returns at its first failed send. That drops the stage's
  `Receiver`, so the stage before it fails too, all the way back to the
  source.
- **Put a bound on every blocking wait in a test.** Rustlings has no test
  timeout, so these tests run the code under a watchdog: `recv_timeout` on
  a result channel fails the test instead of hanging it.

## Channels or `Arc<Mutex<T>>`?

|                        | `Arc<Mutex<T>>`                                        | Actor behind a channel                                         |
| ---------------------- | ------------------------------------------------------ | -------------------------------------------------------------- |
| who touches the state  | any thread that holds the lock                         | only the actor thread                                          |
| a multi-step operation | keep the guard alive across the steps                  | make it ONE command (a `get` then a `put` is not atomic)       |
| typical failures       | lock-order deadlocks, a guard held too long, poisoning | a lost reply, a full mailbox, two actors waiting on each other |
| cost of one read       | one lock                                               | two channel hops and a thread switch                           |
| good for               | small, hot, simple shared data                         | state with a life cycle of its own, I/O, many clients, async   |

An actor is not deadlock-free by construction: if actor A waits for a reply
from actor B while B waits for one from A, both stop, just as two locks taken
in opposite orders do (`53_lock_hazards`). A bounded mailbox adds
backpressure to an actor, and creates the same risk whenever two actors send
to each other.

## Exercise Path

1. **channel1** — `Ingest::offer` sits on an unbounded `channel()`, so nothing
   is ever "full" and the third offer at capacity 2 returns `Ok`. Bound the
   buffer with `sync_channel(capacity)` and offer with `try_send`, turning
   `Full` into `Busy(item)` and `Disconnected` into `Closed(item)`. Capacity
   0 must behave as a rendezvous, and a closed queue must never be reported
   as a busy one.
2. **channel2** — A three-stage pipeline (`source -> square -> batch`) on
   bounded links. `run_pipeline` hands each stage a clone of its output
   `Sender` and keeps the original, so neither the later stages nor the
   consumer ever see a disconnect, and the watchdog reports "pipeline never
   shut down". Every stage also `unwrap()`s its sends, so a consumer that
   stops early makes all three panic. Drop every `Sender` once its job is
   done, and make a failed send mean "return". One test is the thread-pool
   rule: drop the `Sender`, then join.
3. **channel3** — An actor thread owns a `HashMap` and handles an `enum Cmd`,
   but `Cmd::Get` has no way to answer (E0559 and E0026 in the tests). Give
   each request a `reply: Sender<Option<u32>>`, answer on it in the actor
   (shrugging off a caller that stopped waiting), and make `Handle::get` wait
   on a fresh reply channel, reporting `Gone` when the actor is gone.

Related modules: `30_send_sync` (`Send` and `Sync`; std declares `Receiver`
`Send` but explicitly not `Sync`), `52_condvar/condvar2` (a bounded blocking
queue built by hand from a `Mutex` and two `Condvar`s, which is what
`sync_channel` does for you), `53_lock_hazards` (lock-order deadlocks), `31_debugging/debugging8`
(a guard held too long) and `29_async_runtime/runtime3` (a channel as an
executor's ready queue). The tokio versions, with HTTP 503 for a full queue
and a graceful shutdown with `CancellationToken`, are in the `backend-lab/`
crate (`backpressure_http` and `graceful_shutdown`).

## Further Reading

- [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html): [`sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), [`SyncSender::try_send`](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send), [`TrySendError`](https://doc.rust-lang.org/std/sync/mpsc/enum.TrySendError.html) and [`Receiver::recv_timeout`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv_timeout)
- [Using Message Passing to Transfer Data Between Threads (The Book)](https://doc.rust-lang.org/book/ch16-02-message-passing.html)
- [Graceful Shutdown and Cleanup (The Book)](https://doc.rust-lang.org/book/ch21-03-graceful-shutdown-and-cleanup.html): the thread pool that drops its `Sender`, then joins
- [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/) by Alice Ryhl: the `mpsc` mailbox plus `oneshot` reply pattern
- [Channels (Tokio tutorial)](https://tokio.rs/tokio/tutorial/channels): backpressure and bounded channels in async code
- [Rust Atomics and Locks, ch. 5: Building Our Own Channels](https://marabos.nl/atomics/building-channels.html)
- [`crossbeam-channel`](https://docs.rs/crossbeam-channel), the design std's `mpsc` is based on, with `select!` and MPMC receivers
- [Backpressure explained](https://medium.com/@jayphelps/backpressure-explained-the-flow-of-data-through-software-2350b3e77ce7) by Jay Phelps
