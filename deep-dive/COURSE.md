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
>
> `cargo dev check` starts every exercise at once. With 200+ exercises that can exceed
> macOS's per-user process limit (`kern.maxprocperuid`), and the check stops with
> "Resource temporarily unavailable (os error 35)". CI on Linux is not affected. Locally,
> either raise the limit or check a subset of the modules at a time.

## Course map

### Module 1 · Ownership / Lifetimes / Memory model

| Directory                | Exercises              | Focus                                                                                                                                                                                                                                 |
| ------------------------ | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `24_ownership_model`     | `ownership1..6`        | Move vs Copy vs Clone; shared `&T` vs exclusive `&mut T`; the borrow checker and NLL; moving out of `&mut` with `mem::take` / `swap` / `replace` (E0507), enum state transitions, moving out of a `Drop` type (E0509)                 |
| `25_lifetimes_deep`      | `lifetimes4..9`        | Lifetime parameters, structs holding references, lifetimes in `impl`, elision; `T: 'static` vs `&'static T`, the hidden `+ 'static` in `Box<dyn Trait>`, leaking on purpose with `Box::leak`                                          |
| `37_borrowck_errors`     | `borrowck1..4`         | Interview drill, read the error then fix the design: two `&mut` into one slice & split borrows (E0499); temporaries & returned locals (E0716 / E0515); NLL problem case #3 (E0499); no `&mut` downgrade & reborrowing (E0502 / E0382) |
| `38_variance`            | `variance1..2`         | `PhantomData` markers: covariant ids, contravariant sinks, `!Send`, derive bounds; borrowed-forever `&'a mut self` (E0499/E0502)                                                                                                      |
| `26_smart_pointers_deep` | `smartptr1..3`         | `Box<T>` heap allocation & recursive types; `Rc`/`Weak` shared ownership & breaking cycles; `RefCell` interior mutability                                                                                                             |
| `39_drop_raii`           | `drop1..2`, `raii1..4` | Drop-order quizzes (locals, fields, params, `let _`, temporaries); `defer` guard, rollback-on-drop, dropck (E0597), `Rc` leak                                                                                                         |
| `40_interior_mutability` | `cell1..4`             | `Cell` get/set vs `replace`/`take` (E0594/E0599); `OnceCell` memos; `OnceLock`/`LazyLock` statics (E0277/E0015); `thread_local!`                                                                                                      |
| `41_memory_layout`       | `layout1..2`           | Interview quiz: `size_of` of fat pointers, niches, closures, enum tags, `repr(C)` / packed padding; reorder a `repr(C)` header                                                                                                        |

### Traits & Abstraction · the trait-system prerequisites (before async & concurrency)

> All **std + 100% safe**. Ordered right after smart pointers because the async
> and concurrency modules quietly assume closures and dynamic dispatch.

| Directory            | Exercises                                   | Focus                                                                                                                                                                                                                                                             |
|----------------------|---------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `32_dispatch`        | `dispatch1..6`                              | Trait objects (`Box<dyn>` fat pointers), object safety (`where Self: Sized`), static vs dynamic vs **enum** dispatch; `Any` downcasting via trait upcasting; generic methods vs dyn compatibility (E0038, visitor double dispatch)                                |
| `33_closures`        | `closure1..8`                               | Capture modes & `move`; the `Fn` / `FnMut` / `FnOnce` hierarchy; returning `impl Fn` vs `Box<dyn Fn>`; higher-ranked `for<'a>` bounds, let-bound closure inference, async closures & `AsyncFnMut`                                                                 |
| `34_iterators`       | `iter1..4`                                  | Implementing `Iterator` (just `next`); a custom lazy adapter; adapter **laziness**; `IntoIterator` for `for` loops                                                                                                                                                |
| `35_error_design`    | `err1..6`                                   | `?` as `From::from`; custom error enums; `Display` + `Error::source()` chains; `From` vs `TryFrom`; `.context()` chains via an extension trait, `Box<dyn Error + Send + Sync>` across threads, downcasting, why an anyhow-style `Report` cannot implement `Error` |
| `42_coherence`       | `coherence1..3`                             | Orphan rule (E0117) and newtypes, legal `From<Local> for Foreign`, blanket-impl overlap (E0119), extension traits (E0118, E0390)                                                                                                                                  |
| `43_assoc_types`     | `assoc1..2`, `gat1`                         | Associated types vs type params (E0107, E0119, E0283); `Add<Rhs>`/`Output`, `&a + &b`, `T: Add<Output = T>`; GAT lending iterator                                                                                                                                 |
| `44_trait_contracts` | `contracts1..3`                             | `Hash` agreeing with a manual `Eq`; `Ord` agreeing with `Eq` and `PartialOrd`; `f64`: `total_cmp`, float keys, `Reverse` min-heap                                                                                                                                 |
| `45_sized_deref`     | `deref1..2`, `borrow1`, `cow1`, `sized1..3` | Deref coercion limits, Deref is not inheritance, `Borrow<Q>` lookups, `Cow` returns, `?Sized`, forwarding impls, unsized structs                                                                                                                                  |
| `47_type_level`      | `builder1`, `typestate1`                    | `Default` + struct update, a validating builder (`impl Into`, `IntoIterator`, `#[must_use]`), `PhantomData` typestate, sealing                                                                                                                                    |
| `quizzes`            | `quiz4`                                     | Fully qualified syntax for E0034 and a generic E0790; predict rustc verdicts on ten snippets (two-phase borrows, temporaries)                                                                                                                                     |
| `48_macros_deep`     | `macros5..8`                                | `hashmap!` and a recursive `tt` `count!`; bind-once `max!`; `$crate` and mixed-site hygiene (E0425); generating items and tests                                                                                                                                   |
| `49_panics`          | `panic1..3`                                 | `catch_unwind` + `AssertUnwindSafe` (E0277), payload downcasts; poisoned `Mutex` recovery, `clear_poison`; strong panic safety                                                                                                                                    |
| `50_testing_seams`   | `seams1..3`                                 | Trait seams for dependencies; `RefCell`/`Cell` mocks behind `&self`; `Send + Sync` fake clocks: token-bucket limiter, TTL cache                                                                                                                                   |
| `deep-dive/`         | `vtable_lab`                                | The `&dyn Trait` fat pointer built by hand — a data pointer + a static table of function pointers                                                                                                                                                                 |

### Module 2 · Data structures: reading and hand-writing

| Directory / crate    | Exercise      | Focus                                                                                                                              |
|----------------------|---------------|------------------------------------------------------------------------------------------------------------------------------------|
| `27_data_structures` | `linkedlist1` | A **safe** singly linked list with `Option<Box<Node>>` (`Option::take` is key)                                                     |
| `27_data_structures` | `ringbuffer1` | A `Vec`-backed ring buffer: `head`/`len` + modular wraparound — the core of `VecDeque`                                             |
| `27_data_structures` | `hashtable1`  | A separate-chaining hash map: `DefaultHasher` to pick a bucket + collision handling                                                |
| `27_data_structures` | `linkedlist2` | LeetCode `ListNode` classic: reverse in place, relinking the ORIGINAL nodes (checked by node identity)                             |
| `27_data_structures` | `linkedlist3` | Stable merge of two sorted lists through a `&mut` tail cursor                                                                      |
| `27_data_structures` | `linkedlist4` | Fast/slow-pointer split fails with E0502, so count first and walk one `&mut` cursor; remove nth from the end with `checked_sub`    |
| `59_arena`           | `arena1`      | Arena tree: nodes in one `Vec` linked by typed `NodeId`s; `add_child`, loop-based `path_to_root`, `lca`; `Send`, deep `clone`      |
| `59_arena`           | `arena2`      | Generational slab: `(index, generation)` ids make stale handles `None`; `mem::replace` remove, free list, retire at `u32::MAX`     |
| `60_lru_cache`       | `lru1`        | O(log n) warm-up: `HashMap<K, (V, u64)>` + `BTreeMap<u64, K>` of ticks, evict with `pop_first`; every touch must drop the old tick |
| `60_lru_cache`       | `lru2`        | O(1) LRU in safe Rust: an index-linked list in a `Vec` (`unlink` / `push_front`); eviction reuses the tail slot via `mem::replace` |
| `60_lru_cache`       | `lru3`        | Applies `borrow1`: `get<Q>` with `K: Borrow<Q>, Q: ?Sized` so `String` keys take a `&str`; a `peek(&self)` that is not a use       |
| `61_trees`           | `bst1`        | BST insert through a `&mut` link cursor, loop `contains` down one path; exact shape, `i32::MIN`/`MAX` keys, a 200_000-deep path    |
| `61_trees`           | `bst2`        | Lazy in-order iterator over a `Vec<&'a Node>` stack: pending left chain, O(h) stack checked, items borrow the tree (addresses)     |
| `61_trees`           | `bst3`        | Iterative `invert` with a `Vec<&mut Node>` (disjoint field reborrows, same nodes relinked) and BFS `height`; 200_000-deep tree     |
| `61_trees`           | `bst4`        | `return cur` inside a `while let` cursor loop is E0499 (problem case #3); `find_slot`, `take_min`, successor-relinking `remove`    |
| `62_graphs`          | `graph1`      | BFS distances and a parent-array shortest path, Kahn's order with a `Reverse` min-heap, iterative path-compressing union-find      |
| `62_graphs`          | `graph2`      | Recursive `dfs(&mut self)` over `&self.adj[u]` is E0502: go iterative (200k-node path); three-color cycle check via frames         |
| `62_graphs`          | `grid1`       | Grid neighbors without `usize` underflow: `checked_add_signed` + `then_some`, no `as` casts; a maze BFS from the corner            |
| `62_graphs`          | `grid2`       | Islands while a neighbor iterator lives: edition-2024 RPIT capture and `+ use<>` (E0502); `Index<(usize, usize)>` checked per axis |
| `deep-dive/`         | `unsafe_list` | A **doubly** linked list with `NonNull` **raw pointers**, just like the standard library's `LinkedList`                            |
| `deep-dive/`         | `raw_vec`     | `Vec` from scratch: `Layout`/`alloc`/`realloc` growth, `ptr::write`/`read`, and a `Drop` that frees once                           |

### Module 3 · Async model, fully dissected — hand-written async runtime (the focus)

| Directory / crate      | Exercise           | Focus                                                                                                                       |
|------------------------|--------------------|-----------------------------------------------------------------------------------------------------------------------------|
| `28_futures`           | `futures1..2`      | The `Future`/`Poll` trait; `Poll::Pending` and using the `Waker` to ask to be re-polled                                     |
| `28_futures`           | `futures3`         | **Hand-written state machine**: exactly what an `async fn` desugars to                                                      |
| `28_futures`           | `futures4`         | `async`/`await` sugar == the state machine                                                                                  |
| `29_async_runtime`     | `runtime1`         | Building a `Waker` safely with `std::task::Wake`                                                                            |
| `29_async_runtime`     | `runtime2`         | The executor core: the `block_on` poll loop (park/unpark)                                                                   |
| `29_async_runtime`     | `runtime3`         | **Multi-task executor**: a ready-queue + a self-rescheduling `Waker` (the skeleton of tokio's current-thread runtime)       |
| `29_async_runtime`     | `runtime4`         | Why `Pin` exists, and how to safely satisfy `poll`'s `Pin<&mut Self>`                                                       |
| `57_async_combinators` | `join1`            | Hand-written `Join` of `Unpin` children: poll both every poll, keep outputs, never re-poll; `max(n_a, n_b) + 1` polls       |
| `57_async_combinators` | `select1`          | Hand-written `Select` returning `Either`: biased poll order; dropping the loser on the deciding poll is what cancels it     |
| `57_async_combinators` | `cancel1`          | Cancel safety: a `select` heartbeat drops a half-read line; keep progress in the reader, or pin one future outside the loop |
| `58_leaf_futures`      | `oneshot1`         | Oneshot channel leaf: store the latest waker under the value's lock, `Err(Canceled)` when the sender drops, no lost wakeups |
| `58_leaf_futures`      | `timer1`           | Non-blocking `Sleep`: register once with a timer thread and keep the latest waker; why `thread::sleep` in async is a bug    |
| `58_leaf_futures`      | `yield1`           | Cooperative yielding: why `async fn yield_now() {}` never yields; a `YieldNow` leaf and a CPU loop that yields per batch    |
| `deep-dive/`           | `raw_waker`        | Hand-written `RawWaker` + `RawWakerVTable` (four function pointers) — the real `Waker`                                      |
| `deep-dive/`           | `self_referential` | Self-referential struct + `Pin`/`PhantomPinned` — what `Pin` is really protecting                                           |

**The async trinity**: `Future` defines the computation · `Waker` handles notification · `Pin` guarantees safety.

### Module 4 · Advanced: `Send` / `Sync`, atomics, and fearless concurrency

| Directory           | Exercise      | Focus                                                                                                                              |
|---------------------|---------------|------------------------------------------------------------------------------------------------------------------------------------|
| `30_send_sync`      | `send_sync1`  | `Rc` is `!Send`; use `Arc` across threads                                                                                          |
| `30_send_sync`      | `send_sync2`  | `Arc<Mutex<T>>` for shared mutable state                                                                                           |
| `30_send_sync`      | `send_sync3`  | Inferring the `Send`/`Sync` marker traits, and making a type `Send + Sync`                                                         |
| `56_async_bounds`   | `async_send1` | A std `MutexGuard` held across `.await` makes a spawned future `!Send`; why `drop(guard)` fails; end the guard's scope             |
| `56_async_bounds`   | `async_send2` | `spawn` needs `'static`: E0521/E0373 from borrowing into tasks, `async move` and E0382; move clones or an `Arc` in                 |
| `56_async_bounds`   | `async_send3` | `async fn` in a trait gives generics no `Send` (E0277): declare `-> impl Future + Send`, then fix the impl that breaks it          |
| `56_async_bounds`   | `async_send4` | Dyn-compatible async trait (E0038): `Pin<Box<dyn Future + Send + 'a>>`, `Send + Sync` supertraits, lazy `Box::pin` impls           |
| `51_scoped_threads` | `scope1`      | Why `thread::spawn` needs `'static` (E0521); `thread::scope` lets two threads borrow the caller's slice, in place, both at once    |
| `51_scoped_threads` | `scope2`      | One disjoint `&mut` chunk per scoped thread (E0499): `chunks_mut`, the n = 0 / len = 0 edges, no lock, worker panics propagate     |
| `51_scoped_threads` | `scope3`      | Two-phase parallel prefix sum: a shared `Barrier` sized to the real worker count before reading earlier chunks' totals             |
| `52_condvar`        | `condvar1`    | A blocking MPMC queue: `Condvar::wait` in a `while` loop (spurious and stolen wakeups), a waiter count, lost wakeups               |
| `52_condvar`        | `condvar2`    | A bounded queue with `not_empty` / `not_full` condvars: `try_push` hands the item back, `push` blocks, each `pop` wakes a producer |
| `52_condvar`        | `condvar3`    | A counting semaphore from `Mutex` + `Condvar` whose `Permit` gives itself back (and wakes a waiter) in `Drop`, even on panic       |
| `53_lock_hazards`   | `deadlock1`   | Can safe Rust deadlock? Opposite-order bank transfers: reject the same-account re-lock, lock in one global order, stay atomic      |
| `53_lock_hazards`   | `rwlock1`     | Readers share an `RwLock` and writers wait for them; std has no upgradable read, so get-or-insert re-checks under `write()`        |
| `54_channels`       | `channel1`    | Bounded `sync_channel` + non-blocking `try_send`: `Full` is `Busy`, `Disconnected` is `Closed`; capacity 0 is a rendezvous         |
| `54_channels`       | `channel2`    | Disconnect-driven pipeline shutdown: drop every stray `Sender`, return on a failed `send`; drop the `Sender`, then join            |
| `54_channels`       | `channel3`    | Actor owning a `HashMap`: a reply channel in each request (E0559 / E0026); ignore callers that left; `Gone` if the actor dies      |
| `36_atomics`        | `atomics1`    | Lock-free `AtomicUsize` counter with `fetch_add(Relaxed)`                                                                          |
| `36_atomics`        | `atomics2`    | `Release`/`Acquire` publish-subscribe: the happens-before edge that publishes data                                                 |
| `36_atomics`        | `atomics3`    | A CAS-based spinlock (`compare_exchange` + `spin_loop`)                                                                            |
| `36_atomics`        | `atomics4`    | A `compare_exchange_weak` / `fetch_update` retry loop: a bounded counter tested with a deterministic race window                   |
| `36_atomics`        | `atomics5`    | The ABA problem, fixed with a version-tagged `(tag, idx)` head in a lock-free free list                                            |
| `quizzes`           | `quiz5`       | Send and Sync of 14 std types (Mutex vs RwLock of Cell, Arc, &mut, MutexGuard, mpsc ends, raw pointers, dyn), probe-graded         |
| `deep-dive/`        | `myarc`       | `Arc` from scratch: `Relaxed` clone, `Release` drop + `Acquire` fence before free                                                  |
| `deep-dive/`        | `loom_lab`    | Model-check the `atomics2` handoff under `loom` (`--cfg loom`) — why `Relaxed` breaks                                              |

> The `unsafe` use cases live in the `deep-dive/` labs, each with `// SAFETY:` comments.
> "Read the serde/tokio source" and "the rustc frontend: AST/HIR/MIR" are further reading — see the links in each README.

### Module 5 · Interview & debugging practice

| Directory            | Exercise            | Focus                                                                                                                                      |
|----------------------|---------------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| `31_debugging`       | `debugging1`        | `#[derive(Debug)]` + `{:?}` / `{:#?}`                                                                                                      |
| `31_debugging`       | `debugging2`        | Integer overflow: **panics** in debug, silently **wraps** in release — `checked`/`saturating`/`wrapping`                                   |
| `31_debugging`       | `debugging3`        | Implement `Display` / `Debug` by hand                                                                                                      |
| `31_debugging`       | `debugging4`        | `RefCell` runtime `BorrowMutError`: aliasing-XOR-mutability enforced at run time                                                           |
| `31_debugging`       | `debugging5`        | Iterator invalidation caught at **compile time**; fix with `retain`                                                                        |
| `31_debugging`       | `debugging6`        | A guard kept alive by a `match` scrutinee temporary ("RefCell already borrowed"); splitting two fields through one `RefMut`                |
| `31_debugging`       | `debugging7`        | `while let` holds its guard for the whole body; an `if let` then-block still holds it in edition 2024                                      |
| `31_debugging`       | `debugging8`        | `Mutex` self-deadlock from a `match` scrutinee guard (caught with `try_lock`); a quiz on where each guard dies                             |
| `63_slices_strings`  | `window1`           | Two pointers: `len() - 1` underflow on an empty slice, `i32` sum overflow (widen to `i64`), three-sum with deduped triples                 |
| `63_slices_strings`  | `window2`           | Unicode-safe sliding window: `char_indices`, char-boundary panics, chars vs bytes, a let chain, return a `&str` slice                      |
| `63_slices_strings`  | `window3`           | Merge intervals: `sort_unstable_by_key`, then extend `last_mut()` or push in one let chain; touching and contained intervals               |
| `63_slices_strings`  | `window4`           | `partition_point` bounds, `binary_search` on duplicates, `(lo + hi) / 2` overflow vs `midpoint`, binary search on the answer               |
| `64_parsing`         | `parse1`            | A `Lexer<'a>` iterator: `Ident(&'a str)` slices cut at byte offsets, byte-offset errors, `PosOverflow` as `NumberTooLarge`                 |
| `64_parsing`         | `parse2`            | Recursive descent: loops fix 8-3-2 = 7, unary minus, a depth limit, and checked math that tells `DivByZero` from `MIN / -1`                |
| `64_parsing`         | `parse3`            | A boxed `Expr` AST: `FromStr` (which cannot borrow its input), left-deep trees, a fully parenthesized `Display`, round trips               |
| `65_performance`     | `perf1`             | Reuse the caller's `&mut String` with `clear()` + `writeln!` (pointer, capacity checked); `format_push_string` denied, `strict_clippy`     |
| `65_performance`     | `perf2`             | `&str` and `&[impl AsRef<str>]` params that literals, slices and arrays can call (E0308); `Vec<&str>` tokens checked by pointer            |
| `65_performance`     | `perf3`             | Length check before `zip`, one `copy_from_slice` (`strict_clippy`: `manual_memcpy`), `reserve` once then `extend_from_slice`               |
| `66_checked_math`    | `checkedmath1`      | Overflow-safe `a * b / d` on u128: exact 256-bit product via `carrying_mul`, floor and ceil without the `(x + d - 1)` overflow             |
| `66_checked_math`    | `checkedmath2`      | Vault share math: round what the user receives down and what the user pays up, so no trade can lower the share price                       |
| `66_checked_math`    | `checkedmath3`      | Fixed-point `Decimal(u128)` with 18 decimals: strict `FromStr`, canonical `Display`, checked mul/div through `mul_div`                     |
| `66_checked_math`    | `checkedmath4`      | Deterministic ledger: `BTreeMap` order, all-or-nothing integer bps interest, a specified FNV-1a digest, `u64::try_from` over `as`          |
| `67_code_review`     | `review1`           | Unguided review of a ledger PR: seven planted bugs found only through symptom-named failing tests; rank them, then fix each in place       |
| `67_code_review`     | `review2`           | Clippy-graded idiomatic refactor (`strict_clippy`, seven default lints) that must keep behavior: first-wins tie, u64 sum, borrows          |
| `68_mock_interviews` | `set_trie`          | Statement-first 25-min round: an autocomplete trie from scratch; UTF-8 children, sorted suggestions, limit and empty-prefix cases          |
| `68_mock_interviews` | `set_edit_distance` | Statement-first 20-min round: Levenshtein over chars in O(min(n, m)) memory; bytes vs chars vs graphemes; full-table model test            |
| `68_mock_interviews` | `set_kv_tx`         | Statement-first 35-min round: KV store with nested begin/commit/rollback; tombstones; pointer and relative time-budget tests forbid copies |
| `68_mock_interviews` | `set_kv_tx_part2`   | Part-2 follow-up: count(value) kept right across rollbacks with no scan (E0599 until added; relative time budget); split borrows           |

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
