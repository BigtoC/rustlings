# Module 1 · Variance and PhantomData: Marker Choice, Auto Traits and the Borrowed-Forever `&'a mut self`

> Module 1's last stop on lifetimes, after `25_lifetimes_deep` and
> `37_borrowck_errors`. `variance1` picks the right `PhantomData` marker for
> three types: a typed id that must stay covariant and `Send + Sync`, a sink
> that must be contravariant, and a handle that must stay on its thread.
> `variance2` is the bug every intermediate Rust developer hits: a
> `&'a mut self` method on a `Foo<'a>` that leaves the value borrowed for the
> rest of its life. All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements but **not** the fix. Read what rustc
> says first. Press `h` when you want the full answer.

## Core Ideas

- **The only subtyping in Rust is between lifetimes.** A `&'static str` can
  go where a `&'a str` is expected, because it is valid for longer.
  *Variance* says what that means for a generic type: `F<T>` is
  **covariant** if `F<&'static str>` can go where `F<&'a str>` is expected,
  **contravariant** if it is the other way round, and **invariant** if the
  lifetimes must match exactly.
- **Variance is inferred from the fields, and so are `Send` and `Sync`.** A
  type parameter that no field uses is E0392 "type parameter `T` is never
  used", because rustc would have nothing to infer from. A `PhantomData<X>`
  field costs zero bytes and counts as an `X` for both, so choosing `X` is
  choosing the variance *and* the auto traits of your type.
- **`&mut T` is invariant in `T`.** If `&mut Vec<&'static str>` could be used
  as `&mut Vec<&'a str>`, you could push a short-lived `&str` through it and
  read it back later as `&'static str`. The same goes for `Cell<T>` and
  every other type that can change its contents through a shared path.
- **A struct's lifetime parameter is not the lifetime of a borrow of the
  struct.** `impl<'a> Foo<'a> { fn m(&'a mut self) }` asks for a
  `&'a mut Foo<'a>`. Invariance forbids shrinking the `Foo<'a>`, so the
  borrow has to last for all of `'a`: the first call locks the value until
  its last use. Write `&mut self` and keep `'a` for the data the struct
  stores. `&'a self` is usually harmless, because `&T` is covariant.
- **`#[derive]` bounds every type parameter.** `#[derive(Clone)]` on
  `Id<T>` generates `impl<T: Clone> Clone for Id<T>`, even when no field
  stores a `T`. For marker types, write the impls by hand.

## The Variance Table

Adapted from the Rustonomicon ("—" means the type has no such parameter):

| Type                                | In `'a`   | In `T`                                     |
| ----------------------------------- | --------- | ------------------------------------------ |
| `&'a T`                             | covariant | covariant                                  |
| `&'a mut T`                         | covariant | **invariant**                              |
| `Box<T>`, `Vec<T>`, `Option<T>`     | —         | covariant                                  |
| `Cell<T>`, `RefCell<T>`, `Mutex<T>` | —         | **invariant** (all built on `UnsafeCell`)  |
| `*const T`                          | —         | covariant                                  |
| `*mut T`                            | —         | **invariant**                              |
| `fn(T) -> U`                        | —         | **contravariant** in `T`, covariant in `U` |
| `dyn Trait<T> + 'a`                 | covariant | **invariant**                              |
| `PhantomData<T>`                    | —         | covariant                                  |

Two consequences that interviewers like: `fn(T) -> T` is invariant (one
covariant use and one contravariant use disagree), and `dyn Fn(T)` is **not**
contravariant, because every type argument of a trait object is invariant.
That is why `variance1` rejects `PhantomData<Box<dyn Fn(T)>>` as a sink
marker. `Box<dyn Trait + 'static>` still converts to `Box<dyn Trait + 'a>`:
the object lifetime bound itself is covariant.

### Will It Compile?

Interviewers like to fire these one line at a time. Each row is a function
whose body is just its argument, `fn f<'a>(x: From) -> To { x }`, checked on
Rust 1.96. A "no" is the code-less "lifetime may not live long enough".

| From                           | To                          | Compiles? | Why                                                |
| ------------------------------ | --------------------------- | --------- | -------------------------------------------------- |
| `&'a Vec<&'static str>`        | `&'a Vec<&'a str>`          | yes       | `&T` is covariant in `T`                           |
| `&'a mut Vec<&'static str>`    | `&'a mut Vec<&'a str>`      | no        | `&mut T` is invariant in `T`                       |
| `&'a mut &'static str`         | `&'a &'a str`               | yes       | `&mut` coerces to `&`, which is covariant          |
| `fn(&'a str)`                  | `fn(&'static str)`          | yes       | argument types are contravariant                   |
| `fn(&'static str)`             | `fn(&'a str)`               | no        | the same rule, the other way                       |
| `*const &'static str`          | `*const &'a str`            | yes       | `*const T` is covariant                            |
| `*mut &'static str`            | `*mut &'a str`              | no        | `*mut T` is invariant                              |
| `Cell<&'static str>`           | `Cell<&'a str>`             | no        | everything built on `UnsafeCell` is invariant      |
| `Box<dyn Fn() + 'static>`      | `Box<dyn Fn() + 'a>`        | yes       | the object lifetime bound is covariant             |
| `Box<dyn FnMut(&'static str)>` | `Box<dyn FnMut(&'a str)>`   | no        | the type arguments of a trait object are invariant |
| `Box<dyn Fn(&'a str)>`         | `Box<dyn Fn(&'static str)>` | no        | the same: `dyn Fn(T)` is not contravariant         |

## Picking a `PhantomData` Marker

| Marker                                            | Variance in `T` | `Send`       | `Sync`       | Typical use                                                   |
| ------------------------------------------------- | --------------- | ------------ | ------------ | ------------------------------------------------------------- |
| `PhantomData<T>`                                  | covariant       | if `T: Send` | if `T: Sync` | a type that owns `T`s behind a raw pointer (`Vec`, `Box`)     |
| `PhantomData<&'a T>`                              | covariant       | if `T: Sync` | if `T: Sync` | a type that borrows `T`s behind a raw pointer (`slice::Iter`) |
| `PhantomData<&'a mut T>`                          | invariant       | if `T: Send` | if `T: Sync` | a type that borrows `T`s mutably (`slice::IterMut`)           |
| `PhantomData<fn() -> T>`                          | covariant       | always       | always       | a typed id or handle: it can produce a `T`, it never owns one |
| `PhantomData<fn(T)>`                              | contravariant   | always       | always       | something that only consumes `T`s                             |
| `PhantomData<fn(T) -> T>`                         | invariant       | always       | always       | a `T` that must match exactly, without affecting auto traits  |
| `PhantomData<*const ()>` or `PhantomData<Rc<()>>` | — (no `T`)      | never        | never        | opting a type out of both auto traits                         |
| `PhantomData<Cell<()>>`                           | — (no `T`)      | yes          | never        | opting out of `Sync` only                                     |

On stable Rust there is no other way to opt out: `impl !Send for X {}` is
E0658 "negative impls are experimental". std itself uses that unstable feature
for `MutexGuard`, which is not `Send` because POSIX requires a mutex to be
unlocked by the thread that locked it.

Lifetime-only markers follow the same rules: `PhantomData<&'a ()>` makes a
handle covariant in `'a` (it may not outlive some borrow), and
`PhantomData<fn(&'a ()) -> &'a ()>` or `PhantomData<Cell<&'a ()>>` makes `'a`
invariant (the `Cell` one also gives up `Sync`), the trick behind "branded"
lifetimes such as `GhostCell`.

`PhantomData<T>` also tells the drop checker that dropping your type may drop
a `T`. If your type has any drop glue (a `Vec<u8>` field is enough) and `T`
has a `Drop` impl that can see borrowed data, every borrow inside `T` must
still be alive when your value is dropped, even though you wrote no `Drop`
impl (E0597, as in `39_drop_raii`). `PhantomData<fn() -> T>` makes no such
claim. For `Vec`, whose `Drop` impl uses the unstable `#[may_dangle]` (see the
drop-check section of `39_drop_raii`), the `PhantomData<T>` is what still
tells the checker that the elements get dropped.

### Testing That Something Is *Not* `Send`

`assert_send::<X>()` can only prove that `X` is `Send`. The proper tool for
the opposite is a `compile_fail` doctest, which the planned deep-dive lab
`api-surface-lab` (ROADMAP: "testing a library from outside") adds, together
with a positive control. The `variance1` tests use the trick of the
[`impls`](https://docs.rs/impls) crate instead:

```rust
struct Probe<T: ?Sized>(PhantomData<T>);

trait Fallback {
    const IS_SEND: bool = false;
}
impl<T: ?Sized> Fallback for Probe<T> {}

impl<T: ?Sized + Send> Probe<T> {
    const IS_SEND: bool = true; // chosen first, but only if `T: Send`
}
```

An inherent associated item wins over a trait's, but only when the inherent
impl applies, so `Probe::<Rc<u8>>::IS_SEND` is `false`. It answers correctly
for **concrete** types only: inside `fn check<T>()` the `T` has no `Send`
bound, so the probe always falls back to `false`. Wrap it in a macro, never in
a generic function.

## Borrowed Forever: How to Spot It

- rustc says "first borrow later used here" **at the very call that
  conflicts**, or "argument requires that `x` is borrowed for `'a`". Nothing
  in your code uses the first borrow; a signature ties it to the value's own
  type.
- E0621 "explicit lifetime required in the type of `x`", with a help line
  that turns `x: &mut Foo<'a>` into `x: &'a mut Foo<'a>`. Don't follow it. In
  a function that calls the method in a loop, the E0499 next to it stays
  ("mutably borrowed here in the previous iteration of the loop"), and every
  caller of that function now lends its value for good. Fix the method that
  asked for `&'a mut self` instead.
- The field form is the same bug: `&'a mut &'a str`, `&'a mut Vec<&'a T>`,
  `&'a mut Parser<'a>`. Give the outer borrow its own lifetime
  (`&'s mut &'a str`). In a function signature, elision already does that:
  `fn next<'a>(input: &mut &'a str) -> Option<&'a str>`.
- Compare `37_borrowck_errors/borrowck4`: there a *returned* reference keeps a
  `&mut` alive. In `variance2` the method returns a plain `usize`, and the
  signature alone keeps the borrow alive.

## Exercise Path

1. **variance1** — Three wrong markers. `Id<T>` uses `PhantomData<T>`, so an
   id into a table of `Rc`s cannot cross threads (E0277), and its derives
   demand `User: Eq`, `User: Hash`, `User: Debug` and `User: Clone` (E0369,
   E0277): make it covariant, always `Send + Sync`, and implement its traits
   for every `T`. `Sink<T>` is covariant, so `widen` fails with "lifetime may
   not live long enough": make it contravariant. `LocalOnly` is still `Send`
   and `Sync` (a failing test): opt it out of both, on stable.
2. **variance2** — `fn add(&'a mut self, word: &'a str)` on `Interner<'a>`
   makes the second `add` E0499 and `len()` E0502, and rustc's E0621
   suggestion for the helper `intern_all` would spread the bug: borrow `self`
   only for the call. Then `WordCursor { input: &'a mut &'a str }` keeps the
   caller's `rest` borrowed after the cursor is gone (E0502, E0499): give the
   two borrows their own lifetimes.

Related exercises: `25_lifetimes_deep/lifetimes6` (returning `&'a str` rather
than a borrow of `self`, which `Interner::get` relies on),
`37_borrowck_errors/borrowck4` (no downgrade from `&mut`),
`40_interior_mutability` (why every cell is invariant), `47_type_level/typestate1`
(a `PhantomData` state marker), `30_send_sync` (the auto-trait rules in depth)
and `59_arena` (typed ids in practice). The planned deep-dive lab
`deep-dive/src/ub_zoo.rs` (ROADMAP: `miri-ub-zoo`) has
`unsound_covariant_cell`, a cell with the wrong marker that lets safe code
read freed memory.

## Further Reading

- [Subtyping and Variance (The Rustonomicon)](https://doc.rust-lang.org/nomicon/subtyping.html), the source of the variance table
- [PhantomData (The Rustonomicon)](https://doc.rust-lang.org/nomicon/phantom-data.html) and [Send and Sync (The Rustonomicon)](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [Subtyping and variance (The Reference)](https://doc.rust-lang.org/reference/subtyping.html)
- [`std::marker::PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html), [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) and [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html)
- [RFC 738: variance](https://rust-lang.github.io/rfcs/0738-variance.html), which introduced inferred variance and `PhantomData`
- [Common Rust Lifetime Misconceptions](https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md), misconception 5 ("if it compiles then my lifetime annotations are correct") walks through the `&'a mut self` trap
- [Implied bounds and perfect derive (Niko Matsakis)](https://smallcultfollowing.com/babysteps/blog/2022/04/12/implied-bounds-and-perfect-derive/), on why `#[derive]` bounds every type parameter
- [`derive-where`](https://docs.rs/derive-where) and [`educe`](https://docs.rs/educe), derives that let you choose the bounds
- [`impl Read for &[u8]`](https://doc.rust-lang.org/std/io/trait.Read.html) advances the slice it reads from, and [winnow's tutorial](https://docs.rs/winnow/latest/winnow/_tutorial/chapter_1/index.html) writes parsers as `fn(input: &mut &'s str)`
- [Rust error code index](https://doc.rust-lang.org/error_codes/error-index.html): E0392, E0499, E0502, E0621, E0658
