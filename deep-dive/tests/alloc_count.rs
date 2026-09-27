//! Lab · a counting `GlobalAlloc`: predict every allocation, then prove it
//!
//! "How many allocations does this make?" is a guessing game until something
//! counts them. This test binary replaces the global allocator with one that
//! forwards every call to `System` and counts it. Then it runs a table of
//! cases: each case states a prediction (allocations, reallocations, frees),
//! runs a snippet inside a counting window, and prints what it measured.
//! `main` exits non-zero if any measurement disagrees with its prediction.
//! The shipped predictions are the right answers; "Try it" below turns them
//! back into a quiz.
//!
//! The cases, grouped by the interview question they answer:
//!
//! - *Does an empty collection allocate? A `Box<()>`?* No. (Two cases: the
//!   trivia is kept to a minority.)
//! - *`Rc<str>` or `Rc<String>`?* One allocation, holding the counts and the
//!   bytes together, against two: the `String`'s buffer and the `Rc`'s box.
//!   `Arc` behaves the same way.
//! - *What does `.clone()` of a `Vec<String>` cost?* One allocation for the
//!   outer buffer plus one per non-empty string (an empty `String` has no
//!   buffer to copy). A `Vec<Rc<str>>` clones with one.
//! - *Does `collect` pre-size the `Vec`?* From an exact-size iterator, yes:
//!   one allocation and no reallocation. After `filter` the lower size bound
//!   is 0, so it grows by reallocating, and only a bound can be checked.
//!   Reserving an upper bound yourself gets back to one allocation.
//! - *Why is `format!` in a loop slow?* The anti-pattern and the fix of
//!   `65_performance/perf1`, counted: a new `String` per frame plus a
//!   `format!` temporary per row, against zero once the caller's buffer is
//!   warm.
//! - *Word count: why does `entry(word.to_string())` allocate even for words
//!   already in the map?* `entry` takes its key by value, so the `String` is
//!   built before the lookup and dropped if the word was already there.
//!   `get_mut(word)` looks up a `&str` (through `Borrow<str>`) and allocates
//!   only for new words.
//! - *Prove that this hot path allocates nothing.* `Server::handle` after a
//!   warm-up round: borrowed tokens, a `&str` route lookup,
//!   `select_nth_unstable` on a stack array and `write!` into a reused buffer,
//!   under a counting window and under the `assert_no_alloc` guard.
//! - *What the docs don't promise, a test must not assert.* The stable sort's
//!   scratch buffer and the in-place `collect` are printed as observations
//!   (`seen`) and never checked.
//! - *Does an allocation written in the source always happen?* Not in an
//!   optimized build. `deep_dive::perf_lab::boxed_round_trip` boxes a value
//!   and frees it, and in release it does neither; `local_round_trip`, the
//!   same code compiled in this file, does both. Neither result is promised,
//!   so both rows are `seen` too (see "What the optimizer may delete").
//!
//! Compare with `65_performance`: perf1 proves buffer reuse through `as_ptr()`
//! and `capacity()`, perf3 checks growth against a bound, and the module's
//! README notes that counting allocations needs a custom `#[global_allocator]`.
//! This is that allocator. The benchmarking half of the question is
//! `deep-dive/src/perf_lab.rs`.
//!
//! # How the counter works
//!
//! `CountingAlloc` implements `GlobalAlloc` by bumping a counter and then
//! forwarding the call, unchanged, to `System`. `#[global_allocator]` installs
//! it for this binary only: the file is an integration test with
//! `harness = false` (its `[[test]]` entry in `deep-dive/Cargo.toml`), so it
//! has its own `main` and its own allocator, and the library's unit tests and
//! the other labs never see it.
//!
//! The counters are thread-local and gated: they move only while `count` is
//! running, and only for the thread that called it. That keeps the numbers
//! exact (nothing the runtime or another thread does leaks in), at a price you
//! should know about: work handed to another thread is invisible to the count.
//! A process-wide `AtomicUsize` with an on/off flag works too, but it counts
//! every thread while the flag is on.
//!
//! Why `Cell`s with `const` initializers, read with `try_with`: a global
//! allocator must never recurse into itself and must never panic (unwinding
//! out of `GlobalAlloc` is undefined behavior). `GlobalAlloc`'s docs list
//! `thread_local!` among the few std features guaranteed not to allocate
//! through the global allocator (where a platform needs memory for a key, it
//! comes from `System`), so touching a key here does not recurse. A
//! `const { .. }` initializer means no user code runs on first access (a lazy
//! initializer that allocated would recurse). On targets with native
//! thread-locals, Linux and macOS among them, a `const` key of a type without
//! `Drop` is a plain thread-local static: no destructor is registered, so the
//! key is never torn down. `try_with` instead of `with` covers every other
//! case: it returns an error, where `with` would panic, if a key is used
//! after its destruction.
//!
//! # What the optimizer may delete
//!
//! `GlobalAlloc`'s docs: "You must not rely on allocations actually
//! happening, even if there are explicit heap allocations in the source."
//! The optimizer may delete an allocation whose memory nobody observes,
//! together with its free, and the allocator then never hears of either.
//! What that did here, with rustc 1.96 on aarch64-apple-darwin:
//!
//! - **In the library, it deleted one.** `boxed_round_trip` boxes a value,
//!   reads it back and frees the box. The library is compiled without
//!   knowing which allocator the final binary will use, so it calls an
//!   external function, `__rust_alloc`, that rustc marks as an allocator in
//!   the LLVM IR. In release, LLVM deleted the allocation and the free: the
//!   row reads 1 and 1 in a debug build and under Miri, 0 and 0 in release.
//! - **In this binary, it deleted none.** Here `#[global_allocator]` defines
//!   `__rust_alloc` in the same crate, so LLVM sees its body (the counter
//!   update, then the call into `System`) instead of an opaque allocator
//!   function. In `local_round_trip`, the same round trip as
//!   `boxed_round_trip` but written in this file, the release IR inlines that
//!   body and keeps the allocation: 1 and 1 in every build. Every snippet in
//!   the table counted the same in release as in debug. That is an
//!   observation about one compiler, not a promise.
//!
//! So a count describes the build that measured it. The checked counts here
//! hold in debug, in release and under Miri alike; the rows that are up to
//! the optimizer are printed, never checked. `count` also passes each result
//! through `std::hint::black_box` before it closes the window, which keeps
//! the value, and any memory it owns, observed. It cannot bring back an
//! allocation that a callee made and freed internally, and with rustc 1.96
//! removing it changes no row (Try it, 3).
//!
//! # Invariants
//!
//! - Exact counts only where the answer is structural: documented (an empty
//!   `Vec`, `String` or `HashMap` "will not allocate", a new `BTreeMap` "does
//!   not allocate anything on its own", a `Box` of a zero-sized value
//!   "doesn't actually allocate", a `Vec` of zero-sized values does not
//!   allocate space for them, `sort_unstable` and `select_nth_unstable` work
//!   "in-place" and do not allocate, and a buffer whose capacity suffices is
//!   not reallocated), or fixed by how std builds its types today: `collect`
//!   from an exact-size iterator sizes the buffer once (the `FromIterator`
//!   docs list that strategy but do not promise it), and a `String` gets one
//!   block on its first write and grows it with `realloc`.
//! - Growth only with bounds: `Vec` "does not guarantee any particular growth
//!   strategy", only amortized O(1) `push`, so a growing `Vec` is checked for
//!   one allocation and a range of reallocations, never an exact number.
//! - A count the docs leave open is not checked (`any` in a prediction), and a
//!   case that checks nothing is labeled `seen`.
//! - Each case builds its inputs before the window and drops its result after
//!   it, so the numbers belong to the snippet alone.
//! - Every checked count is the same in debug and release builds and under
//!   Miri, which runs this binary in CI.
//!
//! # Sharpest question
//!
//! *How would you prove that a hot path makes zero allocations?* Put a
//! counting `#[global_allocator]` in a test binary, run the hot path once to
//! warm it up (buffers reach their working size on the first call), then run
//! it again inside a counting window and assert that the count is 0. That is
//! the hot-path case and `assert_no_alloc` below. The other half of the
//! question, trustworthy micro-benchmarks, is answered in `perf_lab`.
//!
//! # Run it
//!
//! From `deep-dive/` (plain `cargo test` runs it too, after the unit tests):
//!
//! ```text
//! cargo test --test alloc_count
//! cargo test --release --test alloc_count
//! MIRIFLAGS="-Zmiri-strict-provenance" \
//!   cargo +nightly miri test --test alloc_count
//! ```
//!
//! `main` ignores its arguments, so libtest flags passed after `--` (such as
//! `--nocapture`) do no harm.
//!
//! # Try it
//!
//! 1. Quiz yourself. This resets every exact prediction to 999:
//!
//!    ```text
//!    perl -pi -e 's/\.(allocs|reallocs|frees)\(\d+\)/.$1(999)/g' \
//!      tests/alloc_count.rs
//!    ```
//!
//!    Write your guesses in, then run `ALLOC_QUIZ=1 cargo test --test
//!    alloc_count`: a wrong guess prints `FAIL` with your prediction but not
//!    the measured count, so you can keep guessing. Run without `ALLOC_QUIZ`
//!    to see the answers, and `git checkout -- tests/alloc_count.rs` to get the
//!    shipped predictions back.
//! 2. Break the hot path: add `let _owned = line.to_string();` at the top of
//!    `Server::handle`. The hot-path case fails with 64 allocations and 64
//!    frees, and then the `assert_no_alloc` guard panics, naming the counts.
//! 3. Run `cargo test --test alloc_count`, then
//!    `cargo test --release --test alloc_count`, and compare the last two
//!    rows: `boxed_round_trip` makes 1 allocation and 1 free in debug and
//!    none in release, while `local_round_trip` makes both in either build.
//!    Then delete the `black_box` in `count` and run the release build again:
//!    with rustc 1.96 on aarch64-apple-darwin, no row changes. (`perf_lab`'s
//!    Try it, 6, finds the deleted allocation in the asm.)
//! 4. In the word-count cases, replace `HashMap::with_capacity(8)` with
//!    `HashMap::new()`. The extra counts are the table growing, and they show
//!    up as allocations and frees, not reallocations: the table moves its
//!    entries into a bigger allocation and frees the old one.
//! 5. Skip the warm-up round in the hot-path case: now the first responses
//!    grow the empty `out` buffer, and the case sees it (1 allocation and 2
//!    reallocations with rustc 1.96).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Write as _};
use std::hint::black_box;
use std::process::ExitCode;
use std::rc::Rc;

use deep_dive::perf_lab::boxed_round_trip;

// ---------------------------------------------------------------------------
// The counting allocator
// ---------------------------------------------------------------------------

/// What the allocator saw on this thread while a counting window was open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stats {
    /// Fresh blocks: calls to `alloc` and `alloc_zeroed`.
    allocs: usize,
    /// Calls to `realloc`: a block resized, possibly moved.
    reallocs: usize,
    /// Calls to `dealloc`.
    frees: usize,
}

impl Stats {
    const ZERO: Stats = Stats {
        allocs: 0,
        reallocs: 0,
        frees: 0,
    };
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "allocs {:>4}  reallocs {:>3}  frees {:>4}",
            self.allocs, self.reallocs, self.frees
        )
    }
}

thread_local! {
    // `const` initializers, and no `Drop`: no code runs on first access and no
    // destructor is ever registered (see "How the counter works").
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static STATS: Cell<Stats> = const { Cell::new(Stats::ZERO) };
}

/// Called from inside the allocator: must not allocate, panic or unwind.
fn record(bump: fn(&mut Stats)) {
    // `try_with` returns `Err` instead of panicking if a key is gone; there is
    // nothing useful to do then, so the event is simply not counted.
    let _ = COUNTING.try_with(|counting| {
        if counting.get() {
            let _ = STATS.try_with(|stats| {
                let mut now = stats.get();
                bump(&mut now);
                stats.set(now);
            });
        }
    });
}

/// Forwards every call to `System`, counting it first.
struct CountingAlloc;

// SAFETY: every method passes its arguments unchanged to `System`, which is a
// correct `GlobalAlloc`, and returns its result unchanged, so every contract
// `System` keeps is kept here. The bookkeeping in `record` only touches
// const-initialized thread-local `Cell`s through `try_with` and adds with
// `wrapping_add`: it never allocates, never panics and never unwinds.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(|s| s.allocs = s.allocs.wrapping_add(1));
        // SAFETY: our caller upholds `GlobalAlloc::alloc`'s contract (a
        // non-zero-sized `layout`), which is all `System.alloc` requires.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(|s| s.allocs = s.allocs.wrapping_add(1));
        // SAFETY: same contract as `alloc`, forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(|s| s.frees = s.frees.wrapping_add(1));
        // SAFETY: every block this allocator hands out came from `System`
        // (all four methods forward to it), and our caller guarantees that
        // `ptr` is such a block, allocated with this same `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(|s| s.reallocs = s.reallocs.wrapping_add(1));
        // SAFETY: `ptr` came from `System` with `layout` (as in `dealloc`), and
        // our caller guarantees that `new_size` is non-zero and does not
        // overflow `isize` when rounded up to `layout.align()`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

/// Runs `f` with counting switched on for this thread; returns its result
/// and what the allocator saw. The result goes through `black_box` before the
/// window closes, which asks the optimizer to treat it, and any memory it
/// owns, as observed (best-effort; see "What the optimizer may delete").
fn count<R>(f: impl FnOnce() -> R) -> (R, Stats) {
    assert!(!COUNTING.get(), "counting windows do not nest");
    STATS.set(Stats::ZERO);
    COUNTING.set(true);
    let result = black_box(f());
    COUNTING.set(false);
    (result, STATS.get())
}

/// The counts for `f`. Its result is dropped after the window closes, so
/// freeing it does not count.
fn measure<R>(f: impl FnOnce() -> R) -> Stats {
    count(f).1
}

/// The guard you would put in your own tests: runs `f` and panics, naming
/// the counts, if it called the allocator at all.
fn assert_no_alloc<R>(what: &str, f: impl FnOnce() -> R) -> R {
    let (result, stats) = count(f);
    assert_eq!(stats, Stats::ZERO, "{what} called the allocator");
    result
}

// ---------------------------------------------------------------------------
// Predictions
// ---------------------------------------------------------------------------

/// A prediction for one counter.
#[derive(Clone, Copy)]
enum Count {
    Exactly(usize),
    Between(usize, usize),
    Any,
}

impl Count {
    fn admits(self, n: usize) -> bool {
        match self {
            Count::Exactly(expected) => n == expected,
            Count::Between(low, high) => (low..=high).contains(&n),
            Count::Any => true,
        }
    }
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Count::Exactly(n) => write!(f, "{n}"),
            Count::Between(low, high) => write!(f, "{low}..={high}"),
            Count::Any => write!(f, "any"),
        }
    }
}

/// A prediction for all three counters, built as `predict().allocs(n)...`.
#[derive(Clone, Copy)]
struct Expect {
    allocs: Count,
    reallocs: Count,
    frees: Count,
}

/// Starts a prediction that checks nothing yet.
fn predict() -> Expect {
    Expect {
        allocs: Count::Any,
        reallocs: Count::Any,
        frees: Count::Any,
    }
}

impl Expect {
    fn allocs(self, n: usize) -> Expect {
        Expect {
            allocs: Count::Exactly(n),
            ..self
        }
    }

    fn reallocs(self, n: usize) -> Expect {
        Expect {
            reallocs: Count::Exactly(n),
            ..self
        }
    }

    fn reallocs_between(self, low: usize, high: usize) -> Expect {
        Expect {
            reallocs: Count::Between(low, high),
            ..self
        }
    }

    fn frees(self, n: usize) -> Expect {
        Expect {
            frees: Count::Exactly(n),
            ..self
        }
    }

    fn admits(&self, stats: Stats) -> bool {
        self.allocs.admits(stats.allocs)
            && self.reallocs.admits(stats.reallocs)
            && self.frees.admits(stats.frees)
    }

    fn checks_nothing(&self) -> bool {
        [self.allocs, self.reallocs, self.frees]
            .iter()
            .all(|count| matches!(count, Count::Any))
    }
}

impl fmt::Display for Expect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "allocs {:>4}  reallocs {:>3}  frees {:>4}",
            self.allocs, self.reallocs, self.frees
        )
    }
}

/// One row of the table: a snippet, run inside `measure`, and its prediction.
struct Case {
    name: &'static str,
    expect: Expect,
    run: fn() -> Stats,
}

// ---------------------------------------------------------------------------
// Code under measurement
// ---------------------------------------------------------------------------

/// Enough zero-sized elements to make a point, fewer under Miri (every
/// element is interpreted there).
const ZST_LEN: usize = if cfg!(miri) { 1_000 } else { 1_000_000 };

const FRAMES: usize = 10;
const ROWS: usize = 20;

/// perf1's anti-pattern: a fresh `String` per frame and a `format!`
/// temporary per row.
fn render_naive(frame: usize) -> String {
    let mut out = String::new();
    for row in 0..ROWS {
        out.push_str(&format!("frame {frame:>2} row {row:>2}\n"));
    }
    out
}

/// perf1's fix: the caller's buffer, cleared (which keeps its capacity) and
/// written in place.
fn render_into(frame: usize, out: &mut String) {
    out.clear();
    for row in 0..ROWS {
        // `fmt::Write for String` cannot fail.
        let _ = writeln!(out, "frame {frame:>2} row {row:>2}");
    }
}

/// 10 words, 8 of them distinct ("to" and "be" appear twice).
const TEXT: &str = "to be or not to be that is the question";

fn count_words_entry(text: &str, counts: &mut HashMap<String, u32>) {
    for word in text.split_whitespace() {
        *counts.entry(word.to_string()).or_insert(0) += 1;
    }
}

fn count_words_get_mut(text: &str, counts: &mut HashMap<String, u32>) {
    for word in text.split_whitespace() {
        if let Some(n) = counts.get_mut(word) {
            *n += 1;
        } else {
            counts.insert(word.to_owned(), 1);
        }
    }
}

/// `deep_dive::perf_lab::boxed_round_trip`, copied into this crate: it boxes
/// `x`, reads it back and frees the box.
#[inline(never)]
fn local_round_trip(x: u64) -> u64 {
    let boxed = Box::new(x);
    *boxed
}

/// `len` numbers in a scrambled order (a 64-bit LCG), so sorting has work to
/// do.
fn scrambled(len: usize) -> Vec<u64> {
    let mut x = 1u64;
    (0..len)
        .map(|_| {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            x >> 11
        })
        .collect()
}

/// A toy request handler with the shape of a real hot path.
struct Server {
    /// Route table keyed by owned `String`s, looked up by `&str`.
    routes: HashMap<String, u16>,
    /// The last 8 latencies: a ring buffer on the stack, not on the heap.
    latencies: [u32; 8],
    served: usize,
    /// The response line, reused across requests.
    out: String,
}

impl Server {
    fn new() -> Server {
        let routes = [("/", 200), ("/health", 204), ("/login", 303)]
            .into_iter()
            .map(|(path, status)| (path.to_string(), status))
            .collect();
        Server {
            routes,
            latencies: [0; 8],
            served: 0,
            out: String::new(),
        }
    }

    /// Handles one request line such as `"GET /health"`, recording its
    /// latency, and returns the response line: status, method, path and the
    /// median of the last 8 latencies.
    fn handle(&mut self, line: &str, latency_us: u32) -> &str {
        // Borrowed tokens: slices of `line`, no `Vec<String>`.
        let mut parts = line.split_ascii_whitespace();
        let method = parts.next().unwrap_or("?");
        let path = parts.next().unwrap_or("/");
        // `HashMap<String, _>::get` takes a `&str` through `Borrow<str>`.
        let status = self.routes.get(path).copied().unwrap_or(404);
        self.latencies[self.served % self.latencies.len()] = latency_us;
        self.served += 1;
        // A stack copy, partially sorted in place: no scratch allocation.
        let mut window = self.latencies;
        let middle = window.len() / 2;
        let median = *window.select_nth_unstable(middle).1;
        self.out.clear();
        let _ = write!(self.out, "{status} {method} {path} median={median}us");
        &self.out
    }
}

const REQUESTS: [&str; 4] = ["GET /", "GET /health", "POST /login", "GET /missing"];
const ROUNDS: usize = 64;

/// One round of `ROUNDS` requests; returns the total response length so the
/// work has a result.
fn serve(server: &mut Server) -> usize {
    REQUESTS
        .iter()
        .cycle()
        .take(ROUNDS)
        .enumerate()
        .map(|(i, line)| {
            // Deterministic fake latencies between 100 and 499 us.
            let latency_us = 100 + (i * 37 % 400) as u32;
            server.handle(line, latency_us).len()
        })
        .sum()
}

// ---------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------

fn cases() -> Vec<Case> {
    vec![
        // --- Trivia: empty and zero-sized values ---------------------------
        Case {
            name: "String::new, Vec::new, HashMap::new, BTreeMap::new",
            // All documented: nothing is allocated until the first element.
            // (This is also the first `HashMap::new` in the process, which
            // seeds the thread's `RandomState` keys; that does not touch the
            // global allocator either.)
            expect: predict().allocs(0).reallocs(0).frees(0),
            run: || {
                measure(|| {
                    (
                        String::new(),
                        Vec::<u64>::new(),
                        HashMap::<String, u64>::new(),
                        BTreeMap::<u64, u64>::new(),
                    )
                })
            },
        },
        Case {
            name: "Box::new(()), a boxed non-capturing closure, vec![(); n]",
            // A zero-sized value needs no memory: `Box` and `Vec` use a
            // dangling, aligned pointer. A closure that captures nothing is
            // zero-sized too, even behind `Box<dyn Fn>`.
            expect: predict().allocs(0).reallocs(0).frees(0),
            run: || {
                measure(|| {
                    let unit = Box::new(());
                    let seven: Box<dyn Fn() -> u32> = Box::new(|| 7);
                    let units = vec![(); ZST_LEN];
                    (unit, seven, units)
                })
            },
        },
        // --- Shared strings ------------------------------------------------
        Case {
            name: "Rc::<str>::from(\"hello\")",
            // One block: [strong, weak, h, e, l, l, o]. The bytes live inline.
            expect: predict().allocs(1).reallocs(0).frees(0),
            run: || measure(|| Rc::<str>::from("hello")),
        },
        Case {
            name: "Rc::new(String::from(\"hello\"))",
            // Two blocks: the `String`'s buffer, then the `Rc` box holding the
            // counts and the `String` header (pointer, capacity, length). Every
            // read goes through two pointers.
            expect: predict().allocs(2).reallocs(0).frees(0),
            run: || measure(|| Rc::new(String::from("hello"))),
        },
        // --- Cloning -------------------------------------------------------
        Case {
            name: "clone a Vec<String> of 4 strings, one of them empty",
            // A deep copy: the outer buffer, then one buffer per string that
            // has bytes. The empty `String` has no buffer, so 1 + 3.
            expect: predict().allocs(4).reallocs(0).frees(0),
            run: || {
                let names = vec![
                    "alpha".to_string(),
                    String::new(),
                    "gamma".to_string(),
                    "delta".to_string(),
                ];
                measure(|| names.clone())
            },
        },
        Case {
            name: "clone a Vec<Rc<str>> of the same 4 strings",
            // The outer buffer only; each element clone is a count increment.
            expect: predict().allocs(1).reallocs(0).frees(0),
            run: || {
                let names: Vec<Rc<str>> = ["alpha", "", "gamma", "delta"].map(Rc::from).into();
                measure(|| names.clone())
            },
        },
        // --- Sizing --------------------------------------------------------
        Case {
            name: "collect 1_000 items from an exact-size iterator (map)",
            // `map` over a slice knows its length, so `collect` allocates the
            // whole buffer once.
            expect: predict().allocs(1).reallocs(0).frees(0),
            run: || {
                let data: Vec<u64> = (0..1_000).collect();
                measure(|| data.iter().map(|x| x * 3).collect::<Vec<u64>>())
            },
        },
        Case {
            name: "collect 500 of 1_000 items after filter",
            // `filter`'s lower size bound is 0, so the `Vec` starts small and
            // grows: one fresh block, then reallocations. How many depends on
            // the growth strategy, which is not guaranteed; amortized O(1)
            // `push` only implies geometric growth, so a bound is all a test
            // can check (the current strategy doubles: 7 here).
            expect: predict().allocs(1).reallocs_between(1, 20).frees(0),
            run: || {
                let data: Vec<u64> = (0..1_000).collect();
                measure(|| {
                    data.iter()
                        .copied()
                        .filter(|x| x % 2 == 0)
                        .collect::<Vec<u64>>()
                })
            },
        },
        Case {
            name: "filter into Vec::with_capacity(upper bound)",
            // Reserving the upper bound yourself gets back to one block (at
            // the price of unused capacity when fewer items pass).
            expect: predict().allocs(1).reallocs(0).frees(0),
            run: || {
                let data: Vec<u64> = (0..1_000).collect();
                measure(|| {
                    let mut evens = Vec::with_capacity(data.len());
                    evens.extend(data.iter().copied().filter(|x| x % 2 == 0));
                    evens
                })
            },
        },
        // --- Formatting (65_performance/perf1) -----------------------------
        Case {
            name: "render_naive: 10 frames of 20 rows",
            // Per frame: one `String` (plus reallocations as it grows) and one
            // `format!` temporary per row, all freed again: 10 * (1 + 20) of
            // each. The reallocation count depends on growth and on
            // `format!`'s capacity estimate, neither of which is specified.
            expect: predict().allocs(210).frees(210),
            run: || {
                measure(|| {
                    for frame in 0..FRAMES {
                        black_box(render_naive(frame));
                    }
                })
            },
        },
        Case {
            name: "render_into a reused buffer: 10 frames, after 1 warm-up",
            // The warm-up frame sized the buffer; `clear()` keeps the capacity
            // and `writeln!` formats straight into it. Steady state: nothing.
            expect: predict().allocs(0).reallocs(0).frees(0),
            run: || {
                let mut out = String::new();
                render_into(0, &mut out);
                measure(|| {
                    for frame in 0..FRAMES {
                        render_into(frame, &mut out);
                        black_box(&out);
                    }
                })
            },
        },
        // --- Hash map keys -------------------------------------------------
        Case {
            name: "word count with entry(word.to_string()), 10 words, 8 new",
            // `entry` needs an owned key before it can look, so every word
            // allocates, and the 2 repeated words' keys are dropped again. The
            // map was sized for 8 keys up front, so the table never grows.
            expect: predict().allocs(10).reallocs(0).frees(2),
            run: || {
                let mut counts = HashMap::with_capacity(8);
                measure(|| count_words_entry(TEXT, &mut counts))
            },
        },
        Case {
            name: "word count with get_mut(word), to_owned only when new",
            // The lookup borrows `word` as a `&str`; only the 8 new words
            // allocate a key.
            expect: predict().allocs(8).reallocs(0).frees(0),
            run: || {
                let mut counts = HashMap::with_capacity(8);
                measure(|| count_words_get_mut(TEXT, &mut counts))
            },
        },
        // --- The hot path --------------------------------------------------
        Case {
            name: "Server::handle, 64 requests after a warm-up round",
            // Nothing on the steady-state path touches the heap: see the
            // comments in `Server::handle`.
            expect: predict().allocs(0).reallocs(0).frees(0),
            run: || {
                let mut server = Server::new();
                serve(&mut server);
                measure(|| serve(&mut server))
            },
        },
        // --- Sorting -------------------------------------------------------
        Case {
            name: "slice::sort_unstable of 1_000 u64s",
            // Documented: "in-place (i.e., does not allocate)".
            expect: predict().allocs(0).reallocs(0).frees(0),
            run: || {
                let mut numbers = scrambled(1_000);
                measure(|| numbers.sort_unstable())
            },
        },
        Case {
            name: "slice::sort (stable) of 1_000 u64s",
            // The docs describe the *current* implementation's scratch memory
            // as an implementation detail: observed, not asserted.
            expect: predict(),
            run: || {
                let mut numbers = scrambled(1_000);
                measure(|| numbers.sort())
            },
        },
        Case {
            name: "vec.into_iter().map(..).collect() (in-place collect)",
            // `Vec`'s `FromIterator` docs allow reusing the source buffer but
            // exempt it from stability guarantees: observed, not asserted.
            expect: predict(),
            run: || {
                let numbers: Vec<u64> = (0..1_000).collect();
                measure(move || numbers.into_iter().map(|x| x + 1).collect::<Vec<u64>>())
            },
        },
        // --- The optimizer -------------------------------------------------
        Case {
            name: "deep_dive::perf_lab::boxed_round_trip (library code)",
            // One `Box` in the source: 1 alloc and 1 free in a debug build and
            // under Miri. With rustc 1.96, none in release, where LLVM
            // deleted both while it compiled the library (see "What the
            // optimizer may delete"). Observed, not asserted.
            expect: predict(),
            run: || measure(|| boxed_round_trip(black_box(42))),
        },
        Case {
            name: "local_round_trip (the same code, in this binary)",
            // 1 alloc and 1 free in every build here, release included: in
            // this crate LLVM sees the counting `__rust_alloc`, not an opaque
            // allocator function. Observed, not asserted.
            expect: predict(),
            run: || measure(|| local_round_trip(black_box(42))),
        },
    ]
}

/// The guard form of the hot-path proof, as you would write it in a test.
fn guard_the_hot_path() {
    let mut server = Server::new();
    serve(&mut server); // warm-up: the response buffer reaches its size
    let bytes = assert_no_alloc("Server::handle", || serve(&mut server));
    println!("  ok    assert_no_alloc held for {ROUNDS} requests ({bytes} response bytes)");
}

fn main() -> ExitCode {
    // With ALLOC_QUIZ set, a wrong prediction does not reveal the answer.
    let quiz = std::env::var_os("ALLOC_QUIZ").is_some();
    let cases = cases();
    println!(
        "alloc_count: {} cases, counted on this thread only",
        cases.len()
    );
    let mut wrong = 0;
    for case in &cases {
        let stats = (case.run)();
        if case.expect.checks_nothing() {
            println!("  seen  {stats}  {}", case.name);
        } else if case.expect.admits(stats) {
            println!("  ok    {stats}  {}", case.name);
        } else {
            wrong += 1;
            if quiz {
                let hidden = "(ALLOC_QUIZ: count hidden)";
                println!("  FAIL  {hidden:<37}  {}", case.name);
            } else {
                println!("  FAIL  {stats}  {}", case.name);
            }
            println!("        predicted {}", case.expect);
        }
    }
    if wrong == 0 {
        println!("alloc_count: every prediction held");
    } else {
        println!("alloc_count: {wrong} prediction(s) wrong");
    }
    // Last, so that its panic (if the hot path allocates) cannot hide the
    // table's summary.
    guard_the_hot_path();
    if wrong == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
