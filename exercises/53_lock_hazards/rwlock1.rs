// Module 4 · Lock hazards — part 2: `RwLock` lets readers share, and has no upgrade from read to write.
//
// A `Mutex` lets one thread in at a time, readers included. For data that is
// read all the time and written rarely (configuration, routing tables,
// feature flags), readers then queue up behind each other for nothing: two
// readers never conflict. `std::sync::RwLock<T>` is the thread-safe, blocking
// version of `RefCell`'s rule: any number of `read()` guards (`&T`) at once,
// or exactly one `write()` guard (`&mut T`).
//
// It is not free. Every `read()` still updates the lock's shared state with
// an atomic read-modify-write: an uncontended `read()` costs about as much
// as locking a `Mutex`, and readers on many cores still fight over that one
// cache line. For short critical sections a `Mutex` is often just as fast;
// measure. Its thread-safety bound is stricter, too. `Mutex<T>` is `Sync`
// when `T: Send`, since only one thread at a time ever reaches the `T`.
// `RwLock<T>` also needs `T: Sync`, since readers on several threads hold
// `&T` at the same time. So `Mutex<Cell<i32>>` can be shared between
// threads, and `RwLock<Cell<i32>>` cannot (E0277 "`Cell<i32>` cannot be
// shared between threads safely").
//
// Who goes first when readers and writers both wait is up to the platform.
// std's docs say that a waiting writer "might or might not block concurrent
// calls to `read`". Today std's own lock on Linux, Windows and macOS makes
// new readers wait behind a queued writer, so a stream of readers cannot
// starve writers. The price: a thread that holds a read guard and calls
// `read()` again deadlocks if a writer queued up in between. Never lock the
// same `RwLock` twice on one thread, not even for reading.
//
// The interview question hides in `get_or_insert_with`: look a key up, and
// insert a default on a miss. A hit only reads, so it should need only the
// shared lock. A miss must write, but std has NO upgradable read: a read
// guard cannot become a write guard. Why not? If two readers could both ask
// to upgrade, each would wait for the other to release its read lock, and
// neither ever would. (`parking_lot` has `upgradable_read`, and allows only
// one such guard at a time, for exactly that reason.) std does offer the
// other direction: `RwLockWriteGuard::downgrade` (Rust 1.92) turns a write
// guard into a read guard atomically. And a `write()` call while this
// thread's own read guard is alive never succeeds: a writer waits for every
// reader to leave, including you, so the thread deadlocks (std's docs allow
// a panic instead).
//
// So a miss drops the read guard and takes the write lock from scratch. In
// between, other threads run, and one of them may have inserted the same key
// already: what you saw under the read lock is stale the moment you let go of
// it. That is why you look again under the write lock (double-checked
// locking). Here `make` is cheap, so it runs under the write lock and builds
// each key's value at most once. If it were slow (I/O), you would build the
// value with no lock held and keep whichever value was stored first
// (`31_debugging/debugging8` drills that version), or give each key its own
// `OnceLock`.
//
// The tests catch serialized readers with a `Barrier`: four readers each wait
// INSIDE `with_read` until all four are inside. With a `Mutex`, the first one
// waits there forever while it holds the lock, and a watchdog reports
// "readers were serialized". Another test checks the other half of the
// contract: while a reader is inside `with_read`, `set` must wait. The
// watchdogs never fire for a correct store, and a stuck helper thread dies
// when the test binary exits.
//
// Interviewers ask when an `RwLock` beats a `Mutex` (long reads, few writes)
// and when it does not, why there is no upgrade, and what goes wrong with a
// "read, then write" that forgets to look again.

use std::collections::HashMap;
use std::sync::Mutex;

/// Settings shared by every request handler of a server: read on every
/// request, written rarely (a reload, a flag flipped by an operator).
#[derive(Default)]
struct ConfigStore {
    // TODO: `readers_run_at_the_same_time` fails with "readers were
    // serialized": a `Mutex` lets one thread in at a time, so the test's four
    // readers enter `with_read` one by one, and the first one waits inside
    // for the other three forever (after the 10-second watchdog, the test
    // gives up). `a_hit_does_not_wait_for_readers` fails for the same reason.
    // Requirements:
    //   - any number of threads can be inside `with_read` at the same time,
    //     and `set` waits until all of them have left, then changes the map
    //     with nobody else inside (`a_write_waits_for_readers_to_leave`);
    //   - `with_read` lends `f` the map inside the store, not a copy or a
    //     snapshot of it (a test compares heap pointers);
    //   - std only, no `unsafe`, and don't change the tests. `Default` builds
    //     the store, so no constructor needs to change.
    // Changing this field is the start: rustc then points at every line that
    // still treats it as a `Mutex`. Until readers can share the lock, the
    // tests will fail.
    map: Mutex<HashMap<String, String>>,
}

impl ConfigStore {
    /// Runs `f` with read access to the whole map and returns its result.
    /// Readers can be slow (they render a page, or check a request against
    /// many settings), and many requests read at the same time. While `f`
    /// runs, writers wait: `set` returns only after every reader has left.
    fn with_read<R>(&self, f: impl FnOnce(&HashMap<String, String>) -> R) -> R {
        // TODO: Every caller of `with_read` only reads, so they must all be
        // able to be inside at once. Until they can, the tests will fail.
        let map = self.map.lock().unwrap();
        f(&map)
    }

    /// A copy of the value stored under `key`.
    fn get(&self, key: &str) -> Option<String> {
        self.with_read(|map| map.get(key).cloned())
    }

    /// Stores `value` under `key` and returns the value it replaced.
    fn set(&self, key: &str, value: String) -> Option<String> {
        // TODO: `set` changes the map, so nobody else may be inside while it
        // does. Until the store's lock tells readers and writers apart, the
        // tests will fail.
        self.map.lock().unwrap().insert(key.to_owned(), value)
    }

    /// The value stored under `key`. On a miss, `make()` builds the value
    /// and it is stored first. `make` is cheap, but it must run at most ONCE
    /// per key, even when many threads miss the same key at the same moment,
    /// and never on a hit.
    fn get_or_insert_with(&self, key: &str, make: impl FnOnce() -> String) -> String {
        // TODO: `a_hit_does_not_wait_for_readers` fails: this takes the map
        // exclusively even when `key` is already stored, so a hit waits until
        // every reader has left. Taking the exclusive side of a reader-writer
        // lock here fails the same way. Requirements:
        //   - a hit needs only shared access, and returns the stored value
        //     without calling `make`;
        //   - a miss stores `make()` and returns it, and `make` runs at most
        //     once per key even when several threads miss the same key at the
        //     same moment (`racing_misses_run_make_once` races four threads
        //     onto 1_000 fresh keys), so every caller gets the one stored
        //     value;
        //   - this thread must never wait for a lock it still holds itself
        //     (the tests report that as "deadlock detected");
        //   - no copying the map, no `unsafe`, and don't change the tests.
        // Until a hit only reads and a miss still runs `make` once, the tests
        // will fail.
        self.map
            .lock()
            .unwrap()
            .entry(key.to_owned())
            .or_insert_with(make)
            .clone()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    // The time limit for anything that might block forever. It is far beyond
    // what a correct call needs, even on a machine busy with other work, so
    // only a stuck call ever reaches it.
    const WATCHDOG: Duration = Duration::from_secs(10);

    /// Runs `f` on a helper thread. The result arrives on the returned
    /// channel.
    fn spawn<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Receiver<T> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(f());
        });
        rx
    }

    /// Runs `f` on a helper thread and waits at most `WATCHDOG` for it.
    fn watched<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
        match spawn(f).recv_timeout(WATCHDOG) {
            Ok(value) => value,
            Err(RecvTimeoutError::Timeout) => {
                panic!("deadlock detected: {what} did not return within {WATCHDOG:?}")
            }
            Err(RecvTimeoutError::Disconnected) => {
                panic!("{what} panicked (see the message above)")
            }
        }
    }

    fn store_with(entries: &[(&str, &str)]) -> Arc<ConfigStore> {
        let store = ConfigStore::default();
        for &(key, value) in entries {
            store.set(key, value.to_owned());
        }
        Arc::new(store)
    }

    #[test]
    fn readers_run_at_the_same_time() {
        const READERS: usize = 4;
        let store = store_with(&[("mode", "fast")]);
        // Every reader waits INSIDE `with_read` until all four are inside.
        // That can only happen if `with_read` lets them in together.
        let inside = Arc::new(Barrier::new(READERS));
        let (tx, rx) = mpsc::channel();
        for _ in 0..READERS {
            let (store, inside, tx) = (Arc::clone(&store), Arc::clone(&inside), tx.clone());
            thread::spawn(move || {
                let mode = store.with_read(|map| {
                    inside.wait();
                    map.get("mode").cloned()
                });
                let _ = tx.send(mode);
            });
        }
        drop(tx);
        for _ in 0..READERS {
            match rx.recv_timeout(WATCHDOG) {
                Ok(mode) => assert_eq!(mode.as_deref(), Some("fast")),
                Err(RecvTimeoutError::Timeout) => panic!(
                    "readers were serialized: the {READERS} readers never were inside \
                     `with_read` at the same time, so the first one waits for the others \
                     forever"
                ),
                Err(RecvTimeoutError::Disconnected) => panic!("a reader panicked"),
            }
        }
    }

    #[test]
    fn with_read_lends_the_stored_values() {
        let store = ConfigStore::default();
        let value = String::from("dark");
        let heap = value.as_ptr().addr();
        assert_eq!(store.set("theme", value), None);
        // `f` must see the map inside the store, not a copy of it: copying
        // every value on every read defeats the point of a shared lock.
        let seen = store.with_read(|map| map["theme"].as_ptr().addr());
        assert_eq!(seen, heap, "`with_read` handed `f` a copy of the map");
        assert_eq!(store.get("theme").as_deref(), Some("dark"));
        assert_eq!(store.get("missing"), None);
    }

    #[test]
    fn a_write_is_visible_to_later_reads() {
        let store = store_with(&[("mode", "safe")]);
        let writer = Arc::clone(&store);
        let replaced = watched("`set` on another thread", move || {
            writer.set("mode", String::from("fast"))
        });
        assert_eq!(replaced.as_deref(), Some("safe"));
        assert_eq!(store.get("mode").as_deref(), Some("fast"));
        assert_eq!(store.with_read(|map| map.len()), 1);
    }

    // How long `a_write_waits_for_readers_to_leave` gives `set` to return
    // while a reader is still inside. A correct `set` cannot return then, so
    // the window only decides how surely a wrong one is caught.
    const READER_WINDOW: Duration = Duration::from_millis(200);

    #[test]
    fn a_write_waits_for_readers_to_leave() {
        let store = store_with(&[("mode", "safe")]);
        // This thread stays inside `with_read` while another thread calls
        // `set`. A writer needs the map to itself, so `set` must not return
        // before this reader has left. (Nothing in here calls `read()` again:
        // with a writer waiting, that may deadlock.)
        let (early, writer) = store.with_read(|_| {
            let other = Arc::clone(&store);
            let writer = spawn(move || other.set("mode", String::from("fast")));
            (writer.recv_timeout(READER_WINDOW), writer)
        });
        match early {
            Err(RecvTimeoutError::Timeout) => {}
            Ok(_) => panic!(
                "`set` returned while a reader was still inside `with_read`: a writer must \
                 wait until every reader has left, and `f` must see the map inside the store, \
                 not a snapshot"
            ),
            Err(RecvTimeoutError::Disconnected) => panic!("`set` panicked (see the message above)"),
        }
        // The reader has left, so the writer can finish now.
        match writer.recv_timeout(WATCHDOG) {
            Ok(replaced) => assert_eq!(replaced.as_deref(), Some("safe")),
            Err(RecvTimeoutError::Timeout) => panic!(
                "deadlock detected: `set` did not return within {WATCHDOG:?} after the reader left"
            ),
            Err(RecvTimeoutError::Disconnected) => panic!("`set` panicked (see the message above)"),
        }
        assert_eq!(store.get("mode").as_deref(), Some("fast"));
    }

    #[test]
    fn make_runs_only_on_a_miss() {
        let store = store_with(&[("region", "eu")]);
        let makes = Arc::new(AtomicUsize::new(0));
        let get_or_insert = |key: &'static str| {
            let (store, makes) = (Arc::clone(&store), Arc::clone(&makes));
            watched("`get_or_insert_with`", move || {
                store.get_or_insert_with(key, || {
                    makes.fetch_add(1, Ordering::Relaxed);
                    format!("default {key}")
                })
            })
        };
        // A key that `set` stored is a hit.
        assert_eq!(get_or_insert("region"), "eu");
        assert_eq!(makes.load(Ordering::Relaxed), 0, "`make` ran on a hit");
        // A miss builds the value and stores it...
        assert_eq!(get_or_insert("retries"), "default retries");
        assert_eq!(makes.load(Ordering::Relaxed), 1);
        assert_eq!(store.get("retries").as_deref(), Some("default retries"));
        // ...so the next call is a hit.
        assert_eq!(get_or_insert("retries"), "default retries");
        assert_eq!(makes.load(Ordering::Relaxed), 1, "`make` ran on a hit");
    }

    #[test]
    fn a_hit_does_not_wait_for_readers() {
        let store = store_with(&[("region", "eu")]);
        // This thread stays inside `with_read` while another thread looks up
        // a key that is already stored. A hit only reads, so it must not have
        // to wait for this reader to finish.
        let verdict = store.with_read(|_| {
            let other = Arc::clone(&store);
            let rx = spawn(move || other.get_or_insert_with("region", || unreachable!()));
            match rx.recv_timeout(WATCHDOG) {
                Ok(value) => Ok(value),
                Err(RecvTimeoutError::Timeout) => Err(
                    "a hit in `get_or_insert_with` waited for a reader to finish: it asked \
                     for exclusive access although it only needed to read",
                ),
                Err(RecvTimeoutError::Disconnected) => Err("`get_or_insert_with` panicked"),
            }
        });
        // The reader has left now, so a hit that waited for it is done too.
        match verdict {
            Ok(value) => assert_eq!(value, "eu"),
            Err(why) => panic!("{why}"),
        }
    }

    #[test]
    fn racing_misses_run_make_once() {
        const THREADS: usize = 4;
        const KEYS: usize = 1_000;
        let store = Arc::new(ConfigStore::default());
        let makes: Arc<Vec<AtomicUsize>> =
            Arc::new((0..KEYS).map(|_| AtomicUsize::new(0)).collect());
        // A start line for every key: each thread spins until all of them
        // have arrived, so that they all miss the key at about the same time.
        let arrived = Arc::new(AtomicUsize::new(0));
        let progress = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        for thread_id in 0..THREADS {
            let (store, makes, arrived, progress, tx) = (
                Arc::clone(&store),
                Arc::clone(&makes),
                Arc::clone(&arrived),
                Arc::clone(&progress),
                tx.clone(),
            );
            thread::spawn(move || {
                let mut got = Vec::with_capacity(KEYS);
                for (key, make_count) in makes.iter().enumerate() {
                    arrived.fetch_add(1, Ordering::SeqCst);
                    let started = Instant::now();
                    let mut spins = 0u32;
                    while arrived.load(Ordering::SeqCst) < (key + 1) * THREADS {
                        spins = spins.wrapping_add(1);
                        if spins.is_multiple_of(1_024) {
                            if started.elapsed() > WATCHDOG {
                                return; // Another thread is stuck: give up.
                            }
                            thread::yield_now();
                        } else {
                            std::hint::spin_loop();
                        }
                    }
                    got.push(store.get_or_insert_with(&key.to_string(), || {
                        make_count.fetch_add(1, Ordering::SeqCst);
                        format!("key {key}, made by thread {thread_id}")
                    }));
                    progress.fetch_add(1, Ordering::Relaxed);
                }
                let _ = tx.send(got);
            });
        }
        drop(tx);

        // Wait for every thread, failing once no call at all has returned
        // for `WATCHDOG`.
        let mut results: Vec<Vec<String>> = Vec::new();
        let mut seen = 0;
        let mut last_progress = Instant::now();
        while results.len() < THREADS {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(got) => results.push(got),
                Err(RecvTimeoutError::Timeout) => {
                    let now = progress.load(Ordering::Relaxed);
                    if now != seen {
                        seen = now;
                        last_progress = Instant::now();
                    } else if last_progress.elapsed() > WATCHDOG {
                        panic!(
                            "deadlock detected: no `get_or_insert_with` returned for {WATCHDOG:?}"
                        );
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("a racing thread panicked or got stuck (see the message above)")
                }
            }
        }

        for (key, make_count) in makes.iter().enumerate() {
            assert_eq!(
                make_count.load(Ordering::SeqCst),
                1,
                "`make` ran more than once for key {key}: two threads missed it, and the \
                 second one did not notice that the first had stored a value meanwhile"
            );
            let first = &results[0][key];
            for got in &results[1..] {
                assert_eq!(
                    &got[key], first,
                    "two threads got different values for key {key}"
                );
            }
        }
        assert_eq!(store.with_read(|map| map.len()), KEYS);
    }
}
