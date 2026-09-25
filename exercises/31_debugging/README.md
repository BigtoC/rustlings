# Module 5 · Debugging and Interview Practice

> Corresponds to "Module 5: Practical Debugging and the Debug/Display Mechanism" in the syllabus.
> This directory uses only **std** and **100% safe** code to practice the skill of "locating and fixing problems" itself.

## Core idea

Debugging isn't about lucky guesses — it's about **information**. Rust gives you a few handy tools:

- `{:?}` / `{:#?}` — `Debug` formatting. `{:?}` is compact and single-line, while `{:#?}` is the indented, multi-line "pretty" output that's great for printing complex nested structures. Most types get this for free just by adding `#[derive(Debug)]`.
- `dbg!(x)` — a debugging powerhouse: it dumps `file:line` plus the `{:#?}` representation to **stderr** and **returns `x` unchanged**, so you can drop it inline into an expression as a probe.
  > Note: `dbg!` is only a temporary probe — don't leave it in finished code. Nothing will stop you here, though: `clippy::dbg_macro` is allow-by-default, and while rustlings' own source opts into it (`[workspace.lints.clippy]` in the repo root), the exercise crate you are editing deliberately does not. Stripping your probes is on you. (Add `dbg_macro = "deny"` to `[lints.clippy]` in your own projects if you want the gate.)
- `assert_eq!(a, b)` — on failure it prints **both values** ("expected 15, got 10"), letting you reason backward about the logic.
- Reading **compiler errors** and **assertion messages**: the compiler often tells you outright that "`X` doesn't implement `Debug`" or "the return type doesn't match" — read the message first, then act.

Beyond formatting, this module drills the runtime and compile-time failures that trip people up in interviews and production:

- **Integer overflow** is checked in debug builds (it PANICS) and wrapping in release builds (it silently wraps modulo 2^bits). Neither is a bug in Rust — you are expected to choose behavior on purpose with `checked_add` / `saturating_add` / `wrapping_add`.
- **`RefCell` borrow panics**: interior mutability moves "aliasing XOR mutability" from compile time to run time. Overlapping `borrow()` and `borrow_mut()` panics with `BorrowMutError` — scope your borrows tightly.
- **Iterator invalidation**: mutating a container while iterating is a runtime footgun in other languages, but a plain **compile error** (E0502) in Rust.
- **Guards kept alive by temporaries**: a `RefMut` or `MutexGuard` that never gets a name is a *temporary*, and a temporary is dropped at the end of its *temporary scope*, not after its last use. For a `match` scrutinee that scope covers every arm, for a `while let` scrutinee the whole loop body, and for an `if let` scrutinee the whole then-block. So a second `borrow_mut()` in there panics with "RefCell already borrowed", and a second `lock()` deadlocks the thread on itself. The borrow checker cannot help: `.cloned()` or `pop_front()` end the *borrow* of the guard, not the guard's life. The fix is syntactic: end the statement that created the guard before doing the work.

## Exercise progression

1. **debugging1** — Use `#[derive(Debug)]` to make a type support `{:?}` / `{:#?}`. The unfinished version fails to compile because "`Config` doesn't implement `Debug`"; just add the one derive line.
2. **debugging2** — **Integer overflow**: `checksum` adds `u32`s with `+=`, which PANICS on overflow in debug builds (and would silently wrap in release). The large-input test fails. Decide the intended behavior — here, clamp — and replace `acc += v;` with `acc = acc.saturating_add(v);` (`checked_add`/`wrapping_add` are the other deliberate choices).
3. **debugging3** — Hand-implement `std::fmt::Display` to format a color as `#ff00aa`. The unfinished version's `fmt` body is empty and fails to compile because `()` doesn't match `fmt::Result`; complete it with `write!` and `{:02x}`.
4. **debugging4** — **`RefCell` runtime borrow panic**: a `borrow()` guard is still alive when `borrow_mut()` is taken, so the code compiles but panics with `BorrowMutError`. Copy the value out (`let current = *cell.borrow();`) so the shared borrow ends immediately, then mutate.
5. **debugging5** — **Iterator invalidation**: removing from a `Vec` while `v.iter()` still borrows it is rejected at compile time (E0502). Replace the whole index-removing loop with one safe pass: `v.retain(|&x| x % 2 != 0);`.
6. **debugging6** — **A guard in a `match` scrutinee**: `match self.lanes.borrow_mut().now.pop_front() { .. }` keeps the `RefMut` alive through the arms, so a job that submits follow-ups panics with "RefCell already borrowed". Pop in a `let` statement of its own, then match. Part B swaps two fields behind one `RefCell`: two `borrow_mut()` temporaries in one statement panic, and the one-guard version is E0499, because every field access goes through `DerefMut` and borrows the whole guard. Reborrow once (`&mut *guard`) so the fields split, and swap the two lanes in O(1) instead of moving the jobs.
7. **debugging7** — **`while let` and `if let`**: a `while let` scrutinee's `RefMut` lives for the whole loop body, so the first retry panics. Rewrite the loop so each pop is a statement of its own (`loop` + `let ... else { break }`). Then an `if let` whose then-block still holds a `Ref`: edition 2024 drops scrutinee temporaries before the `else` block only.
8. **debugging8** — **`Mutex` self-deadlock**: `match cache.lock().unwrap().get(&k).cloned()` with a second lock in the `None` arm would block the thread on itself forever; a `try_lock` turns it into a panic here. Release the guard before computing, and keep the first value stored if another thread filled the key meanwhile. Part B is a four-question quiz on where the guard dies, checked against `try_lock` probes.

## `Debug` vs `Display`

| trait     | format specifier | who writes it        | audience         |
| --------- | ---------------- | -------------------- | ---------------- |
| `Debug`   | `{:?}`           | usually `#[derive]`  | programmers/logs |
| `Display` | `{}`             | must be hand-written | end users        |

`Display` can **never** be derived — it represents the wording you present "to the outside world", and only you can decide that. Any type that implements `Display` also gets `.to_string()` for free.

## Where a temporary guard dies

`q` is a `RefCell<VecDeque<_>>`; a `Mutex` behaves the same with `lock()` in place of `borrow_mut()`. Edition 2024 unless noted.

| the guard is created in                                   | it is dropped                                  | borrow again inside?         |
| --------------------------------------------------------- | ---------------------------------------------- | ---------------------------- |
| `let job = q.borrow_mut().pop_front();`                   | at the `;`                                     | yes, from the next statement |
| `let Some(job) = q.borrow_mut().pop_front() else { .. };` | at the `;`, or before `else` runs              | yes, in `else` and after     |
| `if q.borrow().is_empty() { .. }` (no pattern)            | once the condition is evaluated                | yes, in the body             |
| `match q.borrow_mut().pop_front() { .. }`                 | after the `match`, at the end of its statement | no, in no arm                |
| `while let Some(job) = q.borrow_mut().pop_front() { .. }` | at the end of each iteration's body            | no, not in the body          |
| `if let Some(job) = q.borrow_mut().pop_front() { .. }`    | after the then-block                           | no, not in the then-block    |
| the `else` block of that `if let`                         | 2024: before `else` runs; 2021: after it       | 2024: yes; 2021: no          |
| `f(q.borrow_mut().len(), q.borrow_mut().len())`           | both at the `;`                                | no: the second call fails    |

Edition 2024 also drops a block's tail-expression temporaries right after the tail expression, so `match { q.borrow_mut().pop_front() } { .. }` works too. Don't rely on it: it reads like a mistake, and Clippy's `blocks_in_conditions` suggests removing the braces, which brings the bug back. Name the value instead. For lock guards, the `nursery` lint `clippy::significant_drop_in_scrutinee` (off by default) flags the `match` case.

## Further reading

- [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) (formatting syntax and all format specifiers)
- [the `dbg!` macro](https://doc.rust-lang.org/std/macro.dbg.html)
- [`std::fmt::Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html)
- [`u32::saturating_add`](https://doc.rust-lang.org/std/primitive.u32.html#method.saturating_add) (and `checked_add` / `wrapping_add` next to it)
- [`std::cell::RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html)
- [`Vec::retain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.retain)
- [Temporary scopes (The Reference)](https://doc.rust-lang.org/reference/destructors.html#temporary-scopes) (the exact list of places where temporaries are dropped)
- [Rust 2024: `if let` temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html) and [Rust 2024: tail expression temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html)
- [The Book, chapter 21.2](https://doc.rust-lang.org/book/ch21-02-multithreaded.html) (Listing 21-21: the thread pool whose `while let` holds the `Mutex` while the job runs)
- [`Mutex::lock`](https://doc.rust-lang.org/std/sync/struct.Mutex.html#method.lock) (relocking on the same thread "will not return") and [`Mutex::try_lock`](https://doc.rust-lang.org/std/sync/struct.Mutex.html#method.try_lock)
- [`RefMut::map_split`](https://doc.rust-lang.org/std/cell/struct.RefMut.html#method.map_split) (another way to borrow two fields through one `RefMut`)
- [`clippy::significant_drop_in_scrutinee`](https://rust-lang.github.io/rust-clippy/master/index.html#significant_drop_in_scrutinee)
