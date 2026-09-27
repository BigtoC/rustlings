//! Lab · tokio in production: select!, shutdown, 503s, blocking, fan-out
//!
//! The std-only async modules (`28_futures` to `58_leaf_futures`) teach the
//! rules with hand-written futures and executors. This crate repeats their
//! bugs on the real stack (tokio, tokio-util, axum, tower) and shows the
//! ecosystem answers that backend interviewers expect. It is a finished
//! reference to read, run and break, not an exercise: every part keeps its
//! broken variant next to the fix, and a test named `broken_*` shows the bug
//! happening. `README.md` answers the lab's interview question ("implement
//! graceful shutdown for a tokio server; which tokio APIs you use in
//! `select!` are cancel safe?"), with a table of tokio's cancel-safe and
//! cancel-unsafe methods.
//!
//! | Module | Compare with | What it reveals |
//! | --- | --- | --- |
//! | [`select_cancel`] | `57_async_combinators/cancel1` | `read_exact` in a `select!` loop loses half a frame and misaligns the stream; pin the read, or keep the framing state in the reader |
//! | [`graceful_shutdown`] | `54_channels/channel2` | `JoinSet` + `CancellationToken`: stop accepting, drain with a timeout, abort; axum's `with_graceful_shutdown` has no deadline and cannot abort |
//! | [`backpressure_http`] | `54_channels/channel1` | `try_send` into a bounded queue turns `Full` into 503; tower `LoadShed` over a global concurrency limit |
//! | [`blocking_in_async`] | `58_leaf_futures/yield1` | A CPU loop on `current_thread` starves a heartbeat (and a timeout); `spawn_blocking`, `yield_now`, `block_in_place` |
//! | [`bounded_fanout`] | `52_condvar/condvar3`, `57_async_combinators/join1` | At most K in flight: `Semaphore`, a `JoinSet` window, `buffered` / `buffer_unordered`; stop at the first error |
//!
//! Every test is deterministic. Most run in virtual time
//! (`#[tokio::test(start_paused = true)]`: the clock only moves when every
//! task is idle, straight to the next timer), sockets are `tokio::io::duplex`
//! pipes or in-process `ServiceExt::oneshot` calls, and slow handlers wait on
//! a gate the test opens. The tests that run in real time
//! (`blocking_in_async`, `backpressure_http`, and the two
//! `graceful_shutdown::tests::http` tests with a real socket) never race two
//! clocks against each other: a watchdog or a timeout there only turns a
//! hang into a failure.
//!
//! # Run it
//!
//! ```text
//! cargo test --manifest-path backend-lab/Cargo.toml
//! cargo test --manifest-path backend-lab/Cargo.toml select_cancel
//! cargo test --manifest-path backend-lab/Cargo.toml broken_
//! ```
//!
//! # Try it
//!
//! `README.md` lists ten experiments, each of which breaks one line of a
//! fix and names the test that then fails, and what it reports. For
//! example: move the `pin!` in [`select_cancel::read_frames_pinned`] into
//! the inner loop, and `pinned_frames_intact_across_100_ticks` gets the
//! same stitched frames as the broken reader.

pub mod backpressure_http;
pub mod blocking_in_async;
pub mod bounded_fanout;
pub mod graceful_shutdown;
pub mod select_cancel;
