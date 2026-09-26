# Module 1 · Drop and RAII: Drop Order, Guards and Leaks

> The destructor half of Module 1, after `26_smart_pointers_deep`. `drop1` and
> `drop2` are quizzes: predict when each value is dropped. `raii1` to `raii4`
> put `Drop` to work the way interviews ask for it: a `defer` guard, a
> transaction that rolls back, a drop-check error, and a reference cycle that
> leaks. All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements but **not** the fix. Read what rustc or
> the test says first. Press `h` when you want the full answer.

## Core Ideas

- **A value is dropped exactly once, when its current owner goes away.** That
  point is fixed by the source code: the end of a binding's scope, the moment
  a container or struct holding it is dropped, the return of a function that
  took it as a parameter. Moving a value moves its drop along with it.
  `drop(x)` is nothing special: it is an empty function that takes `x` by
  value.
- **RAII runs cleanup on every way out.** Falling off the end of a block, an
  early `return`, a `?` and a panic that unwinds all end the scope, so they all
  run the destructors. That is how `MutexGuard` unlocks and `File` closes, and
  why Rust needs neither `finally` nor `defer`.
- **`Drop::drop` only gets `&mut self`.** The value is still whole while
  `drop` runs, and its fields are dropped right after. So you cannot move a
  field out inside `drop` (E0507), nor out of a `Drop` type you own (E0509,
  `24_ownership_model/ownership6`). Store what `drop` must consume as an
  `Option` and `take()` it (`raii1`). A method that consumes `self` cannot skip
  `drop` either; it can only leave a flag behind (`raii2`).
- **Destructors are not guaranteed to run.** Leaking is safe: `mem::forget`,
  `Box::leak` and `Rc` cycles (`raii4`) are all 100% safe Rust. Neither
  `std::process::exit` nor an abort (`panic = "abort"`, or a panic inside a
  `drop` during unwinding) runs them, and returning from `main` does not unwind
  other threads. Safe code may use RAII for cleanup; unsafe code must never
  rely on a destructor for soundness (see "Leakpocalypse" below).
- **A `Drop` impl changes what borrows are allowed.** The drop check assumes
  that `drop` may read every reference the value holds, so whatever it
  borrows must *strictly* outlive it (`raii3`). Adding `impl Drop` to a public
  type can break its users' code.

## Drop Order Cheat Sheet

Spoilers for `drop1` and `drop2`. Try them first.

| Situation                                       | When the value is dropped                                                                    |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Locals of one scope                             | at the end of the scope, in **reverse** order of declaration                                 |
| Fields of a struct, tuple, array or `Vec` items | the value's own `Drop::drop` (if any) first, then the parts in **declaration / index** order |
| Function parameters                             | when the function returns, after the body's locals, in reverse order                         |
| `let _g = f();`                                 | at the end of the scope (`_g` is an ordinary binding)                                        |
| `let _ = f();`                                  | at the end of the statement: `_` binds nothing                                               |
| `let _ = x;` (an existing binding)              | not at all here: `x` is not moved and lives on                                               |
| `x = new_value;`                                | the old value, at the assignment                                                             |
| A temporary in a statement                      | at the end of the statement, in reverse order of creation                                    |
| `let r = &f();`                                 | lifetime extension: at the end of the enclosing block, like a hidden local                   |
| A temporary in a block's tail expression        | 2024: before the block's locals; 2021: at the end of the enclosing statement                 |
| A temporary in an `if` / `while` condition      | before the body runs                                                                         |
| A `match` / `if let` / `while let` scrutinee    | after the arms / the then-block / the loop body; in 2024 an `if let` one before `else`       |

The last row is where real bugs live: a `RefMut` or `MutexGuard` in a
scrutinee is still alive in the arms. `31_debugging/debugging6..8` (later in
the course) drills it. rustc denies the classic `let _ = m.lock().unwrap();`
mistake outright with the `let_underscore_lock` lint.

To see which answers differ in edition 2021 without touching the course's
`Cargo.toml` (other exercises need 2024), compile one file on its own, from
the rustlings directory:
`rustc --edition 2021 --test exercises/39_drop_raii/drop2.rs -o target/drop2 && target/drop2`.

## Guards You Already Use

| Guard                                        | What its `Drop` does                       | Worth knowing                                                           |
| -------------------------------------------- | ------------------------------------------ | ----------------------------------------------------------------------- |
| `MutexGuard`, `RwLock*Guard`                 | unlocks                                    | a guard in a `match` scrutinee lives through every arm                  |
| `RefCell`'s `Ref` / `RefMut`                 | releases the run-time borrow               | same scoping rules as lock guards                                       |
| `File`                                       | closes the file descriptor                 | errors on close are ignored; call `sync_all` if you need them           |
| `BufWriter`                                  | flushes the buffer                         | errors are ignored in `drop`; call `flush()` to see them                |
| `thread::JoinHandle`                         | **detaches** the thread (it does not join) | use `thread::scope` when threads must finish before you continue        |
| `rusqlite::Transaction`, `sqlx::Transaction` | rolls back unless committed                | the pattern of `raii2`                                                  |

`drop` cannot return an error. Types whose cleanup can fail offer an explicit,
fallible method (`flush`, `sync_all`, `commit`) and keep `drop` as the
best-effort fallback.

## Drop Check and `#[may_dangle]`

For a type with a `Drop` impl, the borrow checker never looks inside `drop`.
It assumes the worst: `drop` may use every reference in the value. So in

```rust
let mut on_duty = Vec::new();
let roster = Roster::parse(text);
on_duty.push(Inspector(&roster.days_left()[0]));
```

`roster` is dropped before `on_duty`, and E0597 says the borrow "might be used
here, when `on_duty` is dropped and runs the `Drop` code for type `Vec`". Swap
the two `let`s and it compiles. Three details interviewers like:

- **Without `impl Drop for Inspector` it compiles as written.** Dropping a plain
  reference does nothing, so a reference may dangle as long as nobody uses it.
- **`Vec<&u8>` is fine in the same order**, even though `Vec` has a `Drop`
  impl. std writes it as
  `unsafe impl<#[may_dangle] T, A: Allocator> Drop for Vec<T, A>`: a promise
  that `Vec::drop` only drops its elements and never reads them. The
  attribute is the unstable `dropck_eyepatch` feature, so only std (and
  nightly code) can make that promise.
- **Dropping the borrower early is not always enough.** `drop(on_duty)` at the
  end of `run_shift` does not satisfy the checker: if a `push` panicked,
  unwinding would still drop `roster` first. Declaration order is the fix.

The Rustonomicon's version stores both in one struct
(`World { inspector: Option<Inspector<'a>>, days: Box<u8> }`) and fails with
E0597 in either field order: values dropped at the same time never *strictly*
outlive each other.

## Leaks Are Safe: the Leakpocalypse

Before Rust 1.0, `thread::scoped` returned a `JoinGuard` whose `Drop` joined
the thread, and the thread could borrow the caller's stack. Safe code could
leak the guard through an `Rc` cycle, which skipped the join and let the
thread outlive the data it borrowed: a use-after-free without any `unsafe`.
The fix was to accept that leaking is safe (RFC 1066 then made `mem::forget`
a safe function) and to remove the API. Its replacement,
`std::thread::scope` (1.63), takes a closure and joins every thread before it
returns, so no destructor has to run.

The planned `deep-dive/src/ub_zoo.rs` lab (ROADMAP: "Lab: what exactly is UB,
run under Miri") shows Miri reporting a leak that is not undefined behavior.

## Exercise Path

1. **drop1** — Quiz. Five scenarios log their drops: locals and `Vec`
   elements, `let _` versus `let _guard`, a struct with its own `Drop` built
   in the "wrong" field order, function parameters, and moves plus `drop()`
   plus assignment. Replace each `&[]` constant with the log you predict.
2. **drop2** — Quiz. Four scenarios with temporaries: two in one `let`
   statement, a `let r = &temp;` (lifetime extension), a block's tail
   expression (the one answer that changed in edition 2024), and an `if`
   condition.
3. **raii1** — E0507. A `Defer` guard calls its `FnOnce` closure from
   `drop(&mut self)`, which would move the closure out. Store it so that
   `drop` can take it, and write a `cancel` that disarms the guard without
   leaking the closure. Tests cover `return`, `?`, unwinding and LIFO order.
4. **raii2** — Failing tests. A `Tx` edits a store in place, but nothing
   rolls back an uncommitted transaction. Implement `Drop` to restore the
   snapshot, and make `commit(self)` leave `drop` a note. Tests cover `?`, a
   caught panic and cleared rows.
5. **raii3** — E0597 from the drop check. A `Vec` of `Inspector`s, whose
   `drop` reads what they borrow, is declared before the `Roster` they borrow
   from. Make the roster outlive them; keep both `Drop` impls.
6. **raii4** — Failing tests. A tree whose child holds a strong `Rc` to its
   parent leaks: the drop counter stays at 0. Make the back-pointer
   non-owning, so that `parent()` works while the parent lives and returns
   `None` afterwards.

Related exercises: `24_ownership_model/ownership4..6` (moving out of `&mut`
and out of `Drop` types), `26_smart_pointers_deep/smartptr2` (`Rc` and
`Weak`), `31_debugging/debugging6..8` (guards kept alive by scrutinees),
`40_interior_mutability` (`Cell`, `thread_local!`), `52_condvar/condvar3` (an
RAII semaphore permit), `57_async_combinators/select1` and `cancel1` (dropping
a future cancels it, and what that loses), and `59_arena` (trees without
reference counting). `49_panics` covers `catch_unwind`, poisoning and the
abort cases in depth.

## Further Reading

- [Destructors (The Reference)](https://doc.rust-lang.org/reference/destructors.html): drop scopes, temporary scopes, lifetime extension, not running destructors
- [Running Code on Cleanup with the `Drop` Trait (The Book)](https://doc.rust-lang.org/book/ch15-03-drop.html) and [Reference Cycles Can Leak Memory](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html)
- [`std::ops::Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html), [`std::mem::forget`](https://doc.rust-lang.org/std/mem/fn.forget.html) and [`std::rc::Weak`](https://doc.rust-lang.org/std/rc/struct.Weak.html)
- [Rust 2024: tail expression temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html) and [`if let` temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html)
- [Drop Check (The Rustonomicon)](https://doc.rust-lang.org/nomicon/dropck.html), the source of `Inspector`, and [Leaking](https://doc.rust-lang.org/nomicon/leaking.html)
- [Pre-Pooping Your Pants With Rust (Gankra)](https://faultlore.com/blah/everyone-poops/), the Leakpocalypse write-up, and [RFC 1066](https://rust-lang.github.io/rfcs/1066-safe-mem-forget.html), which made `mem::forget` safe
- [`std::thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html)
- [The `scopeguard` crate](https://docs.rs/scopeguard), a production `defer`
- [`rusqlite::Transaction`](https://docs.rs/rusqlite/latest/rusqlite/struct.Transaction.html), a real rollback-on-drop guard
