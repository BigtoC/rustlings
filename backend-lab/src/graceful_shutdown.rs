//! Part 2 · `graceful_shutdown`: stop accepting, drain with a timeout, abort
//!
//! The interview question: "Implement graceful shutdown for a tokio server:
//! stop accepting, drain in-flight requests with a timeout, then abort." The
//! answer is four steps, and [`serve`] does them in order:
//!
//! 1. **Accept until told to stop.** A `select!` loop races the job queue
//!    (standing in for `TcpListener::accept`; both are cancel safe) against
//!    `CancellationToken::cancelled`, and spawns each job into a `JoinSet`.
//!    `biased;` puts the token first, so once it is canceled no further job
//!    is accepted, however busy the queue is.
//! 2. **Stop accepting.** `Receiver::close` makes every later `send` fail at
//!    once (the client learns it was refused). Then `recv` until it returns
//!    `None` reports the jobs already queued as rejected, instead of letting
//!    them be dropped unseen with the receiver. `recv`, not `try_recv` until
//!    the queue is empty: a client that reserved a `Permit` before the close
//!    can still send through it, and `recv` returns `None` only once every
//!    such permit has been used or dropped (tokio's `Receiver::close` docs).
//! 3. **Drain within a deadline.** One `timeout(grace, ...)` covers step 2
//!    and this drain, so a client that sits on its permit cannot stall the
//!    shutdown either. The drain is a `join_next` loop. `JoinSet::join_next`
//!    (like `recv`) is cancel safe, which is what makes it legal to put a
//!    timeout around it: when the timeout wins, no finished job has been
//!    taken out of the set and forgotten.
//! 4. **Abort the rest.** `JoinSet::abort_all`, then keep calling
//!    `join_next` until the set is empty: an aborted task reports a
//!    `JoinError` whose `is_cancelled()` is true, and `JoinError::id` maps it
//!    back to its job.
//!
//! The broken variants in [`broken`] each skip one step, and each has a test
//! that shows the damage: `serve_abort_immediately` kills in-flight jobs
//! that had plenty of time to finish, `serve_without_close` lets a client
//! "submit" a job during shutdown that then vanishes, and
//! `serve_drain_forever` never finishes while one job hangs.
//!
//! [`serve_http`] does the same for a real axum server:
//! `axum::serve(..).with_graceful_shutdown(signal)` covers steps 1 to 3
//! without the deadline. It stops accepting when the signal fires and then
//! waits for every connection, however long that takes, so the timeout is
//! yours to add. Step 4 is where axum differs from the `JoinSet` version:
//! axum `tokio::spawn`s a task per connection and keeps no handle to it (it
//! only counts the tasks, through clones of a `watch::Receiver`, to know
//! when the drain is done). So dropping the server future after the grace
//! period does NOT stop a connection that is still busy (a test below shows
//! the request finishing afterwards). In a binary the abort is returning
//! from `main`: shutting the runtime down cancels every task still alive.
//!
//! ```text
//! #[tokio::main]
//! async fn main() {
//!     let token = CancellationToken::new();
//!     let on_ctrl_c = token.clone();
//!     tokio::spawn(async move {
//!         tokio::signal::ctrl_c().await.ok();
//!         on_ctrl_c.cancel();
//!     });
//!     let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     serve_http(listener, app(), token, Duration::from_secs(10)).await;
//!     // Returning drops the runtime: whatever still runs is canceled here.
//! }
//! ```
//!
//! Compare with `54_channels/channel2`, where a std pipeline shuts down by
//! dropping every `Sender`, and with tokio-util's `TaskTracker`, which covers
//! step 3 for tasks you spawn through it (`close()`, then `wait()`) but, unlike
//! a `JoinSet`, keeps no results and does not abort its tasks when dropped.

use std::collections::HashMap;
use std::future::IntoFuture;
use std::pin::pin;
use std::time::Duration;

use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::{self, JoinError, JoinSet};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

/// A unit of work: it takes `work` (virtual time in the tests) to finish.
#[derive(Debug)]
pub struct Job {
    pub id: u32,
    pub work: Duration,
}

impl Job {
    pub fn new(id: u32, work: Duration) -> Self {
        Job { id, work }
    }

    async fn run(self) -> u32 {
        sleep(self.work).await;
        self.id
    }
}

/// Where every job the server took out of its queue ended up. Each job id
/// appears in exactly one list; a job whose `send` failed never reached the
/// server and appears in none.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Finished, in completion order.
    pub completed: Vec<u32>,
    /// Still running when the grace period ran out.
    pub aborted: Vec<u32>,
    /// Queued before shutdown but never started.
    pub rejected: Vec<u32>,
}

/// The running jobs and what became of the finished ones.
#[derive(Default)]
struct InFlight {
    tasks: JoinSet<u32>,
    /// Task id to job id: a `JoinError` only carries the task id.
    ids: HashMap<task::Id, u32>,
    report: Report,
}

impl InFlight {
    fn start(&mut self, job: Job) {
        let id = job.id;
        let handle = self.tasks.spawn(job.run());
        self.ids.insert(handle.id(), id);
    }

    fn record(&mut self, res: Result<(task::Id, u32), JoinError>) {
        match res {
            Ok((task, id)) => {
                self.ids.remove(&task);
                self.report.completed.push(id);
            }
            Err(err) if err.is_cancelled() => {
                let id = self.ids.remove(&err.id()).expect("every task has a job id");
                self.report.aborted.push(id);
            }
            // A job panicked: pass the panic on rather than hide it.
            Err(err) => std::panic::resume_unwind(err.into_panic()),
        }
    }

    /// Step 1: start every job from the queue until `token` is canceled.
    async fn accept_until_cancelled(
        &mut self,
        jobs: &mut mpsc::Receiver<Job>,
        token: &CancellationToken,
    ) {
        let mut queue_open = true;
        loop {
            tokio::select! {
                // Checked first on every round: after the cancellation, no
                // job is accepted even if the queue is ready too. The price
                // of `biased;` is that an always-ready branch starves the
                // ones below it: under a constant stream of jobs, finished
                // tasks wait in the set until the queue has a quiet moment
                // (harmless here, since the drain reaps them).
                biased;
                () = token.cancelled() => return,
                job = jobs.recv(), if queue_open => match job {
                    Some(job) => self.start(job),
                    // Every sender is gone; keep running until the token.
                    None => queue_open = false,
                },
                // Reap finished jobs as we go. With no tasks, `join_next`
                // returns `None` at once, the pattern fails and the branch
                // is disabled for this round.
                Some(res) = self.tasks.join_next_with_id() => self.record(res),
            }
        }
    }

    /// Step 2: refuse every later `send`, and report what is still queued.
    ///
    /// Waits (with `recv`) for clients that reserved a `Permit` before the
    /// close, so run it under a deadline.
    async fn stop_accepting(&mut self, jobs: &mut mpsc::Receiver<Job>) {
        jobs.close();
        while let Some(job) = jobs.recv().await {
            self.report.rejected.push(job.id);
        }
    }

    /// Step 3 without its deadline: wait for every running job.
    async fn drain(&mut self) {
        while let Some(res) = self.tasks.join_next_with_id().await {
            self.record(res);
        }
    }

    /// Step 4: abort what is left and wait until each task has stopped.
    async fn abort_rest(&mut self) {
        self.tasks.abort_all();
        self.drain().await;
    }
}

/// The graceful server: runs jobs from `jobs` until `token` is canceled,
/// then stops accepting, gives the running jobs `grace` to finish, and
/// aborts the rest.
pub async fn serve(
    mut jobs: mpsc::Receiver<Job>,
    token: CancellationToken,
    grace: Duration,
) -> Report {
    let mut in_flight = InFlight::default();
    in_flight.accept_until_cancelled(&mut jobs, &token).await;
    // One deadline for steps 2 and 3. An `Err(Elapsed)` just means some jobs
    // are still running (or a client still holds a permit).
    let _ = timeout(grace, async {
        in_flight.stop_accepting(&mut jobs).await;
        in_flight.drain().await;
    })
    .await;
    in_flight.abort_rest().await;
    in_flight.report
}

/// Each variant skips one step of [`serve`].
pub mod broken {
    use super::*;

    /// BROKEN: no drain. On the signal it aborts every running job at once,
    /// like dropping the `JoinSet` (or returning from `main`) would. Jobs a
    /// second away from finishing are lost.
    pub async fn serve_abort_immediately(
        mut jobs: mpsc::Receiver<Job>,
        token: CancellationToken,
        grace: Duration,
    ) -> Report {
        let mut in_flight = InFlight::default();
        in_flight.accept_until_cancelled(&mut jobs, &token).await;
        // Step 2 as in `serve`, under the deadline; then step 3 is skipped.
        let _ = timeout(grace, in_flight.stop_accepting(&mut jobs)).await;
        in_flight.abort_rest().await;
        in_flight.report
    }

    /// BROKEN: stops pulling jobs but never closes the queue. During the
    /// drain a client's `send` still succeeds; the job sits in the channel
    /// and is dropped with the receiver when this returns, reported nowhere.
    pub async fn serve_without_close(
        mut jobs: mpsc::Receiver<Job>,
        token: CancellationToken,
        grace: Duration,
    ) -> Report {
        let mut in_flight = InFlight::default();
        in_flight.accept_until_cancelled(&mut jobs, &token).await;
        let _ = timeout(grace, in_flight.drain()).await;
        in_flight.abort_rest().await;
        in_flight.report
    }

    /// BROKEN: drains without a deadline. One job that never finishes (or
    /// one client that never uses its permit) keeps the whole shutdown
    /// waiting, and a deployment that kills the process after its own
    /// timeout loses everything at once.
    pub async fn serve_drain_forever(
        mut jobs: mpsc::Receiver<Job>,
        token: CancellationToken,
        _grace: Duration,
    ) -> Report {
        let mut in_flight = InFlight::default();
        in_flight.accept_until_cancelled(&mut jobs, &token).await;
        in_flight.stop_accepting(&mut jobs).await;
        in_flight.drain().await;
        in_flight.report
    }
}

/// How an HTTP shutdown ended.
#[derive(Debug, PartialEq, Eq)]
pub enum HttpShutdown {
    /// Every connection closed within the grace period.
    Drained,
    /// The grace period ran out with connections still open.
    GraceExpired,
}

/// Serves `app` on `listener` until `token` is canceled, then gives the
/// open connections `grace` to finish.
///
/// axum does steps 1 to 3 (it stops accepting as soon as the signal fires,
/// then waits for every connection); this adds the deadline. It cannot do
/// step 4: when the grace period expires, the connections axum spawned keep
/// running until they finish or the runtime shuts down.
pub async fn serve_http(
    listener: TcpListener,
    app: Router,
    token: CancellationToken,
    grace: Duration,
) -> HttpShutdown {
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(token.clone().cancelled_owned())
        .into_future();
    let mut server = pin!(server);
    tokio::select! {
        // `with_graceful_shutdown` only finishes after the signal, so this
        // branch wins only if the token fires and the drain completes in the
        // same poll.
        _ = &mut server => return HttpShutdown::Drained,
        () = token.cancelled() => {}
    }
    match timeout(grace, server).await {
        Ok(_) => HttpShutdown::Drained,
        Err(_elapsed) => HttpShutdown::GraceExpired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use tokio::time::Instant;

    const MS: Duration = Duration::from_millis(1);
    const SEC: Duration = Duration::from_secs(1);
    const GRACE: Duration = Duration::from_secs(5);

    type Server<F> = fn(mpsc::Receiver<Job>, CancellationToken, Duration) -> F;

    /// The shared script. Three jobs start (1 s, 3 s and one hour of work),
    /// the token is canceled, then a client tries to submit job 4.
    /// Returns whether job 4's `send` succeeded, the report, and how long
    /// the server took to return after the cancellation.
    async fn script<F>(server: Server<F>) -> (bool, Report, Duration)
    where
        F: Future<Output = Report> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(8);
        let token = CancellationToken::new();
        let run = server(rx, token.clone(), GRACE);
        let server = tokio::spawn(async move { (run.await, Instant::now()) });
        for (id, work) in [(1, SEC), (2, 3 * SEC), (3, 3600 * SEC)] {
            tx.send(Job::new(id, work)).await.unwrap();
        }
        // Virtual time only moves when every task is idle, so this returns
        // after the server has accepted (and started) all three jobs.
        sleep(MS).await;
        let canceled_at = Instant::now();
        token.cancel();
        // Again: returns after the server has reacted to the token.
        sleep(MS).await;
        let late_send_ok = tx.send(Job::new(4, SEC)).await.is_ok();
        let (report, finished_at) = server.await.unwrap();
        (late_send_ok, report, finished_at - canceled_at)
    }

    fn report(completed: &[u32], aborted: &[u32], rejected: &[u32]) -> Report {
        Report {
            completed: completed.to_vec(),
            aborted: aborted.to_vec(),
            rejected: rejected.to_vec(),
        }
    }

    // ---- the fix ------------------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn drains_within_grace_aborts_the_rest_and_refuses_late_jobs() {
        let (late_send_ok, report, took) = script(serve).await;
        assert!(!late_send_ok, "a job sent after shutdown must be refused");
        // Jobs 1 and 2 finish inside the 5 s grace; job 3 is aborted.
        assert_eq!(report, super::tests::report(&[1, 2], &[3], &[]));
        // Shutdown took the grace period, not an hour.
        assert_eq!(took, GRACE);
    }

    #[tokio::test(start_paused = true)]
    async fn finishes_early_when_everything_drains() {
        let (tx, rx) = mpsc::channel(8);
        let token = CancellationToken::new();
        let server = tokio::spawn(serve(rx, token.clone(), GRACE));
        tx.send(Job::new(1, SEC)).await.unwrap();
        sleep(MS).await;
        let canceled_at = Instant::now();
        token.cancel();
        assert_eq!(server.await.unwrap(), report(&[1], &[], &[]));
        // Done when job 1 finished (1 s after it started), not after 5 s.
        assert_eq!(canceled_at.elapsed(), SEC - MS);
    }

    #[tokio::test(start_paused = true)]
    async fn queued_jobs_are_rejected_not_dropped() {
        let (tx, rx) = mpsc::channel(8);
        let token = CancellationToken::new();
        for id in 1..=3 {
            tx.send(Job::new(id, SEC)).await.unwrap();
        }
        // Canceled before the server ever runs: the three queued jobs must
        // not start (`biased;` checks the token first) and must not vanish.
        token.cancel();
        let report = serve(rx, token, GRACE).await;
        assert_eq!(report, super::tests::report(&[], &[], &[1, 2, 3]));
        assert!(tx.is_closed());
    }

    #[tokio::test(start_paused = true)]
    async fn a_job_sent_through_an_earlier_permit_is_rejected_not_lost() {
        let (tx, rx) = mpsc::channel(8);
        let token = CancellationToken::new();
        let server = tokio::spawn(serve(rx, token.clone(), GRACE));
        // The client reserves a slot while the queue is still open...
        let permit = tx.reserve().await.unwrap();
        token.cancel();
        sleep(MS).await;
        // ...and fills it after the server has closed the queue. A plain
        // `send` is refused now, but the permit still delivers.
        assert!(tx.is_closed());
        assert!(tx.send(Job::new(2, SEC)).await.is_err());
        permit.send(Job::new(1, SEC));
        // `recv` waited for the permit, so job 1 is reported, not dropped
        // unseen with the receiver (which a `try_recv` sweep would do).
        assert_eq!(server.await.unwrap(), report(&[], &[], &[1]));
    }

    #[tokio::test(start_paused = true)]
    async fn the_queue_closes_as_soon_as_shutdown_begins() {
        let (tx, rx) = mpsc::channel(8);
        let token = CancellationToken::new();
        let server = tokio::spawn(serve(rx, token.clone(), GRACE));
        tx.send(Job::new(1, 3600 * SEC)).await.unwrap();
        sleep(MS).await;
        let canceled_at = Instant::now();
        token.cancel();
        // `Sender::closed` resolves when the server calls
        // `Receiver::close`: at the instant of the cancellation, not when
        // the 5 s drain ends and the receiver is dropped.
        tx.closed().await;
        assert_eq!(canceled_at.elapsed(), Duration::ZERO);
        let err = tx.send(Job::new(2, SEC)).await.unwrap_err();
        assert_eq!(err.0.id, 2, "a refused send hands the job back");
        assert_eq!(server.await.unwrap(), report(&[], &[1], &[]));
    }

    // ---- the broken variants, demonstrated -----------------------------

    #[tokio::test(start_paused = true)]
    async fn broken_abort_immediately_loses_jobs_that_would_have_finished() {
        let (late_send_ok, report, took) = script(broken::serve_abort_immediately).await;
        assert!(!late_send_ok);
        // Jobs 1 and 2 needed 1 s and 3 s of a 5 s grace; all three die.
        assert_eq!(report, super::tests::report(&[], &[1, 2, 3], &[]));
        assert_eq!(took, Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn broken_without_close_accepts_a_job_during_shutdown_then_loses_it() {
        let (late_send_ok, report, _) = script(broken::serve_without_close).await;
        // The client was told "sent" after shutdown had begun...
        assert!(late_send_ok);
        // ...but job 4 never ran, was never rejected, and is in no list.
        assert_eq!(report, super::tests::report(&[1, 2], &[3], &[]));
    }

    #[tokio::test(start_paused = true)]
    async fn broken_drain_forever_waits_for_the_hour_long_job() {
        let outcome = timeout(60 * SEC, script(broken::serve_drain_forever)).await;
        assert!(outcome.is_err(), "still draining after a minute");
    }

    // ---- a real HTTP server --------------------------------------------
    //
    // These use a real TCP socket on 127.0.0.1 and real time (paused time
    // would auto-advance while the runtime waits for the socket). No test
    // relies on timing: the slow handler waits on a gate the test opens.

    mod http {
        use super::super::*;
        use axum::extract::State;
        use axum::routing::get;
        use std::net::SocketAddr;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio::sync::watch;

        #[derive(Clone)]
        struct Gate {
            started: mpsc::UnboundedSender<()>,
            open: watch::Receiver<bool>,
        }

        /// Tells the test it has started, then waits until the gate opens.
        async fn slow(State(mut gate): State<Gate>) -> &'static str {
            gate.started.send(()).unwrap();
            gate.open.wait_for(|open| *open).await.unwrap();
            "done"
        }

        struct Harness {
            addr: SocketAddr,
            token: CancellationToken,
            server: tokio::task::JoinHandle<HttpShutdown>,
            started: mpsc::UnboundedReceiver<()>,
            open: watch::Sender<bool>,
        }

        async fn start(grace: Duration) -> Harness {
            let (started_tx, started) = mpsc::unbounded_channel();
            let (open, open_rx) = watch::channel(false);
            let app = Router::new().route("/slow", get(slow)).with_state(Gate {
                started: started_tx,
                open: open_rx,
            });
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let token = CancellationToken::new();
            let server = tokio::spawn(serve_http(listener, app, token.clone(), grace));
            Harness {
                addr,
                token,
                server,
                started,
                open,
            }
        }

        /// A minimal HTTP/1.1 client: one GET, then read until the server
        /// closes the connection.
        async fn http_get(addr: SocketAddr, path: &str) -> String {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let request = format!("GET {path} HTTP/1.1\r\nhost: lab\r\nconnection: close\r\n\r\n");
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).await.unwrap();
            response
        }

        #[tokio::test]
        async fn an_in_flight_request_finishes_and_the_listener_is_gone() {
            let mut h = start(Duration::from_secs(10)).await;
            let client = tokio::spawn(http_get(h.addr, "/slow"));
            h.started.recv().await.unwrap();
            h.token.cancel();
            h.open.send(true).unwrap();
            let response = client.await.unwrap();
            assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
            assert!(response.ends_with("done"), "{response}");
            assert_eq!(h.server.await.unwrap(), HttpShutdown::Drained);
            // axum dropped the listener when the signal fired: nothing
            // accepts connections on this port anymore.
            let refused = TcpStream::connect(h.addr).await;
            assert!(refused.is_err(), "{refused:?}");
        }

        #[tokio::test]
        async fn grace_expiry_does_not_abort_axum_connections() {
            let mut h = start(Duration::from_millis(50)).await;
            let client = tokio::spawn(http_get(h.addr, "/slow"));
            h.started.recv().await.unwrap();
            h.token.cancel();
            // The handler cannot finish while the gate is shut, so the
            // 50 ms grace always expires.
            assert_eq!(h.server.await.unwrap(), HttpShutdown::GraceExpired);
            // `serve_http` has returned and dropped axum's server future,
            // yet the connection task axum spawned is still serving: open
            // the gate and the request completes.
            h.open.send(true).unwrap();
            let response = client.await.unwrap();
            assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        }
    }
}
