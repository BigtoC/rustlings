# Deep Dive into Rust Internals · Advanced Track (Rustlings Deep-Dive)

> An extension of [rustlings](https://rustlings.rust-lang.org) for learners who
> already _use_ Rust and want to understand _why_ it works — aimed at being fluent
> in **interviews and debugging**.
>
> Design principles:
>
> - The **graded exercises** (`exercises/24_*` – `31_*`) are all **std + 100% safe**,
>   so they can be validated automatically by `rustlings dev check` (each exercise
>   "fails while unsolved (compile/test error)"; each solution "passes + clippy
>   `-D warnings` + rustfmt").
> - The parts that **require `unsafe`** (raw-pointer linked list, hand-written
>   `RawWaker`, self-referential structs) live in a separate
>   [``](deep-dive/) crate, as a "read + tinker + run tests" lab.
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
cd ~/somewhere && rustlings init  # generate a workspace containing this course in an empty dir
cd rustlings && rustlings         # start
```

> Restore the official version with `cargo install rustlings`.
>
> Maintainer check of the whole course: `cargo dev check` (compiles and tests every
> exercise and solution).

## Course map (against the five modules)

### Module 1 · Ownership / Lifetimes / Memory model

| Directory                | Exercises       | Focus                                                                                                                     |
| ------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `24_ownership_model`     | `ownership1..3` | Move vs Copy vs Clone; shared `&T` vs exclusive `&mut T`; the borrow checker and NLL                                      |
| `25_lifetimes_deep`      | `lifetimes4..6` | Lifetime parameters, structs holding references, lifetimes in `impl`, elision, `'static`                                  |
| `26_smart_pointers_deep` | `smartptr1..3`  | `Box<T>` heap allocation & recursive types; `Rc`/`Weak` shared ownership & breaking cycles; `RefCell` interior mutability |

### Module 2 · Data structures: reading and hand-writing

| Directory / crate    | Exercise      | Focus                                                                                                   |
|----------------------|---------------|---------------------------------------------------------------------------------------------------------|
| `27_data_structures` | `linkedlist1` | A **safe** singly linked list with `Option<Box<Node>>` (`Option::take` is key)                          |
| `27_data_structures` | `ringbuffer1` | A `Vec`-backed ring buffer: `head`/`len` + modular wraparound — the core of `VecDeque`                  |
| `27_data_structures` | `hashtable1`  | A separate-chaining hash map: `DefaultHasher` to pick a bucket + collision handling                     |
| ``                   | `unsafe_list` | A **doubly** linked list with `NonNull` **raw pointers**, just like the standard library's `LinkedList` |

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
| ``                 | `raw_waker`        | Hand-written `RawWaker` + `RawWakerVTable` (four function pointers) — the real `Waker`                                |
| ``                 | `self_referential` | Self-referential struct + `Pin`/`PhantomPinned` — what `Pin` is really protecting                                     |

**The async trinity**: `Future` defines the computation · `Waker` handles notification · `Pin` guarantees safety.

### Module 4 · Advanced: `Send` / `Sync` and fearless concurrency

| Directory      | Exercise     | Focus                                                                      |
| -------------- | ------------ | -------------------------------------------------------------------------- |
| `30_send_sync` | `send_sync1` | `Rc` is `!Send`; use `Arc` across threads                                  |
| `30_send_sync` | `send_sync2` | `Arc<Mutex<T>>` for shared mutable state                                   |
| `30_send_sync` | `send_sync3` | Inferring the `Send`/`Sync` marker traits, and making a type `Send + Sync` |

> For the outline's `unsafe` use cases, see the three `` labs (each has `// SAFETY:` comments).
> "Read the serde/tokio source" and "the rustc frontend: AST/HIR/MIR" are further reading — see the links in each README.

### Module 5 · Interview & debugging practice

| Directory      | Exercise     | Focus                                                                      |
| -------------- | ------------ | -------------------------------------------------------------------------- |
| `31_debugging` | `debugging1` | `#[derive(Debug)]` + `{:?}` / `{:#?}`                                      |
| `31_debugging` | `debugging2` | Locate and fix a logic bug (reason backwards from debug output/assertions) |
| `31_debugging` | `debugging3` | Implement `Display` / `Debug` by hand                                      |

High-frequency interview topics (ownership, borrowing, lifetimes, `Send`/`Sync`, the
async model) are spread across the exercises above; after finishing each module, use
the "further reading" links in that module's README to revisit the design ideas.

## Directory conventions

- `exercises/NN_*/` — the exercises you fix (one `.rs` per exercise) + that module's `../README.md`
- `solutions/NN_*/` — the corresponding reference solutions
- `` — a separate crate holding the labs that require `unsafe` (not subject to the exercises' `unsafe_code = "forbid"`)
- `../rustlings-macros/info.toml` — the exercise list and hints
