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

## The Golden Rule of the Borrow Checker

**Aliasing XOR mutability**: at any given moment, a value is either shared by any
number of `&T` borrows, or exclusively borrowed by **exactly one** `&mut T` —
never both. This is precisely what rules out data races at compile time. Together
with **NLL (Non-Lexical Lifetimes)**, a borrow ends at its **last use** rather
than at the end of the scope — so a common way to resolve conflicts is to "finish
using the shared borrow first, then take the exclusive borrow."

## Exercise Path

1. **ownership1** — Move semantics and use-after-move: a helper function consumes
   a `String` by value, but the caller still needs it afterwards. Choose `&`
   (borrow) or `.clone()` (clone) to fix it.
2. **ownership2** — `Copy` vs `Move`: add `Copy` to a struct of plain integers,
   and contrast this with non-`Copy` types like `String` that must be `.clone()`d.
3. **ownership3** — The borrow checker: a mutable borrow is requested while a
   shared borrow is still alive, violating "aliasing XOR mutability." Refactor to
   something legal using NLL / by extracting the `Copy` value early.

## Further Reading

- [What Is Ownership? (The Book)](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)
- [References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [The `Copy` trait](https://doc.rust-lang.org/std/marker/trait.Copy.html)
