// Module 1 · Interior mutability — part 3: lazy globals with `OnceLock` and `LazyLock` (E0277, E0015).
//
// A `static` is ONE value for the whole program, and every thread can reach
// it. Two rules follow from that, and each one rejects part of this file:
//
//   1. A static's type must be `Sync`, because any thread may take a `&` to
//      it. `Cell`, `RefCell` and part 2's `OnceCell` are not `Sync`, so
//      `static GREETING: OnceCell<String>` is E0277 "`OnceCell<String>` cannot
//      be shared between threads safely". Each single-threaded cell has a
//      thread-safe twin: `OnceLock` for `OnceCell`, `Mutex` / `RwLock` for
//      `RefCell`, and the atomics for `Cell`.
//   2. A static's initializer is evaluated at COMPILE time, and the result is
//      baked into the binary. So it may only call `const fn`s. `Mutex::new`,
//      `Vec::new`, `String::new` and `AtomicU32::new` are `const`. Anything
//      that allocates, reads a file or seeds a hasher is not, and that
//      includes `HashMap::new` (its `RandomState` gets random keys at run
//      time). Calling one is E0015 "cannot call non-const function `load` in
//      statics".
//
// The answer to both is a LAZY global, and std has two:
//
//   - `OnceLock<T>` (Rust 1.70) starts empty and is filled at most once, at
//     run time, by whoever has the value: `set(v)` (a second `set` fails and
//     hands `v` back as `Err(v)`), `get()` returns `Option<&T>`, and
//     `get_or_init(f)` fills it if needed. Use it when the value comes from
//     OUTSIDE the static, like a command-line flag that `main` parses.
//   - `LazyLock<T>` (Rust 1.80) stores its initializer next to the value.
//     The first access (through `Deref`) runs it, on whichever thread gets
//     there first. Threads that arrive while it runs wait for it, and it runs
//     exactly once. Use it when the static knows how to build its own value.
//     If the initializer panics, the `LazyLock` is poisoned for good: every
//     later access panics too.
//
// Together they replace the `lazy_static!` macro and the `once_cell` crate.
// Once initialized, the value never changes again, so both hand out plain
// `&'static T` borrows: no lock, no guard. A global that DOES change after
// startup needs a lock around it (`Mutex::new` is `const`, so
// `static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());` works as is).
//
// rustc gives away half of the answer: the E0277 note suggests `OnceLock`,
// and the E0015 note suggests `LazyLock::new(|| ...)`. The interview question
// is WHY, and why the tempting alternatives are wrong:
//
//   - `static mut`: every read or write of one is `unsafe` (E0133), because
//     nothing stops two threads from racing on it, and taking a reference to
//     one trips the `static_mut_refs` lint, deny-by-default in edition 2024.
//     (This course forbids `unsafe` anyway.)
//   - `const`: a `const` is not a place, it is pasted in FRESH at every use.
//     `const CONFIG: LazyLock<Config>` compiles, but every use builds a new
//     `LazyLock` and loads the config again (clippy warns with
//     `declare_interior_mutable_const`).
//   - `thread_local!`: one value PER THREAD, which is part 4's topic, not a
//     global.
//
// How interviewers probe this: "You need a lazily initialized global config
// read by many threads. What do you use?", "Why can't a static hold a
// `RefCell`?", "Why not `static mut`?", "`OnceLock` or `LazyLock`?", "What
// happens when two threads touch an uninitialized `LazyLock` at once, and
// what if its initializer panics?".

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{LazyLock, OnceLock};

// ---- Part A — a value that is set once, at run time ------------------------

// The greeting comes from the command line, so nobody knows it before `main`
// runs. Any thread may read it afterwards.
//
// `OnceLock` is the `Sync` twin of `OnceCell`: a thread that finds it empty
// and a thread that is filling it are synchronized, so no reader can see a
// half-written `String`. It starts empty (`OnceLock::new` is `const`), and
// `set` / `get` behave like `OnceCell`'s, so the two functions below compile
// unchanged. The value is never replaced afterwards, which is what makes
// `&'static str` borrows of it sound.
static GREETING: OnceLock<String> = OnceLock::new();

// Stores the greeting. Only the first call succeeds: later calls hand their
// value back as `Err(greeting)`.
fn init_greeting(greeting: String) -> Result<(), String> {
    GREETING.set(greeting)
}

// The greeting, if `init_greeting` has run.
fn greeting() -> Option<&'static str> {
    GREETING.get().map(String::as_str)
}

// ---- Part B — a value that is computed on first use ------------------------

#[derive(Debug, PartialEq)]
struct Config {
    name: String,
    workers: usize,
}

// How many times `load` has run. An `AtomicU32` is a `u32` that many threads
// can update without a lock (`36_atomics`), and `Relaxed` is enough for a
// counter that guards no other data.
static LOADS: AtomicU32 = AtomicU32::new(0);

// Parses the program's config. Real code would read a file here. Don't
// change it: the tests count its calls.
fn load() -> Config {
    LOADS.fetch_add(1, Ordering::Relaxed);
    let text = "name = inventory\nworkers = 4\n";
    let mut config = Config {
        name: String::new(),
        workers: 1,
    };
    for line in text.lines() {
        match line.split_once(" = ") {
            Some(("name", value)) => config.name = value.to_string(),
            Some(("workers", value)) => config.workers = value.parse().expect("not a number"),
            _ => {}
        }
    }
    config
}

// `LazyLock::new` is `const`: it only stores the function pointer. `load`
// runs on the first dereference, on whichever thread gets there first. Racing
// threads block until it has finished, so it runs exactly once, and they all
// get a `&'static Config` to the one value stored in the static.
static CONFIG: LazyLock<Config> = LazyLock::new(load);

// ---- Part C — a global lookup table ----------------------------------------

// The same pattern: a non-capturing closure coerces to the `fn() -> T` that
// `LazyLock<T>` stores by default. `Deref` makes `PORTS["https"]` and
// `PORTS.get(..)` work on the `HashMap` inside.
static PORTS: LazyLock<HashMap<&str, u16>> =
    LazyLock::new(|| HashMap::from([("http", 80), ("https", 443), ("ssh", 22)]));

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::sync::Barrier;
    use std::thread;

    // ---- Part A ----

    // Statics are shared by every test in this process, and the tests run in
    // parallel. So this is the ONLY test that touches `GREETING`.
    #[test]
    fn greeting_is_set_once_and_seen_by_every_thread() {
        assert_eq!(greeting(), None, "nothing was set yet");

        let hello = String::from("hello");
        let heap = hello.as_ptr();
        assert_eq!(init_greeting(hello), Ok(()));
        assert_eq!(greeting(), Some("hello"));
        assert_eq!(
            greeting().map(str::as_ptr),
            Some(heap),
            "the stored greeting must be the `String` that was passed in"
        );

        let again = String::from("again");
        let rejected = again.as_ptr();
        let err = init_greeting(again).expect_err("only the first call may set it");
        assert_eq!(err, "again");
        assert_eq!(err.as_ptr(), rejected, "hand the SAME `String` back");
        assert_eq!(greeting(), Some("hello"), "the first greeting stays");

        // A greeting set on this thread is visible on any other thread.
        let seen = thread::spawn(greeting).join().unwrap();
        assert_eq!(seen, Some("hello"));
    }

    // ---- Part B ----

    #[test]
    fn config_is_loaded_at_run_time() {
        assert_eq!(CONFIG.name, "inventory");
        assert_eq!(CONFIG.workers, 4);
        assert_eq!(
            LOADS.load(Ordering::Relaxed),
            1,
            "`load` must run exactly once"
        );
    }

    #[test]
    fn eight_threads_race_for_the_config_and_it_loads_once() {
        // The barrier releases all 8 threads at once, so (unless another test
        // got there first) they all find `CONFIG` uninitialized together.
        let start = Barrier::new(8);
        let seen: Vec<&'static Config> = thread::scope(|s| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        start.wait();
                        let config: &'static Config = &CONFIG;
                        config
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert_eq!(
            LOADS.load(Ordering::Relaxed),
            1,
            "`load` must run exactly once, however many threads race for it"
        );
        for config in &seen {
            assert!(
                ptr::eq(*config, seen[0]),
                "every thread must see the SAME config"
            );
        }
        assert_eq!(seen[0].workers, 4);
    }

    // ---- Part C ----

    #[test]
    fn ports_table_answers_lookups() {
        assert_eq!(PORTS["https"], 443);
        assert_eq!(PORTS["ssh"], 22);
        assert_eq!(PORTS.get("http"), Some(&80));
        assert_eq!(PORTS.get("gopher"), None);
        assert_eq!(PORTS.len(), 3);
    }

    #[test]
    fn every_thread_sees_the_same_table() {
        let here: &'static HashMap<&str, u16> = &PORTS;
        let there = thread::spawn(|| {
            let table: &'static HashMap<&str, u16> = &PORTS;
            table
        })
        .join()
        .unwrap();
        assert!(ptr::eq(here, there), "the table must be built only once");
    }
}
