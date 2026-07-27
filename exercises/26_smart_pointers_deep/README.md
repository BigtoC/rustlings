# Module 1 · Smart Pointers (In Depth): Box Heap Allocation / Rc·Arc Shared Ownership / RefCell Interior Mutability

> Building on `19_smart_pointers`, this directory focuses on "why these smart
> pointers are needed, and which ownership problem each one solves." All **std**,
> **100% safe**.

## Core Ideas

Smart pointers are types that are "used like a pointer but carry extra semantics."
The three most common ones each have their own role:

- **`Box<T>`**: the simplest **exclusive** heap pointer, of fixed size (one
  machine word). Used to break the "infinitely large" dead loop of **recursive
  types** — putting child nodes on the heap is what makes a node's size
  determinable.
- **`Rc<T>`**: **reference counting**, which lets a single value have **multiple
  owners**. `clone` only increments the strong reference count rather than copying
  the data; the value is `drop`ped only when the last strong reference disappears.
  `Rc::strong_count` lets you query it. (`Arc<T>` is its thread-safe version,
  using atomic counting — see the concurrency module.)
- **`Weak<T>`**: a **non-owning** reference, used to break **reference cycles**
  between `Rc`s (otherwise the count can never reach zero → a memory leak).
  `Rc::downgrade` creates a `Weak`, and `.upgrade()` tries to get back an
  `Option<Rc>`. Typical usage: child nodes point back to their parent via `Weak`.
- **`RefCell<T>`**: **interior mutability** — it defers borrow checking from
  compile time to runtime, so you can call `.borrow_mut()` through a shared
  `&RefCell<T>` to mutate the data (a violation panics at runtime).
  `Rc<RefCell<T>>` is the standard combination for "shared and mutable state."

## Exercise Path

1. **smartptr1** — `Box<T>` and recursive types: wrap the tail of a cons list in
   `Box<List>`, or compilation fails with "recursive type has infinite size."
   Note what `Box` does *not* buy you: both a recursive walk over the chain and
   the compiler's derived drop glue still cost one stack frame per node.
2. **smartptr2** — `Rc<T>` shared ownership + `strong_count`, plus using `Weak<T>`
   (child → parent) to avoid reference cycles. Complete the links with
   `Rc::downgrade`, and assert the strong/weak counts and `upgrade`.
3. **smartptr3** — `RefCell<T>` interior mutability, often combined with `Rc` into
   `Rc<RefCell<T>>`: mutate through one shared handle and observe the change to
   the same data from another handle.

## Further Reading

- [`Box<T>` Points to Data on the Heap (The Book)](https://doc.rust-lang.org/book/ch15-01-box.html)
- [`Rc<T>`, the Reference Counted Smart Pointer](https://doc.rust-lang.org/book/ch15-04-rc.html)
- [`RefCell<T>` and Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [Reference Cycles Can Leak Memory](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html)
