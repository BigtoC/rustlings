# Traits & Dispatch: Trait Objects, Dynamic vs Enum Dispatch

> Part of "Module 2: Traits, Generics, and Dispatch" in the course outline. This
> directory is about **how a method call actually reaches its code** — the three
> ways Rust dispatches a call and the trade-offs between them — using entirely
> **std**, **100% safe**, **stable** code.

## Core Ideas

When you call `value.area()`, the compiler has to decide which concrete function
runs. Rust offers three strategies:

- **Dynamic dispatch** (`dyn Trait`): the concrete type is **erased** behind a
  *fat pointer* — a data pointer plus a pointer to a **vtable** of the trait's
  methods. This is what lets a single `Vec<Box<dyn Shape>>` hold a `Circle` and a
  `Rectangle` at once. The cost: every call is an indirect hop through the vtable
  that the compiler cannot inline, and the value usually lives behind a pointer
  (often heap-allocated via `Box`).
- **Static dispatch** (generics, `<S: Shape>`): the compiler **monomorphizes** —
  it stamps out a specialized copy of the function for each concrete `S`, with the
  real method inlined. Fast and inlinable, but the slice must be homogeneous
  (one `S`) and the binary grows with each instantiation.
- **Enum dispatch** (a `match` over an `enum`): a **closed set** of variants
  stored inline, no heap box and no vtable. Each `match` arm inlines like ordinary
  code. The price is exactly that closedness — you must enumerate every variant
  here, and outside code cannot add a new one.

## Object Safety (dyn Compatibility)

Not every trait can become a `dyn Trait`. To build a vtable the compiler needs
every method to have a fixed calling convention behind a pointer. A method that
returns `Self` **by value** (or is generic) has no single describable slot, so the
trait is **not dyn-compatible** and `Box<dyn Trait>` is rejected with **E0038**.
The fix is not to delete the method but to gate it with `where Self: Sized`: that
removes it from the vtable while keeping it callable on concrete types.

## Exercise Path

1. **dispatch1** — Heterogeneous collection: a `Vec<T>` holds one `T`, but you
   need both a `Circle` and a `Rectangle` in one list. Erase the type behind a
   trait object and return a `Vec<Box<dyn Shape>>`.
2. **dispatch2** — Object safety: a `Widget` trait with `fn duplicate(&self) ->
   Self` is not dyn-compatible, so `Box<dyn Widget>` fails with E0038. Gate the
   offending method with `where Self: Sized` to bring `dyn Widget` back.
3. **dispatch3** — Static vs dynamic, same answer: given the `&[&dyn Shape]`
   dynamic version, write its monomorphized twin `total_static<S: Shape>(&[S])`
   and see both compute the same sum under different cost models.
4. **dispatch4** — Enum dispatch: implement `Shape::area` with a `match` over a
   closed set of variants — no `Box`, no vtable, every arm inlines.

## Further Reading

- [Trait Objects (The Book)](https://doc.rust-lang.org/book/ch18-02-trait-objects.html)
- [Static and Dynamic Dispatch (The Book)](https://doc.rust-lang.org/book/ch18-02-trait-objects.html#trait-objects-perform-dynamic-dispatch)
- [Object Safety / `dyn` compatibility (Reference)](https://doc.rust-lang.org/reference/items/traits.html#object-safety)
- [The `dyn` keyword](https://doc.rust-lang.org/std/keyword.dyn.html)
- [`Sized` marker trait](https://doc.rust-lang.org/std/marker/trait.Sized.html)
