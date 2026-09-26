# Module 4 · Scoped Threads: Borrowing Across Threads, Disjoint `&mut` and Barriers

> The data-parallel corner of Module 4, after `30_send_sync` and
> `56_async_bounds` and before the blocking primitives of `52_condvar`. It
> answers the question every thread interview starts with: why does
> `thread::spawn` want `Send + 'static`, and how do you run threads over data
> you only borrowed? All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements but **not** the fix. Read what rustc or
> the test says first. Press `h` when you want the full answer.

## Core Ideas

- **`thread::spawn` needs `F: Send + 'static`** because nothing ties the new
  thread to its caller. Drop the `JoinHandle` and the thread runs on, detached,
  after the caller's stack frame is gone. Joining on the next line does not
  help: `join` is just a method call, and a panic in between would unwind past
  it. So a closure that captures a borrowed parameter is E0521, and one that
  borrows a local is E0373 (or E0597), the same pair that
  `56_async_bounds/async_send2` showed for spawned futures. `'static` is a
  bound on the type, not a promise that the value lives forever
  (`25_lifetimes_deep/lifetimes7`).
- **`thread::scope` (Rust 1.63) relaxes `'static` to `'scope`.** Every thread
  spawned on the `Scope` is joined before `scope` returns, and even before it
  resumes a panic from your closure. Anything that outlives the call (a
  parameter, a local declared before it) may be borrowed, shared or `&mut`.
  `Send` is still required: sharing `&T` with a thread needs `T: Sync`.
- **Disjoint `&mut` needs no lock.** Two closures that write through the same
  `&mut [T]` are E0499, even when they touch different halves. Split the slice
  first (`split_at_mut`, `chunks_mut`), then move one piece into each thread.
  std has already proven the pieces disjoint, and because no two threads can
  write the same element, there is nothing to lock. A `Mutex` around the slice
  compiles, and makes the threads take turns.
- **Chunk arithmetic has two crashing edges.** `n` threads over `len` elements
  means chunks of `len.div_ceil(n)` elements, but `n == 0` divides by zero and
  a chunk size of 0 (`len == 0`) makes `chunks_mut` panic.
- **A `Barrier` separates phases.** `wait()` on a shared `Barrier::new(k)`
  blocks until `k` threads have arrived, then releases them all, and the same
  barrier can be reused for the next phase. `k` must match the number of
  threads that really wait, or they all block forever. The end of a `scope` is
  a barrier too; a `Barrier` keeps the same threads (and their state) alive
  across phases.
- **Panics are not lost.** A scoped thread that panics and is never joined
  makes `scope` itself panic ("a scoped thread panicked") after every other
  thread has finished. `join` the handle yourself to get the panic as an
  `Err` instead.

## Reading the Error

| Code  | rustc says                                                                             | Typical cause here                                                     | Fix                                                                                         |
| ----- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| E0521 | borrowed data escapes outside of function                                              | `thread::spawn` over a closure that captures a reference parameter     | `thread::scope`; or own the data (`Arc`, a moved `Vec`) if the thread must outlive the call |
| E0373 | closure may outlive the current function, but it borrows `x`                           | `spawn` (or `s.spawn` in a loop) borrows a local, such as a loop index | `move` the closure; `33_closures/closure1` covers it                                        |
| E0597 | `x` does not live long enough ... argument requires that `x` is borrowed for `'static` | `thread::spawn` over a reference to a local                            | `thread::scope`                                                                             |
| E0499 | cannot borrow `*data` as mutable more than once at a time                              | two scoped closures write through the same `&mut` slice                | `split_at_mut` / `chunks_mut`, one piece per thread                                         |

## Why `thread::scoped` Was Unsound

`39_drop_raii` told this story from the destructor's side (its README section
"Leaks Are Safe: the Leakpocalypse"). Here is the thread's side. Before Rust
1.0, std had `thread::scoped`, which spawned a thread at once and returned a
`JoinGuard` whose destructor joined it:

```rust,ignore
// Pre-1.0 std: `F: Send + 'a`, not `'static`. The thread may borrow `'a`
// data, because dropping the returned `JoinGuard<'a, T>` joins the thread.
let mut data = vec![1, 2, 3];
{
    let guard = thread::scoped(|| data.push(4));
    mem::forget(guard); // no destructor, so no join
} // the borrow of `data` ends with the guard's scope
drop(data); // freed while the thread may still be pushing to it
```

The borrow checker was happy: the borrow of `data` was tied to the guard,
and the guard was gone. But soundness depended on a destructor running, and
nothing guarantees that. An `Rc` cycle leaks a value without any `unsafe` at
all (issue #24292), so `mem::forget` was made a safe function (RFC 1066).
`thread::scoped` never reached stable Rust: it was made unstable just before
1.0 ("memory unsafe if destructor is avoided") and deleted later.

`std::thread::scope` fixes the design, not the destructor. You never hold
anything whose destruction matters: `scope` owns the control flow, runs your
closure, and joins every thread in its own code before it returns or resumes
a panic. There is no value to forget, and no way to get out of `scope` while
a thread is still running. The same argument explains why there is no safe
*scoped async spawn*: a future can be forgotten halfway through, so an async
"scope" cannot promise that its tasks finish first (`56_async_bounds`, README
section "Why There Is No Scoped `spawn`").

## Choosing a Tool

| You need                                                     | Use                                                        |
| ------------------------------------------------------------ | ---------------------------------------------------------- |
| threads that borrow locals and are done before you return    | `thread::scope`                                            |
| a thread that outlives its caller (a background worker)      | `thread::spawn` with owned data or an `Arc`                |
| several threads writing different parts of one slice         | `split_at_mut` / `chunks_mut`, one piece per scoped thread |
| every thread must finish phase *k* before any starts *k + 1* | a shared `Barrier` (or end the scope and start another)    |
| a watchdog that can give up on a stuck thread                | `thread::spawn` plus `recv_timeout` (a scope would wait)   |
| a sensible default for `n`                                   | `thread::available_parallelism()`                          |
| data parallelism in production                               | the `rayon` crate (`par_chunks_mut`, `join`, `scope`)      |

## Exercise Path

1. **scope1** — `parallel_sum` hands the two halves of a borrowed slice to
   `thread::spawn`, and rustc rejects both closures with E0521 (once for
   `data`, once for the `on_chunk` test seam). Run the two halves on scoped
   threads instead, both at the same time. The tests check that each half is
   the caller's memory (so a `to_vec` or `Arc` copy fails), that the halves
   run on different threads, and that neither waits for the other to finish.
2. **scope2** — `scale_in_place` spawns two scoped closures that both write
   through `data`: E0499. Cut the slice into at most `n` chunks and move one
   into each thread. The tests cover `len == 0`, `n == 0` and `n > len`, check
   that the chunks tile the caller's slice with one thread each, that all of
   them run at once (a `Mutex` around the slice fails), and that a worker's
   overflow panic reaches the caller.
3. **scope3** — A two-phase parallel prefix sum whose phase 1 (per-chunk scan,
   totals in `AtomicU64`s) is given. Without phase 2 every chunk restarts at
   0, so the tests fail. Add a shared `Barrier` and the offsets. A test seam
   makes one chunk slow to publish its total, so a missing or misplaced barrier
   produces wrong offsets, and a watchdog turns a wrongly sized barrier into a
   failed test instead of a hang.

## Further Reading

- [`std::thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html) (read the "Lifetimes" section on `'scope` and `'env`) and [`Scope::spawn`](https://doc.rust-lang.org/std/thread/struct.Scope.html#method.spawn)
- [`std::thread::spawn`](https://doc.rust-lang.org/std/thread/fn.spawn.html) and [`thread::available_parallelism`](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html)
- [`slice::chunks_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_mut) and [`slice::split_at_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut)
- [`std::sync::Barrier`](https://doc.rust-lang.org/std/sync/struct.Barrier.html)
- [Announcing Rust 1.63.0](https://blog.rust-lang.org/2022/08/11/Rust-1.63.0.html) (scoped threads land in std)
- [Rust Atomics and Locks, ch. 1: Scoped Threads](https://mara.nl/atomics/basics.html#scoped-threads) and [The Leakpocalypse](https://mara.nl/atomics/basics.html#leakpocalypse)
- [The Rustonomicon: Leaking](https://doc.rust-lang.org/nomicon/leaking.html) (`thread::scoped::JoinGuard`), [rust-lang/rust#24292](https://github.com/rust-lang/rust/issues/24292) and [RFC 1066: safe `mem::forget`](https://rust-lang.github.io/rfcs/1066-safe-mem-forget.html)
- [The Rust Book, ch. 16: Using Threads to Run Code Simultaneously](https://doc.rust-lang.org/book/ch16-01-threads.html)
- [Prefix sum](https://en.wikipedia.org/wiki/Prefix_sum) (the two-phase parallel algorithm) and [`rayon`](https://docs.rs/rayon)
- Related: `20_threads`, `30_send_sync`, `37_borrowck_errors/borrowck1` (E0499 and `split_at_mut`), `52_condvar` and `53_lock_hazards` (blocking primitives and deadlocks), `36_atomics` (memory orderings). The planned `miri-ub-zoo` lab (`deep-dive/src/ub_zoo.rs` in `deep-dive/ROADMAP.md`) hand-rolls `split_at_mut` and shows Miri catching a non-atomic data race.
