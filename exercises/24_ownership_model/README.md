# Module 1 · The Ownership Model: Move / Copy / Clone and the Borrow Checker

> The cornerstone of "Module 1: Ownership, Lifetimes, and Smart Pointers" in the
> course outline. This directory focuses on **the reasons behind** the ownership
> rules rather than introductory syntax, using entirely **std**, **100% safe**
> code.

## Core Ideas

Rust has no GC and almost no runtime overhead, and it achieves this through a set
of compile-time rules:

- **Every value has exactly one owner**, and the value is `drop`ped when its
  owner goes out of scope.
- Passing a value **by value** into a function = a **move**: ownership is
  transferred in, and the original binding becomes invalid.
- Want to keep using the original value after a move? There are two options:
  - **Borrow**: `&T` (shared, read-only) or `&mut T` (exclusive, writable) —
    zero-cost, just "borrowing to look at / borrowing to modify"; ownership is
    not transferred.
  - **Clone**: `.clone()` — makes a genuine copy, yielding an independent second
    owner (at a cost).
- A few "small and safely bitwise-copyable" primitives are `Copy` (integers,
  `bool`, `char`): passing them by value **copies** instead of moving, so the
  original value remains usable. A struct is never `Copy` automatically — even one
  whose fields are all `Copy` is only *eligible* and stays a move type until you
  opt in with `#[derive(Copy)]` (exactly what `ownership2` demonstrates). Types
  that own heap allocations (`String`, `Vec<T>`) can never be `Copy`.
- A `&mut T` is exclusive **access**, not ownership. You may change the value
  behind it, even replace it, but you may not move it out and leave the place
  empty (`ownership4`, `ownership5`). The same holds for the fields of a type
  with a `Drop` impl, even when you own the value (`ownership6`).

## The Golden Rule of the Borrow Checker

**Aliasing XOR mutability**: at any given moment, a value is either shared by any
number of `&T` borrows, or exclusively borrowed by **exactly one** `&mut T` —
never both. This is precisely what rules out data races at compile time. Together
with **NLL (Non-Lexical Lifetimes)**, a borrow ends at its **last use** rather
than at the end of the scope — so a common way to resolve conflicts is to "finish
using the shared borrow first, then take the exclusive borrow."

## Moving Out Without Leaving a Hole

The owner of a place behind a `&mut` may read it again and will eventually drop
it, and a type with a `Drop` impl hands its **whole** value to `drop`. So a
value may be moved out of such a place only if a valid value goes back in **in
the same step**. That is what the `std::mem` functions do:

| Tool                        | What it does                               | Needs                                             |
| --------------------------- | ------------------------------------------ | ------------------------------------------------- |
| `mem::take(&mut x)`         | returns the old `x`, leaves `T::default()` | `T: Default` (free for `Vec`, `String`, `Option`) |
| `mem::replace(&mut x, v)`   | returns the old `x`, leaves `v`            | a valid placeholder, such as an enum variant      |
| `mem::swap(&mut a, &mut b)` | exchanges the two values                   | two places of the same type                       |
| `opt.take()`                | `mem::replace(opt, None)` for an `Option`  | an `Option`                                       |

None of them copies heap data. Moving a `Vec` or a `String` moves only its
three-word header (pointer, capacity, length), so the tests can prove "moved,
not cloned" by comparing `as_ptr()` before and after.

| Code  | rustc says                                                                                      | Typical cause                                                     | Idiomatic fix                                                          |
| ----- | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------- |
| E0507 | cannot move out of `self.buf` which is behind a mutable reference                               | returning a field of `&mut self`, or swapping through a temporary | `mem::take`, `mem::swap`                                               |
| E0507 | cannot move out of `self.addr` as enum variant `Connecting` which is behind a mutable reference | `if let Variant { field } = *self` in a state transition          | `mem::replace(self, Placeholder)`, then write the next state back      |
| E0509 | cannot move out of type `T`, which implements the `Drop` trait                                  | moving a field out of an owned value whose type has a `Drop` impl | bind `mut self` and `mem::take` the field; `drop` runs on the leftover |

For a field behind `&mut` and for E0509, rustc's own notes point at `.clone()`.
In an interview that is the wrong first answer: it copies data (and allocates)
only to throw the original away, and a type that owns a unique resource should
not be `Clone` at all.

## Exercise Path

From `ownership4` on, the exercises are interview drills: the `// TODO` comments
name the error and the requirements but **not** the fix. Read what rustc says
first, and press `h` when you want the full answer.

1. **ownership1** — Move semantics and use-after-move: a helper function consumes
   a `String` by value, but the caller still needs it afterwards. Choose `&`
   (borrow) or `.clone()` (clone) to fix it.
2. **ownership2** — `Copy` vs `Move`: add `Copy` to a struct of plain integers,
   and contrast this with non-`Copy` types like `String` that must be `.clone()`d.
3. **ownership3** — The borrow checker: a mutable borrow is requested while a
   shared borrow is still alive, violating "aliasing XOR mutability." Refactor to
   something legal using NLL / by extracting the `Copy` value early.
4. **ownership4** — Moving out of `&mut self` (E0507): `flush` returns the
   batch's `Vec` directly, and `present` swaps a double buffer through a
   temporary. Move the values out while leaving a valid value behind (`mem::take`,
   `mem::swap`). Non-`Clone` events and heap-pointer checks rule out copying.
5. **ownership5** — Enum state transitions on `&mut self` (E0507): the obvious
   `if let Conn::Connecting { addr } = *self` moves out of a borrow. Swap a
   placeholder state in with `mem::replace`, move the fields out of the old
   state, and write the next state (or the unchanged old one) back.
6. **ownership6** — Moving a field out of a `Drop` type (E0509): `into_pending`
   owns the connection, but `drop` still needs the whole value. Take the buffer
   through a `mut self` binding, and let `drop` report the empty leftover.

## Further Reading

- [What Is Ownership? (The Book)](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)
- [References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [The `Copy` trait](https://doc.rust-lang.org/std/marker/trait.Copy.html)
- [`mem::take`](https://doc.rust-lang.org/std/mem/fn.take.html), [`mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html) and [`mem::swap`](https://doc.rust-lang.org/std/mem/fn.swap.html)
- [Running Code on Cleanup with the `Drop` Trait (The Book)](https://doc.rust-lang.org/book/ch15-03-drop.html)
- [Error codes E0507](https://doc.rust-lang.org/error_codes/E0507.html) and [E0509](https://doc.rust-lang.org/error_codes/E0509.html)
- [Learning Rust With Entirely Too Many Linked Lists: `mem::replace` in `push`](https://rust-unofficial.github.io/too-many-lists/first-push.html)
