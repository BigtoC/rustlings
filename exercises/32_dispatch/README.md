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

That gate is only right for a method you never need through `dyn`. A generic
`fn accept<V: Visitor>(&self, v: &mut V)` is a whole family of functions, one
per `V`, and the set of `V`s is open, so there is no finite vtable to build. But
`accept` is the method a visitor calls on a node of unknown type, so gating it
just moves the error to every call site ("the `accept` method cannot be invoked
on a trait object"). Change the signature instead: take the visitor as
`&mut dyn Visitor`, so both sides dispatch through a vtable (double dispatch).

"Which trait items break dyn compatibility, and what is the fix for each?" is a
common interview question. rustc names the culprit in a note under E0038
("...because ..."); these are the notes rustc 1.96 prints:

| Trait item | rustc's note: "...because ..." | Usual fix |
| --- | --- | --- |
| `fn duplicate(&self) -> Self` | method `duplicate` references the `Self` type in its return type | `where Self: Sized` (dispatch2) |
| `fn new() -> Self` (no receiver) | associated function `new` has no `self` parameter | `where Self: Sized` |
| `fn same(&self, other: &Self) -> bool` | method `same` references the `Self` type in this parameter | `where Self: Sized`, or take `other: &dyn Shape` and downcast it through `Any` (dispatch5) |
| `fn accept<V: Visitor>(&self, v: &mut V)` | method `accept` has generic type parameters | take `v: &mut dyn Visitor` (dispatch6); gate it only if it is never called through `dyn` |
| `fn items(&self) -> impl Iterator<Item = u32>` | method `items` references an `impl Trait` type in its return type | return `Box<dyn Iterator<Item = u32> + '_>` |
| `async fn load(&self) -> u32` | method `load` is `async` | return `Pin<Box<dyn Future<Output = u32> + Send + '_>>` (what `#[async_trait]` generates) |
| `const ID: u32;` | it contains associated const `ID` | make it a method, `fn id(&self) -> u32` |
| `type Item<'a> where Self: 'a;` | it contains generic associated type `Item` | add `Self: Sized` to its `where` clause (it is then unusable through `dyn`), or move it to another trait |
| `trait Shape: Clone` | it requires `Self: Sized` | drop the supertrait; add `fn clone_box(&self) -> Box<dyn Shape>` and `impl Clone for Box<dyn Shape>` |

Lifetime parameters (`fn get<'a>(&'a self) -> &'a u32`) are fine, and an
associated type only has to be named in the object type (`dyn Iterator<Item =
u32>`). `where Self: Sized` also works for the `-> impl Trait` and `async fn`
rows, when those methods only need to exist on concrete types.

## Downcasting: `Any`, `TypeId` and Trait Upcasting

Type erasure is one-way unless the value carries a run-time type tag. In std
that tag is `TypeId`, and the trait that exposes it is `Any`.

- **`Any` is implemented for every `'static` type** by a blanket impl. On
  `dyn Any` (and `dyn Any + Send`, `dyn Any + Send + Sync`) std provides
  `is::<T>()`, `downcast_ref::<T>()` and `downcast_mut::<T>()`, which compare
  `TypeId`s and only then hand back a `&T`. Owned versions exist too:
  `Box<dyn Any>::downcast::<T>()` returns `Result<Box<T>, Box<dyn Any>>`. On a
  miss you get a `Box<dyn Any>` back, not your original `Box<dyn Shape>`, so
  check with `is::<T>()` on a borrow first if you need to keep it.
- **Why `'static`.** A `TypeId` is computed with lifetimes erased, so
  `&'a str` would share one with `&'static str`. Allowing borrowed types would
  let a downcast turn a short-lived reference into a `'static` one. The cost of `trait Shape: Any` is that every
  implementor must be `'static`: `impl<'a> Shape for Label<'a>` is E0478
  "lifetime bound not satisfied".
- **Trait upcasting** (stable since Rust 1.86) coerces a trait object to one of
  its supertraits: `&dyn Shape` to `&dyn Any` or `&dyn Debug`, `Box<dyn Shape>`
  to `Box<dyn Any>`. The data pointer stays the same; at most the vtable
  pointer changes. It only works for supertraits; without `Shape: Any`, the coercion is
  E0308 "expected trait `Any`, found trait `Shape`".
- **The `&Box<dyn Shape>` trap.** `Box<dyn Shape>` is `'static`, so it is an
  `Any` of its own. `let any: &dyn Any = s;` with `s: &Box<dyn Shape>` erases
  the **box**, and every downcast to `Circle` quietly returns `None`. Upcast the
  shape inside it instead: `s.as_ref()` or `&**s`. Clippy's `type_id_on_box`
  catches the method-call form of this bug ("calling `.type_id()` on
  `Box<dyn Shape>`"), but not the coercion form. `Box<dyn Error>` does not
  have this trap: `downcast_ref` is an inherent method of `dyn Error`, and
  `Box<dyn Error>` does not itself implement `Error`, so method lookup derefs
  to the inner `dyn Error`.

Before Rust 1.86 you could not upcast, so every implementor wrote the same
boilerplate method:

```rust
trait Shape {
    fn as_any(&self) -> &dyn Any;
}

impl Shape for Circle {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

A blanket helper (`trait AsAny { fn as_any(&self) -> &dyn Any; }` with
`impl<T: Any> AsAny for T`, and `trait Shape: AsAny`) removes the boilerplate but
brings the trap back in another form: `s.as_any()` on a `&Box<dyn Shape>`
resolves to the **box's** own `AsAny` impl before auto-deref reaches the shape,
so you must write `(**s).as_any()`. With upcasting, `trait Shape: Any` needs no
extra method at all.

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
5. **dispatch5** — Downcasting: `all_of::<T>()` should return every `T` in a
   `&[Box<dyn Shape>]`, but it compiles, runs and finds nothing, because its
   `&dyn Any` erases the `Box` rather than the shape inside it. Make `Any` a
   supertrait of `Shape` and upcast the inner `dyn Shape` (trait upcasting,
   Rust 1.86+), so the downcast works even for shape types defined elsewhere.
6. **dispatch6** — Generic methods vs dyn compatibility: a visitor-pattern
   `Node` trait with `fn accept<V: Visitor>` is not dyn-compatible (E0038, "has
   generic type parameters"), and `where Self: Sized` would take away the one
   method you need through `dyn Node`. Drop the type parameter so that any
   visitor, even one that is itself a `&mut dyn Visitor`, can walk the tree.

## Further Reading

- [Trait Objects (The Book)](https://doc.rust-lang.org/book/ch18-02-trait-objects.html)
- [Static and Dynamic Dispatch (The Book)](https://doc.rust-lang.org/book/ch18-02-trait-objects.html#trait-objects-perform-dynamic-dispatch)
- [Object Safety / `dyn` compatibility (Reference)](https://doc.rust-lang.org/reference/items/traits.html#object-safety)
- [The `dyn` keyword](https://doc.rust-lang.org/std/keyword.dyn.html)
- [`Sized` marker trait](https://doc.rust-lang.org/std/marker/trait.Sized.html)
- [The `std::any` module](https://doc.rust-lang.org/std/any/index.html) and the [`Any` trait](https://doc.rust-lang.org/std/any/trait.Any.html)
- [Announcing Rust 1.86.0, "Trait upcasting" section (Rust Blog)](https://blog.rust-lang.org/2025/04/03/Rust-1.86.0/)
- [Unsized coercions, including `dyn T` to `dyn Supertrait` (Reference)](https://doc.rust-lang.org/reference/type-coercions.html#unsized-coercions)
- [Clippy lint `type_id_on_box`](https://rust-lang.github.io/rust-clippy/master/index.html#type_id_on_box)
