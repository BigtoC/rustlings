# Deep Dive into Rust Internals · Advanced Track (Rustlings Deep-Dive)

> An extension of [rustlings](https://rustlings.rust-lang.org) for learners who
> already _use_ Rust and want to understand _why_ it works — aimed at being fluent
> in **interviews and debugging**.
>
> Design principles:
>
> - The **graded exercises** (`exercises/24_*` – `36_*`) are all **std + 100% safe**,
>   so they can be validated automatically by `rustlings dev check` (each exercise
>   "fails while unsolved (compile/test error)"; each solution "passes + clippy
>   `-D warnings` + rustfmt").
> - The parts that **require `unsafe`** (raw-pointer linked list, handwritten
>   `RawWaker`, self-referential structs) live in a separate
>   `deep-dive/` crate, as a "read + tinker + run tests" lab.
> - Everything is in **English**: code comments, hints (the `h` key), and READMEs.

## How to run it

This repository is a **source fork** of rustlings, with the exercises living in the
repo itself. So do **not** use a globally-installed `rustlings` command — in release
mode it detects the source repo and refuses to run, reporting
`old method before version 6`. Use the repo's own **debug build** (`cargo run`):

```bash
# Interactive mode (watch): run from the repo root; press h for a hint when stuck.
cargo run

# Run a single exercise directly:
cargo run -- run futures3

# The unsafe deep-dive labs (separate crate):
cargo test --manifest-path deep-dive/Cargo.toml
```

> ⚠️ Use the debug build (plain `cargo run`); **do not add `--release`** — the release
> build refuses to run inside the source repo because it detects `../dev/rustlings-repo.txt`.

### Optional: install as a standalone learner workspace

If you want the full "install a standalone command and solve in a clean workspace
elsewhere" experience:

```bash
cargo install --path . --locked   # install this fork (replaces global rustlings; exercises are embedded)
cd ~/somewhere && rustlings init  # generate a workspace with the graded exercises in an empty dir
cd rustlings && rustlings         # start
```

> ⚠️ What the binary embeds is the **graded exercises** and their module `README.md`s
> (plus the workspace scaffolding), so the `deep-dive/` labs and this `COURSE.md` are
> **not** part of the workspace `rustlings init` generates. (The `solutions/` files it writes start out
> as placeholders and are filled in as you finish each exercise.) Keep a clone of
> this repo around and run the labs from there:
> `cargo test --manifest-path deep-dive/Cargo.toml`.
>
> Restore the official version with `cargo install rustlings`.
>
> Maintainer check of the whole course: `cargo dev check` (compiles and tests every
> exercise and solution) plus `cargo test --manifest-path deep-dive/Cargo.toml` for the
> labs.

## Course map

### Module 1 · Ownership / Lifetimes / Memory model

| Directory                | Exercises       | Focus                                                                                                                                                                                                                                 |
| ------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `24_ownership_model`     | `ownership1..6` | Move vs Copy vs Clone; shared `&T` vs exclusive `&mut T`; the borrow checker and NLL; moving out of `&mut` with `mem::take` / `swap` / `replace` (E0507), enum state transitions, moving out of a `Drop` type (E0509)                 |
| `25_lifetimes_deep`      | `lifetimes4..9` | Lifetime parameters, structs holding references, lifetimes in `impl`, elision; `T: 'static` vs `&'static T`, the hidden `+ 'static` in `Box<dyn Trait>`, leaking on purpose with `Box::leak`                                          |
| `37_borrowck_errors`     | `borrowck1..4`  | Interview drill, read the error then fix the design: two `&mut` into one slice & split borrows (E0499); temporaries & returned locals (E0716 / E0515); NLL problem case #3 (E0499); no `&mut` downgrade & reborrowing (E0502 / E0382) |
| `26_smart_pointers_deep` | `smartptr1..3`  | `Box<T>` heap allocation & recursive types; `Rc`/`Weak` shared ownership & breaking cycles; `RefCell` interior mutability                                                                                                             |

### Traits & Abstraction · the trait-system prerequisites (before async & concurrency)

> All **std + 100% safe**. Ordered right after smart pointers because the async
> and concurrency modules quietly assume closures and dynamic dispatch.

| Directory         | Exercises      | Focus                                                                                                                                                                                                                                                             |
|-------------------|----------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `32_dispatch`     | `dispatch1..6` | Trait objects (`Box<dyn>` fat pointers), object safety (`where Self: Sized`), static vs dynamic vs **enum** dispatch; `Any` downcasting via trait upcasting; generic methods vs dyn compatibility (E0038, visitor double dispatch)                                |
| `33_closures`     | `closure1..8`  | Capture modes & `move`; the `Fn` / `FnMut` / `FnOnce` hierarchy; returning `impl Fn` vs `Box<dyn Fn>`; higher-ranked `for<'a>` bounds, let-bound closure inference, async closures & `AsyncFnMut`                                                                 |
| `34_iterators`    | `iter1..4`     | Implementing `Iterator` (just `next`); a custom lazy adapter; adapter **laziness**; `IntoIterator` for `for` loops                                                                                                                                                |
| `35_error_design` | `err1..6`      | `?` as `From::from`; custom error enums; `Display` + `Error::source()` chains; `From` vs `TryFrom`; `.context()` chains via an extension trait, `Box<dyn Error + Send + Sync>` across threads, downcasting, why an anyhow-style `Report` cannot implement `Error` |
| `deep-dive/`      | `vtable_lab`   | The `&dyn Trait` fat pointer built by hand — a data pointer + a static table of function pointers                                                                                                                                                                 |

### Module 2 · Data structures: reading and hand-writing

| Directory / crate    | Exercise      | Focus                                                                                                                           |
|----------------------|---------------|---------------------------------------------------------------------------------------------------------------------------------|
| `27_data_structures` | `linkedlist1` | A **safe** singly linked list with `Option<Box<Node>>` (`Option::take` is key)                                                  |
| `27_data_structures` | `ringbuffer1` | A `Vec`-backed ring buffer: `head`/`len` + modular wraparound — the core of `VecDeque`                                          |
| `27_data_structures` | `hashtable1`  | A separate-chaining hash map: `DefaultHasher` to pick a bucket + collision handling                                             |
| `27_data_structures` | `linkedlist2` | LeetCode `ListNode` classic: reverse in place, relinking the ORIGINAL nodes (checked by node identity)                          |
| `27_data_structures` | `linkedlist3` | Stable merge of two sorted lists through a `&mut` tail cursor                                                                   |
| `27_data_structures` | `linkedlist4` | Fast/slow-pointer split fails with E0502, so count first and walk one `&mut` cursor; remove nth from the end with `checked_sub` |
| `deep-dive/`         | `unsafe_list` | A **doubly** linked list with `NonNull` **raw pointers**, just like the standard library's `LinkedList`                         |
| `deep-dive/`         | `raw_vec`     | `Vec` from scratch: `Layout`/`alloc`/`realloc` growth, `ptr::write`/`read`, and a `Drop` that frees once                        |

### Module 3 · Async model, fully dissected — hand-written async runtime (the focus)

| Directory / crate  | Exercise           | Focus                                                                                                                 |
|--------------------|--------------------|-----------------------------------------------------------------------------------------------------------------------|
| `28_futures`       | `futures1..2`      | The `Future`/`Poll` trait; `Poll::Pending` and using the `Waker` to ask to be re-polled                               |
| `28_futures`       | `futures3`         | **Hand-written state machine**: exactly what an `async fn` desugars to                                                |
| `28_futures`       | `futures4`         | `async`/`await` sugar == the state machine                                                                            |
| `29_async_runtime` | `runtime1`         | Building a `Waker` safely with `std::task::Wake`                                                                      |
| `29_async_runtime` | `runtime2`         | The executor core: the `block_on` poll loop (park/unpark)                                                             |
| `29_async_runtime` | `runtime3`         | **Multi-task executor**: a ready-queue + a self-rescheduling `Waker` (the skeleton of tokio's current-thread runtime) |
| `29_async_runtime` | `runtime4`         | Why `Pin` exists, and how to safely satisfy `poll`'s `Pin<&mut Self>`                                                 |
| `deep-dive/`       | `raw_waker`        | Hand-written `RawWaker` + `RawWakerVTable` (four function pointers) — the real `Waker`                                |
| `deep-dive/`       | `self_referential` | Self-referential struct + `Pin`/`PhantomPinned` — what `Pin` is really protecting                                     |

**The async trinity**: `Future` defines the computation · `Waker` handles notification · `Pin` guarantees safety.

### Module 4 · Advanced: `Send` / `Sync`, atomics, and fearless concurrency

| Directory      | Exercise     | Focus                                                                                                            |
|----------------|--------------|------------------------------------------------------------------------------------------------------------------|
| `30_send_sync` | `send_sync1` | `Rc` is `!Send`; use `Arc` across threads                                                                        |
| `30_send_sync` | `send_sync2` | `Arc<Mutex<T>>` for shared mutable state                                                                         |
| `30_send_sync` | `send_sync3` | Inferring the `Send`/`Sync` marker traits, and making a type `Send + Sync`                                       |
| `36_atomics`   | `atomics1`   | Lock-free `AtomicUsize` counter with `fetch_add(Relaxed)`                                                        |
| `36_atomics`   | `atomics2`   | `Release`/`Acquire` publish-subscribe: the happens-before edge that publishes data                               |
| `36_atomics`   | `atomics3`   | A CAS-based spinlock (`compare_exchange` + `spin_loop`)                                                          |
| `36_atomics`   | `atomics4`   | A `compare_exchange_weak` / `fetch_update` retry loop: a bounded counter tested with a deterministic race window |
| `36_atomics`   | `atomics5`   | The ABA problem, fixed with a version-tagged `(tag, idx)` head in a lock-free free list                          |
| `deep-dive/`   | `myarc`      | `Arc` from scratch: `Relaxed` clone, `Release` drop + `Acquire` fence before free                                |
| `deep-dive/`   | `loom_lab`   | Model-check the `atomics2` handoff under `loom` (`--cfg loom`) — why `Relaxed` breaks                            |

> The `unsafe` use cases live in the `deep-dive/` labs, each with `// SAFETY:` comments.
> "Read the serde/tokio source" and "the rustc frontend: AST/HIR/MIR" are further reading — see the links in each README.

### Module 5 · Interview & debugging practice

| Directory      | Exercise     | Focus                                                                                                                       |
|----------------|--------------|-----------------------------------------------------------------------------------------------------------------------------|
| `31_debugging` | `debugging1` | `#[derive(Debug)]` + `{:?}` / `{:#?}`                                                                                       |
| `31_debugging` | `debugging2` | Integer overflow: **panics** in debug, silently **wraps** in release — `checked`/`saturating`/`wrapping`                    |
| `31_debugging` | `debugging3` | Implement `Display` / `Debug` by hand                                                                                       |
| `31_debugging` | `debugging4` | `RefCell` runtime `BorrowMutError`: aliasing-XOR-mutability enforced at run time                                            |
| `31_debugging` | `debugging5` | Iterator invalidation caught at **compile time**; fix with `retain`                                                         |
| `31_debugging` | `debugging6` | A guard kept alive by a `match` scrutinee temporary ("RefCell already borrowed"); splitting two fields through one `RefMut` |
| `31_debugging` | `debugging7` | `while let` holds its guard for the whole body; an `if let` then-block still holds it in edition 2024                       |
| `31_debugging` | `debugging8` | `Mutex` self-deadlock from a `match` scrutinee guard (caught with `try_lock`); a quiz on where each guard dies              |

High-frequency interview topics (ownership, borrowing, lifetimes, `Send`/`Sync`, the
async model) are spread across the exercises above; after finishing each module, use
the "further reading" links in that module's README to revisit the design ideas.

## Roadmap

What comes next for interview prep, prioritized and with compile-checked exercise
specs: **[ROADMAP.md](ROADMAP.md)**.

## Directory conventions

- `exercises/NN_*/` — the exercises you fix (one `.rs` per exercise) + that module's `../README.md`
- `solutions/NN_*/` — the corresponding reference solutions
- `deep-dive/` — a separate crate holding the labs that require `unsafe` (not subject to the exercises' `unsafe_code = "forbid"`)
- `../rustlings-macros/info.toml` — the exercise list and hints
