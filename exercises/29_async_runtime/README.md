# Module 3 · Async (Part 2): Writing an Async Runtime by Hand

> The **Executor**, **wake-up mechanism (Waker/Context)**, and **`Pin`** parts of "Module 3" in the syllabus — the core of your goal to "write an async implementation yourself".
> Entirely **std + safe**: `std::task::Wake` lets us build a real `Waker` without any `unsafe`.

## The three pillars of async

| Role     | Responsibility                                                                | Exercise here    |
| -------- | ----------------------------------------------------------------------------- | ---------------- |
| `Future` | Defines the "computation"                                                     | see `28_futures` |
| `Waker`  | Notifies the executor that a task can make progress                           | `runtime1`       |
| `Pin`    | Guarantees a self-referential state machine "won't be moved", keeping it safe | `runtime4`       |

And what ties them together to drive everything is the **Executor** (`runtime2` / `runtime3`).

## Exercise progression

1. **runtime1** — Safely construct a `Waker` using `std::task::Wake` + `Arc`.
2. **runtime2** — Write the core of an executor: the `poll` loop in `block_on` (park/unpark, zero-CPU waiting).
3. **runtime3** — **Key exercise**: a multi-task executor. A ready-queue (`mpsc` channel) plus a `Waker` that re-enqueues its own task. This is the skeleton of `tokio`'s current-thread runtime.
4. **runtime4** — Why `Pin` exists, and how to use `pin!` / `Box::pin` to safely satisfy `poll`'s `Pin<&mut Self>` requirement.

## `std::task::Wake` vs `RawWaker`

The standard library offers two paths for building a `Waker`:

- **The safe path (this directory)**: `impl Wake for T` + `Waker::from(Arc<T>)`. `Arc`'s reference counting naturally serves as the `clone`/`drop` functions in the `RawWakerVTable`.
- **The low-level path (the `deep-dive/` experiments)**: hand-writing `RawWaker` and the four function pointers of `RawWakerVTable` (`clone`/`wake`/`wake_by_ref`/`drop`), which requires `unsafe`. Understand this and you understand the true in-memory representation of a `Waker`.

## Further reading

- [async book · Executing Futures and Tasks](https://rust-lang.github.io/async-book/02_execution/01_chapter.html)
- [`std::task::Wake`](https://doc.rust-lang.org/std/task/trait.Wake.html)
- [`std::pin`](https://doc.rust-lang.org/std/pin/index.html)
- [`Context::from_waker`](https://doc.rust-lang.org/std/task/struct.Context.html)
