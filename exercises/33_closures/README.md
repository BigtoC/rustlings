# Closures: the `Fn` / `FnMut` / `FnOnce` Hierarchy

> How Rust turns an anonymous block-with-captures into a callable value, why
> a closure's *type* and *trait* depend entirely on what it does with the
> variables it captures, and how to write bounds for closures that take
> references or return futures. Entirely **std**, **100% safe**, **stable** code.

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

## Higher-Ranked Bounds: `for<'a>`

A bound on a callback that takes a reference has to say which lifetimes that
reference may have:

- A lifetime **parameter of the function** (`fn f<'a, F: Fn(&'a str)>`) is
  picked by the caller and outlives the whole call, so the callback can never
  be handed a borrow of something the function itself creates (E0597).
- A **higher-ranked** bound, `F: for<'a> Fn(&'a str) -> usize`, promises the
  impl for *every* lifetime, so each call picks its own, however short.
  Elision in the `Fn(..)` sugar writes it for you: `Fn(&str) -> usize` already
  is `for<'a> Fn(&'a str) -> usize`, and `Fn(&str) -> &str` is
  `for<'a> Fn(&'a str) -> &'a str`.
- Outside the `Fn` sugar you spell the binder out, most often as
  `where for<'a> &'a C: IntoIterator<Item = &'a T>`: "a borrow of `C` of any
  lifetime can be iterated".
- The binder covers only what is written inside it. In `sort_by_key`'s
  `F: FnMut(&T) -> K` the key type `K` is chosen outside, so a key cannot
  borrow from its element: `people.sort_by_key(|p| &p.name)` fails with
  "lifetime may not live long enough", while
  `people.sort_by(|a, b| a.name.cmp(&b.name))` compiles, because an
  `Ordering` borrows nothing.
- A closure gets a higher-ranked signature only when it is written where one
  is **expected**: as an argument to a function with an `Fn` bound, in a
  `Box<dyn Fn(..)>` coercion, or coerced to a `fn(&str) -> &str` pointer. A
  `let`-bound `|s: &str| s.trim()` gets a return lifetime unrelated to its
  argument. The explicit closure binder
  `for<'a> |s: &'a str| -> &'a str { .. }` is still unstable (E0658,
  [rust-lang/rust#97362](https://github.com/rust-lang/rust/issues/97362)).

## Async Closures

`F: FnMut() -> Fut` fixes one future type outside the call, so the future a
call returns cannot borrow the closure's captures ("captured variable cannot
escape `FnMut` closure body"). Async closures (`async || ..`, stable since Rust
1.85) come with the `AsyncFn`, `AsyncFnMut` and `AsyncFnOnce` traits, which are
in the prelude. A call returns a future that may **borrow the closure**, and
`AsyncFnMut` keeps the closure mutably borrowed until that future is gone. The
bound names the output, not the future type (`F: AsyncFnMut() -> T`), and plain
closures that return a future satisfy it too. The same holds for arguments:
`for<'a> Fn(&'a u8) -> Fut` cannot let the future borrow the argument, while
`AsyncFn(&u8)` can. One limit remains on stable: you cannot yet require the
future of an `AsyncFn*` call to be `Send`, because the associated type that
names it is unstable.

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
5. **closure5** — Higher-ranked callbacks. A fn-level `'a` in
   `F: Fn(&'a str) -> usize` can't accept a borrow of a local (E0597): drop it,
   since `Fn(&str)` already means `for<'a> Fn(&'a str)`. Then
   `sort_by_key(|p| &p.name)` fails because the key type sits outside the
   binder: compare in place with `sort_by`, keeping the sort stable.
6. **closure6** — `for<'a>` outside the `Fn` traits. A function that owns its
   container iterates `&c` three times, and a fn-level
   `where &'a C: IntoIterator` gives E0597 on every pass: make the bound
   higher-ranked so it works for `Vec`, arrays, `VecDeque`, `BTreeSet`, a
   type that is iterable only by reference and one that borrows its data.
7. **closure7** — Let-bound closures are not higher-ranked. Boxing
   `let trim = |s: &str| s.trim();` as a `dyn Fn(&str) -> &str` stage gives
   "lifetime may not live long enough" and E0308 "one type is more general
   than the other": give the closures an expected signature (inline, a helper
   with an `Fn` bound, or a named `fn`).
8. **closure8** — Async closures. A `retry` helper bounded by
   `F: FnMut() -> Fut` can't take a closure whose future borrows its captured
   `&mut` log ("captured variable cannot escape `FnMut` closure body"): bound
   it by `AsyncFnMut` and pass an `async ||` closure.

## Further Reading

- [Closures: Anonymous Functions that Capture Their Environment (The Book)](https://doc.rust-lang.org/book/ch13-01-closures.html)
- [The `Fn` trait](https://doc.rust-lang.org/std/ops/trait.Fn.html)
- [The `FnMut` trait](https://doc.rust-lang.org/std/ops/trait.FnMut.html)
- [The `FnOnce` trait](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)
- [Closure types (The Reference)](https://doc.rust-lang.org/reference/types/closure.html)
- [Higher-ranked trait bounds (The Reference)](https://doc.rust-lang.org/reference/trait-bounds.html#higher-ranked-trait-bounds)
- [Higher-Rank Trait Bounds (The Rustonomicon)](https://doc.rust-lang.org/nomicon/hrtb.html)
- [Closure expressions (The Reference)](https://doc.rust-lang.org/reference/expressions/closure-expr.html)
- [`slice::sort_by_key`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by_key)
- [Announcing Rust 1.85.0: async closures](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
- [RFC 3668: async closures](https://rust-lang.github.io/rfcs/3668-async-closures.html)
- [The `AsyncFnMut` trait](https://doc.rust-lang.org/std/ops/trait.AsyncFnMut.html)
