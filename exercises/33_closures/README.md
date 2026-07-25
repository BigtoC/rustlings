# Closures: the `Fn` / `FnMut` / `FnOnce` Hierarchy

> How Rust turns an anonymous block-with-captures into a callable value, and why
> a closure's *type* and *trait* depend entirely on what it does with the
> variables it captures. Entirely **std**, **100% safe**, **stable** code.

## Core Ideas

A closure is an anonymous function that can **capture** variables from the
scope where it is written. Two things about a closure are inferred by the
compiler, not spelled out by you:

- **Its type** is unique and anonymous. Every closure expression has its own
  compiler-generated type — even two closures with the same signature are
  different types.
- **Its capture mode** is the *least-privileged* one the body needs: by shared
  reference (`&T`) if it only reads a variable, by exclusive reference
  (`&mut T`) if it mutates one, and by value only when the body forces it. The
  `move` keyword overrides this and captures every variable **by value**.

## The Trait Hierarchy

Which of the three closure traits a closure implements is decided by how a
**call** must access the closure's captured state:

- **`Fn`** — callable through `&self`: the body only *reads* its captures. Can
  be called repeatedly, even through a shared reference.
- **`FnMut`** — callable through `&mut self`: the body *mutates* a capture.
  Needs exclusive access per call.
- **`FnOnce`** — callable through `self`: the body *consumes* a capture (moves
  it out). Can be called at most once.

These nest: `Fn` implies `FnMut` implies `FnOnce`. So the rule of thumb for a
function parameter is to demand the **weakest** bound that still works — prefer
`FnOnce`, then `FnMut`, then `Fn` — because a weaker bound accepts more
closures.

## Returning Closures

Because each closure has its own anonymous type, a function that returns a
closure names it either as `impl Fn(..)` (exactly **one** concrete, unnamed
type — every path must return the same closure) or as `Box<dyn Fn(..)>` (a
**trait object** that erases the concrete type behind a pointer + vtable, so
structurally different closures can share one return type via dynamic dispatch).

## Exercise Path

1. **closure1** — `move` capture: a closure returned from a function borrows a
   local, but the closure outlives it (E0373). Add `move` so the closure owns
   its captures by value.
2. **closure2** — `FnMut`: a closure that mutates a captured counter is `FnMut`,
   not `Fn` (E0525). Relax a too-strict `F: Fn()` bound to `F: FnMut()`.
3. **closure3** — `FnOnce`: a closure that moves a captured `String` out of
   itself is `FnOnce`, callable only once. Relax `F: Fn() -> String` to
   `F: FnOnce() -> String`.
4. **closure4** — Returning closures: two `if`/`else` arms build different
   capturing closures, so `-> impl Fn(..)` can't unify them (E0308). Return
   `Box<dyn Fn(..)>` and box each arm.

## Further Reading

- [Closures: Anonymous Functions that Capture Their Environment (The Book)](https://doc.rust-lang.org/book/ch13-01-closures.html)
- [The `Fn` trait](https://doc.rust-lang.org/std/ops/trait.Fn.html)
- [The `FnMut` trait](https://doc.rust-lang.org/std/ops/trait.FnMut.html)
- [The `FnOnce` trait](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)
- [Closure types (The Reference)](https://doc.rust-lang.org/reference/types/closure.html)
