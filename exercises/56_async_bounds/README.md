# Module 4 · Async Bounds: Why `spawn` Needs `Send + 'static`

> The async follow-up to `30_send_sync`: the same two marker traits, applied
> to futures instead of closures. It builds on the state machines of
> `28_futures` and the executors of `29_async_runtime`. Each exercise carries
> a tiny thread-per-task `spawn` whose bounds are exactly `tokio::spawn`'s, so
> every error is the real one without pulling in tokio. All **std**, **100%
> safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Read what rustc says
> first. Press `h` when you want the full answer.

## Core Ideas

`tokio::spawn` has the same bounds as `thread::spawn`, one level up:

```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
```

- **`Send`, because tasks migrate.** A multi-threaded runtime may suspend a
  task at an `.await` on one worker thread and resume it on another (work
  stealing). An `async fn` compiles to a state machine that stores every
  local still alive at an `.await`, so the future is `Send` exactly when
  everything it keeps across an `.await` is `Send`.
- **`'static`, because tasks are detached.** `spawn` returns at once, and the
  task may outlive the caller: the `JoinHandle` can be dropped (the task keeps
  running) or forgotten, and even a caller that awaits it can be cancelled at
  that `.await`. So the future may not borrow the caller's stack. As
  in `25_lifetimes_deep/lifetimes7`, `'static` means "contains no borrow that
  could expire", not "lives forever": owned `String`s and `Arc`s qualify.
- **Generic code sees only declared bounds.** With a concrete type, rustc
  looks through an opaque `impl Future` and sees whether it is `Send` (auto
  traits leak). For a type parameter `S`, `S::get(..)`'s future is `Send`
  only if the trait says so, and a trait's `async fn` says nothing.
- **Trait objects need one type.** A vtable entry has one signature, but
  every impl of an `async fn` returns its own future type. So a trait with an
  `async fn` is not dyn compatible, and the dyn version returns a boxed
  future instead.

## Reading the Error

| rustc says                                                                  | What it means                                                    | Idiomatic fixes                                                           |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------- |
| future cannot be sent between threads safely ... has type `MutexGuard<..>`  | a `!Send` value is stored in the future across an `.await`       | end the guard's scope before the `.await`; an async mutex if it must stay |
| E0521 borrowed data escapes outside of function                             | the task borrows through a reference parameter                   | move owned data into `async move`: clone per task, or share an `Arc`      |
| E0373 async block may outlive the current function, but it borrows `x`      | the task borrows a local                                         | `async move`, plus a clone (or `Arc::clone`) per task inside a loop       |
| E0382 use of moved value ... in previous iteration of loop                  | `async move` moved a non-`Copy` local into the first task        | clone before the move, once per task                                      |
| future cannot be sent ... not implemented for `impl Future<Output = ..>`    | a generic `S::method()` future, and the trait promised no `Send` | declare `fn method(..) -> impl Future<Output = ..> + Send` in the trait   |
| E0038 the trait `X` is not dyn compatible ... because method `m` is `async` | `async fn` and `-> impl Trait` methods cannot go in a vtable     | return `Pin<Box<dyn Future<Output = T> + Send + 'a>>` (`#[async_trait]`)  |
| E0277 `dyn X` cannot be sent between threads safely                         | a trait object has only the auto traits written into it          | `trait X: Send + Sync`, or `dyn X + Send + Sync` at the use site          |

The "future cannot be sent" messages have no E-code: they are rustc's
future-specific wording of an unmet `Send` bound, and their notes point at the
culprit, either a value held across an `.await` or the `.await` of an inner
future that is not `Send`. The same message appears inside a trait impl once
the trait promises `Send` ("required by a bound in
`Store::get::{anon_assoc#0}`").

## Why `drop(guard)` Is Not Always Enough

`std::sync::MutexGuard` is `!Send`: with POSIX threads a mutex must be
unlocked by the thread that locked it. Whether a guard counts as "held across
an `.await`" is decided by a conservative analysis, not by whether you moved
the value out:

```rust
let mut guard = stats.lock().unwrap();
let batch = mem::take(&mut guard.queued); // borrows `guard` (DerefMut)
drop(guard);                              // the lock is released here...
upload(&batch).await;                     // ...but the future is still !Send
```

A local that was ever borrowed might still be pointed to by some reference,
so it keeps its slot in the future until the end of its **scope**, moved out
or not. Every `guard.field` access borrows `guard`, through `Deref` or
`DerefMut`. So `drop` fixes the run-time behavior and not the type. It only
helps for a guard that was never borrowed, which is rarely useful. What always
works:

- a block around the locked section: `let batch = { let mut g = ..; .. };`
- a temporary guard, which dies at the end of its statement:
  `stats.lock().unwrap().started += 1;`
- a plain (non-async) helper function that locks, works and returns.

Beware scrutinee temporaries: in
`if let Some(x) = m.lock().unwrap().pop() { x.send().await }` the guard lives
through the whole then-branch, so that future is `!Send` too (edition 2024
drops it before an `else` branch), and a `match` keeps it alive in every arm.
`31_debugging/debugging8`, later in the course, shows the same temporary
deadlocking a thread. Clippy's `await_holding_lock` (in `clippy::suspicious`,
warn by default) flags guards held across an `.await`, including the
explicit-`drop` case above.

## std `Mutex` or an Async Mutex?

Holding a blocking lock across `.await` invites a deadlock even where it
compiles. `tokio::spawn` requires `Send` on every runtime flavor, current-thread
included, but `block_on` and `spawn_local` (on a `LocalSet`) accept a `!Send`
future. While such a task is suspended, another task on the same thread may
call `lock()`. That thread already holds the mutex, so the call never returns
(std's docs: it might deadlock or panic), and the suspended task never runs
again to release the guard.

| Situation                                                            | Use                                                        |
| -------------------------------------------------------------------- | ---------------------------------------------------------- |
| short critical section, no `.await` inside it                        | `std::sync::Mutex` (or `parking_lot`): cheaper and simpler |
| the lock must stay held across an `.await` (an exclusive connection) | `tokio::sync::Mutex`: `lock().await` suspends, not blocks  |
| one owner that serializes all access                                 | an actor task that owns the state, fed by a channel        |

tokio's own documentation says the std `Mutex` is "ok and often preferred" in
async code. The async mutex is for holding a lock across `.await`, not a
general replacement.

## Why There Is No Scoped `spawn`

`std::thread::scope` (coming up in `51_scoped_threads`) lets threads borrow
from the stack because the call **blocks** until every thread has been
joined. An async scope would be a future that relies on its destructor to
wait for its tasks. Safe code may `mem::forget` any future, which ends the
borrow without running the destructor while the tasks still use the data. So
no safe API can offer concurrency, parallelism and borrowing all at once
(withoutboats' "scoped task trilemma"). The practical answers:

- Need parallelism: move owned data into the task (clones or `Arc`).
- Need only concurrency: don't spawn. `join` the futures inside the current
  task (`57_async_combinators/join1`), where they may borrow freely.
- Blocking is acceptable: run the work on `thread::scope` threads outside the
  async context.

## Async Traits and `Send`

`async fn` in traits is stable since Rust 1.75. Its future type has no `Send`
bound, and there are several ways to add one:

| Approach                                                | Generic callers get `Send`? | `dyn` compatible? | Cost / catch                                                |
| ------------------------------------------------------- | --------------------------- | ----------------- | ----------------------------------------------------------- |
| `async fn get(&self) -> T`                              | no                          | no                | fine for single-threaded or concrete use                    |
| `fn get(&self) -> impl Future<Output = T> + Send`       | yes                         | no                | every impl must be `Send`; impls may still write `async fn` |
| return type notation `where S::get(..): Send`           | yes, per use site           | no                | unstable (E0658), tracking issue rust-lang/rust#109417      |
| `fn get(&self) -> Pin<Box<dyn Future<..> + Send + '_>>` | yes                         | yes               | one allocation per call; what `#[async_trait]` generates    |

- Adding `Send` to the trait moves the obligation into every impl: an
  `async fn` impl on a type with a `RefCell` or an `Rc` inside no longer
  compiles, because its future holds `&self`. For a `pub` trait the
  choice is part of the API: rustc's `async_fn_in_trait` lint warns on a plain
  `async fn` in a public trait, because the bound cannot be added or removed
  later without breaking users. The `trait-variant` crate generates both a
  local and a `Send` flavor from one definition.
- The boxed form is the only dyn-compatible one. `futures::future::BoxFuture`
  is exactly `Pin<Box<dyn Future<Output = T> + Send + 'a>>`, and
  `#[async_trait(?Send)]` drops the `Send`. The trait object itself needs
  `Send + Sync` too (as supertraits, or written into every `dyn` type) before
  an `Arc<dyn Trait>` can cross threads.
- To keep zero-cost generics and still offer `dyn`, pair the static trait
  with a boxed twin: a second trait with a blanket impl, or the wrapper type
  the `dynosaur` crate generates.

## Exercise Path

1. **async_send1** — A request recorder holds a std `MutexGuard` across
   `.await`, so the spawned future is not `Send` (no E-code). Scope each
   guard so nothing is locked while the task is suspended. Then a flush that
   already calls `drop(guard)` before its `.await` still fails, because the
   guard was borrowed: end its scope instead.
2. **async_send2** — `greet_all(names: &[String], ..)` spawns async blocks
   that borrow a parameter (E0521) and a local (E0373), and `async move` alone
   only trades them for E0521 plus E0382. Give every task owned data: a clone
   of its name and a shared `Arc<str>` prefix.
3. **async_send3** — A generic `spawn_fetch<S: Store + Send + Sync + 'static>`
   cannot spawn `S::get`'s future, because the trait's `async fn` promises
   nothing about `Send` (E0277). Declare `-> impl Future<..> + Send` in the
   trait, then fix the read-through cache impl that now breaks the promise by
   holding its lock across the remote call.
4. **async_send4** — A `Vec<Arc<dyn DynStore>>` is impossible while the trait
   has an `async fn` (E0038). Define a `BoxFuture<'a, T>` alias, make the
   trait `Send + Sync`, and rewrite both impls with `Box::pin(async move {
   .. })`, keeping the futures lazy.

Real tokio (`spawn_blocking`, `select!`, graceful shutdown) is the planned
`backend-tokio-lab` deep-dive lab (a sibling crate `backend-lab/`, see
`deep-dive/ROADMAP.md`).

## Further Reading

- [`tokio::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) and [Shared state (the tokio tutorial)](https://tokio.rs/tokio/tutorial/shared-state), on holding a `MutexGuard` across an `.await`
- [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html), especially "Which kind of mutex should you use?"
- [`std::sync::MutexGuard`](https://doc.rust-lang.org/std/sync/struct.MutexGuard.html), why it is `!Send`
- [Clippy: `await_holding_lock`](https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock)
- [Announcing `async fn` and return-position `impl Trait` in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/) (the Rust blog), on `Send` bounds and `trait-variant`
- [Dyn compatibility (The Reference)](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- [Return type notation, tracking issue #109417](https://github.com/rust-lang/rust/issues/109417)
- [`async-trait`](https://docs.rs/async-trait), [`trait-variant`](https://docs.rs/trait-variant) and [`dynosaur`](https://docs.rs/dynosaur)
- [The Scoped Task Trilemma (withoutboats)](https://without.boats/blog/the-scoped-task-trilemma/)
- [Asynchronous Programming in Rust (the async book)](https://rust-lang.github.io/async-book/)
