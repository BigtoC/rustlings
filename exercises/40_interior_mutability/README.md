# Module 1 · Interior Mutability: Cell, OnceCell, OnceLock, LazyLock and Thread-Locals

> `26_smart_pointers_deep/smartptr3` introduced `RefCell`. This module covers
> the rest of the family, and the interview question behind all of it: which
> primitive do you pick, and why are the others wrong here? All **std**,
> **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements, but **not** the fix. Read what rustc or
> the test says first. Press `h` when you want the full answer.

## Core Ideas

- **`&T` means shared, not immutable.** Aliasing XOR mutability is the default
  rule. Interior mutability is the opt-out: a type may allow mutation through
  `&T` if it keeps the rule some other way. Every such type is built on
  `UnsafeCell<T>`, the only way to mutate behind a `&` that is not undefined
  behavior.
- **Each primitive keeps the rule differently**, and that is what it costs:
  - `Cell<T>` never lends a reference to its contents: values only move in and
    out whole (`get` for `Copy` types, `replace` / `take` for the rest). No
    flag, no guard, and the same layout as `T`.
  - `RefCell<T>` lends references behind guards and counts them at run time.
    A conflicting borrow panics instead of failing to compile.
  - `OnceCell<T>` is written at most once through `&self`. After that the value
    never changes through `&self` again, so it can lend a plain `&T` with no
    guard.
  - `thread_local!` gives each thread its own value, so no other thread can
    see it.
- **Statics must be `Sync`, and their initializers must be `const`.** A static
  is one value that every thread can reach, built at compile time. That
  rules out `Cell`, `RefCell` and `OnceCell` (E0277) and any initializer that
  allocates or parses (E0015). `OnceLock` and `LazyLock` are the thread-safe,
  lazily initialized answer.
- **Pick the cheapest primitive that fits.** Reach for interior mutability
  only when the aliasing is real (an `Rc`, a `&self` API, a global). When
  you can prove the accesses disjoint, restructure the code instead, as in
  `37_borrowck_errors`.

## Picking the Primitive

| You need                                | One thread                           | Any thread              | Stays sound because                          |
| --------------------------------------- | ------------------------------------ | ----------------------- | -------------------------------------------- |
| a `Copy` value you read and write       | `Cell<T>` (`get` / `set` / `update`) | `AtomicU32` and friends | no reference to the inside ever exists       |
| a non-`Copy` value swapped in and out   | `Cell<T>` (`replace` / `take`)       | `Mutex<T>`              | the same: values move, nothing is lent out   |
| to borrow the inside as `&T` / `&mut T` | `RefCell<T>`                         | `Mutex<T>`, `RwLock<T>` | a borrow counter or a lock, and a guard      |
| a value written once, then read as `&T` | `OnceCell<T>`                        | `OnceLock<T>`           | it never changes through `&` once it is set  |
| a value computed on first access        | `LazyCell<T>`                        | `LazyLock<T>`           | the same, with the initializer stored inside |
| one value per thread                    | `thread_local!`                      | (not shared)            | only its own thread can reach it             |

`Cell`, `RefCell`, `OnceCell` and `LazyCell` are `Send` when their contents
are, but never `Sync`. Every type built on `UnsafeCell<T>` is **invariant** in
`T`. If a `Cell<&'static str>` could be used as a `Cell<&'a str>`, you could
`set` a short-lived `&str` into it and read it back as `'static`
(`38_variance` has the full variance table).

## Globals: What rustc Accepts

| Declaration                                                      | Result                                                                                                       |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `static X: RefCell<T>` (or `Cell<T>`, `OnceCell<T>`)             | E0277 "cannot be shared between threads safely"; the note suggests `OnceLock`, `RwLock` or an atomic         |
| `static X: HashMap<K, V> = HashMap::new();`                      | E0015 "cannot call non-const associated function"; the note suggests `LazyLock::new(\|\| ...)`               |
| `static X: Mutex<Vec<String>> = Mutex::new(Vec::new());`         | fine: both are `const fn`, and the lock is for a value that keeps changing                                   |
| `static X: OnceLock<T> = OnceLock::new();`                       | fine: filled once at run time with `set` or `get_or_init`                                                    |
| `static X: LazyLock<T> = LazyLock::new(init);`                   | fine: `init` runs on first access, exactly once, even when threads race for it                               |
| `static mut X: T`                                                | every access is `unsafe` (E0133); a reference to it trips `static_mut_refs`, deny-by-default in edition 2024 |
| `const X: LazyLock<T> = LazyLock::new(init);`                    | compiles, but every use is a fresh copy that runs `init` again (clippy: `declare_interior_mutable_const`)    |
| `thread_local! { static X: Cell<u32> = const { Cell::new(0) } }` | fine: one value per thread, so `Sync` is not needed                                                          |

`static_mut_refs` is a lint, not a hard error: it can be allowed. It exists
because a reference to a `static mut` is undefined behavior if anything else
writes to the static while that reference is alive, and nothing checks that
for you. This course forbids `unsafe`, so `static mut` is out anyway:
`OnceLock`, `LazyLock`, a `Mutex` or an atomic covers the same needs safely.

## Traps Interviewers Ask About

- **Re-entrant initialization.** If a `OnceCell`'s initializer fills the
  same cell (a nested `get_or_init` or `set`), the outer `get_or_init` panics
  with "reentrant init" once the initializer returns. A getter that simply
  calls itself from its own initializer never gets that far: the cell is
  still empty on every nested call, so it recurses until the stack
  overflows. For `OnceLock` the outcome is unspecified (today it deadlocks).
- **A panicking initializer.** `OnceCell` and `OnceLock` stay empty, and the
  next `get_or_init` tries again. A `LazyLock` is poisoned for good: every
  later access panics.
- **`Mutex<Option<T>>` as a lazy global.** It compiles (`Mutex::new(None)` is
  `const`), but every read takes the lock and gets a guard, so it can never
  lend out a `&'static T`. Once a `OnceLock` is set, a read is one atomic
  load and a plain `&'static T`.
- **Leaking instead.** `Box::leak` also produces a `&'static T` (see
  "Leaking on Purpose" in `25_lifetimes_deep`). For one global value a named
  `OnceLock` or `LazyLock` is clearer and can be set only once.
- **`thread_local!` is per thread, not per task or per job.** A thread pool
  reuses its threads, so the state survives from one job to the next, and a
  multi-threaded async runtime can move a task to another thread at any
  `.await` (tokio's `task_local!` exists for that). Destructors of
  thread-locals are best-effort: on some platforms they do not run for the
  main thread.
- **Tests share statics.** libtest runs each test on its own thread, in
  parallel, in one process. A thread-local starts fresh in every test, but a
  global is shared by all of them. That is why only one test sets `cell3`'s
  `GREETING` and only one test touches `cell4`'s counter, while any test may
  read a `LazyLock`: whichever test gets there first, the value is the same.

## Exercise Path

1. **cell1** — A hit counter bumped in a `&self` method is E0594, and
   `get` on a `Cell<String>` is E0599 (`String: Copy` is not satisfied). Make
   the counter a `Cell<u32>` (not `&mut self`: the tests share the cache
   through an `Rc`), and move the key in and out of the `Cell<String>` with
   `replace` and `take` instead of copying it.
2. **cell2** — A `Doc` recounts its words on every call, and its `summary`
   builds a new `String` and then fails to store it. Memoize both with
   `OnceCell::get_or_init`, lend out a plain `&str` from the cached `String`,
   and clear both caches in `set_text(&mut self)`.
3. **cell3** — Three globals that rustc rejects: a `static` `OnceCell` (E0277,
   not `Sync`) and two statics with non-`const` initializers (E0015). Use a
   `OnceLock` for the greeting that is only known at run time, and
   `LazyLock`s for the config and the lookup table, each built exactly once
   however many threads race for it (a test races 8 threads on the config).
4. **cell4** — A process-wide `AtomicU32` numbers every thread's jobs, so a
   new thread starts at 3 instead of 1. Make it a `thread_local!` `Cell<u32>`
   with a `const` initializer.

## Related Modules

- `26_smart_pointers_deep/smartptr3` — `RefCell` and `Rc<RefCell<T>>`.
- `31_debugging/debugging4` and `debugging6` — `RefCell` borrow panics at run
  time, including guards kept alive by a `match` scrutinee.
- `30_send_sync` — `Send`, `Sync` and `Arc<Mutex<T>>`; `36_atomics` — the
  atomics that replace `Cell` across threads.
- `25_lifetimes_deep` — `T: 'static` versus `&'static T`, and `Box::leak`
  versus a `OnceLock` / `LazyLock` static.
- `53_lock_hazards` (`deadlock1`, `rwlock1`) — what goes wrong once the
  global needs a lock.
- Deep-dive lab `deep-dive/src/ub_zoo.rs` — a write through a `&T` that was
  cast to a mutable pointer, which Miri reports as undefined behavior and
  `Cell` fixes, and `BadCell`, a hand-rolled covariant cell that lets safe
  code store a short-lived `&str` as a `&'static str`.

## Further Reading

- [`std::cell` module docs](https://doc.rust-lang.org/std/cell/index.html), including [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html), [`OnceCell`](https://doc.rust-lang.org/std/cell/struct.OnceCell.html), [`LazyCell`](https://doc.rust-lang.org/std/cell/struct.LazyCell.html) and [`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)
- [`OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) and [`LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html)
- [`thread_local!`](https://doc.rust-lang.org/std/macro.thread_local.html) and [`LocalKey`](https://doc.rust-lang.org/std/thread/struct.LocalKey.html)
- [Interior mutability (The Reference)](https://doc.rust-lang.org/reference/interior-mutability.html) and [static items (The Reference)](https://doc.rust-lang.org/reference/items/static-items.html)
- [`RefCell<T>` and the Interior Mutability Pattern (The Book)](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [Disallow references to `static mut` (Edition Guide, Rust 2024)](https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html)
- [Subtyping and variance (The Rustonomicon)](https://doc.rust-lang.org/nomicon/subtyping.html), for why cells are invariant
- [Rust Atomics and Locks, chapter 1: interior mutability](https://mara.nl/atomics/basics.html#interior-mutability), by Mara Bos
- [Clippy: `declare_interior_mutable_const`](https://rust-lang.github.io/rust-clippy/master/index.html#declare_interior_mutable_const)
