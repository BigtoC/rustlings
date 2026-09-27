# Traits & Abstraction · Panics: `catch_unwind`, `UnwindSafe`, Poisoning and Panic Safety

> Part of the "Traits & Abstraction" group, after `48_macros_deep` and before
> `50_testing_seams`. A panic is how Rust reports a bug, and `lock().unwrap()`
> appears in almost every concurrent program, yet neither says what happens
> next. This module does: catch a panic at a boundary and read its message
> (`panic1`), recover a `Mutex` that a crashed thread poisoned (`panic2`), and
> write a method that a panic cannot leave half done (`panic3`). The cases
> where a panic aborts the whole process would kill the test binary, so they
> are covered below instead of in the exercises. Entirely **std**, **100%
> safe**, **stable** Rust, edition 2024.
>
> As in the other interview modules, the `// TODO` comments name the error (or
> the failing test) and the requirements but **not** the fix. Read what rustc
> or the test says first. Press `h` when you want the full answer.

## Core Ideas

- **A panic is for bugs; a `Result` is for failures you expect.** Parsing user
  input, opening a file or calling a server can fail in normal operation, so
  those return errors (`35_error_design`). An index out of bounds or an
  `unwrap` on `None` means the program is wrong, and a panic stops it before
  it does more damage.
- **Unwinding runs destructors; it does not undo anything.** By default a
  panic walks back up its thread's stack and drops every live value, so
  `MutexGuard`s unlock, files close and `39_drop_raii`'s guards run. Whatever
  the code changed before it panicked stays changed. The thread then ends,
  and `JoinHandle::join` returns `Err(payload)`. A panic in `main` ends the
  process with exit code 101.
- **`catch_unwind` is for boundaries, not for error handling.** Test harnesses
  (libtest wraps every test in `catch_unwind(AssertUnwindSafe(..))`), thread
  pools, servers (tower-http's `CatchPanicLayer` turns a panicking handler into
  a 500 response; tokio's `JoinError::is_panic` reports a panicked task) and
  functions called from C use it to keep one panic from taking everything
  down. It catches only *unwinding* panics, and whether panics unwind at all
  is the final binary's choice (its `panic` setting), not the library's.
- **`UnwindSafe` is a lint in the type system, not a safety guarantee.** A
  panic cannot cause undefined behavior in safe code. `catch_unwind` asks for
  `F: UnwindSafe` so that you notice when the caller could observe a half-done
  change after the panic (through a `&mut` or a `&RefCell`), and the escape
  hatch `AssertUnwindSafe` is a safe wrapper. rustc's help says to add the
  bound to your generic function; at a general-purpose boundary, assert it
  instead and document that nothing is rolled back (`panic1`).
- **The payload is a `Box<dyn Any + Send>`.** `panic!` puts a `&'static str` or
  a `String` in it (which one is an implementation detail), `panic_any` can
  throw any `Send + 'static` value, and you read it with `downcast_ref`. The
  box is itself an `Any`: downcast what is inside it (`&*payload`), not the box
  (`32_dispatch/dispatch5`).
- **Poisoning reports a panic inside a critical section.** A `Mutex` whose
  guard was dropped by unwinding is marked poisoned, and every later `lock()`
  returns `Err(PoisonError)`. Propagating the panic is the usual answer;
  recovering is correct only when you can restore or check the invariant
  (`panic2`).
- **Panic safety means a panic leaves the invariants intact.** Do everything
  that can panic before the first write, then commit with steps that cannot
  (`panic3`). In `unsafe` code this is a memory-safety requirement, not just a
  nicety.

## Unwind or Abort

Unwinding is the default, not a promise. These cases end the process at once,
without running any more destructors, and `catch_unwind` cannot stop them
(the messages are from Rust 1.96):

| Situation                                                 | What happens                                                                                                               |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `panic = "abort"` in the Cargo profile (`-C panic=abort`) | the panic hook prints the message, then the process aborts                                                                 |
| a panic that escapes a `Drop` run during unwinding        | abort: "panic in a destructor during cleanup", then "thread caused non-unwinding panic. aborting."                         |
| a panic that reaches the end of an `extern "C" fn`        | abort (since Rust 1.81): "panic in a function that cannot unwind"                                                          |
| a panic inside the panic hook                             | abort: "thread panicked while processing panic. aborting."                                                                 |
| a failed allocation                                       | abort: "memory allocation of N bytes failed"                                                                               |
| a stack overflow                                          | not a panic at all: "thread 'main' (12345) has overflowed its stack", then "fatal runtime error: stack overflow, aborting" |
| `std::process::abort()` / `std::process::exit(code)`      | the process ends right there: no hook, no unwinding (not even of the current thread)                                       |

A panic in `Drop` is an ordinary, catchable panic when no other panic is in
flight; it is a *second* panic, escaping a destructor while the first one
unwinds, that aborts. That is why a `drop` must never panic, and why
`39_drop_raii/raii2` restores its snapshot with `mem::swap`, which cannot. An
`extern "C-unwind" fn` (Rust 1.71) lets a panic unwind into its caller, which
must be prepared for it; FFI code that calls Rust callbacks usually catches
the panic at the boundary instead, turns it into an error code, and resumes
it on the Rust side with `std::panic::resume_unwind`. The FFI lab
(`deep-dive/src/ffi_lab.rs`) catches a callback's panic at such a boundary,
shows `extern "C-unwind"`, and checks the `extern "C"` abort in a child
process.

**Why choose `panic = "abort"`?** Smaller, sometimes faster binaries (your
code needs no landing pads for the unwinding path), and a service under a
supervisor that restarts it may prefer to crash at once. The cost: no
destructors run on a panic (buffers are not flushed, temporary files stay),
nothing can be caught, and the test harness needs unwinding: Cargo ignores
the setting for tests, benchmarks, build scripts and proc macros. This
course's profiles set `panic = "abort"`, which is why the exercises' `main`
would abort on a panic while their tests unwind normally.

## `UnwindSafe` at a Glance

| Captured by the closure                                  | `UnwindSafe`? | Why                                                               |
| -------------------------------------------------------- | ------------- | ----------------------------------------------------------------- |
| owned values: `i32`, `String`, `Vec<T>`, `Box<T>`        | yes           | nobody else can see them after the panic                          |
| `&T`, `Rc<T>`, `Arc<T>` of plain data                    | yes           | shared but read-only, so nothing can be half changed              |
| `&mut T`                                                 | **no**        | the caller still has the data after the panic                     |
| `&Cell<T>`, `&RefCell<T>`, `&OnceCell<T>`, `Rc<Cell<T>>` | **no**        | interior mutability (`UnsafeCell`) with nothing to report a panic |
| `&Mutex<T>`, `&RwLock<T>`, `Arc<Mutex<T>>`               | yes           | poisoning tells the next user                                     |
| `&AtomicU32` and the other atomics                       | yes           | each operation happens completely or not at all                   |
| `&OnceLock<T>`, `&LazyLock<T>`                           | yes           | initialization completes, or leaves the cell empty or poisoned    |
| `AssertUnwindSafe<T>`                                    | yes           | your promise                                                      |

## Poisoning: Recover or Propagate?

| Primitive                                   | Poisoned by                             | Afterwards                                                                                                                             |
| ------------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `Mutex`                                     | a panic while a guard is held           | `lock`, `get_mut` and `into_inner` return `Err(PoisonError)` (`try_lock`: `TryLockError::Poisoned`) until `clear_poison()` (Rust 1.77) |
| `RwLock`                                    | a panic while a **write** guard is held | the same; a panicking reader poisons nothing                                                                                           |
| `Condvar::wait`                             | the mutex it relocks                    | returns a `LockResult` like `lock`                                                                                                     |
| `Once`                                      | a panic in `call_once`                  | every later `call_once` panics ("Once instance has previously been poisoned"); `call_once_force` retries                               |
| `OnceLock`, `OnceCell`                      | nothing                                 | the cell stays empty, and the next `get_or_init` tries again                                                                           |
| `LazyLock`                                  | a panic in the initializer              | poisoned for good ("LazyLock instance has previously been poisoned"; `40_interior_mutability/cell3`)                                   |
| `parking_lot` locks, `std::sync::nonpoison` | nothing                                 | `lock()` returns the guard directly                                                                                                    |

- **Propagate** (`lock().unwrap()`, or `.expect("...")` with a better message)
  when a poisoned lock means "a bug left this data in an unknown state". It
  is the usual default: the failure spreads to where someone notices it
  instead of corrupting more data quietly.
- **Recover** (`PoisonError::into_inner`, then repair, then `clear_poison`)
  when the data has no invariant a panic could break (a counter, a cache you
  can clear) or when you can rebuild the invariant from a source of truth, as
  `panic2` rebuilds a cached total from the recorded entries. Do it in one
  helper that every access goes through, and repair only when the lock is
  poisoned.
- **The error holds the lock.** A `PoisonError<MutexGuard<T>>` owns the
  guard: after `let e = m.lock().unwrap_err();`, another `m.lock()` on the
  same thread never returns (std only promises that much: it may deadlock or
  panic), so drop `e` first. And repair through the guard the error hands
  you: dropping it and locking again lets another thread take the lock in
  between and read the data before it is repaired.
- **Poisoning is best effort.** It is advisory (`into_inner` walks right past
  it), and the `Mutex` docs list panics it misses, such as a second panic
  while a lock taken inside a `Drop` (during unwinding) is held.
- Poisoning is a debated design: `parking_lot` never poisons, and std is
  adding non-poisoning versions of its locks in `std::sync::nonpoison`
  (unstable, tracking issue 134645).

## Panic Safety

| Guarantee | After a panic in the middle                                        | Example                                                |
| --------- | ------------------------------------------------------------------ | ------------------------------------------------------ |
| basic     | the invariants hold and nothing leaks, but the work is partly done | `Vec::retain` with a panicking predicate               |
| strong    | everything is exactly as it was before the call                    | `panic3`'s `apply`; `39_drop_raii/raii2`'s transaction |
| no-panic  | it cannot panic                                                    | `mem::swap`, `Option::take`, moving a value            |

- **Where panics hide:** closures supplied by the caller, arithmetic in a
  debug build (`31_debugging/debugging2`, `66_checked_math` later in the
  path), indexing, `unwrap` / `expect`, a `RefCell` that is already borrowed,
  and `Drop` impls of values you overwrite.
- **Compute, then commit.** Build the new state in locals, then install it with
  moves and assignments (`panic3`). It is the simplest way to the strong
  guarantee.
- **Or mutate in place behind a guard** whose `Drop` puts things back if the
  panic unwinds through it: a snapshot for the strong guarantee (`raii2`), or
  just enough repair for the basic one (`Vec::retain` shifts the unchecked
  elements down; `BinaryHeap::sift_up` fills its `Hole`).
- **Never `mem::take` the data, transform it and put it back.** A panic in the
  middle leaves the empty default behind: all the data is gone.
- **In `unsafe` code, panic safety is soundness.** Code that has written only
  half of a buffer, or has called `set_len` too early, must stay sound if the
  closure it calls panics, or if a guard it hands out is leaked. The
  Rustonomicon's "Exception Safety" and "Leaking" chapters show how std does
  it (`Vec::drain` shortens the vector before it starts, so a leaked `Drain`
  leaks elements instead of dropping them twice).

## Hooks, Locations and Backtraces

- **The panic hook** runs on the panicking thread before unwinding starts,
  even for a panic that is caught later. The default hook prints a line like
  `thread 'main' (12345) panicked at src/main.rs:12:5:`, then the message.
  `set_hook` and `take_hook` replace it for the whole process (crash
  reporters, structured logging), so never swap it inside tests that run in
  parallel. `PanicHookInfo::payload()` hands the hook the same
  `&(dyn Any + Send)` that `panic1`'s `panic_message` reads, and
  `payload_as_str()` (Rust 1.91) does that downcast for you; a `catch_unwind`
  payload has no such helper.
- **`#[track_caller]`** makes a function report its caller's location, which
  is why a failed `unwrap()` points at your line and not at std's.
  `std::panic::Location::caller()` reads it.
- **Backtraces:** set `RUST_BACKTRACE=1` (or `full`) to see the stack of the
  panicking thread.

## Exercise Path

1. **panic1** — `run_isolated` lets every panic through, and `panic_message`
   ignores the payload. Catch the panic at the boundary (calling std's
   catching function directly is E0277, and rustc's suggested bound breaks
   the callers that capture a `&mut` or a `&Cell`), and read the message
   whether it arrives as a `&'static str` or a `String`, with a fallback for
   anything else.
2. **panic2** — A worker panics halfway through an update to a
   `Mutex<Ledger>`, and every later `lock().unwrap()` panics with
   `PoisonError { .. }`. Make the one locking helper recover: take the guard
   out of the error, rebuild the cached total from the entries in place, and
   clear the poison, while a healthy lock is returned untouched.
3. **panic3** — `Ledger::apply` converts the entries in place with a running
   total, so a panic in the caller's closure (or an overflowing total) leaves
   `[10, 2, 3]` with a total of 10. Give it the strong guarantee: compute the
   new entries and total first, then commit both.

## Related Modules

- `39_drop_raii`: destructors run during unwinding; `raii1` and `raii2` test
  their guards inside `catch_unwind`, and the README covers which cases skip
  destructors altogether.
- `32_dispatch/dispatch5`: downcasting through `Any`, and the `&Box<dyn ..>`
  trap that returns `None`.
- `35_error_design`: when to return an error instead of panicking.
- `40_interior_mutability/cell3`: `LazyLock` poisoning.
- `50_testing_seams` (next): the last module of this group.
- Later in the path: `51_scoped_threads` (`thread::scope` joins every thread
  and panics with "a scoped thread panicked" if one of them did and you did
  not join it yourself), `53_lock_hazards` (`RwLock` poisoning and other lock
  hazards) and `54_channels` (sharing by message passing instead of locks).

## Further Reading

- [`std::panic`](https://doc.rust-lang.org/std/panic/index.html): [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) (read its "Notes"), [`UnwindSafe`](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html), [`AssertUnwindSafe`](https://doc.rust-lang.org/std/panic/struct.AssertUnwindSafe.html), [`resume_unwind`](https://doc.rust-lang.org/std/panic/fn.resume_unwind.html), [`panic_any`](https://doc.rust-lang.org/std/panic/fn.panic_any.html), [`set_hook`](https://doc.rust-lang.org/std/panic/fn.set_hook.html) and [`PanicHookInfo`](https://doc.rust-lang.org/std/panic/struct.PanicHookInfo.html)
- [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html) (the "Poisoning" section), [`Mutex::clear_poison`](https://doc.rust-lang.org/std/sync/struct.Mutex.html#method.clear_poison) and [`PoisonError`](https://doc.rust-lang.org/std/sync/struct.PoisonError.html)
- [Unrecoverable Errors with `panic!` (The Book)](https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html), including "Unwinding the Stack or Aborting in Response to a Panic"
- [Panic (The Reference)](https://doc.rust-lang.org/reference/panic.html): panic strategies, unwinding, and panics across FFI ABIs
- The Rustonomicon: [Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html), [Exception Safety](https://doc.rust-lang.org/nomicon/exception-safety.html), [Poisoning](https://doc.rust-lang.org/nomicon/poisoning.html)
- [Cargo profiles: `panic`](https://doc.rust-lang.org/cargo/reference/profiles.html#panic) ("Tests, benchmarks, build scripts, and proc macros ignore the panic setting")
- [RFC 1236: stabilize `catch_panic`](https://rust-lang.github.io/rfcs/1236-stabilize-catch-panic.html) (why `UnwindSafe` exists) and [RFC 2945: the `"C-unwind"` ABI](https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html)
- [Announcing Rust 1.81.0](https://blog.rust-lang.org/2024/09/05/Rust-1.81.0/) ("Abort on uncaught panics in `extern "C"` functions")
- [Rust Atomics and Locks, ch. 1: Basics of Rust Concurrency](https://mara.nl/atomics/basics.html) (section "Lock Poisoning")
- [`std::sync::nonpoison`](https://doc.rust-lang.org/nightly/std/sync/nonpoison/index.html) (unstable) and [`parking_lot::Mutex`](https://docs.rs/parking_lot/latest/parking_lot/type.Mutex.html), locks without poisoning
- [`tower_http::catch_panic`](https://docs.rs/tower-http/latest/tower_http/catch_panic/index.html) and [`tokio::task::JoinError`](https://docs.rs/tokio/latest/tokio/task/struct.JoinError.html), panic boundaries in async servers
