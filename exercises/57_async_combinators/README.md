# Module 3 · Async Combinators: Join, Select and Cancel Safety

> The composition layer on top of `28_futures` and `29_async_runtime`. There
> you wrote a single future, a waker and an executor; here you write the
> futures that drive OTHER futures, and learn what happens to a future that is
> dropped halfway. All **std**, **100% safe**, **stable** Rust, edition 2024.
> Every test is deterministic: the test executors poll in a bounded loop
> (with `Waker::noop()`, or a waker that only counts its wakeups), and the
> leaf futures count polls instead of reading a clock.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the failing tests and
> the requirements but **not** the fix, as in an interview. Press `h` when you
> want the full answer.

## Core Ideas

- **A combinator is a future that polls other futures.** `join` and `select`
  run their children inside ONE task: that is concurrency, not parallelism.
  Nothing runs on a second thread unless you `spawn` it, and `spawn` is why
  tokio asks for `Send + 'static` (see `56_async_bounds`). A `join` can borrow
  local data precisely because it is not spawned.
- **Poll every unfinished child, and never a finished one.** `join` polls each
  child that is still running on every poll, so it is `Ready` after
  `max(n_a, n_b) + 1` polls instead of `n_a + 1 + n_b`. Polling a future
  after it returned `Ready` may panic (an `async` block does), so store the
  output and remember that the child is done.
- **Pass the caller's `Context` down.** The children register the task's
  waker. A child polled with an unrelated waker (such as `Waker::noop()`) can
  never wake the task again. A combinator that hands its children wakers of
  its own, as `FuturesUnordered` does, must have them wake the task too.
- **Dropping a future cancels it.** There is no built-in cancellation signal:
  a future that is no longer polled stops at the `.await` where it last
  returned `Pending`, and dropping it runs the destructors of whatever it
  holds there (locals, lock guards, sockets). Code between two `.await`s is
  never interrupted. `Drop` is synchronous, so a canceled future cannot
  `.await` its cleanup: stable Rust has no async drop.
- **Cancel safety.** A future is cancel safe if dropping it at any `.await` and
  starting the same operation again loses nothing. Progress kept in the
  future's own locals dies with it; progress kept in the reader, channel or
  stream it borrows survives.
- **Spawned tasks are different.** Dropping a tokio `JoinHandle` does NOT
  cancel the task: it detaches it, and the task keeps running. Cancel a
  spawned task with `JoinHandle::abort` (or a `JoinSet`, which aborts its tasks
  when dropped).

## Sequential, Join and Select

|                                  | `a.await; b.await`     | `join!(a, b)`           | `select!` over `a`, `b`    |
| -------------------------------- | ---------------------- | ----------------------- | -------------------------- |
| When `b` starts                  | after `a` has finished | on the same poll as `a` | on the same poll as `a`    |
| Finishes                         | when both are done     | when both are done      | when the first one is done |
| Polls (leaves `Pending` n times) | `n_a + 1 + n_b`        | `max(n_a, n_b) + 1`     | `min(n_a, n_b) + 1`        |
| The other future                 | —                      | —                       | dropped, which cancels it  |
| Runs in parallel                 | no                     | no                      | no                         |

`join` over many futures needs care: polling a `Vec` of children on every
wakeup costs O(n) per wakeup. `FuturesUnordered` gives each child its own
waker and polls only the woken ones, and `futures::future::join_all` switches
to `FuturesOrdered`, which is built on it, for large inputs.

## `select!` in Practice

- **Fairness.** `tokio::select!` picks a random branch to check first, so a
  branch that is always ready cannot starve the others. `biased;` switches to
  top-to-bottom order, which is deterministic but makes starvation your
  problem. The `Select` in `select1` is biased on purpose, so its tests are
  deterministic.
- **Don't poll a decided race.** Once one branch is `Ready`, polling another
  one can complete it too, and its output (a received message) is then
  dropped with it.
- **Resuming instead of restarting.** Race `&mut fut` (a pinned future, made
  with `std::pin::pin!` or `tokio::pin!`) to keep ONE future alive across
  loop iterations. `futures::future::select` goes further and hands the
  unfinished future back to you: its output is
  `Either<(A::Output, B), (B::Output, A)>`.
- **Timeouts are selects.** `tokio::time::timeout(d, fut)` races `fut` against
  a sleep, so everything about cancel safety applies to it too.

## Cancel Safety at a Glance

From the tokio docs (`select!` and the individual methods):

| Cancel safe                                                                     | Not cancel safe                                                                                 |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `mpsc::Receiver::recv`, `broadcast::Receiver::recv`, `watch::Receiver::changed` | `AsyncReadExt::read_exact`, `read_to_end`, `read_to_string`                                     |
| `AsyncReadExt::read`, `AsyncWriteExt::write`                                    | `AsyncWriteExt::write_all`                                                                      |
| `TcpListener::accept`                                                           | `AsyncBufReadExt::read_line` (partially read data is lost)                                      |
| `Lines::next_line`, `StreamExt::next`                                           | `Mutex::lock`, `Semaphore::acquire`, `Notify::notified` (they lose their place in a fair queue) |

`AsyncBufReadExt::read_until` is the instructive neighbor of `read_line`: it
appends every byte to the caller's `buf` at once, so after a cancellation,
calling it again with the same `buf` continues the line. `read_line` cannot
do that, because a `String` may hold only valid UTF-8, so it keeps the bytes
inside its future until the line is complete. The tokio docs therefore
suggest `read_until` plus your own UTF-8 check, `Lines::next_line`, or
tokio-util's `LinesCodec` when you need a cancel-safe `read_line`.

The two standard fixes, both accepted by `cancel1`: keep the operation's
progress outside the future (in the reader, as `Lines` and tokio-util's
`FramedRead` do), or stop canceling it (create the future once, pin it
outside the loop, and race `&mut` to it against a fresh timer each round).

## A Note on Streams

A stream is the async version of an iterator:

```rust
trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}
```

`futures::StreamExt::next(&mut self)` (with `Self: Unpin`) returns a small
`Next` future that borrows the stream and calls `poll_next`. It keeps no state
of its own, so it is cancel safe for the same reason as the fixed `cancel1`:
the progress lives in the stream. The standard library's version,
`std::async_iter::AsyncIterator`, is still unstable (E0658, feature
`async_iterator`), so interviews ask about `futures::Stream` by concept.

## `Unpin` Children and Pin Projection

`join1` and `select1` accept only `Unpin` children and poll them with the safe
`Pin::new(&mut child)`. An `async` block is `!Unpin`; `Box::pin` it (or pin
it on the stack with `pin!` and pass the `Pin<&mut _>`) first.
`futures::future::select` has the same `Unpin` bounds. Storing `!Unpin`
children in place needs *pin projection*, going from `Pin<&mut Join>` to
`Pin<&mut A>` (the `pin-project-lite` crate, or `unsafe`), which this module
deliberately avoids. The `deep-dive/` lab `self_referential` shows what `Pin`
protects; async recursion and a projection lab are listed under
"Additional topics" in `deep-dive/ROADMAP.md`.

## Exercise Path

1. **join1** — `Join::poll` never polls a child, so nothing finishes. Poll
   both unfinished children on every poll with the caller's `Context`, store
   each output as it arrives, and never poll a finished child again. A
   sequential version fails the poll-count tests and deadlocks the
   producer/consumer rendezvous.
2. **select1** — `Select::poll` never polls a child. Poll `a` first and `b`
   only while `a` is `Pending`, return the winner as `Either`, and drop the
   loser on the very poll that decides the race: the tests check a drop flag
   while the `Select` is still alive, and a lost message if the loser was
   polled after the race was over.
3. **cancel1** — A heartbeat timer races `read_line` in a loop, and each time
   the timer wins, the half-read line is dropped with the future: "hello"
   arrives as "lo". Make `read_line` cancel safe by keeping its progress in
   the reader, or keep one `read_line` future alive across heartbeats.

Next: `58_leaf_futures` writes the leaf futures that these combinators
compose (a oneshot channel, a real timer, cooperative yielding). The
`backend-lab/` crate's `select_cancel` part (`backend-lab/src/select_cancel.rs`)
repeats the `cancel1` bug with tokio's real `select!` and `read_exact`.

## Further Reading

- [`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll), including what polling a completed future may do
- [`std::future::poll_fn`](https://doc.rust-lang.org/std/future/fn.poll_fn.html), [`Waker::noop`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.noop) and [`std::pin::pin!`](https://doc.rust-lang.org/std/pin/macro.pin.html)
- [Pinning, projections and structural pinning (`std::pin`)](https://doc.rust-lang.org/std/pin/index.html#projections-and-structural-pinning)
- [Tokio tutorial: Select](https://tokio.rs/tokio/tutorial/select), including cancellation and "Resuming an async operation"
- [`tokio::select!`](https://docs.rs/tokio/latest/tokio/macro.select.html), especially "Cancellation safety" and "Fairness"
- [`futures::future::select`](https://docs.rs/futures/latest/futures/future/fn.select.html), [`join_all`](https://docs.rs/futures/latest/futures/future/fn.join_all.html) and [`FuturesUnordered`](https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html)
- [Asynchronous Programming in Rust (the async book)](https://rust-lang.github.io/async-book/)
- [Async Cancellation I](https://blog.yoshuawuyts.com/async-cancellation-1/) by Yosh Wuyts
- [Cancelling async Rust](https://sunshowers.io/posts/cancelling-async-rust/) by Rain
