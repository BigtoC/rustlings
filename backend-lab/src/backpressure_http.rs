//! Part 3 · `backpressure_http`: a full queue answers 503, not "accepted"
//!
//! An endpoint accepts jobs and hands them to a background worker through a
//! queue. When the worker falls behind, something has to give. With an
//! unbounded queue ([`broken::router`]) nothing does: every request gets
//! `202 Accepted`, and overload shows up later as memory growth and jobs
//! that wait minutes. With a bounded `mpsc::channel(n)` and `try_send`
//! ([`router`]), the queue never holds more than `n` jobs and request `n + 1`
//! gets `503 Service Unavailable` with `Retry-After` at once: the client
//! learns about the overload while it can still back off or go elsewhere.
//!
//! The pieces an axum interviewer looks for:
//!
//! - `State<Arc<AppState>>`: shared state is built once, wrapped in an `Arc`,
//!   and cloned into every request (`Router::with_state`).
//! - `try_send`, not `send().await`: waiting for queue space inside a handler
//!   just moves the unbounded queue into the pile of waiting requests.
//!   `TrySendError::Full` means "busy" (503 with `Retry-After`) and `Closed`
//!   means "shutting down" (also 503, without the hint).
//! - An error type that implements `IntoResponse`, so the handler returns
//!   `Result<StatusCode, ApiError>` and uses `?` (via `From<TrySendError>`).
//! - Tests without a socket: a `Router` is a tower `Service`, and
//!   `ServiceExt::oneshot` drives one request through it in-process.
//!
//! [`shed_load`] is the middleware version for work done inside the request
//! itself: tower's `LoadShed` over a concurrency limit. The limit's
//! `poll_ready` is `Pending` when all permits are taken, and `LoadShed` turns
//! that into an immediate `Overloaded` error instead of queueing the request;
//! `HandleErrorLayer` maps the error to a 503. Use
//! `GlobalConcurrencyLimitLayer` under `Router::layer`: axum applies the
//! layer to each route separately (to each method of each route, in fact),
//! and `ConcurrencyLimitLayer` creates a new semaphore every time it is
//! applied, so it limits each route on its own
//! ([`broken::shed_load_per_route`]; a test shows two routes admitting one
//! request each under a limit of 1).
//!
//! Compare with `54_channels/channel1`, the std version (`sync_channel` plus
//! `try_send`, with `Full` as `Busy`).

use std::sync::Arc;

use axum::Router;
use axum::error_handling::HandleErrorLayer;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tower::limit::{ConcurrencyLimitLayer, GlobalConcurrencyLimitLayer};
use tower::load_shed::error::Overloaded;
use tower::{BoxError, ServiceBuilder};

/// State shared by every request.
pub struct AppState {
    queue: mpsc::Sender<String>,
}

impl AppState {
    pub fn new(queue: mpsc::Sender<String>) -> Self {
        AppState { queue }
    }

    /// Jobs waiting in the queue right now.
    pub fn depth(&self) -> usize {
        self.queue.max_capacity() - self.queue.capacity()
    }
}

/// Why a job was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum ApiError {
    /// The queue (or the concurrency limit) is full: retry later.
    Overloaded,
    /// The worker is gone: the service is shutting down.
    ShuttingDown,
}

impl<T> From<TrySendError<T>> for ApiError {
    fn from(err: TrySendError<T>) -> Self {
        match err {
            TrySendError::Full(_) => ApiError::Overloaded,
            TrySendError::Closed(_) => ApiError::ShuttingDown,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Overloaded => (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::RETRY_AFTER, "1")],
                "overloaded, retry later\n",
            )
                .into_response(),
            ApiError::ShuttingDown => {
                (StatusCode::SERVICE_UNAVAILABLE, "shutting down\n").into_response()
            }
        }
    }
}

/// `POST /jobs`: queue the body for the worker, or refuse it right away.
async fn submit(State(state): State<Arc<AppState>>, body: String) -> Result<StatusCode, ApiError> {
    state.queue.try_send(body)?;
    Ok(StatusCode::ACCEPTED)
}

/// The fixed service: `POST /jobs` over a bounded queue.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new().route("/jobs", post(submit)).with_state(state)
}

pub mod broken {
    use super::*;

    /// State around an unbounded queue: there is no "full".
    pub struct AppState {
        pub queue: mpsc::UnboundedSender<String>,
    }

    async fn submit(
        State(state): State<Arc<AppState>>,
        body: String,
    ) -> Result<StatusCode, ApiError> {
        // Fails only once the worker is gone. Never "busy".
        state.queue.send(body).map_err(|_| ApiError::ShuttingDown)?;
        Ok(StatusCode::ACCEPTED)
    }

    /// BROKEN: `POST /jobs` over an unbounded queue never answers 503.
    pub fn router(state: Arc<AppState>) -> Router {
        Router::new().route("/jobs", post(submit)).with_state(state)
    }

    /// BROKEN: [`super::shed_load`] with the per-service
    /// `ConcurrencyLimitLayer`. `Router::layer` applies the stack to every
    /// route (every method of every route), and each application creates its
    /// own semaphore, so "at most `max_in_flight` requests" becomes "at most
    /// `max_in_flight` per route".
    pub fn shed_load_per_route(app: Router, max_in_flight: usize) -> Router {
        app.layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(overload_to_response))
                .load_shed()
                .layer(ConcurrencyLimitLayer::new(max_in_flight)),
        )
    }
}

/// Maps the errors of the load-shedding stack to responses.
async fn overload_to_response(err: BoxError) -> Response {
    if err.is::<Overloaded>() {
        ApiError::Overloaded.into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
    }
}

/// Wraps every route of `app` so that at most `max_in_flight` requests run
/// at once, across all of them; any request beyond that gets a 503 at once.
///
/// Order matters: `ServiceBuilder` lists layers outermost first, so a
/// request passes `HandleErrorLayer`, then `LoadShed`, then the limit.
pub fn shed_load(app: Router, max_in_flight: usize) -> Router {
    app.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(overload_to_response))
            .load_shed()
            .layer(GlobalConcurrencyLimitLayer::new(max_in_flight)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::get;
    use tokio::sync::watch;
    use tower::ServiceExt;

    fn post_job(body: &str) -> Request<Body> {
        Request::post("/jobs")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    fn get_req(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).unwrap()
    }

    /// Sends one request through the router, in process: no socket.
    async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Option<String>, String) {
        let response = app.clone().oneshot(req).await.unwrap();
        let status = response.status();
        let retry_after = response
            .headers()
            .get(header::RETRY_AFTER)
            .map(|v| v.to_str().unwrap().to_owned());
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        (
            status,
            retry_after,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }

    /// For a `GET` that must be refused at once. If it is admitted (it then
    /// waits in the gated handler) or queued for a permit instead, the
    /// timeout turns that hang into a test failure.
    async fn call_shed(app: &Router, path: &str) -> (StatusCode, Option<String>, String) {
        let limit = std::time::Duration::from_secs(5);
        match tokio::time::timeout(limit, call(app, get_req(path))).await {
            Ok(response) => response,
            Err(_) => panic!("GET {path} was admitted or queued, not shed: no answer in 5 s"),
        }
    }

    // ---- bounded queue --------------------------------------------------

    #[tokio::test]
    async fn request_n_plus_1_gets_503_and_the_queue_never_exceeds_n() {
        let n = 4;
        let (tx, mut rx) = mpsc::channel(n);
        let state = Arc::new(AppState::new(tx));
        let app = router(state.clone());
        for i in 0..n {
            let (status, _, _) = call(&app, post_job(&format!("job {i}"))).await;
            assert_eq!(status, StatusCode::ACCEPTED);
        }
        let (status, retry_after, body) = call(&app, post_job("one too many")).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(retry_after.as_deref(), Some("1"));
        assert_eq!(body, "overloaded, retry later\n");
        assert_eq!(state.depth(), n);
        assert_eq!(rx.len(), n);

        // The worker takes one job: there is room for exactly one more.
        assert_eq!(rx.recv().await.as_deref(), Some("job 0"));
        assert_eq!(call(&app, post_job("job 4")).await.0, StatusCode::ACCEPTED);
        assert_eq!(
            call(&app, post_job("job 5")).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(state.depth(), n);
    }

    #[tokio::test]
    async fn a_gone_worker_means_shutting_down() {
        let (tx, rx) = mpsc::channel(4);
        let app = router(Arc::new(AppState::new(tx)));
        drop(rx);
        let (status, retry_after, body) = call(&app, post_job("late")).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(retry_after, None);
        assert_eq!(body, "shutting down\n");
    }

    #[tokio::test]
    async fn broken_unbounded_queue_never_says_503() {
        let n = 4;
        let (tx, rx) = mpsc::unbounded_channel();
        let app = broken::router(Arc::new(broken::AppState { queue: tx }));
        // Ten times the intended capacity, and every request is "accepted".
        for i in 0..10 * n {
            let (status, _, _) = call(&app, post_job(&format!("job {i}"))).await;
            assert_eq!(status, StatusCode::ACCEPTED);
        }
        assert_eq!(rx.len(), 10 * n);
    }

    // ---- load shedding middleware ---------------------------------------

    /// A handler that reports that it started, then waits for the gate.
    #[derive(Clone)]
    struct Gate {
        started: mpsc::UnboundedSender<&'static str>,
        open: watch::Receiver<bool>,
    }

    async fn slow(State(mut gate): State<Gate>, req: Request<Body>) -> &'static str {
        let name = if req.uri().path() == "/a" { "a" } else { "b" };
        gate.started.send(name).unwrap();
        gate.open.wait_for(|open| *open).await.unwrap();
        name
    }

    fn two_slow_routes() -> (
        Router,
        mpsc::UnboundedReceiver<&'static str>,
        watch::Sender<bool>,
    ) {
        let (started_tx, started) = mpsc::unbounded_channel();
        let (open, open_rx) = watch::channel(false);
        let app = Router::new()
            .route("/a", get(slow))
            .route("/b", get(slow))
            .with_state(Gate {
                started: started_tx,
                open: open_rx,
            });
        (app, started, open)
    }

    #[tokio::test]
    async fn shed_load_refuses_the_request_over_the_limit() {
        let (app, mut started, open) = two_slow_routes();
        let app = shed_load(app, 1);
        let first = tokio::spawn({
            let app = app.clone();
            async move { call(&app, get_req("/a")).await }
        });
        assert_eq!(started.recv().await, Some("a"));
        // One request is in flight: the next one, on either route, is shed.
        for path in ["/a", "/b"] {
            let (status, retry_after, _) = call_shed(&app, path).await;
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
            assert_eq!(retry_after.as_deref(), Some("1"));
        }
        open.send(true).unwrap();
        assert_eq!(first.await.unwrap().0, StatusCode::OK);
        // The permit is back.
        assert_eq!(call(&app, get_req("/b")).await.0, StatusCode::OK);
    }

    #[tokio::test]
    async fn broken_per_route_limit_admits_one_request_per_route() {
        // Same stack, but with `ConcurrencyLimitLayer`: `Router::layer`
        // applies it to each route, and each application creates its own
        // semaphore. A "limit of 1" admits one request on `/a` AND one on
        // `/b`.
        let (app, mut started, open) = two_slow_routes();
        let app = broken::shed_load_per_route(app, 1);
        let a = tokio::spawn({
            let app = app.clone();
            async move { call(&app, get_req("/a")).await }
        });
        assert_eq!(started.recv().await, Some("a"));
        let b = tokio::spawn({
            let app = app.clone();
            async move { call(&app, get_req("/b")).await }
        });
        // `/b` got into its handler while `/a` was still in flight.
        assert_eq!(started.recv().await, Some("b"));
        // A second request on `/a` is shed: the limit is per route.
        assert_eq!(
            call_shed(&app, "/a").await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        open.send(true).unwrap();
        assert_eq!(a.await.unwrap().0, StatusCode::OK);
        assert_eq!(b.await.unwrap().0, StatusCode::OK);
    }
}
