// Module 4 · Async bounds — part 3: `async fn` in a trait promises nothing about `Send` (E0277).
//
// Since Rust 1.75 a trait may declare an `async fn`. It is sugar for a method
// that returns an opaque future (return-position `impl Trait` in traits,
// "RPITIT"):
//
//     async fn get(&self, key: &str) -> Option<String>;
//     // means
//     fn get(&self, key: &str) -> impl Future<Output = Option<String>>;
//
// Every impl then returns its own anonymous future type. Call `get` on a
// CONCRETE store and rustc looks through the opaque type (auto traits leak
// through `impl Trait`) to see whether that impl's future is `Send`, so
// spawning it works when it is. Inside a generic function it cannot:
// `S::get(..)` stands for the future of ANY impl, including one that keeps an
// `Rc` or a `MutexGuard` across an `.await`. So `spawn` rejects the generic
// `spawn_fetch` below with "future cannot be sent between threads safely",
// because "the trait `Send` is not implemented for `impl
// std::future::Future<Output = Option<String>>`", and the tests' generic
// check fails with E0277 for the same reason. The bounds `S: Send + Sync`
// don't help: they describe the store, not the futures its methods return.
//
// The stable fix is to write the desugared form in the trait, with the bound
// added: `-> impl Future<Output = ..> + Send`. Impls may keep writing `async
// fn`; each one is checked against the promise. That is also the price: EVERY
// impl must now return a `Send` future. The error moves into any impl that
// breaks the promise (`CachedStore` below does), and an `async fn` impl on a
// type with a `RefCell` or an `Rc` inside no longer compiles: its future
// holds `&self`, which is `Send` only if `Self: Sync`. For a public trait the
// choice is permanent, since adding or removing the bound later breaks
// someone, which is why rustc's `async_fn_in_trait` lint warns about a plain
// `async fn` in a `pub trait`. The `trait-variant` crate generates both
// flavors (a local trait and a `Send` one) from one definition.
//
// The alternatives: bound the future at the USE site with return type
// notation (`where S::get(..): Send`), which is still unstable (E0658); or
// box the future (`#[async_trait]`), which costs an allocation per call but
// also makes the trait dyn compatible (part 4).
//
// How interviewers probe this: "How do you make a trait's `async fn` return a
// `Send` future?", "Why does spawning work with the concrete type but not in
// a generic function?", "What does adding `Send` to the trait cost?", "What is
// return type notation?".

use std::collections::HashMap;
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};

// ---- A tiny thread-per-task runtime (given) --------------------------------

// Runs `future` on a new OS thread and returns a handle to join it. The bounds
// are exactly `tokio::spawn`'s: the future moves to another thread (`Send`)
// and may outlive its caller (`'static`), and so does its output.
fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    thread::spawn(move || block_on(future))
}

// Polls `future` on the current thread until it is ready. The leaf future in
// this file wakes itself and is `Pending` for one poll only, so a plain poll
// loop is enough (a real executor sleeps until the waker fires, as in
// `29_async_runtime/runtime2`). The bound turns a future that never finishes
// into a panic instead of a hang.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..1_000 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
    panic!("block_on: the future was still pending after 1000 polls");
}

// Stands in for a network round trip: `Pending` on the first poll, `Ready` on
// the second (the `YieldOnce` future of `28_futures/futures2`).
struct Io {
    done: bool,
}

impl Future for Io {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            Poll::Ready(())
        } else {
            self.done = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn io() -> Io {
    Io { done: false }
}

// ---- The stores -------------------------------------------------------------

// `-> impl Future<..> + Send` is exactly what the `async fn` desugars to,
// plus the bound. Now every impl's future must be `Send` (impls may still be
// written as `async fn`), and generic code such as `spawn_fetch` may rely on
// it.
trait Store {
    fn get(&self, key: &str) -> impl Future<Output = Option<String>> + Send;
}

// A key-value store across the network: every lookup is a round trip.
struct RemoteStore {
    data: HashMap<String, String>,
    lookups: AtomicUsize,
}

impl RemoteStore {
    fn new(pairs: &[(&str, &str)]) -> Self {
        RemoteStore {
            data: pairs
                .iter()
                .map(|&(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            lookups: AtomicUsize::new(0),
        }
    }

    // How many lookups have reached the remote so far.
    fn lookups(&self) -> usize {
        self.lookups.load(Ordering::Relaxed)
    }
}

impl Store for RemoteStore {
    async fn get(&self, key: &str) -> Option<String> {
        io().await;
        self.lookups.fetch_add(1, Ordering::Relaxed);
        self.data.get(key).cloned()
    }
}

// A read-through cache in front of a `RemoteStore`: a key it has seen is
// answered locally, and anything else is fetched from the remote and, if the
// remote has it, remembered. Misses are not cached.
struct CachedStore {
    remote: RemoteStore,
    cache: Mutex<HashMap<String, String>>,
}

impl CachedStore {
    fn new(remote: RemoteStore) -> Self {
        CachedStore {
            remote,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl Store for CachedStore {
    // Each guard is a temporary that dies at the end of its statement, so the
    // cache is unlocked while the remote is awaited, and the future is `Send`
    // as the trait demands. Two tasks that miss at the same time may both ask
    // the remote; the second insert just stores the same value again.
    async fn get(&self, key: &str) -> Option<String> {
        let hit = self.cache.lock().unwrap().get(key).cloned();
        if hit.is_some() {
            return hit;
        }
        let value = self.remote.get(key).await?;
        self.cache
            .lock()
            .unwrap()
            .insert(key.to_string(), value.clone());
        Some(value)
    }
}

// Looks `key` up on a task of its own, for any store that can be shared
// across threads.
fn spawn_fetch<S>(store: &Arc<S>, key: &str) -> JoinHandle<Option<String>>
where
    S: Store + Send + Sync + 'static,
{
    let store = Arc::clone(store);
    let key = key.to_string();
    spawn(async move { store.get(&key).await })
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::ThreadId;

    fn assert_send<T: Send>(_: &T) {}

    // Generic on purpose: for a type parameter, only the trait's declaration
    // can say whether `get`'s future is `Send`.
    fn assert_get_is_send<S: Store + Sync>(store: &S) {
        assert_send(&store.get("any key"));
    }

    fn remote() -> RemoteStore {
        RemoteStore::new(&[("ada", "Lovelace"), ("grace", "Hopper")])
    }

    // A third store, written with `async fn` like the others. It records the
    // thread every lookup runs on and answers with the key in upper case.
    #[derive(Default)]
    struct Recorder {
        threads: Mutex<Vec<ThreadId>>,
    }

    impl Store for Recorder {
        async fn get(&self, key: &str) -> Option<String> {
            io().await;
            self.threads.lock().unwrap().push(thread::current().id());
            Some(key.to_uppercase())
        }
    }

    #[test]
    fn every_store_returns_send_futures() {
        assert_get_is_send(&remote());
        assert_get_is_send(&CachedStore::new(remote()));
        assert_get_is_send(&Recorder::default());
    }

    #[test]
    fn remote_store_hits_and_misses() {
        let store = Arc::new(remote());
        let ada = spawn_fetch(&store, "ada").join().unwrap();
        assert_eq!(ada.as_deref(), Some("Lovelace"));
        assert_eq!(spawn_fetch(&store, "nobody").join().unwrap(), None);
        assert_eq!(store.lookups(), 2);
    }

    #[test]
    fn fetches_run_on_spawned_threads() {
        let store = Arc::new(Recorder::default());
        let handles: Vec<_> = ["a", "b", "c"]
            .iter()
            .map(|key| spawn_fetch(&store, key))
            .collect();
        for (handle, expected) in handles.into_iter().zip(["A", "B", "C"]) {
            assert_eq!(handle.join().unwrap().as_deref(), Some(expected));
        }

        let threads = store.threads.lock().unwrap();
        assert_eq!(threads.len(), 3);
        assert!(threads.iter().all(|&id| id != thread::current().id()));
    }

    #[test]
    fn cached_store_answers_repeats_locally() {
        let store = Arc::new(CachedStore::new(remote()));
        for _ in 0..3 {
            let grace = spawn_fetch(&store, "grace").join().unwrap();
            assert_eq!(grace.as_deref(), Some("Hopper"));
        }
        // Only the first lookup went to the remote.
        assert_eq!(store.remote.lookups(), 1);
    }

    #[test]
    fn cached_store_does_not_cache_misses() {
        let store = Arc::new(CachedStore::new(remote()));
        assert_eq!(spawn_fetch(&store, "nobody").join().unwrap(), None);
        assert_eq!(spawn_fetch(&store, "nobody").join().unwrap(), None);
        assert_eq!(store.remote.lookups(), 2);
        assert!(store.cache.lock().unwrap().is_empty());
    }

    #[test]
    fn the_cache_is_unlocked_during_the_round_trip() {
        let store = CachedStore::new(remote());
        let mut cx = Context::from_waker(Waker::noop());
        let mut lookup = pin!(store.get("ada"));

        // The first poll stops inside the remote's round trip.
        assert!(lookup.as_mut().poll(&mut cx).is_pending());
        assert!(
            store.cache.try_lock().is_ok(),
            "the cache must not stay locked while the remote is awaited"
        );

        assert_eq!(
            lookup.as_mut().poll(&mut cx),
            Poll::Ready(Some("Lovelace".to_string()))
        );
        assert_eq!(
            store.cache.lock().unwrap().get("ada").map(String::as_str),
            Some("Lovelace")
        );
    }

    #[test]
    fn many_tasks_share_one_cached_store() {
        let store = Arc::new(CachedStore::new(remote()));
        let keys = ["ada", "grace", "nobody"];
        let handles: Vec<_> = (0..30).map(|i| spawn_fetch(&store, keys[i % 3])).collect();
        for (i, handle) in handles.into_iter().enumerate() {
            let expected = match keys[i % 3] {
                "ada" => Some("Lovelace"),
                "grace" => Some("Hopper"),
                _ => None,
            };
            assert_eq!(handle.join().unwrap().as_deref(), expected);
        }
        // Now both names are cached: asking again stays local.
        let before = store.remote.lookups();
        assert!(spawn_fetch(&store, "ada").join().unwrap().is_some());
        assert!(spawn_fetch(&store, "grace").join().unwrap().is_some());
        assert_eq!(store.remote.lookups(), before);
    }
}
