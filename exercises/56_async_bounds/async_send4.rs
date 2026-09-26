// Module 4 · Async bounds — part 4: a dyn-compatible async trait returns a boxed `Send` future (E0038).
//
// Part 3's `Store` is right for generics and useless for `dyn`. A trait
// object calls its methods through a vtable, and a vtable entry needs ONE
// signature with ONE return type. But every impl of an `async fn` (or of any
// `-> impl Trait` method) returns its own anonymous future type, of its own
// size. So a trait with an `async fn` is not dyn compatible: E0038 "the trait
// `DynStore` is not dyn compatible ... because method `get` is `async`".
//
// The classic fix, and in essence what the `async-trait` crate's
// `#[async_trait]` expands to, is to erase the future's type as well: return
// a trait object on the heap,
//
//     Pin<Box<dyn Future<Output = T> + Send + 'a>>
//
// usually behind an alias (`futures::future::BoxFuture<'a, T>` is this exact
// type). Every piece has a job:
//
//   - `Box<dyn Future ..>`: one pointer-sized return type for every impl, so
//     the method fits in a vtable.
//   - `Pin`: `poll` takes `Pin<&mut Self>`, and a `Pin<Box<_>>` provides it
//     with no `unsafe` (`Box::pin`; see `29_async_runtime/runtime4`).
//   - `+ Send`: a `dyn` type has only the auto traits written into it
//     (`35_error_design/err5`), and a spawned caller must know the future is
//     `Send`, as in part 3.
//   - `+ 'a`: the future may borrow `self` and `key` for `'a`. Without it the
//     boxed trait object means `+ 'static` (`25_lifetimes_deep/lifetimes8`),
//     and a future that borrows `self` no longer fits.
//
// Each impl moves its old body into `Box::pin(async move { .. })`. The async
// block captures the references and does all the work when it is polled, so
// the future stays lazy: calling `get` still does nothing by itself.
//
// The trait object itself has to cross threads too: `fetch_all` moves an
// `Arc<dyn DynStore>` into every task, and `Arc<T>` is `Send` only when `T` is
// `Send + Sync`. Supertraits put those bounds on every `dyn DynStore` (and
// require them of every impl).
//
// The price is one heap allocation per call and a dynamic call on every
// poll. So a common design keeps part 3's zero-cost `Store` for generic code
// and adds a boxed, dyn-compatible twin next to it: a second trait with a
// blanket impl for every `Store`, or a wrapper struct that the `dynosaur`
// crate generates from `Store` (it would call it `DynStore` too), which
// implements `Store` itself and boxes each future.
//
// How interviewers probe this: "Why can't I write `Box<dyn Store>` when
// `Store` has an `async fn`?", "What does `#[async_trait]` expand to?", "What
// does the boxing cost, and when is it worth it?", "Why `Send + Sync` on the
// trait?".

use std::collections::HashMap;
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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

// TODO: Every `dyn DynStore` in this file is rejected with E0038 "the trait
// `DynStore` is not dyn compatible ... because method `get` is `async`": each
// impl's `async fn` returns a future of its own type, and a vtable needs one.
// Give `get` a single, type-erased return type that a vtable can hold, spelled
// as a `BoxFuture<'a, T>` alias that you define. The future may keep
// borrowing `self` and `key` while it runs, which is what the lifetime
// parameter is for. Then think threads: `fetch_all` and the tests send an
// `Arc<dyn DynStore>` and the futures of `get` to other threads, so both must
// be thread-safe through the trait alone (the tests write plain
// `Arc<dyn DynStore>`, with no `+ Send + Sync`). Until you make `DynStore`
// dyn compatible and thread-safe, this exercise will not compile.
trait DynStore {
    async fn get(&self, key: &str) -> Option<String>;
}

// A key-value store behind a (simulated) network hop. It counts the lookups
// it has actually performed.
struct MemStore {
    data: HashMap<String, String>,
    lookups: AtomicUsize,
}

impl MemStore {
    fn new(pairs: &[(&str, &str)]) -> Self {
        MemStore {
            data: pairs
                .iter()
                .map(|&(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            lookups: AtomicUsize::new(0),
        }
    }

    fn lookups(&self) -> usize {
        self.lookups.load(Ordering::Relaxed)
    }
}

impl DynStore for MemStore {
    // TODO: Rewrite this impl for the new signature. The body must still run
    // when the future is polled, round trip first and lookup second, not when
    // `get` is called (the tests check both). Until you make it match the
    // trait, this exercise will not compile.
    async fn get(&self, key: &str) -> Option<String> {
        io().await;
        self.lookups.fetch_add(1, Ordering::Relaxed);
        self.data.get(key).cloned()
    }
}

// Asks `primary` first and falls back to `backup` on a miss. Either side can
// be any store, including another `Fallback`.
struct Fallback {
    primary: Arc<dyn DynStore>,
    backup: Arc<dyn DynStore>,
}

impl DynStore for Fallback {
    // TODO: Rewrite this impl for the new signature too. The backup is asked
    // only when the primary has no value. Until you make it match the trait,
    // this exercise will not compile.
    async fn get(&self, key: &str) -> Option<String> {
        match self.primary.get(key).await {
            Some(value) => Some(value),
            None => self.backup.get(key).await,
        }
    }
}

// Looks `key` up in every store at once, one task per store, and returns the
// answers in the order of `stores`.
fn fetch_all(stores: &[Arc<dyn DynStore>], key: &str) -> Vec<Option<String>> {
    let handles: Vec<_> = stores
        .iter()
        .map(|store| {
            let store = Arc::clone(store);
            let key = key.to_string();
            spawn(async move { store.get(&key).await })
        })
        .collect();
    handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>(_: &T) {}
    fn assert_send_sync<T: Send + Sync>() {}

    fn people() -> MemStore {
        MemStore::new(&[("ada", "Lovelace"), ("grace", "Hopper")])
    }

    fn languages() -> MemStore {
        MemStore::new(&[("ada", "Ada 83"), ("rust", "Rust 2024")])
    }

    fn some(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    #[test]
    fn dyn_stores_and_their_futures_can_cross_threads() {
        // Exactly the types the rest of the tests use: no extra bounds here.
        assert_send_sync::<Arc<dyn DynStore>>();
        let store: Arc<dyn DynStore> = Arc::new(people());
        assert_send(&store.get("ada"));
    }

    #[test]
    fn one_vec_holds_different_store_types() {
        let people: Arc<dyn DynStore> = Arc::new(people());
        let languages: Arc<dyn DynStore> = Arc::new(languages());
        let both: Arc<dyn DynStore> = Arc::new(Fallback {
            primary: Arc::clone(&languages),
            backup: Arc::clone(&people),
        });
        let stores = vec![people, languages, both];

        assert_eq!(
            fetch_all(&stores, "ada"),
            [some("Lovelace"), some("Ada 83"), some("Ada 83")]
        );
        assert_eq!(
            fetch_all(&stores, "grace"),
            [some("Hopper"), None, some("Hopper")]
        );
        assert_eq!(
            fetch_all(&stores, "rust"),
            [None, some("Rust 2024"), some("Rust 2024")]
        );
        assert_eq!(fetch_all(&stores, "nobody"), [None, None, None]);
    }

    #[test]
    fn every_store_is_fetched_on_a_thread_of_its_own() {
        let stores: Vec<Arc<dyn DynStore>> = vec![Arc::new(people()), Arc::new(languages())];
        let handles: Vec<_> = stores
            .iter()
            .map(|store| {
                let store = Arc::clone(store);
                spawn(async move {
                    let answer = store.get("ada").await;
                    (thread::current().id(), answer)
                })
            })
            .collect();
        let (threads, answers): (Vec<_>, Vec<_>) = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .unzip();
        assert_eq!(answers, [some("Lovelace"), some("Ada 83")]);
        // Each store and its future really crossed to another thread.
        assert_ne!(threads[0], threads[1]);
        assert!(threads.iter().all(|&id| id != thread::current().id()));
    }

    #[test]
    fn get_does_its_work_only_when_polled() {
        let store = people();
        let as_dyn: &dyn DynStore = &store;
        let mut cx = Context::from_waker(Waker::noop());

        let mut lookup = pin!(as_dyn.get("grace"));
        assert_eq!(
            store.lookups(),
            0,
            "calling `get` must not look anything up"
        );
        // The first poll stops in the network hop, before the lookup.
        assert!(lookup.as_mut().poll(&mut cx).is_pending());
        assert_eq!(store.lookups(), 0, "the lookup comes after the round trip");
        assert_eq!(lookup.as_mut().poll(&mut cx), Poll::Ready(some("Hopper")));
        assert_eq!(store.lookups(), 1);
    }

    #[test]
    fn fallback_asks_the_backup_only_on_a_miss() {
        let primary = Arc::new(people());
        let backup = Arc::new(MemStore::new(&[("ada", "backup"), ("linus", "Torvalds")]));
        let store: Arc<dyn DynStore> = Arc::new(Fallback {
            primary: primary.clone(),
            backup: backup.clone(),
        });

        assert_eq!(block_on(store.get("ada")), some("Lovelace"));
        assert_eq!((primary.lookups(), backup.lookups()), (1, 0));
        assert_eq!(block_on(store.get("linus")), some("Torvalds"));
        assert_eq!((primary.lookups(), backup.lookups()), (2, 1));
        assert_eq!(block_on(store.get("nobody")), None);
        assert_eq!((primary.lookups(), backup.lookups()), (3, 2));
    }

    #[test]
    fn fallbacks_nest() {
        let inner: Arc<dyn DynStore> = Arc::new(Fallback {
            primary: Arc::new(people()),
            backup: Arc::new(languages()),
        });
        let outer: Arc<dyn DynStore> = Arc::new(Fallback {
            primary: inner,
            backup: Arc::new(MemStore::new(&[("ferris", "crab")])),
        });
        let stores = [outer];
        assert_eq!(fetch_all(&stores, "rust"), [some("Rust 2024")]);
        assert_eq!(fetch_all(&stores, "ferris"), [some("crab")]);
        assert_eq!(fetch_all(&stores, "nobody"), [None]);
    }

    #[test]
    fn a_short_lived_key_is_enough() {
        let store: Arc<dyn DynStore> = Arc::new(people());
        let answer = {
            let key = String::from("ada");
            block_on(store.get(&key))
        };
        assert_eq!(answer, some("Lovelace"));
    }

    #[test]
    fn no_stores_no_answers() {
        assert!(fetch_all(&[], "ada").is_empty());
    }
}
