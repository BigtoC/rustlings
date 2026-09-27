# Backend lab · tokio, axum and tower in production

> The std-only async modules (`28_futures` to `58_leaf_futures`) teach the
> rules with hand-written futures and executors. This lab repeats their bugs
> on the real stack (tokio 1.53, tokio-util 0.7, axum 0.8, tower 0.5) and
> shows the ecosystem answers backend interviewers expect. It is a sibling
> crate with external dependencies, so it lives outside the root workspace,
> like `deep-dive/`, and CI checks it in its own job.
>
> Like the deep-dive labs, it is a finished reference to read, run and break,
> not a fail-until-solved exercise. Every part keeps its broken variant in the
> source, next to the fix, and a test named `broken_*` shows the bug
> happening. Every test is deterministic: most run in virtual time
> (`#[tokio::test(start_paused = true)]`), sockets are in-memory pipes or
> in-process `ServiceExt::oneshot` calls, and slow handlers wait on a gate the
> test opens. The tests that run in real time (`blocking_in_async`,
> `backpressure_http`, and two HTTP tests on a real socket) never race two
> clocks: a watchdog or timeout there only turns a hang into a failure.

## What it reveals

| File                       | Compare with                                        | What it reveals                                                                                                                                                                                                                                                        |
| -------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/select_cancel.rs`     | `57_async_combinators/cancel1`                      | `read_exact` raced against a heartbeat ticker in a `select!` loop: one tick in the middle of a frame loses 4 bytes and misaligns every later frame. Fixed by pinning one `read_exact` outside the loop, or by a `FrameReader` that keeps the partial frame in itself.  |
| `src/graceful_shutdown.rs` | `54_channels/channel2`                              | `JoinSet` + `CancellationToken`: stop accepting, drain within a grace period, abort the rest, and report every job. Broken variants lose in-flight jobs, accept a job during shutdown and drop it, or wait forever. axum's `with_graceful_shutdown` has no deadline.   |
| `src/backpressure_http.rs` | `54_channels/channel1`                              | An axum handler `try_send`s into `mpsc::channel(n)` and maps `Full` to `503` with `Retry-After` (`State<Arc<AppState>>`, an `IntoResponse` error). The unbounded version never says 503. tower `LoadShed` over a concurrency limit, and why it must be the global one. |
| `src/blocking_in_async.rs` | `58_leaf_futures/yield1`                            | A CPU loop on a `current_thread` runtime starves a heartbeat task, and even a `timeout` around it. Fixed with `spawn_blocking` or `yield_now`; `block_in_place` panics on `current_thread`.                                                                            |
| `src/bounded_fanout.rs`    | `52_condvar/condvar3`, `57_async_combinators/join1` | N requests with at most K in flight: a `Semaphore`, a `JoinSet` sliding window, `buffered` / `buffer_unordered`. Input vs completion order, stopping at the first error, and why `abort` is only a request.                                                            |

## The sharpest question, answered

> Implement graceful shutdown for a tokio server: stop accepting, drain
> in-flight requests with a timeout, then abort. Which tokio APIs you use in
> `select!` are cancel-safe?

`graceful_shutdown::serve` is the full answer. Its shape, with a real
listener:

```rust
let mut tasks = JoinSet::new();
loop {
    tokio::select! {
        biased;                                         // check the token first
        () = token.cancelled() => break,                // cancel safe
        conn = listener.accept() => {                   // cancel safe
            let (stream, _) = conn?;
            tasks.spawn(handle(stream));
        }
        Some(done) = tasks.join_next() => log(done),    // cancel safe
    }
}
drop(listener);                                         // 1. stop accepting
let drained = timeout(grace, async {                    // 2. drain, with a deadline
    while let Some(done) = tasks.join_next().await {
        log(done);
    }
})
.await;
if drained.is_err() {
    tasks.shutdown().await;                             // 3. abort, and wait for it
}
```

- **Every branch in the loop must be cancel safe**, because `select!` drops
  the losing branches on every round. `CancellationToken::cancelled` (per the
  tokio-util docs), `TcpListener::accept` and `JoinSet::join_next` all are:
  losing a round consumes no connection and removes no finished task. A
  non-cancel-safe branch there (`read_exact`, `write_all`, `Mutex::lock`)
  loses data or its place in a queue; `select_cancel` shows the data loss.
- **`biased;` with the token first** means that once shutdown starts, not
  one more connection is accepted, however busy the listener is. Without it,
  `select!` picks a random branch to poll first; experiment 2 below shows
  queued jobs being started after the cancellation in about a third of the
  runs.
- **The drain has a deadline, and `join_next` is what makes that safe:** the
  `timeout` cancels the drain loop, and a cancel-safe `join_next` guarantees
  that no finished task was taken out of the set and forgotten.
- **Abort is a request.** `abort_all` (and dropping a `JoinSet`) only signals
  the tasks; tokio's docs say an idle task "will be shut down as soon as
  possible", which is the next time the runtime gets to it.
  `shutdown().await` aborts and then waits until every task has stopped
  (`bounded_fanout::dropping_a_joinset_only_requests_the_abort` shows the
  difference). An aborted task yields a `JoinError` with `is_cancelled()`.
- **Account for everything.** The lab's server reads jobs from an `mpsc`
  channel instead of a socket. When shutdown begins, `Receiver::close` makes
  every later `send` fail at once, and the server calls `recv` until it
  returns `None`, reporting each job still queued as rejected. Not `try_recv`
  until the queue is empty. tokio's `Receiver::close` docs warn: "Any
  outstanding `Permit` values will still be able to send messages", and
  `recv` returns `None` only once every permit has been used or dropped
  (`a_job_sent_through_an_earlier_permit_is_rejected_not_lost`). That wait
  shares the grace deadline, since a client could keep its permit forever.
  Skipping the `close` is the bug in `broken::serve_without_close`: a client
  is told its job was sent after shutdown began, and the job disappears.
- **axum does steps 1 and 2 without the deadline, and cannot do step 3.**
  `axum::serve(..).with_graceful_shutdown(signal)` stops accepting when the
  signal fires, then waits for every connection, however long that takes. Wrap
  it in a `timeout` (`graceful_shutdown::serve_http`). axum `tokio::spawn`s a
  task per connection and keeps no handle to it (it only counts the tasks,
  through clones of a `watch::Receiver`, to know when the drain is done), so
  dropping the server future aborts nothing:
  `grace_expiry_does_not_abort_axum_connections` completes a request after the
  grace period has expired. In a binary, the abort is
  returning from `main`: shutting the runtime down cancels every task still
  alive.

## Cancel safety in tokio 1.53

A future is cancel safe if dropping it before it completes, and starting the
same operation again, loses nothing. The `select!` docs put it as "it must be
a no-op to drop that future and recreate it". The table follows the tokio
1.53.1 and tokio-util 0.7.19 docs; paths are relative to the `tokio` crate
unless they name another crate, and the quotes are from each method's own
docs.

| Method                                                                                                                 | Cancel safe?                            | What a canceled call leaves behind                                                                             |
| ---------------------------------------------------------------------------------------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `sync::mpsc::Receiver::recv`, `sync::mpsc::UnboundedReceiver::recv`, `sync::broadcast::Receiver::recv`                 | yes (`select!` docs)                    | "no messages were received on this channel"                                                                    |
| `sync::watch::Receiver::changed`                                                                                       | yes (`select!` docs)                    | "no values have been marked seen by this call to `changed`"                                                    |
| `net::TcpListener::accept`, `net::UnixListener::accept`, `signal::unix::Signal::recv`                                  | yes (`select!` docs)                    | nothing lost                                                                                                   |
| `io::AsyncReadExt::read`, `io::AsyncReadExt::read_buf`, `io::AsyncWriteExt::write`, `io::AsyncWriteExt::write_buf`     | yes (`select!` docs)                    | nothing read, nothing written                                                                                  |
| `tokio_stream::StreamExt::next`, `futures::stream::StreamExt::next`                                                    | yes (`select!` docs: "on any `Stream`") | nothing lost: the progress lives in the stream                                                                 |
| `time::Interval::tick`                                                                                                 | yes (method docs)                       | "no tick has been consumed"                                                                                    |
| `task::JoinSet::join_next`, `task::JoinSet::join_next_with_id`                                                         | yes (method docs)                       | "no tasks were removed from this `JoinSet`"                                                                    |
| awaiting a `&mut sync::oneshot::Receiver`                                                                              | yes (type docs)                         | "no message was received on this channel"                                                                      |
| `io::Lines::next_line`, `io::AsyncBufReadExt::fill_buf`                                                                | yes (method docs)                       | nothing lost; for `fill_buf`, "no data was read"                                                               |
| `sync::mpsc::Sender::closed`                                                                                           | yes (method docs)                       | nothing: "Once the channel is closed, it stays closed forever"                                                 |
| `tokio_util::sync::CancellationToken::cancelled`, `CancellationToken::cancelled_owned`                                 | yes (method docs)                       | nothing lost                                                                                                   |
| `io::AsyncReadExt::read_exact`, `io::AsyncReadExt::read_to_end`, `io::AsyncReadExt::read_to_string`                    | no, data loss (`select!` docs)          | for `read_exact`, "some data may already have been read into `buf`", and a new call starts over                |
| `io::AsyncWriteExt::write_all`                                                                                         | no, data loss (`select!` docs)          | "future calls to `write_all` will start over from the beginning of the buffer"                                 |
| `io::AsyncBufReadExt::read_line`                                                                                       | no, data loss (method docs)             | "some data may have been partially read, and this data is lost"                                                |
| `sync::mpsc::Sender::send`                                                                                             | no, data loss (method docs)             | "the message was not sent", but "the message is dropped and will be lost"; `reserve` a permit first, then send |
| `sync::Mutex::lock`, `sync::RwLock::read`, `sync::RwLock::write`, `sync::Semaphore::acquire`, `sync::Notify::notified` | no, queue place (`select!` docs)        | "cancellation makes you lose your place in the queue"                                                          |
| `sync::mpsc::Sender::reserve`, `sync::Semaphore::acquire_owned`, `sync::Mutex::lock_owned`                             | no, queue place (method docs)           | the canceled call loses its place in the queue                                                                 |
| `tokio_util::sync::CancellationToken::run_until_cancelled`                                                             | only if `fut` is (method docs)          | whatever `fut` loses                                                                                           |

Two neighbors worth knowing. `io::AsyncBufReadExt::read_until` appends every
partially read byte to the caller's `buf`, so calling it again with the same
`buf` continues the line (only the returned byte count restarts); that is the
cancel-safe way to read lines by hand. `time::timeout(d, fut)` is a `select!`
in disguise: when the deadline passes, "the future is canceled", so it is
exactly as cancel safe as `fut`. Its docs also warn that "the future is polled
before the timeout is checked", which is why `blocking_in_async` can show a
50 ms timeout returning `Ok` after 200 ms of blocking code.

## Two follow-up questions

**`tokio::sync::Mutex` or `std::sync::Mutex`?** tokio's own docs answer:
"Contrary to popular belief, it is ok and often preferred to use the ordinary
`Mutex` from the standard library in asynchronous code." What the async mutex
adds is "the ability to keep it locked across an `.await` point", which makes
it more expensive, and its primary use case is "shared mutable access to IO
resources such as a database connection". So: plain data goes behind a std
`Mutex` that is locked and unlocked with no `.await` in between
(`56_async_bounds/async_send1` shows the `!Send` future you get when a
std guard lives across an `.await`); a resource that must stay locked across
an `.await` goes behind `tokio::sync::Mutex`, or better, into a task that owns
it and takes requests over a channel (the actor of `54_channels/channel3`).
And `tokio::sync::Mutex::lock` is in the queue-place row of the table above.

**How does tower middleware work?** A `Service` is two methods: `poll_ready`,
which returns `Ready` "if the service expects that it is able to process a
request", and `call`, which takes the request and returns a future of the
response. A `Layer` wraps one service in another, and `ServiceBuilder` lists
layers outermost first. Backpressure travels through `poll_ready`: in
`backpressure_http::shed_load`, the concurrency limit's `poll_ready` is
`Pending` while every permit is taken, `LoadShed` turns that `Pending` into an
immediate `Overloaded` error, and `HandleErrorLayer` turns the error into a
503, because axum only routes to services whose error type is `Infallible`.
`Router::layer` applies the layer to each route separately, which is why the
limit must be `GlobalConcurrencyLimitLayer` (one shared semaphore) and not
`ConcurrencyLimitLayer` (a new semaphore per route).

## Run it

```bash
# Everything (34 tests, well under a second once built; the two HTTP tests use
# a real socket on 127.0.0.1):
cargo test --manifest-path backend-lab/Cargo.toml

# One part, or only the bug demonstrations:
cargo test --manifest-path backend-lab/Cargo.toml select_cancel
cargo test --manifest-path backend-lab/Cargo.toml broken_

# What CI runs (the `backend-lab` job):
cargo fmt --manifest-path backend-lab/Cargo.toml --check
cargo clippy --manifest-path backend-lab/Cargo.toml --all-targets -- --deny warnings
```

The crate has no `unsafe` (`unsafe_code = "forbid"`), so, unlike
`deep-dive/`, it is not run under Miri.

## Try it

Each experiment breaks one line of a fix; the test named next to it then
fails as described. All outcomes below were observed on tokio 1.53.1.

1. **Cancel the pinned read after all.** In
   `select_cancel::read_frames_pinned`, move
   `let mut read = pin!(reader.read_exact(&mut buf));` from above the inner
   `loop` into it. `pinned_frames_intact_across_100_ticks` then gets the
   stitched frames `[0, 0, 0, 0, 1, 1, 1, 1], [1, 1, 1, 1, 2, 2, 2, 2], ...`,
   the same as `broken_one_tick_mid_frame_misaligns_every_later_frame`.
2. **Drop `biased;`** from `InFlight::accept_until_cancelled` and run the
   test in a loop:

   ```bash
   for i in $(seq 20); do
     cargo test --manifest-path backend-lab/Cargo.toml -q queued_jobs_are_rejected
   done
   ```

   About a third of the runs fail (24 of 90 here), most often with
   `Report { completed: [1], aborted: [], rejected: [2, 3] }`: `select!`
   polled the queue before the token and started a job after the
   cancellation. This is the one experiment that depends on tokio's random
   branch order, which is exactly the point.
3. **Forget to close the queue.** Comment out `jobs.close();` in
   `InFlight::stop_accepting`. Five `graceful_shutdown` tests fail. In
   `drains_within_grace_aborts_the_rest_and_refuses_late_jobs` the late
   `send` succeeds although shutdown had begun. In
   `finishes_early_when_everything_drains` (`left: 5s, right: 999ms`) and
   `the_queue_closes_as_soon_as_shutdown_begins` (`left: 5s, right: 0ns`)
   the open queue costs the whole grace period: `recv` returns `None` only on
   a closed channel (or once every `Sender` is gone), so the sweep waits
   until the deadline, and `Sender::closed` resolves only when the receiver
   is finally dropped.
4. **Sweep with `try_recv`.** In `InFlight::stop_accepting`, replace
   `while let Some(job) = jobs.recv().await` with
   `while let Ok(job) = jobs.try_recv()`.
   `a_job_sent_through_an_earlier_permit_is_rejected_not_lost` fails with
   `left: Report { completed: [], aborted: [], rejected: [] }`: the sweep
   found the queue empty and the server returned, so the job the client then
   sent through its permit was dropped with the receiver, reported nowhere.
5. **Use the per-route limit.** In `backpressure_http::shed_load`, replace
   `GlobalConcurrencyLimitLayer::new` with `ConcurrencyLimitLayer::new` (what
   `broken::shed_load_per_route` does).
   `shed_load_refuses_the_request_over_the_limit` fails after its 5 s
   timeout with `GET /b was admitted or queued, not shed`: each route got its
   own semaphore, so `/b` still had a free permit.
6. **Swap the layers.** In `shed_load`, put `.load_shed()` after the
   concurrency-limit layer. The same test fails the same way, this time on
   `GET /a`: with the limit outside `LoadShed`, the extra request waits for a
   permit instead of being refused.
7. **Stop yielding.** Delete `tokio::task::yield_now().await;` from
   `blocking_in_async::crunch_cooperatively`.
   `yielding_every_round_lets_the_heartbeat_run` fails after its 5 s
   watchdog with `too few beats: Probe { beats_seen: 0, gave_up: true }`.
8. **Hide the bug with a multi-threaded runtime.** Change the attribute of
   `broken_inline_crunch_starves_the_heartbeat` to
   `#[tokio::test(flavor = "multi_thread", worker_threads = 1)]`. The test
   fails with `the heartbeat got the thread` and `gave_up: false`: the test
   body now runs on the test's own thread (in `block_on`) and the heartbeat
   on the worker thread, so nothing starves. On a multi-threaded runtime,
   blocking code is easy to miss: the other workers keep most tasks moving.
   But a task in the blocked worker's LIFO slot cannot be stolen (tokio's
   runtime docs), and once blocking code occupies every worker, everything
   stalls.
9. **Drop the permit at once.** In `bounded_fanout::fetch_all_semaphore`,
   change `let _permit = permits` to `let _ = permits`. `let _` drops the
   permit on the spot, so
   `every_bounded_version_caps_in_flight_at_k_and_keeps_input_order` sees a
   peak of 50 requests in flight instead of 5.
10. **Abort without waiting.** In `bounded_fanout::fetch_all_window`,
    replace `set.shutdown().await` with `set.abort_all()`.
    `window_stops_at_the_first_error_and_never_starts_the_rest` finds 2
    requests still in flight when the function has already returned.

## Related modules

- `57_async_combinators` (`select1`, `cancel1`): hand-written `select` and
  the std version of the `select_cancel` bug and both of its fixes.
- `58_leaf_futures` (`timer1`, `yield1`): why `thread::sleep` and CPU loops
  block an executor, and the table of blocking, yielding and offloading.
- `54_channels` (`channel1`, `channel2`, `channel3`): `sync_channel` +
  `try_send` as load shedding, a pipeline that shuts down by dropping its
  senders, and an actor that owns its state.
- `52_condvar/condvar3`: a counting semaphore with an RAII permit, built from
  a `Mutex` and a `Condvar`.
- `56_async_bounds`: why `tokio::spawn` (and so `JoinSet::spawn`) needs
  `Send + 'static`.

## Further reading

- [`tokio::select!`](https://docs.rs/tokio/1.53.1/tokio/macro.select.html), sections "Cancellation safety" and "Fairness"
- [Tokio topics: graceful shutdown](https://tokio.rs/tokio/topics/shutdown)
- [`tokio::task::JoinSet`](https://docs.rs/tokio/1.53.1/tokio/task/struct.JoinSet.html) and [`tokio_util::sync::CancellationToken`](https://docs.rs/tokio-util/0.7.19/tokio_util/sync/struct.CancellationToken.html)
- [`tokio::sync::Mutex`](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Mutex.html), section "Which kind of mutex should you use?"
- [`axum::serve::Serve::with_graceful_shutdown`](https://docs.rs/axum/0.8.9/axum/serve/struct.Serve.html#method.with_graceful_shutdown)
- [`tower::Service`](https://docs.rs/tower/0.5.3/tower/trait.Service.html), [`tower::load_shed`](https://docs.rs/tower/0.5.3/tower/load_shed/index.html) and [`tower::limit`](https://docs.rs/tower/0.5.3/tower/limit/index.html)
- [`tokio::task::spawn_blocking`](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html) and [`tokio::time::timeout`](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html)
- [Alice Ryhl: Async: What is blocking?](https://ryhl.io/blog/async-what-is-blocking/)
- [`futures::stream::StreamExt::buffer_unordered`](https://docs.rs/futures/0.3.34/futures/stream/trait.StreamExt.html#method.buffer_unordered)
- [Cancelling async Rust](https://sunshowers.io/posts/cancelling-async-rust/) by Rain
