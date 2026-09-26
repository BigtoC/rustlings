# Interview-Prep Roadmap

> The plan for extending this course toward **Rust interview preparation**: what
> gets asked in 2024-26 phone screens, live-coding rounds and design follow-ups,
> and which of those topics the course does not yet drill.

## How this roadmap was produced

1. **Inventory.** Every graded module `00`-`36` and every `deep-dive/` lab was read
   and classified: 60 covered topics (with depth), 24 thin spots, and the exercise
   conventions the new work must follow.
2. **Six interviewer lenses** proposed candidates independently: type system,
   memory and `unsafe`, concurrency and async, production API design, live coding,
   and interview-format research (web research of 2024-26 Rust interview question
   lists and process write-ups; see [Sources](#sources)). 71 raw candidates.
3. **Merge and dedupe** into 60 candidates across 44 proposed modules; 24 proposals
   were dropped or folded into others.
4. **Two verifiers.**
   - *Coverage + feasibility*: checked each gap against the repo and compile-checked
     every claimed diagnostic and fail mode on rustc/clippy 1.96, edition 2024.
   - *Interview relevance*: scored each candidate 1-5, refuted or folded weak ones,
     and wrote a sharper interview question.
5. **Tiering**: 27 candidates in Tier 1, 16 in Tier 2, 7 deep-dive labs, 10 cut.

Module numbers were assigned during the merge step and are kept when a module is
built, so cross-references stay valid. Every Tier 1 and Tier 2 module is now
built; the two unused numbers, 46 and 55, belong to cut modules. The directory number is only an ID: a module's place in the learner
path is set by where its `[[exercises]]` entries go in `rustlings-macros/info.toml`.

### How to read an entry

- **Interview value** is the relevance verifier's 1-5 score.
- **Sharpest question** is the verifier's sharper version of the interview question
  the candidate should prepare you for.
- **Exercises** give the proposed name, a one-line task, and how it fails while
  unsolved (as corrected by the feasibility verifier).
- **Authoring notes** are instructions for whoever builds the exercise. Unmarked
  notes are verified corrections and facts from the compile-checked feasibility
  pass. Notes marked *(scope)* come from the interview-relevance review: trims,
  merges and fold-ins.

## Status

Done:

- [x] `37_borrowck_errors` — `borrowck1..4` — built; passes `cargo dev check --require-solutions`
- [x] Extend `24_ownership_model` — `ownership4..6` — built
- [x] Extend `25_lifetimes_deep` — `lifetimes7..9` — built
- [x] Extend `35_error_design` — `err4..6` — built
- [x] Extend `31_debugging` — `debugging6..8` — built
- [x] Extend `33_closures` — `closure5..8` — built
- [x] Extend `32_dispatch` — `dispatch5..6` — built
- [x] Extend `36_atomics` — `atomics4..5` — built
- [x] Extend `27_data_structures` — `linkedlist2..4` — built
- [x] `39_drop_raii` — `drop1..2`, `raii1..4` — built
- [x] `40_interior_mutability` — `cell1..4` — built
- [x] `41_memory_layout` — `layout1..2` — built
- [x] `42_coherence` — `coherence1..3` — built
- [x] `43_assoc_types` — `assoc1..2`, `gat1` — built
- [x] `44_trait_contracts` — `contracts1..3` — built
- [x] `45_sized_deref` (Deref / Borrow / Cow part) — `deref1..2`, `borrow1`, `cow1` — built
- [x] `47_type_level` — `builder1`, `typestate1` — built
- [x] `50_testing_seams` — `seams1..3` — built
- [x] `59_arena` — `arena1..2` — built
- [x] `60_lru_cache` — `lru1..3` — built
- [x] `57_async_combinators` — `join1`, `select1`, `cancel1` — built
- [x] `58_leaf_futures` — `oneshot1`, `timer1`, `yield1` — built
- [x] `56_async_bounds` — `async_send1..4` — built
- [x] `51_scoped_threads` — `scope1..3` — built
- [x] `52_condvar` — `condvar1..3` — built
- [x] `53_lock_hazards` — `deadlock1`, `rwlock1` — built
- [x] `54_channels` — `channel1..3` — built
- [x] `63_slices_strings` — `window1..4` — built
- [x] `66_checked_math` — `checkedmath1..4` — built
- [x] `67_code_review` — `review1..2` — built
- [x] `38_variance` — `variance1..2` — built
- [x] `45_sized_deref` (`?Sized` part) — `sized1..3` — built
- [x] Quizzes after `47_type_level` — merged into one quiz, `quizzes/quiz4` — built
- [x] `48_macros_deep` — `macros5..8` — built
- [x] `49_panics` — `panic1..3` — built
- [x] Quiz `quiz7_send_sync` — `quizzes/quiz5` (Send / Sync only) — built
- [x] `61_trees` — `bst1..4` — built
- [x] `62_graphs` — `graph1..2` (`graph3` / `graph4` merged into `graph1`), `grid1..2` — built
- [x] `64_parsing` — `parse1..3` — built
- [x] `65_performance` — `perf1..3` — built
- [x] `68_mock_interviews` — `set_trie`, `set_edit_distance`, `set_kv_tx`, `set_kv_tx_part2` — built

Deep-dive labs, not started:

- [ ] `deep-dive/src/ub_zoo.rs` (the Miri CI step is done: the `deep-dive-miri` job)
- [ ] `deep-dive/src/ffi_lab.rs`
- [ ] `deep-dive/tests/alloc_count.rs` + `deep-dive/src/perf_lab.rs`
- [ ] `deep-dive/src/ordering_lab.rs` + `treiber.rs`
- [ ] `deep-dive/src/api_surface.rs` + `deep-dive/tests/api_surface.rs`
- [ ] Sibling crate `backend-lab/` (tokio / axum / tower)
- [ ] New crate `config_lab` (serde + Cargo features)

## Tier 1

Build these first. Modules are listed in the merge step's proposed learner order.

### Extend `24_ownership_model` (built)

Theme: moving out of `&mut` and out of `Drop` types (`ownership4..6`).

**As built** (see `exercises/24_ownership_model/`): `ownership5`'s `on_connected` returns `Result<(), Session>` (the unused session goes back to the caller), and there are two transitions (`on_connected`, `close`) instead of three. `Session` counts its drops, so swapping in a dummy session fails a test. On rustc 1.96 the E0509 text reads ``cannot move out of type `Connection<'_>` ``, because the struct borrows the test's log.

#### Moving out of `&mut`: `mem::take`, `mem::replace`, enum state transitions, E0507 and E0509

- Slug `move-out-of-mut` · placement: graded · module: extend `24_ownership_model` (`ownership4..6`) · interview value: **4/5**
- Sharpest question: Implement `fn on_connected(&mut self, session: Session)` for `enum Conn { Connecting { addr: String }, Connected { addr: String, session: Session }, .. }` without cloning `addr`. Why does the obvious `if let Conn::Connecting { addr } = *self` fail?
- Exercises:
  - `ownership4` — `Batch::flush(&mut self) -> Vec<Event>` returns `self.buf`, and `present(&mut self)` swaps front/back buffers through a temporary move; fix with `mem::take` and `mem::swap`. Fails unsolved: E0507 "cannot move out of `self.buf` which is behind a mutable reference".
  - `ownership5` — an `enum Conn` transition on `&mut self` written as `if let Conn::Connecting { addr } = *self { *self = .. }`; rebuild it with `*self = match std::mem::replace(self, Conn::Closed) { .. }`. Fails unsolved: E0507 "cannot move out of `self.addr` as enum variant `Connecting` which is behind a mutable reference".
  - `ownership6` — `Connection` has a `Drop` that logs pending bytes, and `into_pending(self) -> Vec<u8>` returns `self.pending`; take `mut self` and `mem::take(&mut self.pending)`. Fails unsolved: E0509 "cannot move out of type `Connection`, which implements the `Drop` trait".
- Authoring notes:
  - In ownership5 rustc 1.96 names `self.addr`, not `self`, in the E0507 message. Quote that wording in the TODO and hint.
  - Don't repeat the original gap evidence: `mem::take` and `mem::replace` appear nowhere in the repo, and `solutions/22_clippy/clippy3.rs` only uses `mem::swap`.
  - Keep `Event` non-Clone and assert pointer identity (`as_ptr()` before and after) so "just clone it" fails. The verifier confirmed this reliably blocks the clone fix. Tests: flush leaves `buf` empty and a second flush is empty; the swap keeps both heap pointers; `addr.as_ptr()` is identical across the transition; the ownership6 drop log reads "closed with 0 pending bytes".
  - Overlaps with `raii1` (take an `Option` inside `Drop`) and `arena2` (`mem::replace` generation bump) are fine: they reinforce the lesson.
  - Absorbs the dropped `ownership7` (a `mem::swap` double buffer): its pointer-identity check lives in `ownership4`.

### `37_borrowck_errors` (built)

Theme: read-the-error gauntlet (E0499, E0502, E0515, E0716, E0382, NLL problem case #3) with `.clone()` ruled out. Placed right after `25_lifetimes_deep` in `info.toml`, because `borrowck2` relies on explicit lifetime parameters.

#### Borrow-checker error gauntlet with non-Clone fixtures

- Slug `borrowck-error-gauntlet` · placement: graded · module: `37_borrowck_errors` (`borrowck1..4`) · interview value: **5/5**
- Sharpest question: Why does `let a = &mut v[i]; let b = &mut v[j];` fail, and what are three ways to get both (`split_at_mut`, `get_disjoint_mut`, index-based swap)?
- As built (see `exercises/37_borrowck_errors/`):
  - `borrowck1` — E0499 twice. `swap_titles` over a non-Clone `Ticket` slice: use `get_disjoint_mut` or `split_at_mut`, return `SameIndex` / `OutOfBounds` instead of panicking, and keep ids in place (so whole-element `swap` fails). Then an `Editor` whose two `&mut self` accessors conflict: split the borrow through the fields or a `parts_mut` method.
  - `borrowck2` — E0716 in `header_columns` (the `to_lowercase()` temporary), then E0515 twice: `shout_longest` must return an owned `String`, and `first_word` must borrow from its input (a pointer test rejects copies and leaks).
  - `borrowck3` — NLL problem case #3 (E0499) twice: a `HashMap` get-or-default (entry API) and a `Vec` find-or-push (look up the index first). Heap-pointer asserts reject "clone it and put it back".
  - `borrowck4` — E0502: `Log::push(&mut self) -> &str` keeps the whole log exclusively borrowed (no downgrade, and Polonius rejects it too), so separate mutation from reads. Then E0382: a `&mut String` passed to a generic `W: Write` is moved rather than reborrowed, so reborrow with `&mut *buf`.
- Deviations from the proposal: `borrowck4` uses the `Log` API instead of `bump_first` and adds the reborrow (E0382) part. `borrowck2` adds a third, "borrow from the input" case. Hasher counting was dropped in favor of heap-pointer identity checks. The module sits after `25_lifetimes_deep`, not `24_ownership_model`.
- Known limits: the tests cannot detect leaking in `header_columns` or copying the text out in `push_and_report`. The TODOs forbid both in writing. If Polonius reaches stable, `borrowck3`'s starter will compile and `dev check` will flag it as already solved; give it a new starter then.
- Module-37 rule (interview-style recall): each TODO names the diagnostic (E-code plus short rustc wording) and the constraints (no `.clone()`, no `unsafe`, don't change the tests) but not the fix. The `info.toml` hint carries the full fix.
- Problem case #3 recurs in `quiz6_borrowck`, `bst4` and `seams3`: keep `borrowck3` as the canonical exercise and cross-reference it from those.

### Extend `25_lifetimes_deep` (built)

Theme: `'static` in practice: `T: 'static` vs `&'static T`, default trait-object lifetimes, leaking. This also makes the COURSE.md claim that module 25 covers `'static` true.

**As built** (see `exercises/25_lifetimes_deep/`): `lifetimes7` fails with E0308 in the tests until the bound is `T: Display + Send + 'static`; a test-only `Traced` type checks that formatting happens on the logger thread, and a message type that is `Send` but not `Sync` (it holds a `Cell`) rejects an unneeded `Sync` bound. `lifetimes8` fails with E0310, `lifetimes9` with E0515. The LazyLock/OnceLock contrast uses std doc links, not a cross-reference to the not-yet-built `40_interior_mutability`; add it when that module lands.

#### `'static` in practice: `T: 'static` vs `&'static T`, `Box<dyn Trait>` defaults to `+ 'static`, leaking for `&'static`

- Slug `static-lifetimes` · placement: graded · module: extend `25_lifetimes_deep` (`lifetimes7..9`) · interview value: **4/5**
- Sharpest question: Does `T: 'static` mean the value lives for the whole program? Why does `thread::spawn` accept a `String` but reject a `&str` borrowed from a local?
- Exercises:
  - `lifetimes7` — `fn spawn_logger(msg: &'static str)` must accept `format!(..)` Strings and `Arc<str>`; change it to `fn spawn_logger<T: Display + Send + 'static>(msg: T)`. Fails unsolved: E0308 expected `&'static str`, found `String`.
  - `lifetimes8` — `struct EventBus { handlers: Vec<Box<dyn Fn(&str)>> }` with `on(&mut self, f: impl Fn(&str))`; the test registers a handler that borrows a local `RefCell<Vec<String>>` log. Fix with `EventBus<'a>`, `type Handler<'a> = Box<dyn Fn(&str) + 'a>` and `f: impl Fn(&str) + 'a`. Fails unsolved: E0310 "the parameter type `impl Fn(&str)` may not live long enough"; rustc's suggested `+ 'static` then fails again (see notes).
  - `lifetimes9` — `fn leak_config(s: String) -> &'static str` starts as `&s`; fix with `Box::leak(s.into_boxed_str())`. Fails unsolved: E0515 "cannot return reference to function parameter `s`".
- Authoring notes:
  - lifetimes8: once rustc's suggested `+ 'static` is added, a plain (non-`move`) closure that borrows `log` fails with E0373 ("closure may outlive the current function, but it borrows `log`"), not E0597. E0597 only appears if the test binds `let r = &log;` and passes a `move` closure. Write the TODO and hint for the test shape you choose.
  - Verified: E0308 (`&'static str` vs `String`), E0310 on `Box::new(f)` into `Vec<Box<dyn Fn(&str)>>`, and the lifetimes9 E0515 wording.
  - `Box::leak` is not caught by the crate's `mem_forget = deny`. The README must explain why leaking is fine here despite dev/Cargo.toml's "don't leak" comment, and contrast it with `LazyLock` / `OnceLock` (`40_interior_mutability`).
  - Tests: lifetimes7 logs a `String`, an `Arc<str>` and a literal through a joined thread; lifetimes8 gives `["a", "b"]` after `emit("a")`, `emit("b")` and a counting handler reaches 2; lifetimes9's `&'static str` is usable from a spawned thread after the owner's scope ended.
  - The default-object-lifetime lesson was proposed twice (`dispatch6` and `lifetimes10`); only this copy (lifetimes8) is kept.
  - *(scope)* lifetimes9 (`Box::leak`) is the lowest-value part; keep it short.

### `39_drop_raii` (built)

Theme: predict the drop order (quiz), then write `Drop`: guards, rollback, drop check, safe leaks.

**As built** (see `exercises/39_drop_raii/`; adversarially reviewed). Deviations and verified corrections:

- `drop1`: As built: the ROADMAP's long scenario list is cut to 5 scenarios per the (scope) note. A struct built in non-declaration order absorbs the dropped raii1 field-order reorder. The constants start as `&[]` and fail at run time, as the ROADMAP says, and the messages never print the real log. The Vec is built with `.map(Noisy).collect()` to avoid clippy's useless_vec. Review fixes: the header wording about fields (they are dropped with or without a Drop impl), and 'anymore' in the solution. The 5 answers are the same in edition 2021 (re-verified).
- `drop2`: The four scenarios the ROADMAP lists: let-statement temporaries (`let _total = Noisy(..).len() + Noisy(..).len()`), lifetime extension (`let _borrowed = &Noisy(..)` followed by a later local), block tail vs local, and the `if` condition. The if-let / match scrutinee rules are left to 31_debugging, per the (scope) note. Review fixes: the edition-2021 command now uses the path from the rustlings directory and writes into `target/` (re-verified: only S_TAIL differs), and the header says 'Part 1 was mostly about values with an owner'. The module has 9 quiz scenarios in total, against the (scope) note's 'about 6'. I kept them because this exercise list comes from the ROADMAP itself, and the edition note depends on the tail scenario.
- `raii1`: As built: the starter includes `fn cancel(self) {}` with its own TODO. After the E0507 fix, the two cancel tests fail until `cancel` disarms the guard, so it is two-stage. `#[must_use]` on the guard, plus a test showing that `let _ = Defer::new(..)` runs the closure at once. The tests reject FnMut/Fn/Clone/Copy bounds, `mem::forget`, and `cancel(&mut self)`. Review fix: added the missing `// TODO` pointer in `Defer::new` (a change site).
- `raii2`: As built: adds `clear()` (so a truncate/pop rollback fails), `rows()`, and a given `import()` that uses `?` between begin and commit. Two TODOs: `commit` (starts empty) and the missing `Drop` impl. Borrowck in the tests enforces a by-value `commit`. Review fix: the hint wrongly said `mem::forget` / `ManuallyDrop` in `commit` fail the tests. Known limit: `ManuallyDrop::new(self)` passes, because the leaked snapshot cannot be observed. Clippy catches `mem::forget`; `ManuallyDrop` is ruled out only by the TODO.
- `raii3`: Deviates from the ROADMAP shape, which is justified because tests must not need editing. The broken code is a given `run_shift` that declares a `Vec<Inspector<'_>>` before the `Roster` it borrows from, instead of the Nomicon's `let inspector; let days = ..;` inside a test. So E0597 names `roster`, not `days`. `Inspector<'a>(&'a u8)` and its reading Drop are kept, and a `Vec<&u8>` contrast function compiles. Re-verified: the code compiles without Inspector's Drop; it still fails without Roster's Drop; an explicit `drop(on_duty)` does not fix it (unwind path; a plain-u8 control compiles). Review fix: the header now says 'Delete `impl Drop for Inspector`' (there are two Drop impls) and 'Only std (and nightly code)'.
- `raii4`: As built: the tests go through a `parent()` accessor instead of calling `child.parent.upgrade()` on the field, so the starter compiles and fails at run time (drops 0 != 2), as the ROADMAP says. There are Weak-probe, strong/weak-count, 'parent owns its children' and four-node-tree tests. Review fix: added the missing `// TODO` pointer in `Node::new`, which is a change site (`RefCell::new(None)` becomes `RefCell::new(Weak::new())`).

#### Predict the drop order: locals, fields, params, `let _`, moves, temporaries

- Slug `drop-order-quiz` · placement: quiz · module: `39_drop_raii` (`drop1`, `drop2`) · interview value: **3/5**
- Sharpest question: What is the difference between `let _ = mutex.lock().unwrap();` and `let _guard = mutex.lock().unwrap();`, and in what order are a struct's fields and a function's locals dropped?
- Exercises:
  - `drop1` — `Noisy(&'static str)` logs on `Drop` into a thread-local log, and the learner fills one `const S_X: &[&str] = &[];` per scenario: two locals, struct fields built out of order, tuple/array/`Vec` elements, fn params, `let _ = make()` vs `let _g`, `let _ = x` on a binding, reassignment, move then `drop()`, partial move, conditional move. Fails unsolved: every constant starts as `&[]` and every scenario logs something, so each assertion fails with "S_X: wrong prediction" (the value is not printed).
  - `drop2` — temporaries: block tail temporary vs local, `if` condition temporary, `let _n = noisy().len()`, `let _r = &noisy()` (lifetime extension). Fails unsolved: failing assertions while the constants are `&[]`.
- Authoring notes:
  - Don't tell learners to flip the edition to compare: it is set package-wide in dev/Cargo.toml (and in the workspace `rustlings init` generates), and 2021 breaks other exercises that rely on 2024 features. Point them at `rustc --edition 2021 <file>` instead.
  - Orders verified on 1.96 (2024 vs 2021): struct fields drop in declaration order; params drop after body locals, in reverse; `let _ = f()` drops immediately; tuple and `Vec` elements drop in order; a block's tail temporary drops before its locals in 2024; `if`-condition temporaries drop before the body; `let _r = &noisy()` is lifetime-extended. (Also verified, but taught in `scrutinee-guard-temporaries`: an if-let scrutinee drops before `else` in 2024, after it in 2021, and lives through the then-branch; a match scrutinee lives through the arms.)
  - Each `#[test]` runs one scenario against a fresh thread-local log; this isolates cleanly because libtest runs each test on its own thread. Known weakness: a learner can `println!` the log to read the answers, as with any runtime-checked quiz.
  - *(scope)* The proposal had about 17 scenarios. Cut to about 6 core ones (`let _` vs `let _g`, reverse order of locals, field declaration order, params, move then `drop()`, lifetime extension) and leave the if-let / match scrutinee rules to `scrutinee-guard-temporaries`, so they are not taught twice.
  - Absorbs the dropped `raii1` field-order reorder as one quiz scenario.

#### Writing `Drop`: defer guard, transaction rollback, drop check, Rc-cycle leak fixed with `Weak`

- Slug `raii-drop-guards` · placement: graded · module: `39_drop_raii` (`raii1..4`) · interview value: **4/5**
- Sharpest question: Implement a `defer` guard that runs a closure on scope exit, including on `?` and on panic, and supports `cancel()`. Why is the closure stored in an `Option`, and is it guaranteed to run?
- Exercises:
  - `raii1` — `Defer<F: FnOnce()> { f: F }` whose `drop` calls `(self.f)()`; store `Option<F>`, `take()` it in `drop`, add `cancel(mut self)`. Fails unsolved: E0507 "cannot move out of `self.f` which is behind a mutable reference".
  - `raii2` — `Tx<'a> { store: &'a mut Vec<i32>, snapshot, committed }` with `begin`/`push`/`commit(self)` and no `Drop`; implement `Drop` that restores the snapshot unless committed. Fails unsolved: assertion, the store is `[1, 2]` instead of `[1]` after an uncommitted `Tx` drops.
  - `raii3` — `Inspector<'a>(&'a u8)` with a `Drop` that reads `self.0`; the test declares `let inspector; let days = Box::new(1); inspector = Inspector(&days);`. Reorder the declarations; the `Drop` must stay. Fails unsolved: E0597 `days` does not live long enough (the same code compiles without the `Drop` impl, which is the point).
  - `raii4` — `Node { parent: RefCell<Option<Rc<Node>>>, children, drops: Rc<Cell<u32>> }` leaks through the strong back-pointer; make `parent` a `Weak<Node>`. Fails unsolved: assertion, the drop counter is 0 instead of 2 after the scope ends.
- Authoring notes:
  - `mem::forget` is not an escape hatch: clippy `mem_forget = deny` in dev/Cargo.toml.
  - Verified: `(self.f)()` in `Drop` gives E0507 with the note "this value implements `FnOnce`"; the Nomicon `Inspector` case gives E0597 and compiles without the `Drop` impl.
  - Cargo ignores the profile's `panic = "abort"` for test targets, so the `catch_unwind` tests (raii1 still runs during unwinding; raii2 leaves the store unchanged after a caught panic) unwind. Keep them inside `#[test]`.
  - `Tx::commit(mut self)` with a `committed` flag works alongside a `Drop` impl, because assigning a field is allowed where moving out is not.
  - raii1 tests: runs at scope end, on early return and `?`, LIFO for two guards, `cancel` suppresses it. raii4 tests: `child.parent.upgrade()` works while the parent is alive and is `None` after.
  - raii4 is the first exercise that actually observes an `Rc` cycle leak (the gap `smartptr2` leaves).
  - README: drop check and the unstable `#[may_dangle]` (raii3).
  - *(scope)* raii1 and raii2 carry most of the value; raii3 (dropck) is niche but short.

### `40_interior_mutability` (built)

Theme: `Cell`, `OnceCell`, `OnceLock`, `LazyLock` and `thread_local!`; picking the interior-mutability primitive. Placed after `26_smart_pointers_deep`.

**As built** (see `exercises/40_interior_mutability/`; adversarially reviewed). Deviations and verified corrections:

- `cell1`: Follows the ROADMAP entry (`Cache { hits: u32, last: String }`, `get(&self)` doing `self.hits += 1`), with `last` given as a `Cell<String>` and a concrete API: `remember(&self, String) -> String` (fix: `replace`) and `take_last(&self) -> String` (fix: `take`). E0599 and E0594 sit in different function bodies, so rustc reports all three errors at once, with the E0599s first; the header explains the order. Tests call through two `&Cache` and an `Rc<Cache>` (the `&mut self` fix gives E0596), compare `as_ptr()` (clone-based fixes fail), and check that both cells are no bigger than their values (rejects RefCell/Mutex). Only an `AtomicU32` counter passes undetected; the TODO forbids it. Reviewer: reworded one header sentence about `Rc` (Rc::get_mut exists).
- `cell2`: Deviations from the ROADMAP entry: two instrumentation counters (`counted` for `count_words`, `summarized` for `build_summary`) instead of one `computed`, so the summary test can check that it reuses the cached count. The summary starter is a realistic compute-then-`set` bug. Added Part C: `set_text(&mut self)` invalidates both caches lazily via `OnceCell::take`. Added edge-case tests: a cached 0, one cache per `Doc`, nothing computed in `new` or `set_text`, and the returned `&str` points into `self.summary`. Reviewer: corrected two hint statements (the self-recursive getter overflows the stack rather than panicking with 'reentrant init'; the E0515 wording).
- `cell3`: Deviations from the ROADMAP entry: `TABLE: HashMap = HashMap::new()` became `PORTS`, built with `HashMap::from([...])` so the table has real entries. E0015 then names `<HashMap<&str, u16> as From<..>>::from`: the TODO quotes the bin-build wording, and the test build prints `std::collections::HashMap`. The tests check `PORTS["https"] == 443` instead of `TABLE["b"] == 2`, and turn `&CONFIG` into a `&'static Config` by deref coercion instead of `&*CONFIG`. The 8-thread race uses a Barrier and asserts `LOADS == 1` and `ptr::eq`. Only one test touches GREETING. Reviewer: added the Mutex<Option<String>> remark to the hint (the dropped once2 fold-in).
- `cell4`: Follows the ROADMAP entry, framed as per-worker job numbers with a `log_line` helper. One test only, per the ROADMAP rule (the starter's counter is process-wide and libtest runs tests in parallel). It covers: the test thread reads 2, a spawned thread starts at 1, the test thread is still at 2 after join, 4 scoped workers each number 1, 2, 3, and the test thread continues at 3. Verified that libtest runs each test on its own named thread even with --test-threads=1. Not rejectable by tests (the TODO forbids them, the hint explains): Mutex<HashMap<ThreadId,u32>>, and a thread-local RefCell or AtomicU32.

#### Cell, OnceCell, OnceLock and LazyLock statics, thread_local

- Slug `interior-mutability-family` · placement: graded · module: `40_interior_mutability` (`cell1..4`) · interview value: **4/5**
- Sharpest question: You need a lazily initialized global config read by many threads. What do you use, and why can't a `static` hold a `RefCell` or be `static mut` here?
- Exercises:
  - `cell1` — `Cache { hits: u32, last: String }` whose `get(&self)` does `self.hits += 1`; use `Cell<u32>` (`get`/`set` or `Cell::update`) and `Cell<String>` through `replace`/`take`, since `get` needs `Copy`. Fails unsolved: E0594 "cannot assign to `self.hits`, which is behind a `&` reference"; calling `.get()` on `Cell<String>` is E0599 (bounds not satisfied).
  - `cell2` — `Doc { text, word_count: OnceCell<usize>, computed: Cell<u32> }` recomputes on every call; use `get_or_init`; bonus `summary(&self) -> &str` from an `OnceCell<String>` (a plain `&T`, no guard). Fails unsolved: assertion, `computed == 3` after three calls, expected 1.
  - `cell3` — `static GREETING: OnceCell<String>`, `static CONFIG: Config = load();` and `static TABLE: HashMap<..> = HashMap::new();`; switch to `OnceLock<String>` set once at runtime (`init(s) -> Result<(), String>`), `LazyLock<Config>` and `LazyLock<HashMap<..>>`. Fails unsolved: E0277 "`OnceCell<String>` cannot be shared between threads safely" (statics must be `Sync`); E0015 cannot call non-const fn `load` / `HashMap::new` in statics.
  - `cell4` — a global `static CALLS: AtomicU32` must become per-thread: `thread_local! { static CALLS: Cell<u32> = const { Cell::new(0) } }` with `LocalKey::get`/`set`. Fails unsolved: assertion, the spawned thread's first bump returns 3, expected 1.
- Authoring notes:
  - If a starter triggers both cell1 errors, E0599 is reported first: E0594 only appears once typeck passes, because borrowck runs after typeck.
  - `Cell::update` is stable on 1.96.
  - rustc hints half of cell3's answer: the E0277 note suggests `OnceLock`, and the E0015 note suggests `LazyLock`. Word the TODO and hint knowing the learner will see that.
  - README: `static mut` is out (unsafe is forbidden). In edition 2024 `static_mut_refs` is a deny-by-default lint that can be allowed, not a hard error.
  - Statics are process-global and all tests share one process: only one test may call the `GREETING` init, and only one test may touch `CALLS`, or the "3 vs 1" assertion becomes nondeterministic.
  - Tests: a second `set` returns `Err(value)`; 8 scoped threads race on `&*CONFIG` with the init counter at 1 and all pointers `ptr::eq`; `TABLE["b"] == 2`; cell4 main reads 2, the spawned thread reads 1, main still reads 2 after join.
  - Absorbs the dropped `once2` (`OnceLock` replacing `Mutex<Option<..>>`): cell3 already teaches `OnceLock::set`/`get`.

### `41_memory_layout` (built)

Theme: `size_of` drills (the graded niche / padding fix-up exercises were cut, see [Cut](#cut-and-where-the-useful-bits-went)).

**As built** (see `exercises/41_memory_layout/`; adversarially reviewed). Deviations and verified corrections:

- `layout1`: Cut to 9 items: the ROADMAP scope note asks for about 15 across both quizzes. Dropped `&u8`, `Option<&u8>`, `Option<fn()>`, `&[u8]`, `Box<dyn Trait>`, plain `String`/`Vec` (ownership4 already teaches the 3-word header) and `Option<Option<String>>`; their rules stay in the header, README and hint. The ROADMAP's 'closure capturing two u64 by reference' became a closure borrowing two Strings: two u64 captured by copy would also be 2 words on 64-bit, so the wrong lesson would pass, while two Strings by value is 6 words. As the ROADMAP specifies, the placeholders are 999 with question-only messages, not the generic `???` compile-error convention and not `assert_eq!`, which would print the answer. The exercise compiles and fails 9 tests at run time.
- `layout2`: Cut to 8 quiz items, per the scope note of about 15 across both quizzes. Dropped plain `u32`, `Option<bool>`, `Option<char>`, the ZST items, `Result<u32, Void>`, the fieldless enum, `enum { A(u32), B(u8) }` and `Option<[u64; 0]>`. The bool/char niches and ZSTs are still explained in the header and README, and layout1's non-capturing closure covers a ZST. Grading uses compile-time `const _: () = assert!(..)` checks, the ROADMAP's E0080 alternative. The ROADMAP's `PacketHeader` reorder is folded in as Part B: const guards (size 16, alignment of `u64`), tests for the offsets (id 0, len 8, kind 12, bools {14, 15} in either order), for borrowing the fields and for a by-value read of a packed field, plus a `#[deny(improper_ctypes_definitions)] extern "C" fn on_packet` that makes dropping `repr(C)` a compile error. `Message` carries `#[allow(clippy::large_enum_variant)]` because the lint text ("the entire enum is at least 1025 bytes") would reveal Q7. The byte answers are 64-bit specific, as the ROADMAP says to label them (Q_OPTION_F64 is 12 on i686; Q_BOXED_MESSAGE is 4 on 32-bit), which is fine for the x86_64/aarch64/win64 CI.

#### size_of drills: fat pointers, niches, enums, repr(C) and packed, ZSTs, closures

- Slug `layout-size-quiz` · placement: quiz · module: `41_memory_layout` (`layout1`, `layout2`) · interview value: **4/5**
- Sharpest question: What are `size_of::<Option<Box<T>>>()`, `size_of::<Option<u32>>()` and `size_of::<&dyn Trait>()`, and why?
- Exercises:
  - `layout1` — fill `const Q_*: usize = 999;` in words (`W = size_of::<usize>()`) for thin and fat pointers, `Rc<str>`, nested `Option`s, `String`/`Vec`, and closures by what they capture; each test asserts `size_of::<T>() == Q * W`. Fails unsolved: every constant starts at 999; assertion messages name the question only.
  - `layout2` — the same in bytes for integers, `NonZero`, `bool`/`char`/`f64` options, ZSTs, enums, `repr(C)` vs default vs `repr(C, packed)` and a large enum vs its boxed form. Fails unsolved: failing assertions, or E0080 const assertions that don't print values.
- Authoring notes:
  - Answer key verified on rustc 1.96 aarch64. Words: `&u8`, `Option<&u8>`, `Option<Box<u8>>`, `Option<fn()>` are 1W (null-pointer optimization, guaranteed); `&[u8]`, `&str`, `&dyn Trait`, `Box<dyn Trait>`, `Rc<str>`, `Option<Option<&u8>>` are 2W (the last because references expose only the null niche); `String`, `Vec`, `Option<String>`, `Option<Option<String>>` are 3W (capacity niche, not guaranteed); a non-capturing closure or fn item is 0; a `move` closure capturing a `String` is 3W; a closure capturing two `u64` by reference is 2W.
  - Bytes: `u32` 4, `Option<u32>` 8, `Option<NonZeroU32>` 4, `Option<bool>` 1, `Option<char>` 4, `Option<f64>` 16, ZSTs 0, `Result<u32, Void>` 4, fieldless enum 1, `enum { A(u32), B(u8) }` 8, `#[repr(C)] { u8, u32, u8 }` 12 vs default repr 8 (not guaranteed) vs `repr(C, packed)` 6, `Option<[u64; 0]>` 8, `enum { Ping, Data([u8; 1024]) }` 1025 vs boxed 8.
  - A closure capturing two variables by reference is 2W because closures store one reference per captured variable (or place). It is not an effect of edition-2021 disjoint capture.
  - Label layout2's byte answers as 64-bit specific (e.g. `Option<f64>` is 12 on i686, where `f64` has align 4). That is fine for the current CI (x86_64, aarch64, win64).
  - Keep each answer labelled "guaranteed" (null-pointer optimization, `core::option` Representation) or "current rustc" (`String`/`Vec` niches, default field reordering).
  - *(scope)* About 30 items is too many: cut to roughly 15 high-signal ones.
  - Fold-in from the cut `layout-niche-padding`: one field-reorder item in layout2. Verified: `#[repr(C)] PacketHeader { flag: bool, id: u64, kind: u16, ok: bool, len: u32 }` is 24 bytes; reordered by alignment it is 16 bytes, align 8, with `offset_of!` id=0, len=8, kind=12, flag=14, ok=15. E0793 is the code for references to packed fields (README).

### `42_coherence` (built)

Theme: orphan rule and newtypes, blanket-impl overlap, extension traits.

**As built** (see `exercises/42_coherence/`; adversarially reviewed). Deviations and verified corrections:

- `coherence1`: Part B's starter is an `impl Into<(i32, i32)> for Point`, the pre-1.41 habit that clippy's `from_over_into` flags, so the learner has to REPLACE it: a `From` impl added next to it is E0119 against core's `Into` blanket. On rustc 1.96 only E0425 appears for the missing `Polyline` (the ROADMAP says E0425/E0433; no E0433 because no test uses a `Polyline::` path). E0210 and #[fundamental] live in the README. The dropped `Display for Box<Point>` claim is listed there as E0119 (authoring note applied), and `impl Add for &Point` / `impl Neg for Box<Point>` illustrate #[fundamental]; all verified. Leave `strict_clippy` unset. 8 tests.
- `coherence2`: The given blanket impl is `impl<T: fmt::Display + ?Sized> Describe for T`, the same shape as std's `ToString`. One graded site, as the ROADMAP specifies. E0210, the adapter-struct fix, #[fundamental] (`Box<Bytes>`, `&Bytes`, `&mut Bytes`, `Pin<Bytes>` accepted; `Box<Vec<u8>>`, `&Vec<u8>`, `Rc<Bytes>`, `Vec<Bytes>`, `[u8]` rejected with the upstream note; a local Display type rejected without it; all re-verified), the lack of specialization, and the RFC 2451 rule that a blanket impl is semver-major are all in the header and README. 5 tests.
- `coherence3`: This deviates from the ROADMAP fail mode on purpose, and I kept it: the starter is NOT empty. It holds the obvious wrong attempt, the inherent impls `impl<I: Iterator> I { .. }` (E0118) and `impl str { .. }` (E0390, whose help says "consider using an extension trait instead"), with correct method bodies. The ROADMAP's E0599 still appears at every call site in the test build. The unsolved test build also prints a harmless `unused import: super::*` warning. DedupAdjacent uses `Peekable::next_if`, so it needs only PartialEq. 14 tests (not 13): non-Clone `Token` and non-PartialEq `Frame` catch bounds on the blanket impl, a laziness pull-counter, a test-local `Countdown`, chaining, fully qualified calls, 5 receiver types, char-boundary truncation ("héllo wörld" -> "hé…"). The authoring-note tests (empty iterator, `(1..=4).pairwise()`, "aaabccd" -> "abcd") are all present. The hint contains the non-ASCII `…` (no backslashes), so info.toml must stay UTF-8.

#### Coherence: orphan rule and newtypes, blanket-impl overlap, extension traits

- Slug `coherence-orphan-extension` · placement: graded · module: `42_coherence` (`coherence1..3`) · interview value: **4/5**
- Sharpest question: Why can't you write `impl fmt::Display for Vec<Point>`, and what are your options?
- Exercises:
  - `coherence1` — `impl fmt::Display for Vec<Point>` must become `struct Polyline(Vec<Point>)` with `Display` and `FromIterator<Point>`; also add `impl From<Point> for (i32, i32)` (legal: a local type as the trait parameter, no uncovered type parameters). Fails unsolved: E0117 "only traits defined in the current crate can be implemented for types defined outside of the crate", then E0425/E0433 for the missing `Polyline` until it exists.
  - `coherence2` — `trait Describe` with a blanket `impl<T: Display> Describe for T` plus `impl Describe for Vec<u8>`; keep the blanket and move bytes onto a local `Bytes(Vec<u8>)`. Fails unsolved: E0119 conflicting implementations for `Vec<u8>`, with the note "upstream crates may add a new impl of trait `Display` for `Vec<u8>`".
  - `coherence3` — `Pairwise<I>` and `DedupAdjacent` are given as structs; tests call `.pairwise()` / `.dedup_adjacent()` on any iterator and `.is_blank()` / `.truncate_ellipsis(n)` on `&str` and `String`. Write `trait IterExt: Iterator + Sized` with a blanket `impl<I: Iterator> IterExt for I`, and `StrExt` implemented for `str` (covers `String` via auto-deref). Fails unsolved: E0599 no method named `pairwise` found for `RangeInclusive<{integer}>` / `is_blank` for `&str`.
- Authoring notes:
  - Drop the README claim "`impl Display for Box<Point>` is legal (`#[fundamental]`)". It passes the orphan rule, but when `Point: Display` (needed for the `"(0, 0) -> (1, 2)"` output) it conflicts with alloc's `impl<T: Display + ?Sized> Display for Box<T>` and fails with E0119 (verified). Illustrate `#[fundamental]` with a trait that has no `Box` blanket impl.
  - Verified: E0117 for `Display` on `Vec<Point>`; `impl From<Point> for (i32, i32)` is legal; `impl<T> From<Point> for T` gives E0210 (README material); the coherence2 E0119 note; `StrExt` on `str` covers `String` through auto-deref.
  - On rustc 1.96 a missing type is E0425 "cannot find type" (E0412 on older compilers).
  - `truncate_ellipsis` must cut on a char boundary: `"héllo wörld".truncate_ellipsis(2) == "hé…"`. Also test an empty iterator, `(1..=4).pairwise() == [(1, 2), (2, 3), (3, 4)]` and `"aaabccd".chars().dedup_adjacent() == "abcd"`.
  - README: adding a blanket impl is semver-major.

### `43_assoc_types` (built)

Theme: associated types vs generic parameters, GAT lending iterator; also the home for the useful parts of operator overloading.

**As built** (see `exercises/43_assoc_types/`; adversarially reviewed). Deviations and verified corrections:

- `assoc1`: As in the ROADMAP, plus a second implementor (`Grid`, nodes `(usize, usize)`), so the same generic functions run on two graph types, and a `cheapest_step` that bounds a projection (`where G::Edge: Ord`). The tests' `leads_to<G: Graph>` compares `G::Node`s with no bound of its own, which forces the bound onto the associated type. The unsolved build also reports E0576 for the tests' fully qualified `<AdjList as Graph>::Node`, and the TODO names it. ROADMAP tests present: `degree == 2`, `path_cost == Some(7)`, missing edge `None`, single node `Some(0)`. Reviewer change: TODO endings reworded to 'Until you ...'.
- `assoc2`: Folds in the cut `operator-overloading` candidate, as the ROADMAP *(scope)* note asks. Part B covers `&Vector + &Vector` and `Vector + &Vector` (E0369, E0308); Part C is a generic `dot<T>` bounded by `Add<Output = T> + Mul<Output = T>` (E0308). A given `report` becomes E0283 once the trait takes a parameter. Mismatched dimensions panic with "dimension mismatch" (tested). `Sum` is not ported: the README now states the empty-iterator choice (panic, or a zero-dimensional `Vector(Vec::new())`), and the tests fold from `Vector::zeros(dim)`. `Neg`, `AddAssign` and `impl Mul<Vector> for f64` appear in the README only; E0600, E0368, E0277 and orphan legality were re-verified. ROADMAP tests present: annotated Fahrenheit 212, and `<Celsius as ConvertTo<Kelvin>>::convert` about 373.15. Reviewer changes: TODO endings; the Part C TODO no longer forbids changing the body; the fold-test comment was clarified; the hint's wrong `Display` reasoning was fixed.
- `gat1`: As in the ROADMAP. The given `next` body uses `get_mut(start..)?.get_mut(..size)?`, so even `usize::MAX` windows yield `None` without overflow. The tests drive the iterator through a generic `count<L: LendingIterator>`, which forces the provided trait to be implemented; that is why the unsolved build also has E0277. ROADMAP tests present: `[1, 2, 3, 4]` becomes `[1, 3, 6, 10]`, 3 windows of 3 over 5, and empty or oversized sizes yield none. The tests also cover: size 0 panics; `as_ptr` identity (no copies); writes carry into the next overlapping window; non-`Copy` `String` elements. The README explains why `collect` is impossible. Reviewer changes: header now says overlapping `&mut` is UB in unsafe code (it used to say 'never allowed even in unsafe'); TODO endings; hint wording on the workarounds.

#### Associated types vs generic parameters, and GATs (lending iterator)

- Slug `assoc-types-gats` · placement: graded · module: `43_assoc_types` (`assoc1`, `assoc2`, `gat1`) · interview value: **4/5**
- Sharpest question: Why is `Iterator::Item` an associated type while `Add<Rhs>` takes a generic parameter, and when would you choose each?
- Exercises:
  - `assoc1` — `trait Graph<N, E>` implemented for `AdjList`, while the given functions use `G::Node` and `Graph<Edge = u32>`; rewrite the trait with `type Node; type Edge;`. Fails unsolved: E0107 missing generics for trait `Graph` and E0220 associated type `Node` not found.
  - `assoc2` — `trait ConvertTo { type Target; }` with two impls for `Celsius`; make it `ConvertTo<T>`. README covers the E0283 inference cost and `Add<Rhs = Self> { type Output }` using both. Fails unsolved: E0119 conflicting implementations of `ConvertTo` for `Celsius`.
  - `gat1` — `WindowsMut<'s, T>` tries std `Iterator` with `Item = &'s mut [T]` for overlapping windows; implement the provided `LendingIterator { type Item<'a> where Self: 'a; }` and `prefix_sums_in_place` with `while let`. Fails unsolved: "lifetime may not live long enough" (the item cannot borrow from `&mut self`).
- Authoring notes:
  - Verified: E0107 plus E0220 for `G::Node` and `Graph<Edge = u32>`; two `ConvertTo` impls with different `Target` give E0119; `WindowsMut` as a std `Iterator` gives "lifetime may not live long enough".
  - A GAT without `where Self: 'a` gives "missing required bound on `Item`". Because the trait is provided, that is a tinker step, not a graded failure.
  - Tests: assoc1 `degree == 2`, `path_cost == Some(7)`, missing edge `None`, single node `Some(0)`; assoc2 annotated Fahrenheit 212 and `<Celsius as ConvertTo<Kelvin>>::convert` about 373.15; gat1 `[1, 2, 3, 4]` becomes `[1, 3, 6, 10]`, 3 windows of 3 over 5 elements, empty or oversized yields none; README explains why `collect` is impossible on a lending iterator.
  - *(scope)* Fold-in from the cut `operator-overloading` (put it in assoc2): operators take `self` by value, so `&a + &b` needs impls for `&T`; `Add<Rhs = Self> { type Output }`; and writing generic numeric code bounded by `T: Add<Output = T> + Copy`, a frequent junior live-coding ask. Verified codes if you port those items: E0369 for `&Vector + &Vector` without the reference impl, E0600 for unary `-`, E0368 for `+=`, E0277 for `Sum<&Vector>` and `From<Feet>`; `impl Mul<Vector> for f64` is orphan-legal. A ported `Vector` must define `Add` for mismatched lengths (panic or truncate) and choose a zero-dimension identity for `Sum` over an empty iterator; state both in the README or tests.

### `44_trait_contracts` (built)

Theme: the `Eq` / `Hash` / `Ord` contracts and floats (the `std::ops` half of this module was cut; its useful parts moved to `43_assoc_types`).

**As built** (see `exercises/44_trait_contracts/`; adversarially reviewed). Deviations and verified corrections:

- `contracts1`: Built as the ROADMAP specifies: `Header` plus a small `Headers` map. Hash equality is tested with DefaultHasher::new(). An in-test byte-recording `Hasher` checks consistency, information content and prefix-freeness without hard-coded hash values, and both the 0xff terminator and a length prefix are accepted. Review change: `different_names_feed_different_bytes` now also rejects folds looser than `==` (`byte | 0x20`, Unicode `to_lowercase`). Only `equal_headers_hash_equally` fails deterministically when unsolved. The three RandomState tests fail with high probability, and the unsolved binary failed 20 out of 20 runs. The ROADMAP puts clippy's lint first in its 'Fails unsolved' line, but rustlings runs clippy only after the tests pass, so the learner sees the test failure first.
- `contracts2`: ROADMAP deviation that cannot be avoided: a type cannot have both a derived `Ord` and a hand-written `Ord` (E0119). So one `Job` shows both starter failures: a derived `PartialOrd` with `name` declared first, plus a hand-written priority-only `Ord`. The ROADMAP's 'priority descending' becomes 'higher priority is greater' for the max-heap, so a `BTreeSet` lists jobs in reverse run order. Review change: the contains test now uses `insert` and shows the ROADMAP's 'contains() is true for a job never inserted'. The collect-sort-then-search mismatch moved to its own test, `a_collected_btree_set_finds_its_own_jobs`. There are 10 tests in total: 9 fail unsolved, and all 10 pass on the solution.
- `contracts3`: Goes beyond the ROADMAP entry. `TotalF64` must also be `Hash` (via `to_bits`), checked by a `HashSet` test and a nine-distinct-hashes test. There are also a `BTreeSet` test, a negative-NaN test (sorts first), an in-place (as_ptr) check for `sorted`, `k_smallest` edge cases (k = 0, k > len, usize::MAX, NaN, zeros) with a 500-value cross-check, and Dijkstra checked against an in-test Bellman-Ford on a 6x6 grid. NaNs are built with f64::from_bits, as the authoring note requires. Besides E0277 `f64: Ord`, the unsolved build shows E0599s and the `TotalF64` errors coming from the tests. The header now lists (E0277, E0599). The optional `top_k` fold-in was not added here, and `63_slices_strings` does not have it either.
- The optional `top_k<'a>` fold-in from the cut `heaps-topk-dijkstra` was built in neither `44_trait_contracts` nor `63_slices_strings`; it is unassigned.

#### Eq, Hash and Ord contracts and floats: consistent Hash, Ord that agrees with Eq, total_cmp

- Slug `eq-hash-ord-contracts` · placement: graded · module: `44_trait_contracts` (`contracts1..3`) · interview value: **4/5**
- Sharpest question: Why doesn't `Vec<f64>::sort()` compile, and how do you put f64-keyed items into a BinaryHeap as a min-heap?
- Exercises:
  - `contracts1` — `#[derive(Hash, Eq)] Header(String)` with a case-insensitive `PartialEq`; hand-write `Hash` over the ASCII-lowercased bytes plus a `0xff` terminator. Fails unsolved: clippy's deny-by-default `derived_hash_with_manual_eq` rejects the starter; once the derive is removed, a naive `Hash` fails `hash("Content-Type") == hash("content-type")` (deterministic `DefaultHasher::new()`).
  - `contracts2` — `Job { name, priority, submitted }` in a `BinaryHeap` and a `BTreeSet`, starting from a derived `Ord` with fields in the wrong order and a manual `Ord` that compares only priority; write `Ord` as priority descending, then FIFO by `submitted`, then `name`, so it agrees with the derived `Eq`, and make `PartialOrd` return `Some(self.cmp(other))`. Fails unsolved: the heap pops by name; `BTreeSet::insert` drops the second equal-priority job (len 2 of 3) and `contains()` is true for a job never inserted.
  - `contracts3` — `sorted(v: Vec<f64>)` uses `v.sort()`, `k_smallest` uses `BinaryHeap<f64>`, and a Dijkstra-style `State { cost: f64, node }` goes into a heap; use `sort_by(f64::total_cmp)`, a `TotalF64` wrapper (`PartialEq` via `total_cmp(..).is_eq()`, `Ord` via `total_cmp`) and `Reverse` for a min-heap. Fails unsolved: E0277 the trait bound `f64: Ord` is not satisfied.
- Authoring notes:
  - contracts3: `f64::NAN`'s bit pattern, including its sign, is not guaranteed, and on x86 SSE `0.0 / 0.0` produces a negative NaN at runtime. The expected "NaN sorts last under `total_cmp`" needs a positive NaN: build it with `f64::from_bits(0x7ff8_0000_0000_0000)` (or `f64::NAN.abs()`).
  - contracts3 tests: `[1.0, NaN, -0.0, 0.0, -inf, -1.0, inf]` sorts to `[-inf, -1.0, -0.0, 0.0, 1.0, inf, NaN]` compared by `to_bits`; `TotalF64(NaN)` equals itself; `-0.0 != 0.0`; `k_smallest([5, 1, 4, 2], 2) == [1, 2]`.
  - Verified: `derived_hash_with_manual_eq` is deny-by-default and fires on the `derive(Hash)` + manual `PartialEq` starter; `v.sort()` and `BinaryHeap<f64>::push` both give E0277 `f64: Ord`; `DefaultHasher::new()` is deterministic per the docs; the priority-only `Ord` really does collapse `BTreeSet` entries.
  - The contracts2 solution must stay clean under clippy's `derive_ord_xor_partial_ord` and `non_canonical_partial_ord_impl`.
  - This candidate already absorbs the heap proposals' `Reverse` / min-heap / f64 Dijkstra-state lessons and the dropped Job-scheduler `Ord` exercises (`heap1`, `heap3`).
  - Optional fold-in from the cut `heaps-topk-dijkstra`: its `heap1`, `fn top_k<'a>(words: &[&'a str], k: usize) -> Vec<&'a str>`, can live here or in `slice-string-algorithms`. Verified: the elided `fn top_k(words: &[&str], k) -> Vec<&str>` gives E0106, and the results must outlive the dropped input `Vec` (the `&'a [&'a str]` attempt gives E0597).

### `45_sized_deref` (Deref, Borrow and Cow part) (built)

Theme: Deref coercion and its limits, `Borrow<Q>` lookups, `Cow`-returning APIs. The `?Sized` half of this module (`sized1..3`) is in [Tier 2](#45_sized_deref-sized-part-built).

**As built** (see `exercises/45_sized_deref/`; adversarially reviewed). Deviations and verified corrections:

- `deref1`: Follows the ROADMAP entry. The tests also show where coercion stops (operator, pattern, generic parameter, Display) using an explicit `&*name`, which adds E0614 to the starter errors. `&Rc<Username>` is a second multi-step coercion alongside `&Box<Username>`. `Target = str` and "no DerefMut" are checked at run time by a test-only inherent-const probe (concrete types, no macro). Review changes: the header uses the real `shout(text: S)` signature and mentions C-DEREF; the TODO explains the `Box<Username>` E0599 wording.
- `deref2`: Follows the ROADMAP entry. The user's greeting has a nickname rule, so a test can tell delegation (reusing `User::greet`) from rebuilding the text. A `Box<dyn Greet>` cast is tested alongside the `&dyn Greet` one. A test-only probe checks that `Admin` no longer implements `Deref`. No changes in review.
- `borrow1`: Follows the ROADMAP entry, plus the layout6 fold-in as Part B (`HashSet<Rc<String>>` becomes `HashSet<Rc<str>>` with `Rc::from(s)`, checked with `Rc::ptr_eq` and strong_count 3). Extra tests: a given `score_of`, a deterministic thread-local hash-count test that rejects scans and double lookups, a drop-the-key-first lifetime test, std `contains_key`/`Index`/`remove`, and a Borrow-contract hash test with an unkeyed hasher. Correction to the ROADMAP fail mode: the `UserId` lookups are E0308 "expected `&UserId`, found `&str`" and stay E0308 after `lookup` is generic. Review change: the interner test now also asserts `interner.set.contains("hello")`, so `Vec` or `HashMap<String, Rc<str>>` storage no longer passes.
- `cow1`: Follows the ROADMAP entry (clean input is Borrowed and `ptr::eq` to the input, "a b" becomes Owned("a b"), "" is Borrowed). Part B `normalize` (trim, then collapse) absorbs the dropped perf2 `normalize()` Cow return: a clean input must come back as a borrowed slice that starts right after the leading whitespace (pointer checked). The starter also has E0599 (`to_mut`, `into_owned` on `String`), so the header lists (E0308, E0599). No changes in review.

#### Deref coercion and its limits, Deref-as-inheritance, Borrow lookups, Cow-returning APIs

- Slug `deref-borrow-cow` · placement: graded · module: `45_sized_deref` (`deref1`, `deref2`, `borrow1`, `cow1`) · interview value: **4/5**
- Sharpest question: Why can you call `map.get("k")` on a `HashMap<String, V>`, and what is the difference between `AsRef<str>` and `Borrow<str>`?
- Exercises:
  - `deref1` — validated `Username(String)` with no `Deref`; tests call `takes_str(&name)`, `name.len()` and `takes_str(&boxed)`; implement `Deref<Target = str>` and deliberately no `DerefMut`. Fails unsolved: E0308 expected `&str`, found `&Username`; E0599 no method `len`.
  - `deref2` — `Admin` derefs to `User` for "inheritance", and `welcome<T: Greet>(&admin)` plus a `Vec<&dyn Greet>` fail; remove the `Deref`, add `impl Greet for Admin` by delegation plus `fn user(&self)`. Fails unsolved: E0277 `Admin: Greet` not satisfied (for both the generic call and the dyn coercion) although `admin.greet()` auto-derefs.
  - `borrow1` — `lookup(map: &HashMap<K, V>, key: &K)` is called with `String`/`&str`, `Vec<u8>`/`&[u8]`, `PathBuf`/`&Path`, and a `HashMap<UserId, u32>` with `"alice"`; copy `HashMap::get`'s signature (`K: Borrow<Q>, Q: Hash + Eq + ?Sized`) and add `impl Borrow<str> for UserId`. Fails unsolved: E0308 expected `&String`, found `&str`.
  - `cow1` — `collapse_spaces(s: &str) -> String` always allocates, and the tests match on `Cow::Borrowed`; return `Cow<'_, str>`, borrowing when nothing changes. Fails unsolved: E0308 mismatched types.
- Authoring notes:
  - borrow1: the starter error is a plain E0308 on the `key: &K` parameter type. "Inference picks reflexive `Borrow`" does not explain it; reflexive `Borrow` only matters once the signature is generic over `Q` and the caller passes `&String`.
  - Verified: Deref-for-inheritance gives E0277 `Admin: Greet` for both the generic call and the `&Admin` to `&dyn Greet` cast, even though `admin.greet()` auto-derefs. deref1's E0308/E0599 are the standard errors.
  - `UserId(String)`'s `Borrow<str>` works because a derived `Hash` on a single-field newtype hashes exactly like `str`. README: `String` is `AsRef<[u8]>` but not `Borrow<[u8]>` (the hashes differ), and a case-insensitive key must not implement `Borrow<str>`.
  - Keep borrow1 as the canonical `K: Borrow<Q>` lesson; `lru3` applies it rather than re-teaching it. The `AsRef` side of "AsRef vs Borrow" is already in `23_conversions/conversions5`.
  - cow1 tests: clean input is `Borrowed` and `ptr::eq` to the input; `"a   b"` becomes `Owned("a b")`; `""` is `Borrowed`. Absorbs the dropped `perf2` `normalize()` Cow return.
  - Fold-in from the cut `layout-niche-padding` (its `layout6`, as a borrow1 extra): `Interner { set: HashSet<Rc<String>> }` with `intern(&mut self, s: &str)` doing `set.get(s)` fails with E0277 "the trait bound `Rc<String>: Borrow<str>` is not satisfied" (verified); switch to `HashSet<Rc<str>>` and `Rc::from(s)`, and assert `Rc::ptr_eq` for repeated strings.

### `47_type_level` (built)

Theme: runtime-checked builder, then typestate (the derive-bound and const-generics half was cut).

**As built** (see `exercises/47_type_level/`; adversarially reviewed). Deviations and verified corrections:

- `builder1`: Implements the ROADMAP entry. The fail mode matches it (E0308 for &str vs String and the empty build, E0599 for tags, E0599 for default; on 1.96 the E0599 wording is "no associated function or constant named `default`"). Extras stated in the TODOs and tested: a blank host counts as missing, the host is checked before workers, owned host and tag Strings are moved (as_ptr identity), tags append, and the last setter call wins. `#[must_use]` is graded through `#[deny(clippy::return_self_not_must_use)]` on the builder impl. rustlings always runs clippy after the tests, and a deny-level lint fails it even without strict_clippy, so no info.toml flag is needed. Reviewer changes: header line 1 now names the clippy lint, and the struct-update paragraph now states the real run-time cost and E0451. Tests are unchanged.
- `typestate1`: Implements the ROADMAP entry: `PhantomData<S>`, ZST markers NoUrl/HasUrl, a sealed State trait, url() only on NoUrl, header() on every S, send() -> Request only on HasUrl, Default only for `RequestBuilder<NoUrl>`, a size_of equality test, and a url + 2 headers test. The fail mode deviates: the tests also name `State` and `sealed::Sealed`, so the first compile shows E0425/E0405/E0433 plus 2 E0107s, and the remaining E0107s, E0283 and E0308 appear after TODO 1. The TODOs say so. Reviewer additions: two tests, a_builder_without_a_url_has_no_send and the_url_cannot_be_set_twice, use a test-local `&self` trait-method fallback and TypeId to prove that no inherent send()/url() exists in the wrong state. This catches send()/url() implemented for every state, which passed all tests before. The real negative guarantee (a compile_fail doctest) stays with the planned api-surface-lab, as the README and header say. TODO endings now follow the 'Until you ...' convention. The solution passes 11/11 tests.

#### From runtime-checked builder to typestate

- Slug `builder-typestate` · placement: graded · module: `47_type_level` (`builder1`, `typestate1`) · interview value: **4/5**
- Sharpest question: Design a `RequestBuilder` so that calling `send()` without `url()` is a compile error. What does it cost at runtime, and why seal the state trait?
- Exercises:
  - `builder1` — `ServerConfig` lacks `Default`, the builder's setters take `String`, there is no `tags()`, and `build()` is empty; hand-write `Default` (8080, 4 workers, 30 s), take `impl Into<String>`, add `tags<I: IntoIterator>(I) where I::Item: Into<String>`, return `Result<_, BuildError>` (`MissingHost`, `ZeroWorkers`), mark the builder `#[must_use]`. Fails unsolved: E0308 (`&str` vs `String`; the empty `build` body), E0599 no method `tags`, E0599 no `default`.
  - `typestate1` — a runtime `RequestBuilder { url: Option<String> }` becomes `RequestBuilder<S>` with `PhantomData<S>` and ZST markers `NoUrl` / `HasUrl` behind a sealed `State` trait: `url()` only on `NoUrl`, `header()` on every `S`, `send() -> Request` only on `HasUrl`, `Default` for `RequestBuilder<NoUrl>`. Fails unsolved: E0107 struct takes 0 generic arguments; E0425 cannot find type `NoUrl`; E0308 `Result` vs `Request`.
- Authoring notes:
  - On rustc 1.96 a missing marker type is E0425 "cannot find type `NoUrl`" (E0412 on older rustc); a generic argument on a non-generic struct is E0107; `ServerConfig::default()` without an impl is E0599.
  - Graded bins can only check that correct code compiles, so the negative check ("`send()` without `url()` must not compile") lives in `api-surface-lab` as a `compile_fail` doctest.
  - Tests: defaults fill in; `&str` and `String` are both accepted; tags extend; missing host and `workers(0)` return errors; struct update `..ServerConfig::default()` works; the typestate request has its url and 2 headers; the `NoUrl` and `HasUrl` builders have the same `size_of`.

### Extend `35_error_design` (built)

Theme: context chains, `Send + Sync` errors, downcasting, an anyhow-style `Report` (`err4..6`).

**As built** (see `exercises/35_error_design/`): `err4` gives a `From<Report> for BoxError` impl, so the shortcut "implement `Error` for `Report`" is rejected with E0119. `err6` also has the learner write `From<Report> for BoxError` (the anyhow hand-off). `err4`'s and `err6`'s `Report` types are separate structs; each file stands alone.

#### Production errors: context chains, `Box<dyn Error + Send + Sync>`, downcasting, why anyhow's error type isn't `Error`

- Slug `error-context-send-downcast` · placement: graded · module: extend `35_error_design` (`err4..6`) · interview value: **4/5**
- Sharpest question: Why do applications use `Box<dyn Error + Send + Sync + 'static>` (or anyhow), how do you add context without losing the cause, and why doesn't `anyhow::Error` implement `std::error::Error`?
- Exercises:
  - `err4` — `Report { msg, source: Option<BoxError> }` and `trait Context<T>` have no impls and `chain()` is empty; add a blanket impl for `Result<T, E: Error + Send + Sync + 'static>`, a second impl for `Result<T, Report>` (legal because `Report` isn't `Error`), and a `chain()` that walks `source()`. Fails unsolved: E0599 no method named `context` found for enum `Result`; E0308 from the empty `chain`.
  - `err5` — `type BoxError = Box<dyn Error>` returned from a worker run in `thread::spawn` and joined; change it to `Box<dyn Error + Send + Sync + 'static>` and recover the cause with `downcast_ref`. Fails unsolved: E0277 "`dyn std::error::Error` cannot be sent between threads safely".
  - `err6` — `Report(Box<dyn Error + Send + Sync>)` implements `Error`, and the starter adds `impl<E: Error + ..> From<E> for Report`; resolve it by removing `impl Error` and exposing `AsRef<dyn Error + Send + Sync>`. Fails unsolved: E0119 "conflicting implementations of `From<Report>` for `Report`" (conflicting impl in crate `core`).
- Authoring notes:
  - Verified: the blanket `impl<T, E: Error + Send + Sync + 'static> Context<T> for Result<T, E>` and `impl<T> Context<T> for Result<T, Report>` coexist because `Report` is local and not `Error`; a thread returning `Result<_, Box<dyn Error>>` gives the E0277 above; `impl<E: Error> From<E> for Report` with `Report: Error` gives E0119 against core's `From<T> for T`. This is an accurate lesson on why `anyhow::Error` can't implement `Error`.
  - Tests: `load("80x").chain() == ["loading config", "parsing port", "invalid digit found in string"]` with a `ParseIntError` root cause; err5's joined error downcasts to `ParseIntError` and `"0"` gives "zero is not allowed"; in err6 `?` accepts both `ParseIntError` and `ParseFloatError`, `as_ref().downcast_ref::<ParseIntError>()` works, and `run("2") == Ok(3)`.
  - Hint: libraries expose matchable (`#[non_exhaustive]`) error enums; applications use opaque reports.

### `50_testing_seams` (built)

Theme: trait seams, `RefCell` mocks, injected clocks (token-bucket limiter, TTL cache).

**As built** (see `exercises/50_testing_seams/`; adversarially reviewed). Deviations and verified corrections:

- `seams1`: Deviations from the ROADMAP entry, all kept from the builder and verified. The mock sits outside `mod tests`, with a comment saying where it would normally live, so the learner can fix it without editing the tests. Additions: a one-shot scripted failure (`Cell<Option<MailError>>` plus `take` for a non-Copy value), a `&self` service method `resend_confirmation`, and a shared-reference test; together they reject rustc's suggestion to change the trait to `&mut self`. The starter reports E0594 alongside the E0596 the ROADMAP names. Review change: the header now points to smartptr3/cell1 instead of re-teaching the `Cell` API.
- `seams2`: Deviations from the ROADMAP entry, verified and kept. (1) Fail mode has two stages: first E0277 in the test build (the `Rc<Cell<Duration>>` fake clock and the `RefCell` per-key map are not Send/Sync), then, once Part B is fixed, 10 of 16 Part A tests fail, including the ROADMAP's assertion (400 ms polls at 1 token/s: `left: []`). The ROADMAP 'Fails unsolved' line should mention both stages. (2) Following the authoring note, the concurrent test needs a thread-safe fake; the learner converts the single `FakeClock` instead of the file adding a second `SyncFakeClock`. (3) The tests require the textbook `min(capacity, ..)` semantics (time spent full is lost). The ROADMAP's literal fix ('advance `last` by `added * refill_every`, clamp') fails 4 tests; the ROADMAP text should say 'unless the bucket is full, then `last = last.max(now)`'. (4) Backwards clock readings use high-water-mark semantics. Review change: added `a_full_bucket_does_not_bank_part_of_an_interval_either` (16 tests now) and made the idle-hour test fractional (3600.5 s), so skipping the full reset when `added == 0` no longer passes.
- `seams3`: Deviations from the ROADMAP entry, verified and kept. The given `get` signature is `get<Q>(&mut self, key: &Q) where K: Borrow<Q>, Q: Hash + Eq + ?Sized`, so the tests can look up a `&str` in a `String`-keyed map. `insert` returns the replaced value only if it was still live, and an overflowing deadline (`Duration::MAX`) means never. The README shows the rejected single-lookup `get` (checked: E0502) and cross-references `borrowck3`. Review change: only the hint's let-chain explanation.

#### Testing seams: trait-injected dependencies, RefCell mocks, injected clocks

- Slug `test-doubles-fake-clock` · placement: graded · module: `50_testing_seams` (`seams1..3`) · interview value: **4/5**
- Sharpest question: Design a token-bucket rate limiter and unit-test it deterministically without sleeping. How do you inject the clock (generic or dyn), and why does the mock need RefCell or Mutex?
- Exercises:
  - `seams1` — `trait Mailer { fn send(&self, ..) }` and `SignupService<M: Mailer>`; the test's `MockMailer` pushes into a `Vec` through `&self`; use `RefCell<Vec<_>>` (and `Cell` counters). Fails unsolved: E0596 "cannot borrow `self.sent` as mutable, as it is behind a `&` reference".
  - `seams2` — `TokenBucket<C: Clock>` with a fake clock; the starter refills with `elapsed / refill_every` and then sets `last = now`, dropping remainders. Advance `last` by `added * refill_every`, clamp at capacity, and make a `Mutex`-wrapped per-key limiter `Send + Sync`. Fails unsolved: assertion, at 1 token/s, polling every 400 ms for 3 s never succeeds.
  - `seams3` — `TtlMap<K, V, C: Clock>` with `insert(k, v, ttl)`, a `get(&mut self)` that lazily removes expired entries (`now >= expires_at`), and `purge_expired` via `retain`. Fails unsolved: E0308 from the empty bodies.
- Authoring notes:
  - Not feasible as proposed: `FakeClock { offset: Rc<Cell<Duration>> }` is `!Send` and `!Sync`, so a `TokenBucket<FakeClock>` inside a `Mutex` can't be shared with the "8 threads, same `now`, exactly `capacity` successes" test. The concurrent test needs a thread-safe fake (e.g. `Arc<AtomicU64>` nanos or `Arc<Mutex<Duration>>`), or a second `SyncFakeClock`.
  - Verified: E0596 for the seams1 mock; the seams2 refill-remainder bug is deterministic with a fake clock (400 ms polls at 1 token/s never refill); seams3's empty bodies give E0308.
  - seams2 tests: burst to capacity then deny; refill at exactly `refill_every`; an idle hour still leaves at most `capacity`; a clock going backwards mints nothing; no sleeps anywhere. seams3 tests: before, at and after expiry (the boundary is inclusive); overwrite refreshes the TTL; purge count; an expired value is never returned.
  - The seams3 README shows the rejected single-lookup early return (problem case #3): cross-reference `borrowck3` rather than re-explaining it.
  - README: don't make the trait take `&mut self` just for a test; `Send` doubles need a `Mutex`.
  - The `impl Mailer for &T` forwarding scenario (the dropped `tests5`) lives in `sized2`; it duplicates this module's mock, so keep only one.

### `51_scoped_threads` (built)

Theme: `thread::scope`, disjoint `&mut` chunks, `Barrier`.

**As built** (see `exercises/51_scoped_threads/`; adversarially reviewed). Deviations and verified corrections:

- `scope1`: Deviation (builder, kept): `parallel_sum(data: &[u64])` from the ROADMAP is a wrapper around a `parallel_sum_with(data, on_chunk: &(dyn Fn(&[u64]) + Sync))` test seam. The seam lets the tests reject copies, sequential sums and serialized spawn-then-join. All the ROADMAP tests are present, plus an address/thread-identity test and a 20 s Condvar meeting test. Review fixes: two imprecise header sentences reworded, and the hint's panic statement corrected.
- `scope2`: Deviation (builder, kept): `scale_in_place(data, factor, n)` is a wrapper around a `scale_in_place_with(.., on_chunk: &(dyn Fn(&[u64]) + Sync))` test seam. The authoring note is applied: `len.div_ceil(n.max(1)).max(1)`. The ROADMAP's len 0/1/7/1000 x n 0/3/4/64 matrix is fully covered, split across named tests. Tests added: tiling of the caller's memory, at most max(n,1) chunks each on its own thread, 4 busy chunks for 1000 elements on 4 threads, a 20 s meeting with a give-up flag, and a #[should_panic] overflow test. The unsolved build also warns that `n` is unused (intended; the TODO mentions it).
- `scope3`: Kept despite the *(scope)* note that scope3 is niche; the note does not say to drop or merge it. The seam is `prefix_sum_with(data, n, before_publish: &(dyn Fn(usize) + Sync))`, called between the given phase-1 scan and the store. Each call runs on a plain `thread::spawn` helper under a 20 s `recv_timeout` watchdog, so a wrongly sized or per-worker Barrier fails instead of hanging. The unsolved failure is deterministic (phase 2 is missing). Catching a missing or misplaced barrier depends on timing, as the ROADMAP allows. Review fix: two inaccurate hint statements corrected.

#### Scoped threads: borrowing stack data, disjoint `&mut` chunks, Barrier phases

- Slug `scoped-threads` · placement: graded · module: `51_scoped_threads` (`scope1..3`) · interview value: **4/5**
- Sharpest question: Why does `thread::spawn` require `Send + 'static`, how does `thread::scope` soundly relax that, and how do several threads mutate disjoint parts of one `&mut [T]`?
- Exercises:
  - `scope1` — `parallel_sum(data: &[u64])` splits the slice and calls `thread::spawn` over the borrowed halves; use `thread::scope` with two scoped spawns. Fails unsolved: E0521 borrowed data escapes outside of function (the owned-local variant gives E0373).
  - `scope2` — `scale_in_place(data: &mut [u64], factor, n)` with two scoped closures indexing the same `&mut` slice; split with `chunks_mut` and move each chunk into its thread. Fails unsolved: E0499 "cannot borrow `*data` as mutable more than once at a time".
  - `scope3` — two-phase parallel prefix sum: phase 1 (per-chunk scan plus totals in `AtomicU64`s) is given; add a shared `Barrier` and the phase-2 offsets. A test hook delays chunk 0. Fails unsolved: `assert_eq!` fails because chunk scans restart at 0 without phase 2, and without the barrier the delayed chunk makes the offsets wrong.
- Authoring notes:
  - Not feasible as proposed: the reference fix `chunks_mut(len.div_ceil(n).max(1))` panics with "attempt to divide by zero" for `n == 0`, and the proposed tests include `n = 0`. Use `len.div_ceil(n.max(1)).max(1)`; the outer `max(1)` still guards `chunks_mut(0)` when `len == 0`.
  - scope2 tests: match the sequential result for len 0/1/7/1000 and n 0/3/4/64 (len 0 catches the `chunks_mut(0)` panic). scope1 tests: sum of `1..=10_000`, empty is 0, odd length, a stack array input, and the `Vec` mutated after the call.
  - dev/Cargo.toml sets clippy `disallowed_methods = "allow"`, so the root `clippy.toml` ban on `Scope::spawn` does not apply to exercises.
  - scope3's delay hook can only make wrong solutions fail, so it is safe to keep.
  - README: why the guard-based `thread::scoped` was unsound (`mem::forget`).
  - *(scope)* scope1 and scope2 carry the value; scope3 (Barrier prefix sum) is niche.

### `52_condvar` (built)

Theme: `Condvar` queues and a semaphore.

**As built** (see `exercises/52_condvar/`; adversarially reviewed). Deviations and verified corrections:

- `condvar1`: Follows the ROADMAP entry and all its authoring notes: FIFO; consumer blocks until waiters == 1, then gets 7; still not finished after a bare notify_all (250 ms window); 4x4 MPMC stress test with a count, uniqueness and per-producer order check; 20s watchdogs. The builder's design keeps the waiter count part of the protocol: `push` notifies only when waiters > 0. The review changed no code in this exercise, only the hint.
- `condvar2`: Follows the ROADMAP entry (`Ok, Ok, Err(3)` at capacity 2; a blocked producer released by one pop; "push did not block when the queue was full"). The review added the test `two_quick_pops_let_in_both_blocked_producers`, because a pop that notifies only on the full-to-not-full transition passed every test before. The solution's `pop` comment now explains why every pop notifies. The builder's additions are kept: capacity > 0 assert, fill-to-exact-capacity test, bare notify_all on not_full, one pop admits one of two producers, try_push wakes a consumer, and the 4x4 single-slot stress test.
- `condvar3`: Follows the ROADMAP entry: 8 threads on 3 permits with the peak measured by `fetch_max`, and a panicking holder returns its permit. The ROADMAP's 'fourth try_acquire' check sits in `dropping_a_permit_gives_it_back`, right after an `available() == 1` assertion, which is the first thing to fail. The review added the test `two_permits_dropped_back_to_back_wake_two_waiters`, because a `Drop` that notifies only when the count was 0 passed every test before. The solution's `Drop` comment now explains why every drop notifies.

#### Condvar: blocking queue with wait-in-a-loop, bounded queue with two condvars, RAII semaphore

- Slug `condvar-blocking-queue` · placement: graded · module: `52_condvar` (`condvar1..3`) · interview value: **4/5**
- Sharpest question: Implement a bounded blocking MPMC queue with Mutex + Condvar. Why must `wait` be in a loop, and why two condvars?
- Exercises:
  - `condvar1` — `BlockingQueue<T> { inner: Mutex<Inner { items, waiters }>, not_empty }` with `push` given and `pop` empty; wait in a `while` loop (or `wait_while`), counting waiters. Fails unsolved: E0308 from the empty `pop`; a solution using `if` instead of `while` fails the simulated spurious-wakeup test (`notify_all` without a push).
  - `condvar2` — `BoundedQueue<T>` with `not_empty` / `not_full`, where `push` ignores capacity and `try_push` always returns `Ok`; implement `try_push -> Err(item)` when full, `push` waiting on `not_full`, and `pop` notifying `not_full`. Fails unsolved: `try_push` at capacity returns `Ok`; the watchdog panics "push did not block when the queue was full".
  - `condvar3` — `Semaphore { permits: Mutex<usize>, cv }` with `acquire() -> Permit<'_>`, `try_acquire`, and a `Permit` without `Drop`; add waiting and a `Drop` that returns the permit and notifies. Fails unsolved: peak concurrency exceeds the permit count, and the fourth `try_acquire` fails because permits never return.
- Authoring notes:
  - The if-vs-while check and the "push did not block" check are sleep-window observations. Under heavy load they can let a wrong learner solution through, but they can never fail a correct one. Use generous watchdogs (tens of seconds): rustlings runs `cargo test` with no timeout, and `dev check` runs every exercise in parallel.
  - Tests: condvar1 FIFO, the consumer blocks until `waiters == 1` then gets 7, after a bare `notify_all` the consumer is still not finished, a 4x4 MPMC stress sum/count; condvar2 `Ok, Ok, Err(3)` at capacity 2 and a blocked producer released by one pop; condvar3 with 8 threads and 3 permits keeps peak at most 3 (measured with `fetch_max`), and a panicking holder still returns its permit.

### `53_lock_hazards` (built)

Theme: lock-ordering deadlocks and `RwLock` semantics.

**As built** (see `exercises/53_lock_hazards/`; adversarially reviewed). Deviations and verified corrections:

- `deadlock1`: Implements the ROADMAP entry, with every authoring note applied (verified). The deterministic SameAccount re-lock test is kept. Watchdogs use recv_timeout and fire only on the unsolved path. The 2 x 100_000 opposite-transfer test uses a progress watchdog. Extra tests: two try_lock probes where the test holds account 1, an unrelated-transfer check that rejects a bank-wide lock, and a concurrent `total()` auditor. Review changes: two header wording fixes (try_lock back-off, debugger advice). Tests and code are unchanged.
- `rwlock1`: Implements the ROADMAP entry, with every authoring note applied (verified): Barrier(4) inside `with_read` with a 'readers were serialized' watchdog, a writer's update is visible, no double insert (racing_misses_run_make_once), and the RwLock Send + Sync bound in the header and README. Review change: added a 7th test, `a_write_waits_for_readers_to_leave`, because two stores that hold no lock while `f` runs passed all 6 original tests (Mutex<Arc> and RwLock<Arc> copy-on-write). The doc comment and TODO now state that `set` waits for readers. Header claims about cost, platform priority and write-under-own-read were corrected. The unsolved failures are unchanged (the new test passes on the Mutex starter).

#### Deadlocks by lock order and RwLock semantics

- Slug `lock-ordering-rwlock` · placement: graded · module: `53_lock_hazards` (`deadlock1`, `rwlock1`) · interview value: **4/5**
- Sharpest question: Two threads transfer money between accounts A and B in opposite directions and deadlock. Why doesn't Rust prevent this, and how do you fix it?
- Exercises:
  - `deadlock1` — `Bank { accounts: Vec<Mutex<u64>> }` whose `transfer(from, to, amt)` locks `from` then `to`; reject `from == to`, lock in index order, check funds. Fails unsolved: the watchdog panics "deadlock detected" when 2 threads each run 100_000 opposite transfers on a helper thread.
  - `rwlock1` — `ConfigStore { map: Mutex<HashMap<..>> }` with `with_read(f)`; switch to `RwLock` (`read` / `write`); `get_or_insert_with` must re-check under `write()` because std has no upgradable read. Fails unsolved: 4 readers each wait on a `Barrier(4)` inside `with_read`, a `Mutex` serializes them, and the watchdog reports "readers were serialized".
- Authoring notes:
  - README correction: `impl Sync for RwLock<T>` requires `T: Send + Sync`, not just `T: Sync`.
  - deadlock1's opposite-order deadlock is probabilistic (the naive order deadlocked in 5 of 5 proposal runs). The `SameAccount` re-lock case always fails the unsolved starter, either by deadlock plus watchdog or by panic (std documents that re-locking "might panic or deadlock"), so fail-while-unsolved is deterministic. Keep that test.
  - In rwlock1, `Barrier(4)` inside `with_read` deterministically deadlocks a `Mutex`, which makes a good watchdog trigger.
  - Watchdogs (`recv_timeout`) fire only on the unsolved path, because rustlings runs `cargo test` with no timeout. Unjoined deadlocked threads die at process exit, so the test binary terminates. Size the solution-side watchdog for 2 x 100k transfers under parallel `dev check` load.
  - Tests: total money is conserved; insufficient funds leaves balances untouched; `SameAccount` is an `Err`; all 4 readers are inside concurrently; a writer's update is visible; no double insert.
  - README: OS-dependent reader/writer priority; a recursive read may deadlock.
  - A natural extension is lock striping and hot-swapped config; see [Additional topics](#additional-topics-not-yet-verified).

### Extend `31_debugging` (built)

Theme: guards kept alive by `match` / `while let` / `if let` scrutinee temporaries (`debugging6..8`).

**As built** (see `exercises/31_debugging/`): `debugging6` Part B absorbs the RefMut field-split trick from the cut `leetcode-rc-refcell-trees`, and `debugging7` Part B absorbs `lc_tree4`. `debugging8` catches the self-deadlock with `try_lock` (it can never hang), adds a "first stored value wins" requirement, and ends with auto-graded constants on where each guard dies. On 1.96 the RefCell panic message is "RefCell already borrowed"; the older `debugging4` hint still quotes the previous wording.

#### Guards kept alive by scrutinee temporaries: RefCell panics and Mutex self-deadlock

- Slug `scrutinee-guard-temporaries` · placement: graded · module: extend `31_debugging` (`debugging6..8`) · interview value: **4/5**
- Sharpest question: Why does `match cache.lock().unwrap().get(&k) { None => { cache.lock()...insert(..) } .. }` deadlock, and how do you shorten the guard's lifetime?
- Exercises:
  - `debugging6` — `match self.q.borrow_mut().pop_front() { Some(job) => { /* push follow-ups via self.q.borrow_mut() */ } .. }`; bind the popped value first, then match. Fails unsolved: the test panics with "RefCell already borrowed".
  - `debugging7` — `while let Some(job) = q.borrow_mut().pop_front() { /* re-queue via q.borrow_mut() */ }`; rewrite as `loop { let Some(job) = .. else { break }; .. }` (let-else drops temporaries at the end of the statement). Include an if-let then-branch variant, which still holds the `Ref` in 2024. Fails unsolved: panics with "RefCell already borrowed" on the first re-queue.
  - `debugging8` — `match cache.lock().unwrap().get(&k).cloned() { None => { compute; cache.try_lock().expect(..).insert(..) } .. }`, plus quiz constants `MATCH_ARM_CAN_RELOCK`, `IF_LET_THEN_CAN_RELOCK`, `IF_LET_ELSE_CAN_RELOCK_2024`, `LET_STMT_CAN_RELOCK`; release the guard before computing. Fails unsolved: `try_lock` returns `Err(WouldBlock)` and `expect` panics; the constants start undefined (E0425) and are then checked against `try_lock` probes.
- Authoring notes:
  - Verified on 1.96: the match-scrutinee `borrow_mut` and the while-let `borrow_mut` both panic with "RefCell already borrowed" (not "already borrowed: BorrowMutError"); the let-else loop rewrite works; `try_lock` probes in the match arm, the if-let then-branch, the if-let else-branch (2024) and after a `let` statement give false, false, true, true, exactly the proposed constants.
  - Tests: debugging6 leaves exactly the follow-ups in order; debugging7's processed count and final queue match; debugging8 computes on the first call and hits the cache on the second (counter 1).
  - *(scope)* Make this the single home for the edition-2024 if-let rescoping lesson; `drop-order-quiz` drops its scrutinee scenarios.
  - Absorbs the dropped `lc_tree4` (a `Ref` temporary in an if-let scrutinee).
  - Optional fold-in from the cut `leetcode-rc-refcell-trees` (the relevance review suggests `26_smart_pointers_deep` or `31_debugging`): one test on reborrowing once with `let n = &mut *node.borrow_mut();` so two fields can be split. Verified: `mem::swap(&mut n.left, &mut n.right)` through a `RefMut` gives E0499 "cannot borrow `n` as mutable more than once".

### `54_channels` (built)

Theme: `sync_channel` backpressure, disconnect-driven shutdown, actors.

**As built** (see `exercises/54_channels/`; adversarially reviewed). Deviations and verified corrections:

- `channel1`: Implements the ROADMAP entry and every test the ROADMAP asks for (Ok, Ok, Busy; capacity-0 rendezvous; dropped receiver gives Closed; buffered items delivered after the senders drop). Extra tests: a non-Clone `Job` fixture checked by `as_ptr` identity, 'Closed wins over Busy when full', clones share one buffer, and 100-offer load shedding. Every test body runs under a 10 s `recv_timeout` watchdog, so a blocking `send` fails instead of hanging. Review changes: the queue-full aside in the header and the 'Until you ...' TODO endings.
- `channel2`: Implements the ROADMAP entry: a three-stage `source -> square -> batch` pipeline on `sync_channel(2)` links. `run_pipeline` keeps the original Senders while each stage gets a clone, and the stages unwrap their sends. The thread-pool fold-in is `drop_the_sender_first_then_join_the_stage`, which asserts `!is_finished()` while the Sender is alive (deterministic). The `recv_timeout` watchdog follows the ROADMAP note. Review changes: the `run_pipeline` TODO now says `collect()` also ends at `limit`; the solution's `source` comment now says 'the stage downstream has hung up'; 'Until you ...' TODO endings.
- `channel3`: Fails with E0559, plus E0026 from the stand-in actor's pattern, as the ROADMAP's corrected note says; the fix steps then surface E0027 and E0063. Has the tests the ROADMAP asks for (4 threads interleaving consistently; the actor exits after all Handles drop, with `!is_finished()` asserted while clones are alive). Extra tests: one answer per reply channel; out-of-order routing through a stand-in actor; a caller that gave up must not kill the actor; an actor that dies mid-request yields `Err(Gone)`; a compile-time `Handle: Clone + Send + Sync` check. Review changes: the header now names the actor's `for cmd in commands` loop correctly; the `get` TODO now says `Handle` must keep its single `tx` field; 'Until you ...' TODO endings.

#### Channels beyond basics: sync_channel backpressure, disconnect-driven shutdown, actor with reply channels

- Slug `channels-backpressure-actor` · placement: graded · module: `54_channels` (`channel1..3`) · interview value: **4/5**
- Sharpest question: How do you apply backpressure between a fast producer and a slow consumer, and how does a channel pipeline shut down cleanly when the producer finishes?
- Exercises:
  - `channel1` — `Ingest::offer` is backed by an unbounded `channel()`; use `sync_channel(cap)` plus `try_send`, mapping `Full` to `Busy(e)` and `Disconnected` to `Closed(e)`. Fails unsolved: assertion, the third offer at capacity 2 returns `Ok`.
  - `channel2` — a three-stage pipeline built on `for x in rx`, where `run_pipeline` keeps an extra `Sender` clone and stages `unwrap()` their sends; drop every `Sender` so disconnection cascades, and exit cleanly when downstream is gone. Fails unsolved: the watchdog reports "pipeline never shut down: a Sender is still alive"; the early-drop test sees stage joins return `Err` from the `unwrap` panics.
  - `channel3` — an actor thread owns a `HashMap` and processes `enum Cmd`, but `Cmd::Get` has no reply field; add `reply: Sender<Option<u32>>` and a cloneable `Handle` with `get` / `put`. Fails unsolved: E0559 "variant `Cmd::Get` has no field named `reply`" in the provided tests.
- Authoring notes:
  - channel3: tests that construct `Cmd::Get { key, reply }` against an enum without `reply` give E0559 (E0026 when it appears in a pattern), not E0063 as proposed.
  - channel2's hang from the extra `Sender` must be caught with a `recv_timeout` watchdog, because rustlings has no test timeout.
  - `sync_channel` / `try_send` with `TrySendError::{Full, Disconnected}` is straightforward std.
  - Tests: channel1 `Ok, Ok, Busy`, capacity-0 rendezvous behavior, a dropped receiver gives `Closed`, buffered items are still delivered after senders drop; channel2 yields squares in order and every stage joins `Ok` after an early receiver drop; channel3 has 4 threads interleaving consistently and the actor exits after all `Handle`s drop.
  - Fold-in from the cut `thread-pool`: add its "drop the `Sender` first, then join the workers" shutdown as one channel2 test.

### `56_async_bounds` (built)

Theme: `Send` / `'static` for spawned futures, `async fn` in traits, dyn async traits.

**As built** (see `exercises/56_async_bounds/`; adversarially reviewed). Deviations and verified corrections:

- `async_send1`: Two change sites instead of the ROADMAP's one. `record` holds the guard across an `.await`. `flush` already calls `drop(guard)` before its `.await` and still fails, which answers the ROADMAP's sharpest question directly. Correction to the ROADMAP authoring note, re-verified on 1.96: `drop(g)` before the `.await` also fails after a read-only `g.field` access through `Deref`, not only after a `DerefMut` mutation. It passes only if `g` was never borrowed. The README and header say so. The tests poll the futures by hand with `Waker::noop` to check that the lock is free while the task is suspended, interleave two tasks on one thread, check that ids queued during an upload survive, and keep the 50-task spawn test. Reviewer fix: the header and README no longer imply that a current-thread runtime accepts `!Send` futures.
- `async_send2`: Adds a local `prefix` next to the `names: &[String]` parameter, so one `spawn` shows both E0521 (parameter) and E0373 (local), and following rustc's `move` suggestion leads to E0382. `for name in names` replaces the ROADMAP's `&names[i]`, which would add an extra E0373 for `i`. The given `spawn` has a thread-local `SPAWNED` counter, so a test can require one `spawn` per name and reject the sequential `block_on` and `thread::scope` bypasses. The tests use local `Vec`s and sub-slices (this rejects `&'static [String]`) and check that the caller keeps its names, the order, trimming, the empty case, and 64 names. Reviewer fixes to the header: an async block without `move` only borrows the variables it reads (it captures by value when it consumes one), and an awaiting caller can be cancelled, which is why `'static` is needed even when the handle is awaited right away. Known gap: leaking memory or copying the slice per task passes the tests (can't be detected without unsafe), and the TODO forbids both.
- `async_send3`: Deliberately two stages. Fixing the trait (which rustc's own help suggests) reveals an intended second error in the `CachedStore` impl: a read-through cache that holds its `MutexGuard` across the remote call. This shows that declaring `Send` moves the obligation into every impl. A generic `assert_get_is_send<S: Store + Sync>` in the tests pins the trait-level fix and rejects building the future on the spawned thread. A test-defined `Recorder` impl written as `async fn` rejects boxing in the trait and shows that the fetches run on spawned threads. Other tests cover hits and misses, that only hits are cached, the cache unlocked mid round trip (polled by hand), and 30 concurrent tasks. Reviewer fix: the hint's cost sentence is now scoped to `async fn` impls (verified with a `RefCell` probe).
- `async_send4`: The starter trait is `DynStore` with an `async fn` and without `Send + Sync` supertraits, so E0038 names `DynStore`, not `Store` as the ROADMAP's fail mode says (the tests need `Vec<Arc<dyn DynStore>>`). After the E0038 fix, the learner meets the `Send`/`Sync` errors next, and the TODO warns about them. All impls (`MemStore`, and a `Fallback` over two `Arc<dyn DynStore>`) live in the exercise body. The tests only go through `dyn DynStore`. They cover `assert_send_sync::<Arc<dyn DynStore>>()`, `assert_send` on the future, a heterogeneous `Vec<Arc<dyn DynStore>>` through `fetch_all`, every store spawned on its own thread, laziness, the backup asked only on a miss, nested fallbacks, a short-lived key, and no stores. The unsolved build is noisy (38 identical E0038). Reviewer fixes: the dynosaur wording in the header no longer clashes with the `DynStore` name, and the `Fallback` WHY comment is precise.

#### Why spawn needs `Send + 'static`: guards across `.await`, borrowed data, async fn in traits

- Slug `async-send-bounds` · placement: graded · module: `56_async_bounds` (`async_send1..4`) · interview value: **5/5**
- Sharpest question: Why does `tokio::spawn` reject an async block that holds a `std::sync::MutexGuard` across `.await`, and why doesn't calling `drop(guard)` before the await always fix it?
- Exercises:
  - `async_send1` — `async fn record(stats: Arc<Mutex<Stats>>)` holds a std `MutexGuard` across an `.await` and is spawned; scope each guard in a block. Fails unsolved: "future cannot be sent between threads safely ... `MutexGuard<'_, Stats>` which is not `Send`" (no E-code); an explicit `drop(g)` gives the same error.
  - `async_send2` — `greet_all(names: &[String])` spawns `async { log(&names[i]) }`; move owned clones or an `Arc<[String]>` into `async move`. Fails unsolved: E0521 borrowed data escapes (the local variant gives E0373).
  - `async_send3` — `trait Store { async fn get(&self, k) -> Option<String>; }` used by a generic `spawn_fetch<S: Store + Send + Sync + 'static>`; declare `fn get(..) -> impl Future<Output = ..> + Send` while impls keep `async fn`. Fails unsolved: "future cannot be sent between threads safely: the trait `Send` is not implemented for `impl Future<Output = ..>`".
  - `async_send4` — tests need `Vec<Arc<dyn DynStore>>`; define `type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>` and `trait DynStore: Send + Sync { fn get<'a>(&'a self, k: &'a str) -> BoxFuture<'a, ..>; }` implemented with `Box::pin(async move { .. })` (what `#[async_trait]` generates). Fails unsolved: E0038 the trait `Store` is not dyn compatible.
- Authoring notes:
  - Verified on 1.96: a guard held across `.await` gives "future cannot be sent between threads safely". `drop(g)` before the `.await` still fails once `g` was mutated through `DerefMut`, and passes if `g` was never borrowed, so phrase the README precisely. A scoped block passes.
  - Verified: an `async fn` in a trait fails the `Send` spawn bound; declaring `-> impl Future + Send` with impls still written as `async fn` compiles; RTN (`S::get(..): Send`) is E0658 unstable; a trait with `async fn` gives E0038 for `dyn`.
  - Use a thread-backed `spawn` with tokio's exact bounds so the errors are real without tokio. Exercises are single-file bins, so each file carries its own copy of the helper (as `futures1..4` do with `block_on`).
  - Tests: `assert_send(fut)`; 50 spawned tasks give `started == finished == 50`; async_send2's collected output equals `names` and the slice is usable afterwards; every store in the dyn `Vec` is fetched on its own thread with the expected `Some` / `None`.
  - README: use `tokio::sync::Mutex` only when a lock truly must be held across `.await`; there is no safe scoped async spawn (futures can be forgotten); the `async_fn_in_trait` lint; RTN is unstable.
  - Absorbs the RPITIT + `Send` lesson of the cut `impl-trait-capture-rpitit` (async_send3).

### `57_async_combinators` (built)

Theme: join, select, drop-as-cancel, cancel safety.

**As built** (see `exercises/57_async_combinators/`; adversarially reviewed). Deviations and verified corrections:

- `join1`: Unchanged by this review. Follows the ROADMAP entry: `Unpin` children, `Option` outputs, a starter `poll` that returns `Pending`, bounded manual poll loops with `Waker::noop()`, and the max(n_a, n_b) + 1 formula in the header and test comments. `Box::pin(async {..})` children appear in the rendezvous test, which a sequential join fails. Additions beyond the ROADMAP: a manual `impl Unpin for Join` (without it `self.get_mut()` fails with E0277 for generic outputs, re-verified), a counting-waker test, and a mailbox stall guard.
- `select1`: Code unchanged by this review; only the hint's spelling changed. Follows the ROADMAP entry: `Option` children, `Either` output, a starter that is never `Ready`, a `Left(5)` case and `Right` cases, and the loser's `DropFlag` checked right after the deciding poll while the `Select` is still alive. The select is biased (poll `a` first) so the tests are deterministic; the header and README explain `tokio::select!`'s random order and `biased;`.
- `cancel1`: Follows the ROADMAP entry: the source is `Pending` between bytes, and the failing assertion shows ["lo", "ld"] instead of ["hello", "world"]. Both fixes pass: progress kept in the reader, or one future pinned outside an inner loop. Exact heartbeat counts, always >= 1 where a stall exceeds the patience, reject removing the timeout. This review added a test-only `thread_local!` fire guard to the given `Timeout`, so a timer-first wrong fix fails instead of livelocking. It also added the line-first requirement to the TODO and made the comments precise about what gets dropped. The dropped `stream1` is a README note (`Stream`, `StreamExt::next`, unstable `AsyncIterator`: E0658 `async_iterator`, re-verified on 1.96).

#### Hand-written join and select, drop means cancel, cancellation safety

- Slug `join-select-cancellation` · placement: graded · module: `57_async_combinators` (`join1`, `select1`, `cancel1`) · interview value: **4/5**
- Sharpest question: What happens to a future's state when it loses a `select!`, and why is `read_exact` not cancel-safe inside a `select!` loop?
- Exercises:
  - `join1` — `Join<A, B>` for `Unpin` children storing `Option` outputs, whose `poll` returns `Pending`; poll both children on every poll and never re-poll a finished one. Fails unsolved: "not Ready after 100 polls"; a sequential solution fails the dependency test.
  - `select1` — `Select<A, B> -> Either` with `Option` children that drops the loser immediately. Fails unsolved: never `Ready`; keeping the loser alive fails the "dropped promptly" `DropFlag` assertion.
  - `cancel1` — `read_line` on a source that is `Pending` between bytes keeps partial bytes in a local buffer, so a `select` timeout loses them; keep progress in `self.pending` (or pin the future outside the loop). Fails unsolved: assertion, `Some("lo")`, `Some("ld")` instead of `"hello"`, `"world"`.
- Authoring notes:
  - Use `Waker::noop` (stable since 1.85) and bounded manual poll loops to keep everything deterministic.
  - Pin down the exact outer-poll-count formula in the test comments, tied to the leaf futures' `Pending` counts (join1 expects `max(n_a, n_b) + 1` outer polls, with both children first polled on poll 1; `Box::pin(async { .. })` children also work).
  - Tests: select1 covers `Left(5)` and the `Right` cases, and the loser's `DropFlag` is set on the poll that returned `Ready`; cancel1's lines are intact after cancellation and at least one timeout fired.
  - join1 and select1 require `Unpin` children, which sidesteps pin projection; see "Async recursion and pin projection" under [Additional topics](#additional-topics-not-yet-verified).
  - README: `tokio::select!` polls branches in random order unless `biased;`. A hand-written `Stream` plus `Next` adapter was dropped during the merge (std `AsyncIterator` is unstable and interviews ask about `futures::Stream` by concept), so cover `Stream` as a README note.

### `58_leaf_futures` (built)

Theme: external wakeups (oneshot, timer) and cooperative yielding.

**As built** (see `exercises/58_leaf_futures/`; adversarially reviewed). Deviations and verified corrections:

- `oneshot1`: Follows the ROADMAP entry: the Sender half (send plus Drop, waking outside the lock) is given, and the learner writes only Receiver::poll. Reviewer changes: block_on is now deterministic (it takes the sender's JoinHandle and panics at once when that thread has finished but nothing woke the receiver; the 30 s STUCK_AFTER only bounds a sender thread that never finishes). The stress test now lines up each delivery with the receiver's poll (ready/go spin handshake, SignalThenPoll wrapper, 0-7 spin delay sweep), so the two-lock lost-wakeup wrong fix is caught reliably. Tests beyond the ROADMAP list: repolling with the same waker wakes it once; nothing is woken before the value exists; a value sent before the sender dropped still wins; a cross-thread drop ends block_on with Canceled; a 500-round aligned hand-off stress test. The will_wake/clone_from optimization cannot be observed from safe tests, so a plain replace passes.
- `timer1`: Follows the ROADMAP entry. The timer is a given single background thread that receives registrations over mpsc, keeps them in a Vec sorted by deadline, fires due entries earliest first and waits with recv_timeout. Test seams in the given Timer: registrations() (one registration per Sleep; rejects thread-per-sleep), sleep_until(Instant) (fixed deadlines for the order test), and, added by the reviewer, unfired() (entries not yet fired, decremented only after the wake). With unfired(), the test executor and the repoll test detect 'nothing can ever wake this task' at once instead of through a 5 s watchdog; the 30 s STUCK_AFTER is only a backstop. shorter_sleeps_finish_first now uses 600/200/400 ms deadlines for scheduling margin. Extra tests beyond the ROADMAP: overlapping sleeps end in deadline order (catches the yield-then-block wrong fix); a lower-bound duration check; exact poll count [3] and registrations == 2 for two sequential sleeps.
- `yield1`: Differs from the ROADMAP entry, which has a single change site (the YieldNow leaf). 28_futures/futures2 already drills the leaf as `YieldOnce` with step-by-step TODOs, so under the 'don't re-teach' rule the header points to futures2, and a second TODO makes the given CPU-bound `checksum` loop yield once per batch of `every` items. The ROADMAP's 'Fails unsolved' text may want to mention that second site. The heartbeat test requires at least 9 ticks while running, at most `every` items between ticks, and at most 12 ticks in total. Yielding before or after a batch, including i == 0, a countdown or chunks all pass; every item, every/2 and 2*every are rejected. The executor is runtime3's, with try_recv 'stuck' detection (every wake here is synchronous) and a 100_000-poll budget. The heartbeat loop is bounded.

#### Leaf futures woken from outside: oneshot, non-blocking Sleep, cooperative yield

- Slug `leaf-futures-wakeups` · placement: graded · module: `58_leaf_futures` (`oneshot1`, `timer1`, `yield1`) · interview value: **4/5**
- Sharpest question: Why is `std::thread::sleep` inside an async fn a bug, how does a real timer wake the task, and why must `poll` store the latest waker?
- Exercises:
  - `oneshot1` — a oneshot channel over `Arc<Mutex<Shared { value, waker, sender_alive }>>` whose `Receiver::poll` stores no waker; store or replace `cx.waker()` under the value's lock (skip the clone if `will_wake`) and return `Err(Canceled)` when the sender is gone. Fails unsolved: assertion, the second poll's waker B is woken 0 times after `send`.
  - `timer1` — `Sleep::poll` calls `thread::sleep` and returns `Ready`; register with a background timer thread, return `Pending`, and wake the latest waker. Fails unsolved: the event-order assertion sees `[a:start, a:done, b:ran]` instead of `[a:start, b:ran, a:done]`.
  - `yield1` — `async fn yield_now() {}` never yields, so two tasks don't interleave; implement a `YieldNow` future (`wake_by_ref` plus `Pending` once). Fails unsolved: assertion, `[a0, a1, a2, b0, b1, b2]` instead of the expected interleaving.
- Authoring notes:
  - Feasible with std only. Every exercise is a standalone bin, so inline `runtime2`'s `block_on` and the executor in each file.
  - timer1's event-order assertion is deterministic with a single-threaded executor as long as the sleep durations are well separated.
  - Tests: oneshot1 wakes B once and not A, send-before-poll is `Ready`, a dropped sender wakes the receiver with `Canceled`, and a cross-thread value arrives through `block_on`; timer1 `Duration::ZERO` is `Ready` on the first poll and re-polling with a new waker wakes the new one; yield1 lets a heartbeat task tick while a CPU loop yields every N iterations.

### `59_arena` (built)

Theme: index / arena structures and generational ids. This module underpins the LRU and graph modules.

**As built** (see `exercises/59_arena/`; adversarially reviewed). Deviations and verified corrections:

- `arena1`: Built as the ROADMAP specifies: E0308 starters for add_child, path_to_root and lca, and every ROADMAP test item is covered. Additions beyond the ROADMAP: add_child must panic on an unknown parent BEFORE it changes anything (checked with catch_unwind, including NodeId(len)); a 200_000-deep chain rejects a recursive path_to_root and shows the flat drop; lca is checked on every pair of a 60-node LCG tree against a brute-force oracle. lca is deliberately not run on the deep chain, so O(depth) is guidance, not a test. Unlike what the builder's note said, the line-1 header does carry (E0308), which matches `60_lru_cache`. The review fixed the header's RefCell panic wording and added a `smartptr3` cross-reference.
- `arena2`: Built as the ROADMAP specifies: `enum Slot { Occupied { generation, value }, Vacant { generation } }`, `Id { index, generation }`, E0308 starters for get, get_mut and remove, and `insert` is given. It covers all of the ROADMAP's test items: remove once then None, a stale id on a reused index is None, and 10_000 cycles keep slots.len() == 1. One requirement goes beyond the ROADMAP: a slot at generation u32::MAX is retired (tested by building a Slab directly, so the field names and `free: Vec<usize>` are part of the test contract). Other added tests: a double remove, removing from the middle and front, and ids that were never handed out. The values are a non-Clone `Token`, with an as_ptr check. The review fixed the E0507 part of the hint and qualified the header's release-build wrap claim.

#### Arena and index structures: typed NodeId trees and generational slabs

- Slug `arena-indices` · placement: graded · module: `59_arena` (`arena1`, `arena2`) · interview value: **4/5**
- Sharpest question: How would you represent a tree with parent pointers, or a graph, in safe Rust without `Rc<RefCell>`, and how do you detect a stale index after removal?
- Exercises:
  - `arena1` — `Tree<T> { nodes: Vec<NodeData> }` addressed by `NodeId(usize)`, with empty `add_child`, `path_to_root` and `lca`. Fails unsolved: E0308 from the empty bodies.
  - `arena2` — `Slab<T>` with `enum Slot { Occupied { generation, value }, Vacant { generation } }` and `Id { index, generation }`, with empty `get` / `get_mut` / `remove`; check the generation, remove via `mem::replace` while bumping it, and push the slot onto a free list. Fails unsolved: E0308 from the empty bodies.
- Authoring notes:
  - A straightforward std-only exercise with E0308 starters. The `mem::replace` generation bump overlaps `ownership5`, which is fine.
  - Tests: sequential ids with both links set, `path_to_root`, `lca` for siblings / cousins / ancestor / self, `Tree<String>: Send` (unlike an `Rc` version), `clone` is deep; remove once then `None`, a reused index with a stale id returns `None`, and 10_000 insert/remove cycles keep `slots.len() == 1`.
  - The original `arena2` (`get_disjoint_mut`) moved to `borrowck1`.

### `60_lru_cache` (built)

Theme: LRU from an O(log n) recency map to an O(1) index-linked list.

**As built** (see `exercises/60_lru_cache/`; adversarially reviewed). Deviations and verified corrections:

- `lru1`: Kept as a short graded warm-up (two bodies, 6 tests) even though the *(scope)* note says 'lru1 could become a README warm-up'. The note is a suggestion ('could'), the ROADMAP fixes the names as lru1..3, and the exercise drills the two-index sync invariant, which is the usual first interview answer. If the orchestrator wants the trim: drop lru1 and README Exercise Path item 1; lru2/lru3 do not depend on it. Review added the comparison-counting test. Known limit: an O(n) smallest-tick scan of `map` during eviction cannot be observed, so it passes; the TODO forbids it.
- `lru2`: Follows the ROADMAP entry (E0308, then the ordering and slot-reuse tests). Review added `hits_updates_and_evictions_are_hash_lookups_not_scans`, fixed the raw-pointer wording (the `lru` crate uses `*mut` links), and repointed the Rc<RefCell> references to the earlier `26_smart_pointers_deep/smartptr2`/`smartptr3`. `use std::mem;` exists only in the solution, so the starter has no unused-import warning. The 2 unused-variable warnings from the empty helpers are harmless.
- `lru3`: Deviation, kept: the unsolved file also fails with E0599 because the tests call the not-yet-existing `peek`. No stub is given, so the learner chooses the receiver; the RwLock read-guard test and the `&LruCache` helper force `&self`. That makes 21 errors, all E0308 or E0599. Review trimmed the header to point at `45_sized_deref/borrow1` instead of re-teaching the Borrow contract, AsRef and ?Sized, as the ROADMAP asks (borrow1 is canonical; lru3 applies it). The given code is lru2's finished cache.

#### LRU cache: BTreeMap recency, then an O(1) index-linked list, then a `Borrow<Q>` get and peek

- Slug `lru-cache` · placement: graded · module: `60_lru_cache` (`lru1..3`) · interview value: **4/5**
- Sharpest question: Implement an O(1) LRU cache in safe Rust. Why can't the list nodes hold prev/next references, and why does `get` take `&mut self`?
- Exercises:
  - `lru1` — `LruCache` over `HashMap<K, (V, u64)>` plus `BTreeMap<u64, K>` with empty `get` / `put`; re-tick on every touch and evict with `pop_first`. Fails unsolved: E0308 from the empty bodies.
  - `lru2` — O(1): `Vec<Entry { key, value, prev, next }>` plus `HashMap<K, usize>` with `head` / `tail`, and empty `unlink` / `push_front` helpers; recycle the tail slot on eviction via `mem::replace`. Fails unsolved: E0308, then the ordering and slot-reuse tests.
  - `lru3` — tests call `get("k")` on an `LruCache<String, _>`; generalize to `get<Q>(&mut self, key: &Q) where K: Borrow<Q>, Q: Hash + Eq + ?Sized` and add a `peek(&self)` that doesn't change recency. Fails unsolved: E0308 expected `&String`, found `&str`.
- Authoring notes:
  - `BTreeMap::pop_first` is stable (1.66).
  - lru3 repeats `borrow1`'s `HashMap::get`-style `K: Borrow<Q>` signature: make lru3 an application of `borrow1`, not a new lesson.
  - Tests: the LeetCode 146 sequence; an update promotes without evicting; capacity 1; 1_000 LCG-generated ops checked against a naive model with `map.len() == order.len()`; 10_000 puts into capacity 3 keep `nodes.len() == 3`; forward and backward walks agree; `peek` doesn't save the LRU entry but `get` does.
  - *(scope)* lru2, the index-linked list in a `Vec` with slot reuse, is the answer interviewers want. Three variants is a lot: lru1 could become a README warm-up.

### `63_slices_strings` (built)

Theme: two pointers, Unicode-safe windows, intervals, binary search. Also fills the thin strings curriculum (`char_indices`, char-boundary panics).

**As built** (see `exercises/63_slices_strings/`; adversarially reviewed). Deviations and verified corrections:

- `window1`: As in the ROADMAP. two_sum_sorted returns 0-based indices and any valid pair is accepted. Model tests driven by a 64-bit LCG check both functions, with values from the i32 extremes, and catch checked/saturating/wrapping fixes. Review changes: none to code or tests.
- `window2`: Deviation (kept): the solution map is `HashMap<char, (usize, usize)>` (char position, byte offset just past the char) instead of the ROADMAP's `HashMap<char, usize>`, because "longest" is counted in chars. The `HashMap<char, usize>` + char-counter version is in the hint and passes. On 1.96 the starter panics with the START form of the message; the ROADMAP note's 'end byte index' form is the other variant, and the README quotes only the shared substring. Review changes: header line 1 reworded (no non-E-code parenthetical), and `offset_in` accepts an empty answer for an empty input.
- `window3`: As in the ROADMAP (let chain with `last_mut()`, `sort_unstable_by_key`). Tests add a 2000-case model, an equal-starts case and extreme endpoints. Review changes: none.
- `window4`: Deviations (kept): `first_true(lo: u64, hi: u64, pred: impl FnMut(u64) -> bool) -> u64` searches a half-open u64 range, where the ROADMAP says u32, and `min_ship_capacity(&[u32], u32) -> u64`, so heavy u32 packages cannot overflow the total or the capacity. The near-MAX tests therefore use u64::MAX. `min_ship_capacity` also starts empty, so the unsolved file has two E0308s. A probe checker (every probe inside lo..hi, at most 64 probes) makes bad midpoints fail fast instead of hanging. Review changes: first_true TODO overflow wording made precise, and `count_in_range(&[2; 8], 2, 3) == 8` added so the header's binary_search-on-duplicates fact has a matching test.

#### Slice and `&str` algorithms: two pointers, Unicode-safe sliding window, interval merge, binary search on the answer

- Slug `slice-string-algorithms` · placement: graded · module: `63_slices_strings` (`window1..4`) · interview value: **4/5**
- Sharpest question: Return the longest substring without repeating characters as a `&str` slice of the input. What breaks if you index by bytes on input like "éã"?
- Exercises:
  - `window1` — `two_sum_sorted` starts with `j = nums.len() - 1` and an `i32` sum, and `three_sum` is empty; handle the empty slice, widen to `i64`, dedupe triples. Fails unsolved: E0308 from `three_sum`; the empty-slice test panics with subtract overflow; a near-`MAX` add overflows.
  - `window2` — a byte-based `longest_unique_substring(&str) -> &str` using `[usize; 256]`; rewrite with `char_indices`, `HashMap<char, usize>` and a let chain. Fails unsolved: `"éã"` panics with a "not a char boundary" slicing error.
  - `window3` — `merge_intervals` is empty; `sort_unstable_by_key`, then `if let Some(last) = out.last_mut() && s <= last.1 { .. } else { out.push(..) }`. Fails unsolved: E0308.
  - `window4` — `count_in_range` via two `partition_point` calls (empty), `first_true` given with `(lo + hi) / 2`, and `min_ship_capacity` reusing it; use `lo.midpoint(hi)` and half-open ranges. Fails unsolved: E0308; `first_true(u32::MAX, ..)` panics with add overflow.
- Authoring notes:
  - window2: on 1.96 the panic text is "end byte index 1 is not a char boundary; it is inside 'é' (bytes 0..2 of string)" when the range end is misaligned. Match a substring, not the full old message.
  - Verified: the let chain in window3 compiles on edition 2024; `u32::midpoint` is stable.
  - Tests: window1 examples, empty and 1-element inputs give `None`, `[MAX - 1, MAX]` with target 5 gives `None`, exact `three_sum` output; window2 `abcabcbb`, `pwwkew`, emoji cases, `"éã"`, `""`, and the result is a subslice of the input; window3 unsorted, touching, contained and empty inputs; window4 count 3, `first_true` near `MAX`, ship capacity 15.
  - README: `binary_search` may return any of several duplicates.
  - Optional fold-in from the cut `heaps-topk-dijkstra`: `heap1`, `fn top_k<'a>(words: &[&'a str], k: usize) -> Vec<&'a str>` (LeetCode 692; the elided signature is E0106), either here or in `eq-hash-ord-contracts`.
  - The cut `algorithm-semantics-quiz`'s most useful facts already live here: char vs byte length (window2) and `binary_search` on duplicates (window4).

### `66_checked_math` (built)

Theme: overflow-safe `mul_div`, rounding direction, fixed point, determinism.

**As built** (see `exercises/66_checked_math/`; adversarially reviewed). Deviations and verified corrections:

- `checkedmath1`: Deviations kept from the build: - The ROADMAP's starter `a.checked_mul(b)? / d` is a type error in a function returning `Option<u128>`, so the starter is `Some(a.checked_mul(b)? / d)`. - The `mul_div_ceil` starter uses the textbook `(x + d - 1) / d`, a second planted overflow. - The given `div_wide(hi, lo, d)` returns `(quotient, remainder)`, because the ceiling needs the remainder. Tests beyond the ROADMAP: 30k exact cases with products up to about 2^254, a 40x40 grid near MAX, and a ceiling that rounds past MAX. Reviewer change: the line-1 header is now a single line, and 'usually 18 decimals' now reads 'often'.
- `checkedmath2`: Framed as generic vault share math, per the *(scope)* note; ERC-4626 is cited only as a reference. The four previews take a `Rounding` argument, and the learner changes only those arguments. The given trade code refuses zero-outcome trades (`VaultError::RoundsToZero`), which sets up a first-depositor inflation-attack test. Reviewer change: the line-1 header is now a single line; nothing else needed fixing (10 wrong fixes all caught).
- `checkedmath3`: The ROADMAP leaves the error enum unspecified. As built, it has Empty, Invalid, TooManyDecimals, Overflow and DivisionByZero, with Display and Error given. The grammar is strict: "1." and ".5" are Invalid, and more than 18 decimals is TooManyDecimals even when they are zeros. `checked_add` is given. Tests beyond the ROADMAP: 20k seeded round trips, `MAX * 1` and `MAX / 1`, the exact overflow boundary (`2^64 * 1e9` raw, squared, is `2^128` raw), and fractions on both sides. Reviewer changes: the line-1 header is now a single line, and the hint now says correctly which multiplication shortcut overflows on which input. The unused-variable warnings in the starter are left as is, the same as upstream rustlings `todo!()` starters.
- `checkedmath4`: Deviation kept from the build: the ROADMAP puts `as u64` in `state_digest`. As built, `as u64` appears twice. In the digest it is a truncation bug, fixed by hashing all 16 bytes. In a new `export()` for a u64-amount public API, `u64::try_from` returns `Err(TooLargeForApi { account })`, which covers the ROADMAP's 'Err for a truncating cast' test. `accrue_interest` takes a u32 rate in bps, rounds down through the given `mul_div_floor`, and is all-or-nothing. `Fnv1a` is given, deliberately not a `std::hash::Hasher`. I re-verified both golden constants with an independent Python FNV-1a. Reviewer changes: - `export_refuses_to_truncate` now also checks, over 30 fresh ledgers, that the Err names alice, the first of the six over-u64 accounts by name. This closes a roughly 50% flaky pass for 'HashMap + sort the Vec afterwards'. - `interest_that_would_overflow_changes_nothing` adds a u32::MAX-rate case whose interest alone overflows. This closes a pass for the unchecked split. - The float bullet says 'NaN bit patterns aside'. - The line-1 header is now a single line. The orchestrator may want to mention `export()` in the ROADMAP entry.

#### Overflow-safe arithmetic: 256-bit mul_div, rounding direction, fixed-point Decimal, deterministic state digests

- Slug `checked-fixed-point-math` · placement: graded · module: `66_checked_math` (`checkedmath1..4`) · interview value: **4/5** for the blockchain / DeFi track (the relevance review puts it at about 2 for general backend roles)
- Sharpest question: Compute `a * b / c` on u128 without overflowing the intermediate product, and explain which way deposits vs withdrawals must round and why.
- Exercises:
  - `checkedmath1` — `mul_div_floor` / `mul_div_ceil(a, b, d) -> Option<u128>` start as `a.checked_mul(b)? / d`; use `carrying_mul(b, 0)` for the 256-bit product and a provided `div_wide`. Fails unsolved: assertion, `mul_div_floor(5e26, 1.5e18, 1e18)` returns `None` instead of `Some(7.5e26)`.
  - `checkedmath2` — vault share math: `convert_to_*` rounds down, `preview_mint` / `preview_withdraw` round up; the starter floors everywhere. Fails unsolved: the invariant `A'·S >= A·S'` fails (A=3, S=2, mint 1).
  - `checkedmath3` — `Decimal(u128)` with 18 digits: `FromStr`, trimmed `Display`, checked mul/div through `mul_div`, an error enum; the bodies are `todo!()`. Fails unsolved: the `todo!()` panics (and clippy's `todo` lint).
  - `checkedmath4` — `state_digest` iterates a `HashMap`, uses `f64` interest, `DefaultHasher` and `as u64`; switch to `BTreeMap`, integer basis-point math, an in-file FNV-1a and `u64::try_from`. Fails unsolved: the digest doesn't match the golden constant, and interest is off on a 1e30 balance.
- Authoring notes:
  - `u128::carrying_mul` compiles on stable 1.96; `widening_mul` is still unstable.
  - `todo!()` starters fail by test panic. dev/Cargo.toml's clippy `todo = forbid` only hits the unsolved state, which is fine; the solution must contain no `todo!()`.
  - Tests: checkedmath1 200k differential cases below 2^64, `d == 0` and quotient overflow give `None`, `(MAX, MAX, MAX) == Some(MAX)`, ceil boundaries; checkedmath2 the invariant over a grid plus golden values; checkedmath3 round trips, rejects 19 decimals, `"-1"`, `"1e3"` and overflow, and `1.1 * 3 == 3.3` exactly; checkedmath4 gives equal golden digests for different insertion orders, exact interest, and `Err` for a truncating cast.
  - *(scope)* checkedmath2 is Ethereum-flavoured (ERC-4626); frame it as generic vault share math.

### `67_code_review` (built)

Theme: an unguided multi-bug PR and an idiomatic refactor.

**As built** (see `exercises/67_code_review/`; adversarially reviewed). Deviations and verified corrections:

- `review1`: Built as the ROADMAP specifies: seven planted bugs, one generic TODO, and progressive hints. Deviations from the ROADMAP wording: the observer panic is "RefCell already mutably borrowed" (the observer calls `borrow()` while `transfer` holds a `RefMut`), and the ledger is about 180 code lines rather than ~150. Reviewer changes: I strengthened three tests so that divide-by-100-twice fees, a checked_mul/divide-by-100 hybrid, and `amount >= limit + 1` now fail. Details: `fee_is_rounded_down` adds 399, the large-transfer amount is now 12_345_678_901_234_567_399 (fee 37_037_036_703_703_702), and the limit test adds a `Some(u64::MAX)` limit. I also replaced a misleading header example, used American test data, applied the 'Until you ...' TODO ending, and fixed cross-links (`31_debugging/debugging4` and `debugging6`, not `40_interior_mutability`). Each bug reintroduced alone into the solution still fails exactly its own test.
- `review2`: Built as the ROADMAP specifies. It REQUIRES `strict_clippy = true` (and `test = true`) in info.toml, because unsolved it builds and passes its tests. clippy 0.1.96 reports 8 errors from the 7 lints. `find_player` compares with `eq_ignore_ascii_case`, because `player.name == *name` suppresses `ptr_arg` on the `&String`. `manual_clamp` fires only for constant, correctly ordered bounds (verified: swapped constants or parameter bounds do not fire). Reviewer change: added an ASCII-only case-folding check ("ZOë" matches "Zoë", "ZOË" does not) so a `to_lowercase` rewrite fails; applied the 'Until you ...' TODO ending. Scope: review2 keeps `ptr_arg` / `needless_range_loop` as its ROADMAP entry requires. The dedupe with `perf-allocation-aware` should happen on the perf side when 65_performance (Tier 2) is built.
- On rustc 1.96 the observer bug panics with "RefCell already mutably borrowed" (the observer calls `borrow()` while `transfer`'s `RefMut` is alive).
- `review2` is graded by clippy and registered with `strict_clippy = true`.

#### Code-review rounds: a PR with planted bugs found via symptom-named tests, and a clippy-graded refactor

- Slug `code-review-drills` · placement: graded · module: `67_code_review` (`review1`, `review2`) · interview value: **4/5**
- Sharpest question: Here is a 150-line PR implementing a ledger. What would you flag before approving it, and why is `&memo[..16]` a bug?
- Exercises:
  - `review1` — a ~150-line `Ledger` with 7 planted bugs: `amount * fee / 10_000` overflow, `&memo[..16]` mid-character, `balance as u32` truncation, a `HashMap`-ordered statement, a `RefCell` `borrow_mut` held while notifying an observer, `>` vs `>=` on a limit, `parse().unwrap()` on input. Only a generic TODO header; hints reveal the bugs progressively. Fails unsolved: seven symptom-named failing tests (overflow panic, char-boundary panic, "RefCell already borrowed", unwrap panic, wrong assertions).
  - `review2` — correct but unidiomatic code (`&Vec` / `&String` params, an index loop, a manual map `match`, `is_some` plus `unwrap`, `len() == 0`, a trailing `return`, a hand-written clamp) with `strict_clippy = true`. Fails unsolved: clippy `-D warnings` on `ptr_arg`, `needless_range_loop`, `manual_map`, `unnecessary_unwrap`, `len_zero`, `needless_return`, `manual_clamp`.
- Authoring notes:
  - Verified: all seven review2 lints warn by default on clippy 0.1.96, so `strict_clippy = true` makes them fail.
  - The HashMap-order test uses 12 accounts, so a random order passes by luck with probability 1/12!, about 2.1e-9 (verified).
  - All symptom tests plus the unchanged happy-path tests must pass in the solution.
  - *(scope)* review2 overlaps `perf-allocation-aware`'s clippy lints (`ptr_arg`, `needless_range_loop`); dedupe between them.

## Tier 2

Built after Tier 1; every entry below is built. Same order convention.

### `38_variance` (built)

Theme: variance, `PhantomData` marker choice, the borrowed-forever bug. Placed after `25_lifetimes_deep`.

**As built** (see `exercises/38_variance/`; adversarially reviewed). Deviations and verified corrections:

- `variance1`: Deviations from the ROADMAP entry: (1) Part 3 (`LocalOnly`) is checked by a runtime #[test] on the CONCRETE type, with the inherent-impl-precedence probe (`Probe::<T>::IS_SEND` / `IS_SYNC` inside a `send_sync!` macro, never inside a generic fn). This was reverified on 1.96: concrete u32 gives true, Rc gives false, and a generic `fn f<T>()` always gives false. So it fails as a test, not as E0080, and only after parts 1 and 2 compile. Per the *(scope)* note, the compile_fail check belongs to the planned api-surface-lab. (2) The derive fold-in is in. `Clone` goes through a generic `duplicate<C: Clone>` helper, so it reports E0277 instead of E0599 (a direct `.clone()` on a Copy type trips clippy's clone_on_copy). (3) A small `Table<T>` gives the ids context, and a thread test sends an `Id<Rc<String>>` to a worker. (4) Reviewer addition: a `same_hash` assertion (DefaultHasher) catches a Hash/Eq-inconsistent Hash, such as hashing the address, which passed before. It adds a second E0277 `User: Hash` to the unsolved build. (5) The header topic is now 'Variance and PhantomData', to match the README.
- `variance2`: Built as the graded core, as the *(scope)* note asks. Beyond the ROADMAP entry: `add` deduplicates and returns a `usize` symbol, which shows that nothing returned holds the borrow; the signature alone does. There are `get(&self) -> Option<&'a str>` and `into_words(self)`. A given helper `intern_all(&mut Interner<'a>, &'a str)` adds E0621. Its help does NOT fix the helper: the loop keeps its E0499, and callers break. The builder's text claimed the help compiles `intern_all`; that was false and is now corrected everywhere. The cursor also reports E0499 when a test starts a second cursor on `rest`, besides the ROADMAP's E0502 (which is on `rest`, not `text`, because the tests advance a separate `rest` variable). Reviewer addition: the test `a_filled_interner_can_be_shared` (`assert_sync(&interner)`) rejects `RefCell` + `add(&self)`, which passed before. The solution now passes 8 tests.
- Correction: the note that the will-it-compile variance drills belong to `resolution-compile-quiz` is superseded by that quiz's *(scope)* merge. The module README carries a checked "Will It Compile?" table instead.
- Correction: following E0621's help (`&'a mut Interner<'a>`) does not make a looping caller compile; the E0499 in the loop stays.

#### Variance and PhantomData: marker choice and the borrowed-forever `&'a mut Foo<'a>` bug

- Slug `variance-phantomdata` · placement: graded · module: `38_variance` (`variance1`, `variance2`) · interview value: **3/5**
- Sharpest question: Why does `impl<'a> Interner<'a> { fn add(&'a mut self, w: &'a str) }` make the second `add` call fail, and what does the invariance of `&mut T` have to do with it?
- Exercises:
  - `variance1` — three types start with the wrong markers: `Id<T>` (a typed row id that must be covariant and `Send + Sync` for all `T`: `PhantomData<fn() -> T>`), `Sink<T>` (must be contravariant: `PhantomData<fn(T)>`) and `LocalOnly` (must be `!Send` / `!Sync`: `PhantomData<*const ()>`). Fails unsolved: (1) E0277 `Rc<i32>` cannot be sent, from `assert_send::<Id<Rc<i32>>>()`; (2) "lifetime may not live long enough" in `fn widen(s: Sink<&str>) -> Sink<&'static str>`; (3) a Send check on `LocalOnly` (see notes for how to build it).
  - `variance2` — `Interner<'a> { words: Vec<&'a str> }` with `fn add(&'a mut self, w: &'a str)`, called three times before `len()`, plus the cursor form `struct WordCursor<'a> { input: &'a mut &'a str }`; fix with `&mut self` or two lifetimes `<'s, 'a>`. Fails unsolved: E0499 on the second `add` and E0502 on `len()` (the cursor gives E0502 on the text).
- Authoring notes:
  - The proposed "autoref-specialization `Probe<T>::IS_SEND`" is mislabelled: an associated-const path never autorefs. What works on stable (verified, E0080 on failure) is inherent-impl precedence, the `impls`-crate pattern: `impl<T: ?Sized + Send> Probe<T> { const IS_SEND: bool = true; }` plus a trait whose default is `false`, blanket-implemented for all `Probe<T>`.
  - *(scope)* Even so, that probe is fragile (it only works on concrete types, through a macro). Replace it with `assert_send` plus a `compile_fail` doctest in `api-surface-lab`. If the const assertion is kept, note that it is item-level: it fails at `cargo build` and hides the other parts until fixed, which is acceptable.
  - Verified on 1.96: `PhantomData<T>` gives E0277 for `assert_send::<Id<Rc<i32>>>`; a covariant `Sink` fails `widen()` with "lifetime may not live long enough"; `PhantomData<fn() -> T>` is covariant and `Send + Sync`; `PhantomData<fn(T)>` makes `widen` compile; `PhantomData<*const ()>` makes the probe false; the `Interner` gives E0499 twice plus E0502 on `len()`; the `&'a mut &'a str` cursor gives E0502 on `text`.
  - Tests: `shrink` (covariance) and `widen` (contravariance) compile; `size_of::<Id<String>>() == size_of::<u64>()`; variance2 gives `len() == 3` with words in order, borrowed from a `String` declared earlier and still readable; the cursor yields `("hello", "big", "world")`.
  - *(scope)* variance2 (`fn add(&'a mut self)` leaving the struct borrowed forever) is the part every intermediate developer hits and deserves the grading; the variance table matters mostly for senior, systems and unsafe roles.
  - Fold-in from the cut `derive-bounds-const-generics`: add tests on `Id<T>` that need hand-written `Clone` / `Copy` / `PartialEq` / `Eq` / `Hash` / `Debug` for all `T` (derive adds `T: Trait` bounds). Verified: `#[derive(..)]` on `Id<T>` with `PhantomData<fn() -> T>` and a derive-less `User` gives E0369 for `==`, E0277 `User: Eq` / `User: Hash` via `HashSet::insert`, and E0599 for `.clone()`. Write `Clone` as `*self` to keep clippy's `non_canonical_clone_impl` quiet.
  - The will-it-compile variance drills belong to `resolution-compile-quiz`, and the soundness demo (an unsound covariant cell) to `miri-ub-zoo`.
  - README: the Nomicon variance table; `dyn Fn(T)` is not contravariant (trait-object arguments are invariant).

### Extend `33_closures` (built)

Theme: closure bounds beyond the basics: higher-ranked `for<'a>` and async closures (`closure5..8`).

**As built** (see `exercises/33_closures/`): The `sort_by_key(|p| &p.name)` anchor is Part B of `closure5`. `closure8` ships its own `block_on` that panics on `Pending` instead of spinning, so a wrong fix can never hang the tests.

#### Closure bounds beyond basics: higher-ranked `for<'a>`, IntoIterator over `&C`, let-bound closure inference, async closures

- Slug `closure-bounds-hrtb-async` · placement: graded · module: extend `33_closures` (`closure5..8`) · interview value: **3/5**
- Sharpest question: Why does `people.sort_by_key(|p| &p.name)` not compile while `sort_by(|a, b| a.name.cmp(&b.name))` does, and what does `for<'a> FnMut(&'a T) -> K` have to do with it?
- Exercises:
  - `closure5` — `fn apply_to_local<'a, F: Fn(&'a str) -> usize>(f: F)` calls `f` on a local `String`; remove the fn-level lifetime (`F: Fn(&str) -> usize` is sugar for `for<'a>`). Fails unsolved: E0597 `s` does not live long enough.
  - `closure6` — `fn stats<'a, C: 'a>(c: C) -> (usize, i32, Option<i32>) where &'a C: IntoIterator<Item = &'a i32>` iterates `&c` three times; make it `where for<'a> &'a C: IntoIterator<Item = &'a i32>`. Fails unsolved: E0597 `c` does not live long enough.
  - `closure7` — `Pipeline { stages: Vec<Box<dyn Fn(&str) -> &str>> }`, where `build_pipeline` binds `let trim = |s: &str| s.trim();` before adding it; pass closures inline so the `for<'a>` bound drives inference, or use a `constrain<F: for<'a> Fn(&'a str) -> &'a str>(f: F) -> F` helper. Fails unsolved: E0308 "one type is more general than the other", plus "lifetime may not live long enough".
  - `closure8` — `async fn retry<F, Fut>(f: F, n) where F: FnMut() -> Fut`, called as `retry(|| attempt(&mut log), 5)`; rebound to `F: AsyncFnMut() -> Option<u32>` and call it with `async || attempt(&mut log).await`. Fails unsolved: "captured variable cannot escape `FnMut` closure body".
- Authoring notes:
  - closure7 emits E0308 "one type is more general than the other" as well as the code-less "lifetime may not live long enough", so don't describe it as "no error code".
  - Verified: E0597 for the fn-level `'a` on a local `String` and for the non-HRTB `IntoIterator` bound; `for<'a> &'a C: IntoIterator` works for `Vec`, `[i32; 3]`, `VecDeque` and `BTreeSet`; an inline `Box::new(|s| s.trim())` infers from the dyn bound; `for<'a> |..|` closure binders are E0658 (unstable, README); `AsyncFnMut` is in the 2024 prelude and `async || attempt(&mut log).await` compiles.
  - Each exercise is a standalone bin, so copy `block_on` into closure8 (as `futures1..4` do).
  - Tests: closure5 `apply_to_local(|s| s.len()) == 11` and a word count of 2; closure6 gives `(3, 6, Some(3))` for `Vec`, array, `VecDeque` and `BTreeSet` and `(0, 0, None)` for an empty `Vec`; closure7 `run("  > hello world ") == "hello"` with the result pointer inside the input; closure8 returns `Some(7)` after exactly 3 attempts.
  - *(scope)* Anchor the module on the HRTB-shaped error people actually hit in live coding: `sort_by_key(|p| &p.name)`, which the relevance review checked fails on 1.96 with "lifetime may not live long enough". closure5 and closure6 are the useful core; closure7 is a quirk and closure8 (async closures) is too new to be asked much, so keep both short.

### `45_sized_deref` (`?Sized` part) (built)

Theme: `?Sized`, DSTs and forwarding impls. The Deref / Borrow / Cow half is in Tier 1.

**As built** (see `exercises/45_sized_deref/`; adversarially reviewed). Deviations and verified corrections:

- `sized1`: Deviation (kept): adds a Part B, `fn trimmed(text: &impl AsRef<str>) -> &str`, showing that `impl Trait` in argument position has the implicit `Sized` bound too. rustc 1.96 suggests `&impl AsRef<str> + ?Sized`, which fails with "ambiguous `+` in a type"; the follow-up help gives the parenthesized form. Part A follows the ROADMAP ("a,b", "1,x", sized T still works). The tests turbofish `join_all::<str>` and `join_all::<dyn Display>`, so `&[&dyn Display]` and `&[T]` signatures don't pass. Review change: the header now quotes the real E0277 message for by-value `T: ?Sized`; the "function arguments must have a statically known size" line is only rustc's help. Tests unchanged.
- `sized2`: Follows the ROADMAP (a boxed dyn slice sums to PI + 4, `&[&c, &c]` is 2 * PI, the borrowed mock records calls after the service used it). Additions: the trait has a provided `name` that the wrapper must forward, and the tests also cover `Box<Square>`, `&dyn Shape`, nested `&Box<dyn Shape>` / `Box<&dyn Shape>`, an empty slice, `&dyn Mailer`, two services sharing one mock, and an owned mock. The `impl<T: Mailer + ?Sized> Mailer for &T` forwarding stays in, per the module note (50_testing_seams/README.md deferred it here; seams1 doesn't have it). That overrides the ROADMAP's "keep only one" only in form: the mock is given and minimal, and it points to seams1 instead of re-drilling it. Review change: added the test `trait_objects_with_auto_traits_are_shapes` (`Box<dyn Shape + Send>` and `&(dyn Shape + Sync)`). Without it, sized generic impls plus concrete `Box<dyn Shape>` / `&dyn Shape` impls passed every test while skipping `?Sized`. Header, TODO, hint and README updated to match. One non-idiomatic alternative is still accepted: the `Deref` blanket (documented).
- `sized3`: Kept despite the *(scope)* niche note, and kept short (6 tests). The module note prefers keeping it, and it closes the fn -> impl -> struct progression and answers the ROADMAP interview question "Can a struct have an unsized field?". Deviations: the starter declares `payload` before `id`, so the learner also meets the last-field rule. The starter's single `impl<T>` has `new` next to `id`/`payload`, so relaxing it shows that by-value methods need `T: Sized` back. `checksum(&Packet<[u8]>)` and `describe(&Packet<dyn Display>)` are given rather than written by the learner (the ROADMAP had the learner write `checksum`); this keeps the niche exercise short. All ROADMAP checks are present: checksum == 10, payload().to_string() == "7.5", `size_of::<&Packet<[u8]>>()` == 2W, `&Packet<[u8; 4]>` == W, `size_of_val(&*b)` == 8. Review change: added the test `every_unsized_payload_gets_the_methods` (`[u16]` and `dyn Debug` payloads). Without it, concrete `impl Packet<[u8]>` / `impl Packet<dyn Display>` blocks passed. Header "all in one allocation" is now "stored together in one value". The TODO and solution comment are updated.

#### `?Sized`, DSTs and forwarding impls for `&T` and `Box<T>`

- Slug `sized-dst-forwarding` · placement: graded · module: `45_sized_deref` (`sized1..3`) · interview value: **3/5**
- Sharpest question: Why does `fn f<T: Display>(x: &T)` reject a `&str` argument, and why doesn't `Box<dyn Shape>` implement `Shape` automatically?
- Exercises:
  - `sized1` — `fn join_all<T: Display>(items: &[&T], sep)` is called with `str` and `dyn Display` items; relax to `T: Display + ?Sized`. Fails unsolved: E0277 the size for values of type `str` cannot be known at compilation time.
  - `sized2` — `fn total_area<S: Shape>(&[S])` must accept `&[Box<dyn Shape>]` and `&[&Circle]`, and a second test passes `&mock` to `SignupService<M: Mailer>`; write `impl<S: Shape + ?Sized> Shape for Box<S>` and for `&S`, and `impl<T: Mailer + ?Sized> Mailer for &T`. Fails unsolved: E0277 `Box<dyn Shape>: Shape` not satisfied; without `?Sized`, E0277 the size of `dyn Shape` cannot be known.
  - `sized3` — `struct Packet<T> { id: u32, payload: T }`, where tests unsize `Box<Packet<[u8; 4]>>` to `Box<Packet<[u8]>>` and use `&Packet<dyn Display>`; declare `T: ?Sized` (it is the last field) and write `checksum(&Packet<[u8]>)`. Fails unsolved: E0277 the size for `[u8]` cannot be known (implicit `T: Sized`).
- Authoring notes:
  - Verified: E0277 "size for values of type `str`" from the implicit `Sized` bound; with only `impl<S: Shape> Shape for Box<S>`, `total_area(&Vec<Box<dyn Shape>>)` gives E0277 "size of `dyn Shape`"; `Packet<[u8]>` without `?Sized` gives E0277; struct unsizing `Box<Packet<[u8; 4]>>` to `Box<Packet<[u8]>>` is stable when the last field is generic.
  - Folding the `impl Mailer for &T` forwarding (the dropped `tests5`) into sized2 duplicates `seams1`'s mock; keep only one.
  - Tests: sized1 `"a,b"`, a mixed dyn slice `"1,x"`, and sized `T` still works; sized2 boxed slice sums to `PI + 4`, `&[&c, &c]` is `2 * PI`, the borrowed mock records the call after the service used it; sized3 `checksum == 10`, `payload.to_string() == "7.5"`, `size_of::<&Packet<[u8]>>()` is 2W and `&Packet<[u8; 4]>` is W, `size_of_val(&*b) == 8`.
  - *(scope)* sized3 (a user-defined DST with an unsized last field) is niche; it could move to the README or the layout quiz.

### Extend `32_dispatch` (built)

Theme: `Any` downcasting via trait upcasting; generic methods vs dyn compatibility (`dispatch5..6`).

**As built** (see `exercises/32_dispatch/`): `dispatch5` downcasts with a generic `all_of::<T: Any>()` (so a closed-world `as_circle` hook cannot pass) and fails at run time (0 found instead of 2). `dispatch6` was kept despite the *(scope)* note: it teaches the case where `dispatch2`'s `where Self: Sized` is the wrong fix, and pins a `&mut dyn Visitor` signature.

#### Advanced trait objects: downcasting via Any and trait upcasting, generic methods vs dyn compatibility

- Slug `trait-objects-advanced` · placement: graded · module: extend `32_dispatch` (`dispatch5`, `dispatch6`) · interview value: **3/5**
- Sharpest question: How do you get a `&Circle` back out of a `Box<dyn Shape>`, and why does the downcast silently return `None` when done through `&Box<dyn Shape>`?
- Exercises:
  - `dispatch5` — `circles(&[Box<dyn Shape>])` does `let any: &dyn Any = s;` with `s: &Box<dyn Shape>`, then `downcast_ref::<Circle>()`; add `trait Shape: Any (+ Debug)` and upcast the inner object with `s.as_ref()` (trait upcasting, stable since 1.86). Fails unsolved: assertion, `circles()` returns 0 instead of 2 (the `Box`'s `TypeId` is checked).
  - `dispatch6` — `trait Node { fn accept<V: Visitor>(&self, v: &mut V); }` used as `Vec<Box<dyn Node>>`; change it to `accept(&self, v: &mut dyn Visitor)` (`where Self: Sized` is the wrong fix here). Fails unsolved: E0038 "the trait `Node` is not dyn compatible because method `accept` has generic type parameters".
- Authoring notes:
  - Verified on 1.96: `let any: &dyn Any = s` with `s: &Box<dyn Shape>` silently checks the `Box`'s `TypeId`, so it counts 0; `s.as_ref()` with trait upcasting (`Shape: Any + Debug`) yields `[1.0, 3.0]`; `fn as_debug(&dyn Shape) -> &dyn Debug` compiles; a generic `accept<V: Visitor>` gives E0038 "not dyn compatible". The silent 0-vs-2 assertion is a strong fail mode.
  - dispatch6 test: a `CountingVisitor` over `[Leaf, Branch, Leaf]` tallies leaf 2, branch 1.
  - README: the old `as_any` pattern, clippy's `type_id_on_box`, why `Any` needs `'static`.
  - *(scope)* dispatch6 overlaps `dispatch2` and the dyn slot in `quiz5_compiles`; keep only one of them.
  - The default-object-lifetime exercise originally proposed for this module moved to `static-lifetimes` (lifetimes8).

### Quizzes after `47_type_level` (built as `quizzes/quiz4`)

Theme: auto-graded predict and will-it-compile drills for method resolution, variance and borrowck.

**As built** (see `exercises/quizzes/`; adversarially reviewed). Deviations and verified corrections:

- `quiz4`: Following the *(scope)* note, the ROADMAP's three quizzes (quiz4_resolution, quiz5_compiles, quiz6_borrowck) are merged into ONE quiz4, placed after 47_type_level. Dropped: the autoref predictions ((&s).name(), (&&s).who(), r.clone() on &NoClone with its noop_method_call issue, the Rc::clone count) and every variance and dyn slot of quiz5_compiles, which 38_variance covers. Part A is a fix-it with E0034 (Pilot and Wizard `fly` on Human) and E0790. The E0790 site is GENERIC (`fn baby_name_of<T: Animal>()`), so rustc's help (`<Dog as Animal>` / `<Cat as Animal>`) is a wrong answer, and the tests catch it with a Duck defined in the tests. The fix is `<T as Animal>::baby_name()`, not the ROADMAP's concrete `<Dog as Animal>::baby_name()`. Dog's inherent `baby_name` ("Spot") catches the `Dog::baby_name()` wrong fix. The ROADMAP's `<Human as Pilot>::fly(&h)` also passes, through deref coercion. Part B keeps all 8 verified borrowck verdicts and adds 2 verified two-phase items (`v.push(v.pop().unwrap_or(0))` is E0499; array `a[a.len() - 1] = 7` compiles), 10 verdicts in all. Instead of one `[Verdict; 8]` array fingerprint, each question has its own test with an FNV-1a fingerprint salted by the question id. A failure therefore names the wrong question without revealing the answer. Unanswered placeholders replace the ROADMAP's E0425-first fail mode. Problem case #3 is Q9; it is only cross-referenced to 37_borrowck_errors/borrowck3, not re-explained. The header no longer quotes the ROADMAP's sharpest question verbatim, because that would give away Q1 and Q3.

#### Auto-graded drills: method resolution and fully qualified syntax, will-it-compile, borrowck verdicts

- Slug `resolution-compile-quiz` · placement: quiz · module: `quizzes/quiz4_resolution`, `quiz5_compiles`, `quiz6_borrowck` (after `47_type_level`) · interview value: **3/5**
- Sharpest question: Why does `v.push(v.len())` compile while `v[v.len() - 1] = 7` does not?
- Exercises:
  - `quiz4_resolution` — fill `const P1..P6` with method-resolution predictions (inherent vs trait method, `Hello::hi(&s)`, impls for both `S` and `&S`, a by-value `who(self)` through `&&s`, `r.clone()` on `&NoClone`, an `Rc::clone` count), plus a fix-it part needing `<Human as Pilot>::fly(&h)` and `<Dog as Animal>::baby_name()`. Fails unsolved: "prediction N is wrong" (no value shown); the fix-it part fails with E0034 and E0790.
  - `quiz5_compiles` — six slots; uncomment the one candidate per slot that compiles (variance conversions, `*const` vs `*mut`, `Box<dyn Fn() + 'static>` to `+ 'a`, an invariant `FnMut(&'a str)`, and which trait is dyn-usable). Fails unsolved: E0425 cannot find function `slotN` until one is chosen; wrong picks give borrowck errors or E0038.
  - `quiz6_borrowck` — eight snippets in `#[cfg(any())]` blocks (flip to `cfg(all())` to see the error); fill a `[Verdict; 8]`. Fails unsolved: the fingerprint (FNV of the answers) doesn't match; answers start as `Unanswered`.
- Authoring notes:
  - quiz4: calling `r.clone()` on `&NoClone` triggers rustc's warn-by-default `noop_method_call` ("call to `.clone()` on a reference in this situation does nothing"), so the solution fails `clippy -D warnings` unless it has `#[allow(noop_method_call)]`, and the warning text gives the answer away.
  - Verified quiz4 answers on 1.96: `(&s).name() == "S"`, `(&&s).who() == "&S"`; two traits that both define `fly` give E0034 only when there is no inherent `fly`; `Animal::baby_name()` gives E0790.
  - Verified quiz5 slots: `&Vec<&'static str>` to `&Vec<&'a str>` compiles (the `&mut` form fails); `fn(&'a str)` to `fn(&'static str)` compiles; `&'a mut &'static str` to `&'a &'a str` compiles; `*const` is covariant and `*mut` is not; `Box<dyn Fn() + 'static>` to `+ 'a` compiles; `FnMut(&'static str)` to `FnMut(&'a str)` fails.
  - Verified quiz6 verdicts: `v.push(v.len())` compiles (two-phase borrow); `v[v.len() - 1] = 7` is E0502; `mem::replace(&mut v, v.clone())` is E0502; `let r = &mut v; r.push(r.len())` compiles; `let r = &String::from("x")` compiles (lifetime extension); `.as_str()` on a temporary is E0716 when used; problem case #3 is E0499 (may change under Polonius); `let f = |s: &str| -> &str { s }` errors.
  - *(scope)* Merge down to one quiz: fully qualified syntax (E0034 / E0790) plus the borrowck verdicts. quiz4's autoref puzzles (`(&&s).who()`) are trivia, and quiz5 duplicates `variance-phantomdata`. Keep problem case #3 canonical in `borrowck3` and cross-reference it. This quiz also covers the will-it-compile set of the cut `algorithm-semantics-quiz`.

### `48_macros_deep` (built)

Theme: `macro_rules!` repetition, recursion, hygiene, `$crate`, code generation.

**As built** (see `exercises/48_macros_deep/`; adversarially reviewed). Deviations and verified corrections:

- `macros5`: Tests go beyond the ROADMAP's "capacity at least 3". That check cannot tell pre-sizing from growth: both give capacity 3 for 3 keys. So the tests compare `capacity()` with `HashMap::with_capacity(n).capacity()` for the same K/V types, including five pairs that share one key (pre-sized: 7, grown on demand: 3). Also added: tt-group counting, count! in a const and as an array length, a 20-token count, an evaluation-order log, a non-Clone value moved in (pointer check), a call from a module with no `use` lines, and a given caller `status_names()` so `cargo build` fails too. Review: header and hint now use the current `${count($x)}` syntax; the count! TODO wording and both TODO endings are fixed.
- `macros6`: As in the ROADMAP (run-time failure, left 7 / right 3). The starter already has the `$(,)?` base case, so it compiles and fails only at run time. Non-Copy values are built inline so the starter never hits E0382. Added tests: evaluation-order log, 10 increasing arguments (1023 vs 10 calls), a side effect in a block argument, f64/&str/char (rejects `std::cmp::max`), non-Copy by-value arguments (rejects binding by reference, E0507), hygiene with caller variables named first/rest/a/b, and nested calls. Review: added a non-Copy, non-Clone `Bid` case so the TODO's "neither Copy nor Clone" requirement is enforced (rejects `.clone()`), and fixed the TODO ending. The no-helper-function rule is not machine-checked; a PartialOrd helper still evaluates each argument once.
- `macros7`: strict_clippy = true is an addition to the ROADMAP. Inside one crate, no test can tell `crate::util::clamp_percent` from `$crate::...`, so clippy's warn-by-default `crate_in_macro_def` under -D warnings is what grades the `$crate` lesson. Verified: the `crate::` variant builds and passes 8/8 tests, then fails clippy -D warnings. The starter still fails at compile time (E0425), so dev check's unsolved check is unaffected. Design additions: the helper returns a `Percent` newtype with a private field, so the macro cannot inline the clamp (E0603) and tests use `percent!(..).get()`; `pub mod util`; a given `report` caller; the no-import and impostor callers sit two modules deep, so `super::util::` cannot resolve by accident. Review: TODO ending fixed; no other change. Graded by clippy: registered with `strict_clippy = true`.
- `macros8`: Table syntax extends the ROADMAP's `name: input => expected;` rows. It adds a leading `function;` header, so the macro is reusable: two tables test `slugify` and `word_count`, which catches a hard-coded function. Rows may carry attributes through a `meta` fragment; one row has `#[should_panic(expected = "left == right")]`. The ROADMAP's "three generated tests run by name" check became two checks. First, the tables live in a `#[deny(dead_code)]` module, so a missing `#[test]` is a compile error; the source attribute overrides the Cargo `-A dead_code`. Second, a `catch_unwind` test calls the should-panic row by name to prove its body asserts. The newtype tests supply the literal `#[test]`, including a second invocation with a trailing comma from a module with no `use` lines. Review: the Part B TODO now asks for `assert_eq!` (the should_panic row expects its message), and both TODO endings are fixed.

#### Declarative macros in depth: repetition, recursion, single evaluation and hygiene, `$crate`

- Slug `macro-rules-deep` · placement: graded · module: `48_macros_deep` (`macros5..8`) · interview value: **3/5**
- Sharpest question: Write a `hashmap!{ k => v, .. }` macro that accepts a trailing comma and pre-sizes the map. Why must you bind `$x` once in `max!`, and why use `$crate::`?
- Exercises:
  - `macros5` — `hashmap!{}` and `count!()` stubs, while tests call `hashmap!{"a" => 1, ..,}` and `count!(a b c d)`; add an arm `($($k:expr => $v:expr),+ $(,)?)` pre-sized with a recursive tt `count!`. Fails unsolved: `` no rules expected `"a"` `` (no E-code).
  - `macros6` — a recursive `max!` re-evaluates its arguments; bind each once in a block, with base case `($x:expr $(,)?)`. Fails unsolved: runtime assertion, the call-counting closure reports left 7, right 3.
  - `macros7` — a `#[macro_export] percent!` inside `mod util` expands to `clamp_percent($e)` and is invoked from `mod tests`; use `$crate::util::clamp_percent`. README: mixed-site hygiene (locals resolve at the definition site, items at the call site). Fails unsolved: E0425 cannot find function `clamp_percent` in this scope.
  - `macros8` — stubs with no matching arm: `newtype_units!(Meters, Seconds)` generating a derive, `Add` and a `Display` via `stringify!`, and `test_cases! { name: input => expected; .. }` generating one `#[test]` per row. Fails unsolved: "no rules expected ..." from both stubs.
- Authoring notes:
  - Keep a separate `()` arm in `hashmap!`: a single `$(..),*` arm used with zero repetitions emits `unused_mut`, which fails the solution's `-D warnings` (verified).
  - Verified: a stub arm gives `` no rules expected `"a"` ``; a `#[macro_export]` macro calling `clamp_percent($e)` from another module gives E0425 (items resolve at the call site).
  - macros8: the stubs must not expand to nothing (otherwise zero tests would run and pass), and the file still needs a literal `#[test]` outside the generated tests because `dev check` greps for it; the newtype tests provide one.
  - Tests: macros5 len 3, trailing comma accepted, capacity at least 3, empty map, `count == 4`; macros6 `max!(next(3), next(9), next(4)) == 9` with exactly 3 calls, and a hygiene test shows the caller's `a` / `b` unchanged; macros7 `percent!(-5) == 0`, `percent!(42) == 42`, `percent!(1_000) == 100`; macros8 `Meters(1.5) + Meters(2.0) == Meters(3.5)`, `"2 Seconds"`, three generated tests run by name.
  - Fold-in from the cut `proc-macro-lab`: link dtolnay's proc-macro-workshop from the README, with a short "declarative vs procedural" Q&A (when you need a proc macro instead of `macro_rules!`, why it must live in its own crate, how to emit a span-accurate error).

### `49_panics` (built)

Theme: `catch_unwind`, payloads, poisoning, panic safety.

**As built** (see `exercises/49_panics/`; adversarially reviewed). Deviations and verified corrections:

- `panic1`: Two change sites: `panic_message(payload: &(dyn Any + Send)) -> String` (the type std's panic hook payload has, so the `&payload` Box trap from dispatch5 fails the tests) and `run_isolated`, which starts as `Ok(f())` per the ROADMAP. Beyond the ROADMAP tests (Ok path, "boom", "job 7 failed" formatted at run time, panic_any(42u8) fallback, a `&mut` capture) it adds a `&Cell` capture and an `Rc` result (these reject UnwindSafe/Send bounds and thread-based versions), std's own unwrap/expect messages, one panicking job not stopping the others, and a `JoinHandle::join` payload. Review: the header's claim that a library cannot know the panic strategy is corrected (`cfg(panic)` is stable).
- `panic2`: The ROADMAP has `read_total` call `lock().unwrap()` itself. Here `post`, `read_total` and `entries` all go through one `lock_ledger` helper, which is where a poisoning policy belongs, so the TODO sits there. Poisoning is deterministic: a spawned worker pushes and panics before updating `total`, then the test calls `join()`. Beyond the ROADMAP tests (is_poisoned before, total == sum(entries) after, a later lock() Ok) it checks that every entry is kept (rejects reset/pop), that the repair happens in place (heap-pointer check), that only a poisoned lock is repaired (probe test), a crash after a complete update, that a panic outside the lock poisons nothing, a second crash (needs clear_poison), and 4 workers continuing after a crash. Review: the TODO now requires holding the lock throughout, because the drop/clear/relock variant passes every test while being racy; the header's RwLock-reader rationale is corrected.
- `panic3`: Uses the same `Ledger { entries, total }` as panic2 (the ROADMAP's `items` became `entries`), so part 3 reads as 'never leave anything to repair'. The ROADMAP *(scope)* note calls panic3 senior-niche but short, so it was kept with a one-function change site. Added tests: overflow (each converted entry fits, the sum does not; rejects committing the entries before the total), a once-per-entry in-order call log (rejects validate-then-apply), a panic on the last entry, an empty ledger, reuse after a failed apply, and a Mutex test tying back to panic2. Undetectable without unsafe, so forbidden only in the TODO: a snapshot restored via catch_unwind + resume_unwind or a drop guard. Review: C++ guarantee naming (no-throw) and the `Ledger::new` overflow doc were corrected.

#### Panics: catch_unwind, UnwindSafe and payloads, Mutex poisoning recovery, panic-safe invariants

- Slug `panics-unwind-poison` · placement: graded · module: `49_panics` (`panic1..3`) · interview value: **3/5**
- Sharpest question: What is Mutex poisoning, when is it correct to recover with `into_inner`, and what do `UnwindSafe` / `AssertUnwindSafe` actually guarantee?
- Exercises:
  - `panic1` — `run_isolated<R>(f: impl FnOnce() -> R) -> Result<R, String>` starts as `Ok(f())`; use `catch_unwind(AssertUnwindSafe(f))` and downcast the payload (`&'static str` or `String`, else a fallback). One test closure captures `&mut`, so a `+ UnwindSafe` bound fails. Fails unsolved: the panic propagates and the test fails; calling `catch_unwind(f)` directly is E0277 "may not be safely transferred across an unwind boundary".
  - `panic2` — `Ledger { entries, total }` in a `Mutex` is poisoned by a thread that pushes and then panics before updating `total`, and `read_total` uses `lock().unwrap()`; recover with `unwrap_or_else(PoisonError::into_inner)`, repair the invariant, and `clear_poison()`. Fails unsolved: the test panics with "called `Result::unwrap()` on an `Err` value: PoisonError { .. }".
  - `panic3` — `Ledger::apply(f: impl FnMut(i64) -> i64)` mutates items in place while updating `total`; make it panic-safe by computing the new items first, then committing both fields. Fails unsolved: assertion, after a caught panic on the 2nd element the items are `[10, 2, 3]` but `total` is 10.
- Authoring notes:
  - Verified: `catch_unwind(f)` on an unbounded `impl FnOnce() -> R` gives the E0277 above; the poisoned-lock message is "called `Result::unwrap()` on an `Err` value: PoisonError { .. }"; `clear_poison()` is stable (1.77).
  - Test targets unwind even though dev/Cargo.toml's profiles set `panic = "abort"` (Cargo ignores it for tests), but `main()` does not unwind, so all `catch_unwind` logic must live in `#[test]`s. Keep the abort cases (a panic in `Drop` during unwinding, a panic across `extern "C"`) in the README, since they would kill the test process.
  - Tests: panic1 `Ok` path, `panic!("boom")` gives `"boom"`, a formatted panic gives `"job 7 failed"`, `panic_any(42u8)` gives the fallback; panic2 `is_poisoned()` is true before, `total == sum(entries)` after, and a later `lock()` is `Ok`.
  - *(scope)* panic1 and panic2 are the valuable pieces; panic3 is senior-niche but short.

### Extend `36_atomics` (built)

Theme: weak-CAS / `fetch_update` loops and ABA (`atomics4..5`).

**As built** (see `exercises/36_atomics/`): `atomics4` keeps the load-then-store starter but makes the lost race deterministic with a `race_window` test seam, so it fails on every run; the 8-thread stress tests remain as solution-quality checks. The solution uses a `compare_exchange_weak` loop; the hint shows `try_update` (stable since 1.95; `fetch_update` is being deprecated in 1.99). `atomics5` confines the learner's change to `next_head` and cross-references the planned Treiber-stack lab; update those references when the lab lands. The module README's std links now point at `Atomic<T>`, because the old `AtomicUsize` / `AtomicBool` doc pages return 404.

#### CAS retry loops and the ABA problem

- Slug `atomics-cas-aba` · placement: graded · module: extend `36_atomics` (`atomics4`, `atomics5`) · interview value: **3/5**
- Sharpest question: Implement a bounded atomic counter with a CAS loop. Why `compare_exchange_weak` in a loop, and what is the ABA problem?
- Exercises:
  - `atomics4` — `BoundedCounter::try_inc(&self, max) -> Result<u64, Full>`, implemented with a `compare_exchange_weak` loop (or `fetch_update`). Fails unsolved: must be redesigned, see notes (the proposed load-then-store race is not a reliable fail mode).
  - `atomics5` — a safe lock-free free-list over an arena: `head: AtomicU64` packs `(tag, idx)`, and pop is split into `pop_begin` / `pop_commit` so tests can force an interleaving; the new-head helper always writes tag 0, so increment the tag on every successful CAS. Fails unsolved: a deterministic single-thread ABA scenario where the stale `pop_commit` returns `Ok(0)` and drain re-lists slot 1 (expected `Err` and `[0, 2]`).
- Authoring notes:
  - Not feasible as proposed: atomics4's fail-while-unsolved relied on a load-then-store race actually showing up in 8 x 10_000 increments. That is nondeterministic; on a lightly loaded or low-core CI runner the unsolved exercise could pass, and `rustlings dev check` would report it as already solved. Start from an empty body (E0308) or add a deterministic interleaving hook (as atomics5 does), and keep the 8-thread stress test (final value `== min(total, max)`, successes `==` final value) as a solution-quality check.
  - atomics5's split `pop_begin` / `pop_commit` scenario is deterministic and feasible in safe Rust. Tests: the stale commit is `Err`, a retry pops 0, and an 8-thread stress run drains `0..64` exactly.
  - `fetch_update` and the newer `try_update` both compile without deprecation warnings on 1.96.
  - *(scope)* The arena-plus-tag ABA in atomics5 is contrived: keep it short, or make it a design explanation that points to the Treiber stack in `lock-free-ordering-lab`.

### Quiz `quiz7_send_sync` (built as `quizzes/quiz5`)

Theme: `Send` / `Sync` of std types. Proposed position: after `55_thread_pool`, which is cut, so place it after the last concurrency module.

**As built** (see `exercises/quizzes/`; adversarially reviewed). Deviations and verified corrections:

- `quiz5`: Built as quiz5 (the ROADMAP's 'quiz7_send_sync / quiz_send_sync'), placed after 36_atomics. The `quiz_orderings` part (store buffering and IRIW) is dropped: per the *(scope)* note it moves to the planned lock-free-ordering-lab. Instead of 12 loose booleans, the format is 14 types, each classified SendAndSync / SendOnly / SyncOnly / Neither (28 facts). It keeps all 12 ROADMAP facts and adds `Cell<i32>`, `MutexGuard<'_, Cell<i32>>` and `Box<dyn Fn() + Send>`. Unanswered placeholders replace the ROADMAP's E0425-first fail mode. The tests grade against the compiler through the inherent-impl-precedence probe, which the scope note allows. The probe is used on concrete types only. So no expected value appears in the source or in failure messages, and each failure asks a leading question instead of stating the rule. I cross-checked all 28 facts on 1.96 without the probe: assert_send / assert_sync compile for every true fact, and every false fact is an E0277 compile failure. The tests use no threads.
- The store-buffering and IRIW questions moved to `lock-free-ordering-lab` (see its Receives note).

#### Quiz: Send and Sync of std types, store-buffering outcomes

- Slug `send-sync-ordering-quiz` · placement: quiz · module: `quizzes/quiz7_send_sync` · interview value: **3/5**
- Sharpest question: Is `Mutex<Cell<i32>>` Sync? Is `RwLock<Cell<i32>>`? Is `MutexGuard` Send? State the rule behind each answer.
- Exercises:
  - `quiz_send_sync` — answer about 12 booleans: `Mutex<Cell>` Sync, `RwLock<Cell>` Sync, `Arc<Cell>` Send, `&Cell` Send, `&mut Cell` Send, `MutexGuard<u32>` Send and Sync, `Receiver` Sync, `Sender` Sync, `Mutex<Rc>` Send, `*const u8` Send, `Arc<Mutex<RefCell>>` Send. Fails unsolved: E0425 until the constants are defined, then an assertion with an explanatory rule message.
  - `quiz_orderings` — `SB_REL_ACQ_BOTH_ZERO_ALLOWED`, `SB_SEQCST_BOTH_ZERO_ALLOWED`, plus an IRIW question. Fails unsolved: E0425, then an assertion with an explanation.
- Authoring notes:
  - Verified answers: `Mutex<Cell>` Sync = true; `RwLock<Cell>` Sync = false; `Arc<Cell>` Send = false; `&Cell` Send = false; `&mut Cell` Send = true; `MutexGuard` Send = false, Sync = true; `Receiver` Sync = false; `Sender` Sync = true (since 1.72); `Mutex<Rc>` Send = false; `*const u8` Send = false; `Arc<Mutex<RefCell>>` Send = true.
  - Store buffering: Release/Acquire allows both threads to read 0; SeqCst forbids it (verified).
  - IRIW: "not observable on x86 / ARMv8" only holds for acquire (or dependency-ordered) loads. With `Relaxed` loads ARMv8 may reorder the two reads and the outcome is observable, and the Rust/C++ model permits IRIW even under Acquire/Release. The question must name the load ordering and say it asks about hardware, not language guarantees.
  - *(scope)* Check the Send/Sync answers with plain `assert_send` / `assert_sync` plus compile-fail cases rather than a fragile autoref probe. If a probe is kept, it must use the inherent-impl const trick (see `variance-phantomdata`).
  - *(scope)* Move the store-buffering litmus items to `lock-free-ordering-lab`.
  - The message-passing litmus constants were dropped during the merge (`atomics2` and `loom_lab` already cover them).

### Extend `27_data_structures` (built)

Theme: LeetCode `ListNode` classics (`linkedlist2..4`).

**As built** (see `exercises/27_data_structures/`): Correction to the authoring note: address checks alone do NOT reject a rebuilt list, because the allocator reuses freed blocks LIFO; the tests also number every node (a serial id). A relink through a `Vec<Box<ListNode>>` still passes, since it reuses the original nodes; the TODOs forbid it in prose. Recursive answers are rejected by 200_000-node tests, which abort the test binary with a stack overflow rather than printing FAILED.

#### LeetCode ListNode classics: reverse, merge with a tail cursor, split-half and remove-nth

- Slug `linked-list-classics` · placement: graded · module: extend `27_data_structures` (`linkedlist2..4`) · interview value: **3/5**
- Sharpest question: Reverse a singly linked `Option<Box<ListNode>>` in place. Why doesn't the fast/slow-pointer middle-finding idiom translate directly?
- Exercises:
  - `linkedlist2` — LeetCode `ListNode` with an empty `reverse(head)`; write an in-place loop with `take()`. Fails unsolved: E0308 from the empty body.
  - `linkedlist3` — `merge(a, b)` is empty; the hint gives the `tail = &mut tail.insert(node).next` cursor. Fails unsolved: E0308.
  - `linkedlist4` — `split_half` written with fast/slow `&mut` and `&` cursors, and an empty `remove_nth_from_end`; count the length first, then walk a `&mut` cursor, using `checked_sub` for `n > len`. Fails unsolved: E0502 from `split_half`; E0308 from `remove_nth_from_end`.
- Authoring notes:
  - Comparing node addresses with `ptr::eq` or `as *const` casts is safe code, and `Box` contents don't move when the `Box` moves, so the "old first node is the new tail" identity check is sound; it makes "rebuild a new list" fail (also use node reuse checks in merge).
  - The empty-body E0308 starters are deterministic.
  - Tests: `[1, 2, 3]` reverses to `[3, 2, 1]`, empty and single-node lists; merge `[1, 4, 5]` with `[1, 2, 6, 7]`, empty inputs, duplicates; split `[1..=5]` into `([1, 2, 3], [4, 5])`; remove the 2nd from the end; `n > len` leaves the list unchanged.
  - The recursive-drop depth point is already covered by `smartptr1` and `linkedlist1` (including the 200k-node drop test); don't repeat it.

### `61_trees` (built)

Theme: an `Option<Box<Node>>` BST (the LeetCode `Rc<RefCell<TreeNode>>` half was cut).

**As built** (see `exercises/61_trees/`; adversarially reviewed). Deviations and verified corrections:

- `bst1`: Built as the ROADMAP specifies: the iterative Drop is given, and the empty insert/contains fail with E0308. Beyond the ROADMAP tests: exact shape via pre-order, `i32::MIN`/`i32::MAX` keys, a sorted-insert degenerate-spine test, and a 200_000-deep zigzag path built bottom-up in O(n) by the test helper `path`. Reviewer addition: `contains_looks_only_along_the_search_path`, a hand-built broken tree with a key hidden off the search path, so a whole-tree scan `contains` now fails.
- `bst2`: Deviation from the ROADMAP: besides the empty `next()`, `Bst::iter()` is empty too (a second E0308), so the learner writes the initial left-chain push; a helper method on `InOrder` is allowed. `IntoIterator for &'a Bst` is given code. Laziness is checked white-box (`it.stack.len() <= height` on a 1023-node perfect tree). Items are checked by address. The deep tests are a 200_000-deep chain of left children (catches a recursive helper) plus a zigzag. Reviewer changes: only the header's LeetCode 173 title.
- `bst3`: Built as the ROADMAP specifies: invert's empty `()` body compiles, and height gives E0308. Height counts nodes (LeetCode 104: empty is 0, a single node is 1). The invert tests compare every node's (key, left, right) by address before and after, so key rewriting or rebuilding fails. Both functions run on a 200_000-deep zigzag built in O(n). The legitimate alternatives pass: VecDeque invert, take-and-reassign swap, and DFS (node, depth) height. Reviewer changes: only the header wording (`insert` (or any search), since bst3 has no `contains`).
- `bst4`: Unsolved fails with the ROADMAP's E0499 (wording and note verified) plus two E0308s: the empty `remove` and the empty free function `take_min`, which the ROADMAP names but does not list as a TODO. `insert` is given code built on `find_slot`. `remove` must relink the successor NODE (every remaining key keeps its node address). The deep test uses a zigzag plus a root whose successor is 200_000 left links down. Reviewer changes: two non-leaf one-child removal cases in remove_a_node_with_one_child (they catch successor/predecessor-always removal). The header's borrow-rule sentence is made precise (bst1's insert is the counterexample to the old wording). 'textbook C version'. The TODO ends with 'Until you restructure the loop ...'.

#### Binary search tree with `Option<Box<Node>>`: cursor insert, lazy in-order iterator, iterative invert and height, delete

- Slug `bst-option-box` · placement: graded · module: `61_trees` (`bst1..4`) · interview value: **3/5**
- Sharpest question: Implement iterative insert and delete for a `Option<Box<Node>>` BST. Why does `while let Some(n) = cur { if .. { return cur } cur = &mut n.left }` fail to compile?
- Exercises:
  - `bst1` — a `Bst` with a provided iterative `Drop`, and empty `insert` / `contains`; write an iterative `&mut` cursor insert. Fails unsolved: E0308 from the empty bodies.
  - `bst2` — `InOrder<'a> { stack: Vec<&'a Node> }` with an empty `next()`; use `as_deref`. Fails unsolved: E0308.
  - `bst3` — `invert` (an empty `()` body, which compiles) and `height` (empty), both iterative with a `Vec<&mut Node>` stack and BFS. Fails unsolved: E0308 from `height`, then the invert tests fail.
  - `bst4` — `find_slot` with `return cur` inside `while let`, and an empty `remove`; restructure the loop (an `as_ref().is_some_and(..)` guard) and remove 0-, 1- and 2-child nodes with `take_min`. Fails unsolved: E0499 "cannot borrow `*cur` as mutable more than once" (the problem case #3 shape).
- Authoring notes:
  - Verified: bst4's `return cur` inside `while let Some(node) = cur` gives E0499 "cannot borrow `*cur` as mutable more than once".
  - A 200_000-deep spine overflows a 2 MiB test-thread stack under recursion, so it forces iterative code, and the `Drop` must be iterative too (as proposed).
  - Tests: shuffled inserts; duplicates return `false`; the 200_000-deep spine for insert, contains, iteration and height; in-order is sorted and `take(3)` gives the smallest; double invert is the identity; leaf, one-child and root removal keep sorted order; removing everything empties the tree.
  - *(scope)* bst4 overlaps `borrowck3` (problem case #3): cross-reference it rather than re-explaining.
  - bst2 is the first borrowing `Iter<'a>` over the learner's own container; the mutable counterpart is listed under [Additional topics](#additional-topics-not-yet-verified).

### `62_graphs` (built)

Theme: graph traversal and grids (the heaps / Dijkstra part was cut; its lessons live in `44_trait_contracts`).

**As built** (see `exercises/62_graphs/`; adversarially reviewed). Deviations and verified corrections:

- `graph1`: Merged per the ROADMAP (scope) note. The ROADMAP's graph1 (bfs + shortest_path), graph3 (Kahn topo_order with BinaryHeap<Reverse<usize>> and CycleError { unordered }) and graph4 (DSU with find and union by size) are now one exercise with five E0308 bodies. To keep it short, Kruskal's `mst_weight` is GIVEN, and its tests exercise the learner's `union`. The path-compression test uses a hand-built 200_000-node chain instead of the ROADMAP's 1_000, so a recursive find overflows the stack. Review change: two assertions added to union_reports_whether_it_merged, so a union that takes parent[x] for the root now fails.
- `graph2`: The centerpiece, as specified: `Graph { adj, seen }`, a recursive `dfs(&mut self)` over `&self.adj[u]` (E0502, verified wording), plus an empty `has_cycle_directed(&self)` (E0308). Deviation: the deep tests use 200_000 nodes instead of the ROADMAP's 100_000, for margin. The given `count_components` relies on `dfs` keeping the `seen` field. Review changes: the stack-depth paragraph now points to `61_trees` instead of re-teaching it; the has_cycle TODO requires O(V + E). Known pass-throughs, stated in the TODO and hint: cloning a neighbor list inside an ITERATIVE loop, and Kahn's algorithm for the cycle check.
- `graph3` (dropped): merged into graph1 (Kahn's topo_order with BinaryHeap<Reverse<usize>> and CycleError is graph1 part B), per the ROADMAP (scope) note.
- `graph4` (dropped): merged into graph1 (the DSU with an iterative path-compressing find and union by size is graph1 part C; Kruskal's mst_weight is given there and tested through union), per the ROADMAP (scope) note.
- `grid1`: As specified: a run-time failure, where the corner tests panic with "attempt to subtract with overflow". `neighbors` returns `impl Iterator<Item = (usize, usize)>` rather than a Vec, as the bridge to grid2's capture lesson. Added: a brute-force comparison over six grid shapes (rejects saturating_sub), a test with coordinates beyond i32::MAX and isize::MAX (rejects i32 casts and `as isize` arithmetic), and a GIVEN grid BFS `min_steps` whose maze tests start in the corner. The Index/IndexMut fold-in lives in grid2, where the `Grid` type is, so grid1 keeps the ROADMAP's pure run-time failure. No review changes to code or tests.
- `grid2`: Deviations: (1) the iterative flood fill in `count_islands` is GIVEN. The ROADMAP's E0502 on `self.cells` requires the write to sit inside the neighbor loop of an explicit-stack fill, so the learner's work is `+ use<>`. The 400x400 all-land test rejects a recursive rewrite (verified stack overflow). (2) Added test the_neighbor_iterator_does_not_borrow_the_grid: it mutates `grid.cells` and drops a grid while an iterator is alive, which forces the signature fix; it adds E0502 + E0597 to the unsolved errors. (3) The operator-overloading fold-in is here: Index<(usize, usize)> / IndexMut on `Grid` (E0608), a read/write test, and two #[should_panic] tests for `(0, cols)` (read and write). Accepted alternatives: `+ 'static`, and `Box<dyn Iterator>` (which allocates on every call). rustc's own help line prints `+ use<>`, so the first part is guided by the compiler. No review changes to code or tests.

#### Graph algorithms: BFS with paths, iterative DFS and cycle detection, Kahn topological sort, union-find

- Slug `graph-algorithms` · placement: graded · module: `62_graphs` (`graph1..4`) · interview value: **3/5**
- Sharpest question: Your recursive `fn dfs(&mut self, u: usize)` iterates `&self.adj[u]` and fails with E0502. Why, and how do you restructure it, recursively or iteratively?
- Exercises:
  - `graph1` — `bfs(adj, src) -> Vec<Option<usize>>` and `shortest_path` via a parent array, both empty. Fails unsolved: E0308.
  - `graph2` — `Graph { adj, seen }` with a recursive `dfs(&mut self)` that iterates `&self.adj[u]`, and an empty `has_cycle_directed`; use iterative DFS or disjoint borrows, and white-gray-black coloring with a `(node, edge_idx)` stack. Fails unsolved: E0502 "cannot borrow `*self` as mutable because it is also borrowed as immutable"; E0308.
  - `graph3` — `topo_order(n, edges) -> Result<Vec<usize>, CycleError>` using Kahn's algorithm with `BinaryHeap<Reverse<usize>>` for the lexicographically smallest order. Fails unsolved: E0308.
  - `graph4` — a DSU with iterative path-compressing `find`, union by size, and `mst_weight` (Kruskal), all empty. Fails unsolved: E0308.
- Authoring notes:
  - Verified: graph2's E0502 wording. The 100_000-node path test forces an iterative solution, since a recursive one aborts the test binary with a stack overflow.
  - Tests: graph1 distances, unreachable is `None`, a valid shortest path, `src == dst`; graph2 component counts, the 100_000-node path, a cycle, a DAG, a self-loop; graph3 the exact order on a DAG with many valid orders, isolated nodes, a 3-cycle `Err`; graph4 `union` returns `false` for an already joined pair, after one `find` on a 1_000-node chain every node's parent is the root, MST weight, disconnected gives `None`.
  - *(scope)* The algorithms themselves are better drilled on LeetCode; the Rust-specific value is graph2. Keep graph2 as the centerpiece and shorten graph1, graph3 and graph4 into one exercise.

#### Grid algorithms: underflow-free neighbours, islands with `+ use<>` neighbour iterators

- Slug `grid-algorithms` · placement: graded · module: `62_graphs` (`grid1`, `grid2`) · interview value: **3/5**
- Sharpest question: Write a neighbours function for a grid cell that cannot underflow at row or column 0, and count islands while mutating the grid you iterate over.
- Exercises:
  - `grid1` — `neighbors(r, c, rows, cols)` builds `(r - 1, c)` and so on; use `checked_add_signed` over direction pairs plus `then_some` bounds checks. Fails unsolved: the corner test panics with "attempt to subtract with overflow".
  - `grid2` — `Grid::neighbors(&self) -> impl Iterator` (its `move` closure copies `rows` / `cols`) is used while mutating `self.cells` in `count_islands`; add `+ use<>` and an iterative flood fill. Fails unsolved: E0502 "cannot borrow `self.cells` as mutable because it is also borrowed as immutable".
- Authoring notes:
  - Verified: `checked_add_signed` and `then_some` are stable; grid2's E0502 comes with the edition-2024 capture note, and `+ use<>` fixes it once the closure copies `rows` / `cols`; the corner overflow panic is deterministic because tests run with overflow checks on.
  - Tests: grid1 corner 2, edge 3, interior 4, and a 1x1 grid 0 neighbours; grid2 LeetCode 200 examples, all water 0, diagonal cells separate, 400x400 all land is 1 island.
  - This is the single home for the edition-2024 `+ use<>` lesson now that `impl-trait-capture-rpitit` is cut. Verified facts from that cut item for the README: `use<>` on a generic fn errors with "`impl Trait` must mention all type parameters in scope in `use<...>`"; the 2024 capture note reads "Rust 2024 has adjusted the `impl Trait` lifetime capture rules"; the 2021 `+ '_` / E0700 story only applies when the hidden type actually borrows.
  - Fold-in from the cut `operator-overloading`: one extra test implementing `Index<(usize, usize)>` / `IndexMut` on the grid. Verified: indexing before the impl exists is E0608 "cannot index into a value of type `Grid`"; add a `#[should_panic]` for `g[(0, g.w)]`, which must panic on the column check even though `row * w + col` is in range.

### `64_parsing` (built)

Theme: lexer, recursive descent and an AST.

**As built** (see `exercises/64_parsing/`; adversarially reviewed). Deviations and verified corrections:

- `parse1`: Deviation from the ROADMAP: the iterator's Item is `Result<(usize, Token<'a>), LexError>` instead of `Result<Token<'a>, LexError>`, so each token comes with its byte offset, like `char_indices` (LALRPOP uses a (start, token, end) triple). parse2 and parse3 need the offset to report `UnexpectedToken { pos }` and `TrailingInput { pos }`. Tokens are Num(i64), Ident(&'a str) and + - * / ( ). LexError is { UnexpectedChar { ch, pos }, NumberTooLarge { pos } }, with Display and Error. The given helpers `is_ident_start` / `is_ident_continue` pin the identifier grammar (Unicode alphabetic / alphanumeric, or `_`); digits are ASCII only. After an error the lexer resumes. All ROADMAP tests are present, plus tests for Unicode whitespace offsets, ASCII-only digits, leading zeros, resumption after errors, and a 3000-case model. Reviewer change: `tokens_outlive_the_lexer` now takes at most src.len() + 1 items, so a lexer that stops consuming fails that test instead of collecting forever.
- `parse2`: Deviations from the ROADMAP: the starter is parse1's lexer (given) plus a Parser with given `peek`, `bump` and a `nested` depth guard (MAX_DEPTH = 128, serde_json's default recursion limit). The right-recursive starter recurses through `nested`, so a long flat chain fails deterministically with TooDeep instead of overflowing the stack. EvalError adds Lex(LexError), UnexpectedToken { pos } and TooDeep to the ROADMAP's UnexpectedEnd, TrailingInput { pos }, DivByZero and Overflow, with Display, Error::source and From<LexError>. Identifiers are UnexpectedToken, since the calculator has no variables. The unsolved failures include exactly the ROADMAP's 8-3-2 = 7 (expected 3) and 8/4/2 = 4 (expected 1). 100_000 minus signs may give Ok(1) or TooDeep, so a loop-based unary minus passes. Reviewer change: the `term` and `factor` TODOs now name their failing tests.
- `parse3`: Deviations from the ROADMAP: the starter fails with two E0308s (an empty `from_str` AND an empty `Display::fmt`), where the ROADMAP names only from_str. `eval` is given rather than a TODO, because parse2 already drills checked arithmetic. Variables were added (`Var(String)`, `eval(&self, &HashMap<&str, i64>)`), so the lexer's identifiers get used and the point that FromStr output cannot borrow its input becomes concrete. Errors are split into ParseError (Lex, UnexpectedEnd, UnexpectedToken { pos }, TrailingInput { pos }, TooDeep) and EvalError (UnknownVariable, DivByZero, Overflow). Expr is `Binary(BinOp, Box<Expr>, Box<Expr>)`, `Neg`, `Num` and `Var`, with no Clone. The ROADMAP's 10-input Display round trip is `display_round_trips`. Also tested: tree shapes, 2000 random trees reparsed from both a minimal-parentheses printer and Display, and TooDeep for 100_000 nested parentheses or minus signs. The header and README note, verified but not tested, that a flat 100_000-term chain parses but overflows the stack when its tree is dropped.

#### Tokenizer and recursive-descent calculator: borrowed tokens, left associativity, never-panicking evaluation

- Slug `parser-calculator` · placement: graded · module: `64_parsing` (`parse1..3`) · interview value: **3/5**
- Sharpest question: Write an expression evaluator with a lexer that yields borrowed `Token<'a>` values. Why does naive right recursion give 8-3-2 = 7, and how do you avoid panicking on `i64::MIN / -1`?
- Exercises:
  - `parse1` — `Lexer<'a>`, an `Iterator` yielding `Result<Token<'a>, LexError>`, with an empty `next`; slice identifiers by byte offsets and map `PosOverflow` to `NumberTooLarge`. Fails unsolved: E0308 from the empty `next`.
  - `parse2` — the given right-recursive grammar evaluates `8-3-2` as 7; write loop-based left associativity, unary minus and checked arithmetic. Fails unsolved: assertions, `8-3-2` gives 7 (expected 3) and `8/4/2` gives 4 (expected 1).
  - `parse3` — an `enum Expr` AST with an empty `FromStr`, `eval`, and a fully parenthesized `Display`. Fails unsolved: E0308 from the empty `from_str`.
- Authoring notes:
  - Verified: right recursion gives `8-3-2 = 7` and `8/4/2 = 4`.
  - `checked_div` can't tell a zero divisor apart from `MIN / -1`, so check `divisor == 0` first to report `DivByZero` separately from `Overflow`.
  - `IntErrorKind::PosOverflow` is stable.
  - Tests: parse1 exact tokens, `"héllo"` as one identifier, `'$'` errors with its byte position, a huge literal errors, identifiers borrow the source; parse2 precedence, parentheses, unary minus, and `UnexpectedEnd`, `TrailingInput { pos }`, `DivByZero`, `Overflow` for `MIN / -1`, with nothing panicking; parse3 tree shape and a `Display` round trip for 10 inputs.

### `65_performance` (built)

Theme: allocation-aware APIs, clippy-graded.

**As built** (see `exercises/65_performance/`; adversarially reviewed). Deviations and verified corrections:

- `perf1`: Reviewer change: strict_clippy is now TRUE (the builder had false). This follows the ROADMAP note that warn-by-default lints go through strict_clippy, and it catches `write!(out, "{}", format!(..))` via `format_in_format_args`. The in-file `#![deny(clippy::format_push_string)]` stays, because that lint is allow-by-default (pedantic) and `-D warnings` does not enable it. Unsolved, perf1 FAILS its tests (2 of 4) as well as clippy. The clippy stage was checked on an intermediate state: pointer bug fixed with `out.clear(); out.push_str(&render(rows))`, all 4 tests pass, and clippy fails with 3 x format_push_string. Dedupe with 67_code_review/review2 (builder's deviation, kept): the starter has no index loop, so needless_range_loop is not graded here. ROADMAP correction: on clippy 0.1.96, `format_push_string` is `pedantic`, not `restriction` (verified with -W clippy::pedantic vs -W clippy::restriction). Graded by clippy: registered with `strict_clippy = true`.
- `perf2`: Dedupe with 67_code_review/review2 (builder's deviation, kept): Clippy does not grade `ptr_arg` here. The tests are callers that a `&String` / `&Vec<String>` signature shuts out (literals, sub-slices, arrays, a `Vec<&str>`), so the starter fails with 15 x E0308 instead of the ROADMAP's pointer mismatch plus ptr_arg. Once the signatures are fixed, heap-pointer checks (`as_ptr_range().contains` and exact offsets) grade the borrowed return. The ROADMAP's two functions are kept. The Cow part stays in `45_sized_deref/cow1`, as the ROADMAP note says. Reviewer changes: the count_long TODO now also requires 'allocate nothing per word'; the hint names the uncatchable `&[impl ToString]` wrong turn and says the IntoIterator alternative draws Clippy warnings in the tests. strict_clippy=false: no warn-by-default lint grades it.
- `perf3`: Refocused per the *(scope)* dedupe with review2 (builder, kept). `needless_range_loop` is not graded; clippy 0.1.96 does not even fire it on dot's two-slice index loop (re-verified). Kept `dot` (length check before `zip`, three should_panic tests) and `copy_prefix` (`[1, 2, 0, 0]`, a longer src, edge cases, then `manual_memcpy`). Added `append_chunks` (`reserve` once + `extend_from_slice`), graded by: a capacity bound of total + total/8 (push gives 2048, per-chunk extend or flat_map 1600, reserve 1025; re-verified); pointer and capacity unchanged for a buffer with room; capacity 0 when nothing is appended. strict_clippy=true because `manual_memcpy` is warn-by-default. Unsolved, perf3 FAILS its tests (6 of 11). The clippy stage was checked on the intermediate state: with a min-length index copy loop, all 11 tests pass, strict clippy fails with manual_memcpy only, and plain clippy passes. Reviewer changes: header 'exactly as with push' is now 'much as it does with push'; the hint now names per-chunk reserve_exact (quadratic, uncatchable) and reserve-then-push, and the zip-copy sentence was reworded. Graded by clippy: registered with `strict_clippy = true`.
- Correction: on clippy 0.1.96 `format_push_string` is in the `pedantic` group, not `restriction`. It is still allow-by-default, so `perf1` keeps its in-file `#![deny(clippy::format_push_string)]`.
- No test can see a temporary allocation (a bound `format!`, a `concat()`, a per-chunk `reserve_exact`) without a counting allocator, which needs `unsafe`. The TODOs and hints name each such variant, and the planned `alloc-perf-lab` is where zero allocations get proven.

#### Allocation-aware code: buffer reuse, `&str` and `&[T]` params, borrowed returns, iterator vs index loops

- Slug `perf-allocation-aware` · placement: graded · module: `65_performance` (`perf1..3`) · interview value: **3/5**
- Sharpest question: This function allocates a new String on every call inside a hot loop. How do you rewrite it to reuse the caller's buffer, and why take `&str` instead of `&String`?
- Exercises:
  - `perf1` — `render_into(rows, out)` does `*out = render(rows)` using `+= &format!(..)`; `clear()` the caller's buffer, `writeln!` into it, and iterate directly. Fails unsolved: `buf.as_ptr()` changes (a new `String` is allocated while the old one is alive), plus clippy errors (`format_push_string`, `needless_range_loop`).
  - `perf2` — `tokens(line) -> Vec<String>` and `count_long(&Vec<String>)`; return `Vec<&str>` and take `&[impl AsRef<str>]`. Fails unsolved: `t[1].as_ptr() != line[4..].as_ptr()`, plus clippy `ptr_arg` ("writing `&Vec` instead of `&[_]`").
  - `perf3` — `dot` via an index loop and a manual copy loop; assert the lengths first, then use `zip` / `sum` and `copy_from_slice`. Fails unsolved: clippy errors (`needless_range_loop`, `manual_memcpy`), and the `#[should_panic(expected = "length mismatch")]` test fails because the index loop over the shorter slice returns silently.
- Authoring notes:
  - `ptr_arg`, `needless_range_loop` and `manual_memcpy` are warn-by-default. Use `strict_clippy = true` in `info.toml` (already supported, used by `22_clippy`), which is sturdier than a file-level `#![deny]` the learner can delete. Only `format_push_string` (restriction group) needs an explicit `#![deny(clippy::format_push_string)]`.
  - The pointer and capacity assertions are deterministic: a new `String` allocated while the old one is still alive must have a different address.
  - Tests: after a warm-up, 100 `render_into` calls keep `(as_ptr, capacity)` and the content; tokens and pointers match; `count_long` accepts a `Vec` and an array; `dot == 11.0`; a mismatch panics with the message; `copy_prefix` gives `[1, 2, 0, 0]`.
  - The `Cow` part of the original proposal moved to `cow1`; perf2 keeps only the borrowed-return and parameter-type parts.
  - *(scope)* perf3 overlaps `review2` (the same clippy lints); dedupe.

### `68_mock_interviews` (built)

Theme: statement-first timed sets with part-2 follow-ups.

**As built** (see `exercises/68_mock_interviews/`; adversarially reviewed). Deviations and verified corrections:

- `set_trie`: Statement-first, 25-minute time box, as in the ROADMAP. The follow-up tests cover a prefix that leads nowhere, a word that is its own prefix, a limit of 0 and usize::MAX, the empty prefix and the empty word, and non-ASCII words. Deviation from the ROADMAP: it says "über" is passed only by a char-keyed trie. The 26-array does fail (it panics), but a 256-way byte trie is also correct and passes, so the test pins "not lowercase-ASCII-only". Review fixes: the sort-order comment now says bytes rather than code points, and the hint says drop glue rather than derived Drop and adds the BTreeSet/Vec caveat.
- `set_edit_distance`: Statement-first, 20-minute time box. The ROADMAP tests are all there (kitten/sitting 3, empty vs abc 3, café/cafe 1). Added on top: ab/ba = 2 (pins Levenshtein), composed vs decomposed café = 2, 4-byte chars, 3_000-char and 100_000-char inputs, and 2_000 random pairs checked against a full-table model. O(min(n, m)) memory stays rubric-only, because no safe test can observe it. Review fixes: the header no longer gives the approach away, the timing comment is corrected for the debug profile, and the comment and hint now separate normalization from graphemes.
- `set_kv_tx`: Statement-first, 35-minute time box. COMMIT merges only the innermost transaction into its parent. Added beyond the ROADMAP: the pointer-identity test, 1_000-level nesting, a model test with 15_000 random operations, and (added in review) `begin_commit_and_rollback_never_copy_the_store`, a time budget over 30_000 keys (see the last bullet). The budget test exists because a copy per begin that shares values through `Rc<str>`, or copies on write, passed every original test. The reference runs at about 1.4x a plain `HashMap` doing the same writes; copying stores run 850-1,900x as long and fail within about 2 s. The statement now pins the allocation instead of the `String` type.
- `set_kv_tx_part2`: Applies the ROADMAP no-spoiler note: the starter repeats only part 1's signatures, with todo!() bodies and an empty struct, and the solution holds the full reference. Deviation: rustc 1.96 words the E0599 as "`Kv` is not an iterator", not "no method named `count`" (re-verified). Six part-1 tests sit above the part-2 banner. Review fix: the budget test now makes a transaction rewrite all 40_000 keys before 40_000 `count` calls, then runs 40_000 rollback cycles. Before, a count that walked the open transactions' written keys passed, and the fastest full scan beat the budget by only about 2x. Now the reference runs at about 2.5x the plain-map baseline, while every scan or recount variant runs 12,000-37,000x as long and fails within about 4 s.
- Correction: `set_kv_tx_part2` fails unsolved with E0599 worded "`Kv` is not an iterator" (rustc resolves `count` toward `Iterator::count`), not "no method named `count`".
- Correction: "`über`, which only a char-keyed trie passes" overstates it. A 256-way byte-keyed trie is also correct and passes, because every `&str` prefix ends on a char boundary; the test pins "not lowercase-ASCII-only".
- The two budget tests are the course's only timed tests, and they are relative, not absolute: `dev check` runs every solution at once, so a fixed wall-clock limit could fail a correct solution on a busy CI runner. A `Budget` helper alternates chunks of 20 steps on the `Kv` with the same steps on a plain `HashMap<String, String>` and fails only when the `Kv` is more than 64x slower both in total (plus 500 ms of slack) and in each of the last 5 chunks. Load pauses make single chunks look slow, never five in a row. The reference passed every run (20 sequential, 32 concurrent copies, and 32 copies next to 400 busy loops; worst compared ratio 4.0). All seven wrong designs (Rc-sharing snapshots, copy-on-write maps, key-scanning `count`, a recounting `rollback`) fail every run.

#### Timed interview sets: statement-first problems with rubric hints and part-2 follow-ups

- Slug `interview-set-format` · placement: graded · module: `68_mock_interviews` (`set_trie`, `set_edit_distance`, `set_kv_tx`, `set_kv_tx_part2`) · interview value: **3/5**
- Sharpest question: Implement an in-memory key-value store with nested BEGIN/COMMIT/ROLLBACK, then add COUNT(value) that stays correct across transactions.
- Exercises:
  - `set_trie` — a 25-minute statement with `todo!()` signatures for `insert` / `contains` / `starts_with` / `suggest(prefix, limit)`; the learner picks `BTreeMap<char, Node>` or `[Option<Box<Node>>; 26]`. Fails unsolved: the `todo!()` panics (and clippy's `todo` lint).
  - `set_edit_distance` — Levenshtein over chars with two rolling rows; the hint is a rubric (O(min(n, m)) memory, stating the complexity). Fails unsolved: `todo!()` panic.
  - `set_kv_tx` — a KV store with nested begin / commit / rollback (suggested design: layered `Vec<HashMap<String, Option<String>>>` with tombstones; `get` returns `&str`). Fails unsolved: `todo!()` panic.
  - `set_kv_tx_part2` — add `count(&self, value)` that stays correct across transactions. Fails unsolved: E0599 no method named `count`.
- Authoring notes:
  - `dev check` rejects extra files and `cargo test` skips `#[ignore]` tests, so follow-up tests must be ordinary tests placed below a banner comment.
  - `set_kv_tx_part2` "starts from part 1": if its starter ships part 1's solution, part 1 is spoiled for anyone who reads ahead. Consider `todo!()` bodies with only the part-1 signatures.
  - Tests: set_trie examples plus follow-ups (absent prefix, exact word, limit, empty prefix, and `"über"`, which only a char-keyed trie passes); set_edit_distance `kitten`/`sitting` 3, empty vs `abc` 3, `café`/`cafe` 1 (a byte-based version gives 2); set_kv_tx rollback restores, nested commit merges into the parent, delete plus rollback, `NoTransaction` errors; part 2 counts through set, overwrite, delete, rollback, nesting and tombstone shadowing.
  - *(scope)* The value is the format (unseen, timed, statement-first) more than the specific problems; `set_kv_tx` plus its part 2 is a very common backend interview problem.

## Deep-dive labs

Labs are complete reference implementations to read, tinker with and run, not fail-while-unsolved exercises. Some ship TODO-driven "finish this" parts or `#[ignore]`d demonstrations. Build them alongside the tiers they support.

### In the existing `deep-dive/` crate

Theme: `unsafe`, Miri, FFI, allocators, loom, and testing a library from outside.

#### Lab: what exactly is UB, run under Miri, plus a Miri CI step

- Slug `miri-ub-zoo` · placement: deep-dive-lab · module: `deep-dive/src/ub_zoo.rs` plus a Miri step in `.github/workflows/rust.yml` · interview value: **4/5**
- Sharpest question: Name three kinds of UB that safe Rust cannot trigger but unsafe can. Is a data race UB, and is a race condition?
- Exercises:
  - `ub_zoo` — `#[ignore]`d `ub_*` tests, each with a `fixed_*` twin (starting as `unimplemented!()`): Stacked Borrows aliasing (two `&mut` from one raw pointer, a hand-rolled `split_at_mut`), a write through a `&T` cast (fix: `Cell`), transmuting `2u8` to `bool`, `assume_init` on uninitialized memory, use-after-free, an unaligned read (fix: `read_unaligned`), a non-atomic data race, library vs language UB (`from_utf8_unchecked`, `get_unchecked` with debug precondition checks), and leaks (reported, but not UB). Fails: `cargo +nightly miri test -- --ignored --exact <test>` reports each diagnostic, while plain `cargo test` mostly passes (that is the lesson); the `fixed_*` twins fail until written.
  - `unsound_covariant_cell` — `BadCell<T> { ptr: NonNull<T>, _m: PhantomData<T> }` with `set(&self, T)` lets safe code store a short-lived `&str` into a `BadCell<&'static str>`; make it invariant (`PhantomData<Cell<T>>` or `UnsafeCell<T>`) and move the exploit into a `compile_fail` doctest. Fails: before the fix Miri reports a use-after-free; after it the exploit is E0597.
  - `ci_miri_job` — a nightly Miri step for the whole deep-dive crate, with a grep guard proving tests ran (like the loom step). Fails: a lab change that introduces UB turns CI red.
- Authoring notes:
  - `invalid_reference_casting` is deny-by-default, so the write-through-`&T` case needs a scoped `#![allow(..)]` with a justification; that keeps `clippy -D warnings` clean.
  - Correct the rationale: `deep-dive/README.md` only tells learners to run Miri; it doesn't claim the labs are Miri-clean. The core point stands: CI never runs Miri.
  - Stable rustdoc ignores `compile_fail` error codes (verified, see `api-surface-lab`), so the doctest cannot pin E0597 on stable.
  - In the use-after-free case, note that `let _ = *p` reads nothing.
  - Fold-in from the cut `uninit-lab`: one case showing that uninitialized memory is UB even for integers, and that `Vec::with_capacity(n)` plus `set_len(n)` plus assigning elements of a `Vec<String>` drops garbage (fix: `spare_capacity_mut` + write + `set_len`). Verified: that starter is rejected by clippy's deny-by-default `uninit_vec`, so it needs a justified `#[allow(clippy::uninit_vec)]` or the deep-dive CI step `clippy --all-targets -D warnings` goes red.
  - Already absorbs the dropped `miri_hunt` cases (`split_at_mut` aliasing, `assume_init`, transmute to `bool`) and the `variance_soundness` demo.

#### Lab: FFI with edition-2024 unsafe extern and safe fn, repr(C), CString ownership, closure trampolines

- Slug `ffi-lab` · placement: deep-dive-lab · module: `deep-dive/src/ffi_lab.rs` (libc symbols only, no crates) · interview value: **3/5**
- Sharpest question: How do you pass a Rust closure to a C API that takes `void (*cb)(void*, int)` plus a `void* user` pointer, and what happens if the closure panics?
- Exercises:
  - `call_libc` — `unsafe extern "C" { fn strlen(..); pub safe fn abs(..); safe fn div(..) -> DivT; fn qsort(..); }` with a `repr(C)` `DivT` and a Rust comparator for `qsort`. Fails: a plain `extern` block gives "extern blocks must be unsafe"; the stubs are `unimplemented!()`.
  - `cstring_ownership` — a `#[unsafe(no_mangle)]` export, an `into_raw` / `from_raw` round trip, and fixing `CString::new(s).unwrap().as_ptr()`. Fails: stubs; the dangling variant fails under Miri.
  - `callback_trampoline` — a generic `extern "C" fn trampoline<F: FnMut(i32)>(user: *mut c_void, x: i32)` behind a safe `for_each(&[i32], impl FnMut(i32))`, with `catch_unwind` at the boundary and an `extern "C-unwind"` example. Fails: the stub never calls the closure; a panic through `extern "C"` aborts (tested via a child process).
  - `send_c_handle` — `Handle(NonNull<Ctx>)` is `!Send`; add `unsafe impl Send` with a SAFETY argument, but not `Sync`, and share it through `Mutex<Handle>`. Fails: `assert_send::<Handle>()` fails to compile until added.
- Authoring notes:
  - Verified on 1.96 / edition 2024: a plain `extern` block gives "extern blocks must be unsafe"; `safe fn` items inside `unsafe extern` compile; `#[unsafe(no_mangle)]` is accepted; `c"..."` literals work; `div_t` via `repr(C)` is fine on the ubuntu CI job.
  - `CString::new(..).unwrap().as_ptr()` fires the warn-by-default `dangling_pointers_from_temporaries` lint. The deep-dive CI runs clippy with `-D warnings`, so the deliberately dangling variant needs a scoped, justified `#[allow]`.
  - Tests: `strlen(c"hello") == 5`; `abs` callable without `unsafe`; `div(7, 2) == (3, 1)`; `qsort` sorts; an interior NUL gives `Err` with `nul_position() == 1`; `to_string_lossy` shows the replacement char; the closure collects values; a panicking closure returns `Err`; the child process exits abnormally; `assert_sync::<Mutex<Handle>>()` compiles.
  - README: why `unsafe impl Sync` for the handle would be unsound. Lab convention: every unsafe op sits in its own `unsafe {}` block with a `// SAFETY:` comment.

#### Lab: a counting GlobalAlloc to predict allocations, black_box micro-benchmarks, bounds-check elimination

- Slug `alloc-perf-lab` · placement: deep-dive-lab · module: `deep-dive/tests/alloc_count.rs` (`harness = false`) plus `deep-dive/src/perf_lab.rs` · interview value: **3/5**
- Sharpest question: How would you prove that a hot path makes zero allocations, and what makes a Rust micro-benchmark trustworthy (black_box, warm-up, confirming bounds-check elimination in asm)?
- Exercises:
  - `alloc_count` — an `unsafe impl GlobalAlloc` forwarding to `System` with counters; the learner fills predicted counts: 0 for `String::new` / `Vec::new` / `HashMap::new` / `Box::new(())` / `Vec<()>`, `Rc<str>::from` (1) vs `Rc::new(String)` (2), an exact-size `collect` (1 alloc, 0 reallocs), a naive render vs a reused buffer (0). Fails: predictions start at 999 and `main` exits non-zero.
  - `bench_and_bce` — `sum_indexed` vs `zip` vs a hoisted-assert version, timed with `Instant` plus `black_box` (optionally criterion with `harness = false`); inspect the asm for `panic_bounds_check`. Not pass/fail: tests assert the variants agree.
- Authoring notes:
  - Needs a `[[test]]` entry with `harness = false` for the integration binary; this keeps the `#[global_allocator]` out of the other labs.
  - Thread-local counters inside a `#[global_allocator]` are fragile: lazy `thread_local` initialization can allocate or re-enter the allocator, and access during TLS teardown can panic. Use const-initialized `Cell` thread-locals with `try_with`, or atomics plus a per-test gate.
  - The listed counts are correct (verified): 0 for empty collections and ZST boxes, `Rc<str>::from` 1 vs `Rc::new(String)` 2. Check exact counts only for structural cases, and check growth only with bounds (the growth strategy is not guaranteed).
  - README: bounds-check elimination must be confirmed in the asm, not assumed.
  - criterion is an external crate: per the lab conventions it must be `cfg`-gated like loom, or live in a sibling crate excluded from the root workspace with its own CI job.
  - *(scope)* Keep trivia predictions (`Vec::new`, `Box::new(())`) a small minority of the lab.

#### Lab: SeqCst vs Acquire/Release, Peterson's lock, false sharing, Treiber stack reclamation

- Slug `lock-free-ordering-lab` · placement: deep-dive-lab · module: `deep-dive/src/ordering_lab.rs` plus `treiber.rs` (with `cfg(loom)` tests) · interview value: **3/5**
- Sharpest question: Give the store-buffering example where both threads read 0 under Acquire/Release. Why does SeqCst forbid it, and why is memory reclamation the hard part of a lock-free stack?
- Exercises:
  - `sb_litmus` — `store_buffering(store, load, iters)` counts `r1 == r2 == 0` outcomes with padded atomics, plus a loom model fixed with `fence(SeqCst)`. Not graded: the Release/Acquire loom model is `#[should_panic]`, the hardware stress run is `#[ignore]`, and the SeqCst count being 0 is asserted (guaranteed).
  - `peterson_and_padding` — Peterson's lock with Release/Acquire (broken) vs SeqCst, plus an `#[ignore]`d false-sharing benchmark comparing adjacent counters with `CachePadded`. Not graded: lost updates are printed for the broken variant.
  - `treiber` — `TreiberStack<T>` on `AtomicPtr` with a split begin/commit API, deferring frees until `Drop` vs eager free (the use-after-free test is `#[ignore]`d and run under Miri), an optional crossbeam-epoch variant behind a feature, and a loom model of `atomics3`'s spinlock. Fails: Miri reports UB for the eager free.
- Authoring notes:
  - Loom treats SeqCst accesses as AcqRel, so only the `fence(SeqCst)` variants can be verified under loom. A Peterson lock that uses only SeqCst accesses would be falsely flagged by loom; check that variant by stress testing.
  - Add the new loom tests to the CI grep guard, which currently only matches `loom_lab::tests::`.
  - Correct placement: it needs unsafe `AtomicPtr` reclamation and loom.
  - Tests: LIFO order, a stress run, and a drop counter proving each element is dropped exactly once.
  - Receives: the store-buffering and IRIW litmus questions from `send-sync-ordering-quiz` (dropped from `quizzes/quiz5`; the IRIW question must name the load ordering and say whether it asks about hardware or language guarantees); the Treiber-stack ABA follow-up from `atomics-cas-aba`; and false sharing from the cut `layout-niche-padding`. Verified layout facts for the latter: `#[repr(align(128))] struct CachePadded<T>(T)` has align 128, `[CachePadded<AtomicU64>; 2]` is 256 bytes, and `(AtomicU64, AtomicU64)` is 16 bytes (explain why 128 on x86_64 / aarch64).
  - *(scope)* Cross-link `atomics-cas-aba`, `myarc` and `loom_lab` rather than repeating them.

#### Lab: testing a library from outside: doctests, compile_fail guarantees, integration tests, proptest, semver

- Slug `api-surface-lab` · placement: deep-dive-lab · module: `deep-dive/src/api_surface.rs` plus `deep-dive/tests/api_surface.rs` (and a proptest dev-dependency) · interview value: **3/5**
- Sharpest question: How do you test that misuse of your API fails to compile, and which of these changes are semver-major: adding a variant to a non-`#[non_exhaustive]` enum, adding a method to a sealed vs an unsealed trait?
- Exercises:
  - `compile_fail_guarantees` — a sealed `Format` trait, `#[non_exhaustive]` `Status` / `Opts`, a typestate `RequestBuilder` and a `#[must_use]` builder, each with a positive doctest and a `compile_fail` doctest; tinker by removing a guarantee. Fails: removing a guarantee turns its `compile_fail` doctest red.
  - `integration_and_proptest` — `tests/api_surface.rs` (helpers in `tests/common/mod.rs`), a proptest encode/decode round trip and an `insert_sorted` invariant with a planted escaping bug so you can watch shrinking. Fails: enabling the bug yields a shrunk counterexample.
  - `semver_drill` — a README table classifying changes (a variant on an exhaustive vs `#[non_exhaustive]` enum, a pub field, a trait method on a sealed vs unsealed trait, loosening vs tightening a bound), linked to doctests; mention cargo-semver-checks. Not auto-graded.
- Authoring notes:
  - Stable rustdoc does not check `compile_fail` error codes: a `compile_fail,E0999` doctest passes on stable 1.96 (verified in a scratch crate). Pair every `compile_fail` doctest with a positive control, and enforce codes with a nightly `--doc` CI step or trybuild snapshots.
  - `#[non_exhaustive]` has no effect inside the defining crate, so exercise it from `tests/`.
  - deep-dive currently has no `tests/` directory, no dev-dependencies and only `text` code blocks, which confirms the gap.
  - proptest is an ungated external crate: per the lab conventions it must be `cfg`-gated like loom, or live in a sibling crate in the root `[workspace] exclude` list with its own CI job. Decide before adding the dev-dependency.
  - Receives the negative (must-not-compile) checks from `builder-typestate` and `variance-phantomdata`, and the compile-fail Send/Sync cases suggested for `send-sync-ordering-quiz`.

### New crates with external dependencies

Theme: the ecosystem answers (tokio, axum, tower, serde, Cargo features) that single-file std-only exercises cannot express.

#### Lab (tokio, axum, tower): select cancel safety, graceful shutdown, 503 backpressure, spawn_blocking

- Slug `backend-tokio-lab` · placement: deep-dive-lab · module: new sibling crate `backend-lab/` (tokio, tokio-util, axum, tower), excluded from the root workspace · interview value: **5/5**
- Sharpest question: Implement graceful shutdown for a tokio server: stop accepting, drain in-flight requests with a timeout, then abort. Which tokio APIs you use in select! are cancel-safe?
- Exercises:
  - `select_cancel` — `select!` over `read_exact` (not cancel-safe) and a ticker; fix by pinning the read outside the loop or by keeping framing state. Fails: a corrupted frame under `start_paused` time over `tokio::io::duplex`.
  - `graceful_shutdown` — `JoinSet` plus `CancellationToken`: drain in-flight jobs within a timeout, then abort the rest. Fails: lost jobs, or work accepted after shutdown.
  - `backpressure_http` — an axum handler `try_send`s into `mpsc::channel(n)` and maps `Full` to 503, with `State<Arc<AppState>>` and an `IntoResponse` error, tested with `ServiceExt::oneshot`. Fails: an unbounded channel never returns 503.
  - `blocking_in_async` — a CPU loop on a `current_thread` runtime starves a heartbeat task; move it into `spawn_blocking`. Fails: a bounded 2 s wait sees no heartbeat.
- Authoring notes:
  - Add the crate to the root `Cargo.toml` `[workspace] exclude` list (as `deep-dive` is) and give it its own CI job.
  - `start_paused` requires tokio's `test-util` feature.
  - tokio's docs list `read_exact` as not cancel-safe, as the lab claims.
  - Tests: frames intact across 100 timeouts; jobs started before the token complete, later ones are rejected, a long job is aborted; with `n + 1` requests a 503 appears and queue depth stays at most `n`; the heartbeat counter is above 0 once the work moves to `spawn_blocking`.
  - *(scope)* The relevance review scores this 5/5 and recommends raising it to P0 among the labs, and adding bounded concurrency ("at most K in flight" with `Semaphore` / `JoinSet` / `buffer_unordered`; see [Additional topics](#additional-topics-not-yet-verified)).

#### Lab: serde at the boundary and additive Cargo features

- Slug `serde-features-lab` · placement: deep-dive-lab · module: new crate `deep-dive/config_lab` · interview value: **3/5**
- Sharpest question: How do you guarantee that a deserialized `Email` field is always valid, and why must Cargo features be additive?
- Exercises:
  - `serde_boundary` — an `Email` newtype with `#[serde(try_from = "String")]`, `deny_unknown_fields`, `rename_all` and defaults, plus a proptest round trip. Fails (tinker): removing `deny_unknown_fields` fails the typo test.
  - `features_and_cfg` — optional serde via `dep:serde` and `cfg_attr`, a README on additive features, and a `build.rs` emitting `cargo::rustc-check-cfg` and `rustc-cfg` with `rerun-if-env-changed`. Fails: the CI `--no-default-features` build breaks without the `cfg_attr` gating.
- Authoring notes:
  - The `cargo::rustc-check-cfg` syntax needs Cargo 1.77+, which is fine on 1.96.
  - Correct placement: external dependencies and feature flags can't be declared by single-file graded exercises.
  - Crate layout: per the lab conventions, a crate with ungated external dependencies goes in the root `[workspace] exclude` list with its own CI job. If it instead becomes a member of a deep-dive workspace, the deep-dive CI job must pass `--workspace` to fmt, clippy and test (it currently targets only the root package via `--manifest-path`), as the verifier noted for the proc-macro lab.
  - Tests: an invalid email gives the expected message, an unknown key is rejected, defaults apply, round trips hold; both feature sets build and pass with no `unexpected_cfgs`.

## Additional topics (not yet verified)

> These came from the interview-relevance reviewer as gaps the six lenses missed.
> They have **not** been through the coverage + feasibility pass: no fail modes
> were designed and nothing below was checked by the verifier that compile-checked
> the candidates above. The reviewer's own spot checks are marked as such.

- **Self-referential structs in safe Rust** — refactoring "owner plus borrowed view" (`struct Doc { text: String, words: Vec<&str> }`) into ranges or indices, into separate owner and view types, or into `Rc<str>`.
  - Placement: graded, extend `25_lifetimes_deep` (`lifetimes10`: store `Vec<Range<usize>>`, or split into `Doc` + `DocView<'a>`); the README points to the Pin lab and to ouroboros / yoke.
  - Why: "Why can't a struct hold a String and references into it?" is one of the most common Rust questions on Stack Overflow and in interviews. The repo only answers it with `Pin` / `PhantomPinned` (`runtime4` and the unsafe `self_referential` lab), never with the safe restructurings interviewers expect. The reviewer checked that on 1.96 the naive build gives E0515 plus E0505.
- **Borrowing and mutable iterators over your own collection** — `impl IntoIterator for &'a Coll` / `&'a mut Coll`, and `IterMut` via `mem::take(&mut self.slice).split_first_mut()` or `Option<&mut Node>::take()`.
  - Placement: graded, extend `34_iterators` (`iter5`: `IntoIterator` for `&Grid` / `&mut Grid`; `iter6`: `IterMut` for a slice wrapper and for `linkedlist1`'s list).
  - Why: "Implement `iter_mut` for your container or linked list" is a classic senior question (the Too Many Linked Lists IterMut chapter), and it is the key contrast with GAT lending iterators: `IterMut` needs no GATs, `windows_mut` does. `iter4` implements only the owning `IntoIterator`, and `bst2` only an immutable `Iter`.
- **`std::io`: `Read` / `Write` / `BufRead`** — generic `R: BufRead` / `W: Write` functions tested with `Cursor`, `&[u8]` and `Vec<u8>`; reusing a buffer with `read_line` vs allocating per line with `lines()`; `io::ErrorKind`; `BufWriter` swallowing flush errors on drop.
  - Placement: graded, a new module `io1..3` (std only; in-memory readers and writers, no filesystem).
  - Why: live-coding and take-home tasks (log parsing, CLI filters, line protocols) constantly need I/O, and "make this testable without touching the filesystem" is a standard follow-up. The repo has zero hits for `BufRead`, `io::Read`, `io::Write` or `Cursor`.
- **Pattern matching and binding modes** — match ergonomics (why `match &opt { Some(x) => .. }` gives `x: &T`), `ref` / `ref mut`, `|&&x|` in iterator closures, `@` bindings, slice patterns `[first, .., last]`, let-else, and the edition-2024 restriction on explicit `&` / `ref` / `mut` under a non-move default binding mode.
  - Placement: graded, extend `08_enums` or a new `patterns1..2` (each failing with E0308 / E0507 until the binding mode is fixed), plus a few items in a will-it-compile quiz.
  - Why: the `filter(|x| **x > 0)` vs `|&&x|` confusion and "what does `ref` do?" come up in almost every junior and mid-level live-coding session, and slice patterns are expected idiomatic answers. There are zero hits for `ref mut`, and no exercise explains default binding modes.
- **Async recursion and pin projection** — E0733 "recursion in an async fn requires boxing" fixed with `Box::pin`, and writing a `Timeout<F>` / `Join` combinator over `!Unpin` child futures (structural pinning, pin-project-lite).
  - Placement: graded, extend `29_async_runtime` (`runtime5`: async recursion via `Box::pin`; `runtime6`: a safe `Timeout<F>` storing `Pin<Box<F>>`). Deep-dive lab: structural pinning with unsafe `map_unchecked_mut` vs pin-project-lite.
  - Why: "What is pin-project for?" and "how do you implement a future that wraps another future?" are standard senior async questions. `join1` and `select1` deliberately restrict children to `Unpin`, so the course never faces projection. The reviewer checked that E0733 is a real error on 1.96.
- **Bounded concurrency and streams in async** — "process 10k items with at most K in flight" using `Semaphore`, `JoinSet`, or `Stream` + `buffer_unordered`; the `Stream` (`poll_next`) trait; ordered vs unordered results; short-circuiting on the first error.
  - Placement: deep-dive lab, a `bounded_fanout` part in `backend-tokio-lab` (tokio `Semaphore` / `JoinSet` / `futures::StreamExt::buffer_unordered`). Optional graded std version: a `Stream` trait plus a `buffer_unordered`-style combinator on the course's executor.
  - Why: "fetch N URLs with at most 50 concurrent requests" is one of the most common tokio interview prompts, and neither the std-only async modules nor `backend-tokio-lab` (select, shutdown, backpressure, `spawn_blocking`) cover limiting concurrency or streams.
- **Designing a concurrent cache or map** — lock striping (`Vec<RwLock<HashMap<..>>>` chosen by hash), read-mostly hot-swapped config (`RwLock<Arc<Config>>` clone-out, the arc-swap idea), and `Arc::make_mut` copy-on-write.
  - Placement: graded, extend `53_lock_hazards` (`shard1`: a `ShardedMap` that passes a contention test; `hotswap1`: readers never block during a config swap).
  - Why: "design a thread-safe cache that many threads read" is a frequent Rust backend and infra design question. The candidates cover a single-threaded LRU, a single-`RwLock` `ConfigStore` and lock ordering, but never sharding or contention trade-offs.
- **`no_std` / `core` / `alloc`** — what disappears without std (`HashMap`, threads, `println!`, panicking behavior), `#![no_std]` library design, a panic handler, and building for wasm32.
  - Placement: deep-dive lab, a new sibling crate (a `#![no_std]` + `alloc` library tested with the std test harness; CI builds it for `wasm32-unknown-unknown` and a thumbv7 target).
  - Why: blockchain runtimes (Solana programs, CosmWasm contracts, Substrate runtimes) and embedded roles run in `no_std` or wasm environments, and "can this crate be `no_std`, and what would you change?" is a common question for those roles. The repo only mentions `no_std` inside third-party build artifacts.
- **Profiling and observability of Rust services** — flamegraphs (perf, samply, cargo-flamegraph), heap profiling (dhat, heaptrack), tokio-console for stuck or starving tasks, and tracing spans with `#[instrument]` across `.await`.
  - Placement: a qa-drill README of diagnosis scenarios, plus a deep-dive lab extension of `backend-tokio-lab` with a tracing-instrumented handler and a deliberately blocking task to find with tokio-console.
  - Why: infra and backend interviews routinely ask "p99 latency spiked or memory keeps growing in your Rust service; how do you find out why?". `alloc-perf-lab` covers micro-benchmarks, but nothing covers profiling or tracing a running async system.
- **Hashing and collection choice** — the default SipHash and HashDoS resistance vs FxHash / ahash, plugging in a `BuildHasher`, `BTreeMap` vs `HashMap` (ordering and determinism), and `VecDeque` vs `Vec`.
  - Placement: graded, extend `27_data_structures` (`hasher1`: implement an FNV `BuildHasher`, use `HashMap::with_hasher`, assert deterministic iteration via `BTreeMap`), plus two or three items in the layout / alloc quiz.
  - Why: "why is Rust's default HashMap slower than you expect, and when would you swap the hasher?" is common in performance and blockchain (determinism) interviews. `hashtable1` uses `DefaultHasher` without discussing it, and custom hashers appear only as test scaffolding in the candidates (`borrowck3`, `checkedmath4`).

## Cut, and where the useful bits went

All ten were refuted by the interview-relevance review. Their fail modes had been verified, so the fold-ins above carry verified facts.

| Cut item | Proposed as | Why cut (condensed) | Useful bits go to |
| --- | --- | --- | --- |
| `layout-niche-padding` | `41_memory_layout` `layout3..6` | `NonZero` ids and field reordering retest the quiz facts as one-line fixes; false sharing only makes sense where it can be measured. | One `repr(C)` reorder item in `layout2`; `CachePadded` false sharing in `lock-free-ordering-lab`; `Rc<str>` + `Borrow<str>` interner in `borrow1`. |
| `operator-overloading` | `44_trait_contracts` `ops1..2` | Operator mechanics are a five-minute docs lookup and rarely asked. | `self` by value / `&T` impls, `Rhs` vs `Output` and `T: Add<Output = T>` generic code in `assoc2`; `Index` / `IndexMut` as one test in `grid-algorithms`. |
| `impl-trait-capture-rpitit` | `46_impl_trait` `impl_trait1..3` | The edition-2024 `use<..>` capture rules are a real footgun but rarely an interview question; argument vs return position `impl Trait` is already in `closure4` / `iter1`. | The one `+ use<>` lesson in `grid2`; RPITIT + `Send` in `async_send3`. |
| `derive-bounds-const-generics` | `47_type_level` `typed_id1`, `constgen1` | Both topics are asked infrequently. | Hand-written `Clone` / `Copy` / `Eq` / `Hash` for all `T` as extra tests on `variance1`'s `Id<T>`; const generics were expected to appear in `uninit-lab` (also cut) and `std::array::from_fn` usage, so they currently have no dedicated home. |
| `thread-pool` | `55_thread_pool` `pool1..3` | Rust Book chapter 21 walks through this exact design, and every component is already a candidate (condvar, channels, `panic1`, `debugging7`, `Option<JoinHandle>::take`). | "Drop the `Sender`, then join" as one test in `channel2`. |
| `leetcode-rc-refcell-trees` | `61_trees` `lc_tree1..3` | Only matters for solving LeetCode in Rust; interviewers use their own signatures, and `bst-option-box` covers traversal. | The `let n = &mut *node.borrow_mut();` reborrow trick as one test in `26_smart_pointers_deep` or `31_debugging`. |
| `heaps-topk-dijkstra` | `62_graphs` `heap1..3` | Largely redundant with `contracts3` (`BinaryHeap<Reverse<..>>`, min-heaps, f64 Dijkstra state); `heap2` / `heap3` are generic algorithm practice. | `heap1`'s `top_k<'a>` returning `Vec<&'a str>` (E0106 / E0597) in `slice-string-algorithms` or `contracts3`. |
| `algorithm-semantics-quiz` | `quiz8_algo_semantics` | Mostly trivia (`-7 % 3`, `max_by_key` tie-breaking, `into_sorted_vec` order). | Already taught elsewhere: char vs byte length in `window2`, `binary_search` on duplicates in `window4`, the will-it-compile set in `quiz6_borrowck`. |
| `uninit-lab` | `deep-dive/src/uninit_lab.rs` | Niche outside unsafe-heavy library roles; `raw_vec` already covers raw allocation. | One `set_len` / uninitialized-memory case in `ub_zoo`. |
| `proc-macro-lab` | new crate `deep-dive/derive_lab` | Near step-for-step copy of dtolnay's proc-macro-workshop builder, which already ships staged tests and trybuild UI tests; not worth turning deep-dive into a workspace. | A link plus a short "declarative vs procedural" Q&A in the `48_macros_deep` README. |

### Dropped or corrected during the merge

- **join-select-cancellation / `stream1`** (hand-written `Stream` + `Next` adapter) — P2 and ecosystem-specific: std `AsyncIterator` is unstable and interviews ask about `futures::Stream` by concept. Reduced to a README note in `57_async_combinators`.
- **arena-index-graphs / `arena2`** (`get_disjoint_mut` contract) — same lesson as `borrowck1`, which comes earlier; its error-variant mapping was folded into `borrowck1`.
- **mem-take-replace-swap / `ownership7`** (`mem::swap` double buffer) — `22_clippy/clippy3` already has the learner switch to `mem::swap`; the pointer-identity check was folded into `ownership4`.
- **raii-drop-guards / `raii1`** (struct field declaration order) — a one-line reorder duplicating the field-order scenario in `drop1`; kept only as a quiz question.
- **leetcode-rc-refcell-trees / `lc_tree4`** (a `Ref` temporary in an if-let scrutinee) — same bug as `debugging6` / `debugging7`, which absorbed it. Also corrected: on 1.96 the panic message is "RefCell already borrowed", not "already borrowed: BorrowMutError".
- **lazy-globals-rwlock / `once2`** (`OnceLock` replacing `Mutex<Option<..>>` with E0515) — merged into `cell3`, which already teaches `OnceLock::set` / `get`.
- **test-doubles-di / `tests5`** (`impl Mailer for &T` forwarding) — same forwarding-impl lesson as `sized2`, which picked up the Mailer scenario as an extra test.
- **ffi-miri-ub-lab / `miri_hunt`** (`split_at_mut` aliasing, `assume_init`, transmute to `bool`) — all three are already UB classes in `ub_zoo`.
- **send-sync-guard-ordering-quiz / MP litmus constants** — `atomics2` teaches message passing and `loom_lab` model-checks Relaxed vs Release/Acquire; only the SB and IRIW questions were kept.
- **send-sync-guard-ordering-quiz / padding size constants** — moved to `layout5` (itself now cut; false sharing goes to `lock-free-ordering-lab`).
- **algorithm-semantics-predict-quiz / `size_of::<Option<Box<u64>>>` question** — duplicates `layout-size-quiz`.
- **linked-list-classics / recursive `ListNode` drop depth** — already covered by `smartptr1` and `linkedlist1`, including the 200k-node drop test.
- **trait-objects-advanced / `dispatch6` and variance-hrtb / `lifetimes10`** (two default-object-lifetime exercises) — the same lesson proposed twice; one copy kept as `lifetimes8`.
- **systems-rounds-clock-locks** (as its own module) — split up: `limiter1` and `ttl1` went to `seams2` / `seams3`, `bank1` to `deadlock1`.
- **mock-interview-sets** (as its own module) — split up: `mock1` to `lru-cache`, `mock2` to `seams2`, `mock3` to `eq-hash-ord-contracts` and `heaps-topk-dijkstra`; the timeboxed-README format kept in `interview-set-format`.
- **deep-dive-type-contracts-lab** (as its own lab) — split up: `variance_soundness` to `miri-ub-zoo`, `negative_type_tests` to `api-surface-lab`.
- **counting-global-allocator** (as its own lab) — merged into `alloc-perf-lab`; unguaranteed `Vec` growth predictions are checked only with bounds.
- **perf-measurement-lab / `large_enum_variant` size demos** — duplicates `layout2`'s big-enum boxed vs unboxed question.
- **cost-model-layout-quiz, memory-layout-niche / `layout1`, type-system-quiz / `quiz6_sizes`** — three copies of one drill, merged into `layout-size-quiz`. Also corrected: a closure capturing two variables by reference is 2 words because closures store one reference per captured variable (or place), not because of edition-2021 disjoint capture.
- **trait-patterns-coherence / `heap1` and heaps-ordering-topk / `heap3`** (Job scheduler `Ord`) — same derived-Ord-field-order and Ord/Eq-consistency lesson as `contracts2`, merged into it.
- **deref-asref-borrow-cow and perf-allocation-aware-apis / `perf2` `normalize()` Cow return** — duplicates `cow1`; `perf2` keeps only the borrowed-return and `ptr_arg` parts.
- **lazy-globals-rwlock / `once1` wording "static mut refs are a hard error in 2024"** — corrected, not dropped: `static_mut_refs` is a deny-by-default lint in edition 2024 that can be allowed; `cell3` states this.
- **`typestate1` / `constgen1` / `coherence1` missing-type error code** — corrected, not dropped: rustc 1.96 reports "cannot find type" as E0425 (older compilers used E0412); `constgen1`'s first error is the missing type, not E0107.
- **`raii2` and panics gap evidence "the libtest harness ignores panic = abort"** — corrected, not dropped: it is Cargo that ignores the profile's `panic` setting for test targets.

## Authoring checklist for a new module

Condensed from the inventory of this repo's conventions. Paths are relative to the repo root.

### Graded module

1. **Files.** One flat file per exercise at `exercises/NN_topic/<name>.rs` plus a mandatory `exercises/NN_topic/README.md` (`rustlings-macros` `include_bytes!`-embeds it, so the build fails without it). Mirror each exercise at `solutions/NN_topic/<name>.rs`; CI runs `cargo dev check --require-solutions`. Nothing else may live in those directories.
2. **Names.** 1-32 chars of `[A-Za-z0-9_]`, unique across the whole course. Avoid upstream clashes by continuing a number (`lifetimes7`) or using a distinct stem (`iter`, `err`, `smartptr`).
3. **File shape.** Line 1 header (`// Module 1 · Borrow-checker errors — part N: <topic> (EXXXX).`), then `//` paragraphs explaining the *why*, then `use` lines and the given code. At each change site a `// TODO:` block (the literal `// TODO` is required by `dev check`) that ends with "Until you ..., this exercise will not compile." Then `fn main()` whose body is the single comment `// You can optionally experiment here.` (the literal `fn main()` is required). Then `#[cfg(test)] mod tests { use super::*; ... }` with descriptive snake_case test names and explanatory comments.
4. **Tests.** Part of the exercise; the learner never edits them. Cover edge cases (empty, full, wraparound, collisions, overflow boundaries, stale wakeups). Compare floats as `(a - b).abs() < 1e-9`. Concurrency tests are finite, join every thread and avoid sleeps; watchdogs may fire only on the unsolved path, since rustlings has no test timeout. Keep every test binary fast in the debug profile (well under 5 s). A test that must bound complexity compares against a baseline measured in the same test (the `Budget` helper in `68_mock_interviews/set_kv_tx`), never an absolute wall-clock limit, because `dev check` runs every solution at once.
5. **Fails while unsolved.** `dev check` runs build, then test, then `clippy --profile test`, then the binary; any failure counts as unsolved, and a fully passing unsolved exercise is an error (only `intro1` sets `skip_check_unsolved`). Accepted mechanisms: an empty body whose `()` mismatches the return type (E0308), the concept's own diagnostic, or a deterministic runtime test failure. Never rely on a race showing up (see `atomics4`).
6. **Recall, not recognition (module-37 rule, recommended for new interview modules).** The TODO names the diagnostic (E-code plus short rustc wording) and states the requirement and constraints (for example "no `.clone()`, no `unsafe`, don't change the tests") but does not spell out the fix; the `info.toml` hint carries the full fix. Domain types used to demonstrate borrow errors don't derive `Clone`, so "clone everything" is not an escape hatch.
7. **Crate constraints** (dev/Cargo.toml): edition 2024, std only, single file. Lints: `unsafe_code = "forbid"`, `unstable_features = "forbid"`, `dead_code = "allow"`; clippy `todo = "forbid"`, `empty_loop = "forbid"`, `infinite_loop = "deny"`, `mem_forget = "deny"`, `disallowed_methods = "allow"`. Profiles use `panic = "abort"`, but test targets still unwind. Tests run in the debug profile, so overflow checks are on.
8. **Solution file.** Identical header comments and tests; each TODO block replaced by a short comment explaining why the fix works. It must pass build, tests, `clippy --profile test -D warnings` (test code included), running `main`, and `rustfmt --check --edition 2024`.
9. **`info.toml` entry.** A section comment (e.g. `# MODULE 4 · ATOMICS — ...`), then `[[exercises]]` with `name`, `dir` and a non-empty `hint` (restate the error with its E-code and why it happens, give the fix, often as full code in fences, plus cross-links). Optional keys: `test = false`, `strict_clippy = true`, `skip_check_unsolved = true`. Insert the entries at the right pedagogical spot; the learner order is the order of `info.toml` (summarized in `deep-dive/COURSE.md`). Set `strict_clippy = true` whenever warn-by-default clippy lints do the grading: without it rustlings runs clippy without `-D warnings`.
10. **`dev/Cargo.toml` bin list.** Never edit it by hand: run `cargo dev update` (debug build). Validate with `cargo dev check --require-solutions` and `cargo run -- run <name>`; don't use `--release`, which refuses to run inside the source repo.
11. **Module README.** H1 `# Module N · Title: subtitle`; a `>` blockquote placing it in the course and stating `std + 100% safe` (and stable); `## Core Ideas`; optional concept sections; `## Exercise Path` as `1. **name** — what is broken and what to do`; `## Further Reading` (Book, std docs, Reference, external resources); cross-link the related lab. Must pass the rumdl CI lint (`.rumdl.toml` disables MD013 and MD057).
12. **Course docs.** Add rows to the right `### Module N · ...` table in `deep-dive/COURSE.md` (or a new section with a table and one sentence of rationale), including `deep-dive/` rows for companion labs. Update the hard-coded `24_ownership_model` through `68_mock_interviews` range in the root `README.md`; optionally add the module to `exercises/README.md`. Tick the module off in [Status](#status).

Standalone checks before integration (run in a scratch directory, never inside the repo):

```bash
# Unsolved exercise: the error list must be exactly the intended diagnostics.
rustc --edition 2024 --test -A dead_code -o /dev/null exercise.rs 2>&1 | grep -E '^error'

# Solution: tests, main, clippy (tests included) and formatting.
rustc --edition 2024 --test -A dead_code -o t solution.rs && ./t
rustc --edition 2024 -A dead_code -o m solution.rs && ./m
clippy-driver --edition 2024 --test -D warnings -A dead_code -o c solution.rs
rustfmt --check --edition 2024 exercise.rs solution.rs
```

### Lab checklist

- Code in `deep-dive/src/<lab>.rs`, opening with `//! Lab · title`; the module doc explains what it reveals, points to the graded exercise to compare with, and lists the invariants.
- Register it: `pub mod <lab>;` in `deep-dive/src/lib.rs`, a row in the `lib.rs` doc table, a row in the `deep-dive/README.md` "Lab index" table (File, Course module, What it reveals), and a COURSE.md row.
- Every unsafe op in its own `unsafe {}` block with a `// SAFETY:` comment, including inside `unsafe fn` (edition 2024).
- Keep `cargo clippy --all-targets -D warnings` clean; test with a `#[cfg(test)] mod tests` block that doubles as a usage example, using drop-counter payloads to prove each value is dropped exactly once; aim to stay Miri-clean.
- Optional external deps are `cfg`-gated (like `[target.'cfg(loom)'.dependencies]`) with a matching `unexpected_cfgs` check-cfg and CI step. Ungated external crates or proc macros need a sibling crate in the root `Cargo.toml` `[workspace] exclude` list with its own CI job.

## Sources

Collected by the interview-format-research lens (web research) and cited in its raw gap evidence; the merge step kept the conclusions but not the links, so they are restored here. They were not re-fetched for this roadmap.

### Interview question lists and process write-ups

- [techinterview.org — Rust interview questions (autonomy systems)](https://www.techinterview.org/post/3233477224/rust-interview-questions-autonomy-systems/) — guards across `.await`, blocking the executor, `.clone()` reflexes, lock decision tables; cited for async, sync primitives, borrowck, drop order, mock rounds.
- [Second Talent — Rust interview guide](https://www.secondtalent.com/interview-guide/rust/) — Pin, `tokio::Mutex` vs `std::Mutex`, graceful shutdown, backpressure, scoped tasks, rate limiters; cited for async, sync primitives, the tokio lab, mock rounds.
- [Advanced Rust interview questions you didn't see coming (Medium, fennsaji)](https://medium.com/@fennsaji/advanced-rust-interview-questions-you-didnt-see-coming-01d86e751510) — deadlocks in spawned tasks, false sharing, Send/Sync wrappers around C libraries, closures to C function pointers; cited for async, layout, FFI.
- [Turing — Rust interview questions](https://www.turing.com/interview-questions/rust) — channels, mutexes, trait bounds, destructors; cited for sync primitives, trait system, drop order.
- [Rust Skill — Rust developer interview process 2025](https://www.rust-skill.com/blog/rust-developer-interview-process-2025) — timed exercises that implement a data structure or debug concurrent code; cited for sync primitives, mock rounds, code review.
- [CoderPad — Rust interview questions](https://coderpad.io/interview-questions/rust-interview-questions/) — "what's wrong with this code" snippets; cited for the borrowck gauntlet and code-review drills.
- [CodeSubmit — Rust interview questions](https://www.codesubmit.io/interview/rust-interview-questions) — code-review scenarios as a question category; cited for code-review drills.
- [Zero To Mastery — Rust interview questions and answers](https://zerotomastery.io/blog/rust-interview-questions-and-answers/) — advanced trait-system questions; cited for coherence, associated types, Borrow, typestate, macros.
- [interviewing.io — LRU cache](https://interviewing.io/questions/lru-cache) — the LRU cache as a staple; cited for mock rounds and `lru-cache`.
- [FlakM — Rust interview questions](https://flakm.github.io/posts/rust_interview_questions/) — unsafe and FFI examples; cited for the FFI / UB labs.
- [KORE1 — Blockchain developer interview questions](https://www.kore1.com/blockchain-developer-interview-questions/) — overflow checks in the release profile; cited for checked math.
- [Comprehensive Rust discussion #1943](https://github.com/google/comprehensive-rust/discussions/1943) — niche optimization as a common interview question; cited for the layout quiz.
- [dtolnay/rust-quiz](https://github.com/dtolnay/rust-quiz) — the predict-the-output format; cited for the borrowck quiz and the drop-order quiz.

### Topic references cited as evidence

- [Cancelling async Rust (sunshowers)](https://sunshowers.io/posts/cancelling-async-rust/) and [Cancel safety in async and tokio select (users.rust-lang.org)](https://users.rust-lang.org/t/cancel-safety-in-async-and-tokio-select/92381) — cancel safety; cited for async pitfalls.
- [Enabling Polonius alpha on nightly (Rust blog)](https://blog.rust-lang.org/2026/08/04/enabling-polonius-alpha-on-nightly/) — why problem case #3 may compile in future; cited for `borrowck3`.
- [Rust 2024: if-let temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html) and [Rust 2024: tail expression temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html) — cited for drop order and scrutinee temporaries.
- [Common Rust lifetime misconceptions (pretzelhammer)](https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md) and [The Reference: subtyping and variance](https://doc.rust-lang.org/reference/subtyping.html) — cited for `'static`, variance and HRTB.
- [Microsoft Rust Training: newtype and type-state patterns](https://microsoft.github.io/RustTraining/rust-patterns-book/ch03-the-newtype-and-type-state-patterns.html) — cited for typestate and newtypes.
- [Helius — Solana arithmetic](https://www.helius.dev/blog/solana-arithmetic), [sec3 — arithmetic overflow and underflow in Rust and Solana smart contracts](https://sec3.dev/blog/understanding-arithmetic-overflow-underflows-in-rust-and-solana-smart-contracts), [Archway — smart contract math](https://docs.archway.io/developers/smart-contracts/math), [CosmWasm issue #1156](https://github.com/CosmWasm/cosmwasm/issues/1156) and [fermat-core (docs.rs)](https://docs.rs/fermat-core/latest/fermat_core/) — checked and saturating math in release builds, rounding policy, 18-digit decimals, U256 intermediates for `a * b / c`; cited for checked math.
