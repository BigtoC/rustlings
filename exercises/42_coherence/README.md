# Traits & Abstraction · Coherence: Orphan Rule, Blanket Impls, Extension Traits

> Part of the "Traits & Abstraction" group, after `32_dispatch` through
> `35_error_design` (whose `err4` already built an extension trait and whose
> `err6` ran into an E0119) and before `43_assoc_types`. It answers a classic
> trait-system interview question: "Why can't you write
> `impl Display for Vec<Point>`, and what are your options?". All **std**,
> **100% safe**, **stable** Rust, edition 2024.
>
> As in the rest of the interview track, the `// TODO` comments name the error
> and the requirements but **not** the fix. Read what rustc says first. Press
> `h` when you want the full answer.

## Core Ideas

- **Coherence**: for any type and any trait (with its generic arguments, so
  `From<u8>` and `From<u16>` count as two traits) there is at most **one** impl
  in the whole program, your crate plus every crate it depends on. Every crate
  that asks "which impl runs here?" must get the same answer.
- rustc enforces it with two checks:
  - the **orphan rule** decides *where* an impl may be written (part 1);
  - the **overlap check** rejects an impl that could apply to the same type as
    another impl, in your crate or in a dependency (part 2).
- Both checks assume that your dependencies will keep evolving. Implementing
  an existing trait for an existing type (anything short of a new blanket impl)
  counts as a compatible change for the crate that owns one of them, so rustc
  never relies on the *absence* of an impl that an upstream crate could add
  that way. That is where the note "upstream crates may add a new impl" comes
  from.
- **Negative reasoning** ("type `X` does not implement trait `Y`, so these two
  impls can't overlap") is allowed only when the answer cannot change behind
  your back: for **local** types, and for the fundamental wrappers `&T`,
  `&mut T`, `Box<T>` and `Pin<T>` around them.
- **Inherent impls** are stricter still: `impl SomeType { .. }` may only be
  written in the crate that defines `SomeType`. To add methods to a type you
  don't own, you write an **extension trait**.

## The Orphan Rule

The Reference states it for `impl<P1..=Pn> Trait<T1..=Tn> for T0`. The impl is
allowed if `Trait` is local, or if at least one of `T0..=Tn` is a local type
with no *uncovered* type parameter (a bare `P`, or `&P` / `Box<P>`) before the
first local one. A type is local when its outermost constructor is defined in
this crate: `Point` is local, and so is `Polyline` although it wraps a `Vec`,
but `Vec<Point>` is not. Type aliases don't change locality.

Why so strict? If you could write `impl Display for Vec<Point>`, std could
never add `impl<T: Debug> Display for Vec<T>` without breaking you. The orphan
rule keeps that door open for the crate that owns the trait or the type.

Verified on rustc 1.96, where `Point` and `Polyline(Vec<Point>)` are local and
`Point: Display`:

| Impl                                             | Result | Why                                                                                         |
| ------------------------------------------------ | ------ | ------------------------------------------------------------------------------------------- |
| `impl Display for Vec<Point>`                    | E0117  | `Display` and `Vec` are both std's; the element type doesn't count                          |
| `type Line = Vec<Point>;` then `for Line`        | E0117  | an alias is only a second name for the same type                                            |
| `impl Display for [Point]`                       | E0117  | slices aren't local either ("... can be implemented for arbitrary types")                   |
| `impl Display for Polyline`                      | ok     | a local newtype, the standard fix (coherence1)                                              |
| `impl From<Point> for (i32, i32)`                | ok     | the trait parameter `T1 = Point` is local (coherence1)                                      |
| `impl<T> From<Point> for Vec<T>`                 | ok     | `T` is covered by `Vec`; allowed since Rust 1.41 (RFC 2451)                                 |
| `impl<T> From<Point> for T`                      | E0210  | the uncovered `T` comes before the first local type                                         |
| `impl<T: Shape> Display for T`                   | E0210  | a foreign trait for a bare type parameter: no local type anywhere                           |
| `impl Add for &Point`, `impl Neg for Box<Point>` | ok     | `&` and `Box` are fundamental, so `&Point` and `Box<Point>` count as local                  |
| `impl Display for Box<Point>`                    | E0119  | passes the orphan rule, but overlaps alloc's `impl<T: Display + ?Sized> Display for Box<T>` |

The last row is a trap: the orphan rule is not the only check. `Box<Point>` is
local, but since `Point` is `Display`, std's impl for `Box<T>` already covers
it. `impl Add for &Point` is the shape of impl that `43_assoc_types/assoc2`
writes for `&a + &b`.

## Overlap and Negative Reasoning

A **blanket impl** is one over a bare type parameter, such as the given
`impl<T: Display + ?Sized> Describe for T` in coherence2. Next to it, every
other `Describe` impl must be for a type that can **never** be `Display`:

| Next to the blanket impl                                        | Result                 | Why                                                                       |
| --------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------- |
| `impl Describe for Vec<u8>`                                     | E0119, "upstream" note | std may add `impl Display for Vec<u8>`                                    |
| `impl Describe for Vec<Bytes>`                                  | E0119, "upstream" note | std may add `impl<T> Display for Vec<T>`, which would cover it            |
| `impl Describe for &Vec<u8>`                                    | E0119, "upstream" note | `&T: Display` whenever `T: Display`, and `Vec<u8>` might become `Display` |
| `impl Describe for [u8]`                                        | E0119, "upstream" note | the blanket is `?Sized`, so it covers unsized types too                   |
| `impl Describe for Bytes`                                       | ok                     | local and not `Display`; only this crate could change that (coherence2)   |
| `impl Describe for Box<Bytes>`                                  | ok                     | `Box` is fundamental, so rustc may reason about `Box<Bytes>` like `Bytes` |
| `impl Describe for Rc<Bytes>`                                   | E0119, "upstream" note | `Rc` isn't fundamental, so rustc can't rule out `Rc<Bytes>: Display`      |
| `impl Describe for Celsius`, where the local `Celsius: Display` | E0119, no note         | a real overlap, not a possible one                                        |

What that costs:

- **No specialization on stable.** Once a blanket impl covers a type, you
  can't give that type a "more specific" impl. Full specialization (RFC 1210)
  is still unstable because it is unsound when the specific impl depends on
  lifetimes. std uses the restricted `min_specialization` internally, which is
  how `ToString`, `impl<T: Display + ?Sized> ToString for T`, stays fast for
  `str` and `String`. It is also why you implement `Display` and never
  `ToString`.
- **A blanket impl is semver-major.** RFC 2451 says so explicitly: adding
  `impl<T: Display> MyTrait for T` to a published trait breaks any
  downstream crate that already implements `MyTrait` for a type that is
  `Display`.
- **A blanket impl closes the trait.** With `impl<I: Iterator> IterExt for I`
  in place, nobody, you included, can write `impl IterExt for MyIter`: it is
  E0119. For extension traits that is usually what you want.

`35_error_design/err4` is the local-type case in action (its two `Context`
impls coexist because `Report` is local and not an `Error`), and
`35_error_design/err6` is the reflexive `impl<T> From<T> for T` getting in the
way.

## Extension Traits

An inherent impl on a foreign type is rejected three different ways, and every
one of them points at the same fix:

| You write                    | rustc 1.96 says                                                                                       |
| ---------------------------- | ----------------------------------------------------------------------------------------------------- |
| `impl String { .. }`         | E0116 cannot define inherent `impl` for a type outside of the crate where the type is defined         |
| `impl str { .. }`            | E0390 cannot define inherent `impl` for primitive types ("consider using an extension trait instead") |
| `impl<I: Iterator> I { .. }` | E0118 no nominal type found for inherent implementation                                               |

An extension trait is a local trait, so the orphan rule is satisfied for any
type you implement it for. The decisions that matter (coherence3):

- **Every iterator**: `trait IterExt: Iterator` with *provided* methods and one
  blanket impl with an empty body. Methods that take `self` by value need
  `Self: Sized`, as a supertrait or per method. `itertools` puts
  `where Self: Sized` on its by-value methods and blanket-implements for
  `T: Iterator + ?Sized`, so even the unsized `dyn Iterator` implements the
  trait. A bound that one method needs (`Item: Clone`) goes on that method,
  never on the impl.
- **Every string**: implement for `str`. Auto-deref takes `String`,
  `&String`, `Box<str>` and `Rc<str>` down to `str`, and auto-ref supplies
  the `&str` that `&self` expects. An impl for `String` misses literals; one
  for `&str` never matches a `String`.
- **Scope**: the trait must be imported where it is used. Otherwise rustc says
  E0599 and helps with "trait `IterExt` which provides `pairwise` is
  implemented but not in scope; perhaps you want to import it".
- **Name collisions**: if two traits in scope have a method of that name, the
  call is E0034 "multiple applicable items in scope"; write
  `IterExt::pairwise(it)`. `Iterator` itself counts: an extension method called
  `count` collides with `Iterator::count`. When std adds an unstable method
  with your name (itertools met this with `intersperse`), stable code gets the
  `unstable_name_collisions` warning first. This is why the Cargo SemVer guide
  lists even a defaulted trait method as "possibly-breaking".
- **Sealing**: to let others *use* a trait but not *implement* it, seal it with
  a private supertrait. `47_type_level/typestate1` uses a sealed `State`
  trait, and the `deep-dive/src/api_surface.rs` lab drills which trait
  changes are semver-breaking.

## Reading the Error

| Code  | rustc says                                                                                         | What it means                                                  | Idiomatic fixes                                                                       |
| ----- | -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| E0117 | only traits defined in the current crate can be implemented for types defined outside of the crate | a foreign trait for a foreign type, no local type in the impl  | a local newtype; a local trait; a local type as the trait parameter                   |
| E0210 | type parameter `T` must be used as the type parameter for some local type                          | a foreign trait for a bare `T`, no local type anywhere         | a local wrapper, `impl<T: Shape> Display for Shown<'_, T>`, as `Path::display()` does |
| E0210 | type parameter `T` must be covered by another type when it appears before the first local type     | a bare `T` ahead of the local type, as in `From<Point> for T`  | cover `T` (`impl<T> From<Point> for Vec<T>`), or write impls for concrete types       |
| E0119 | conflicting implementations of trait `Describe` for type `Vec<u8>`                                 | two impls could apply; with the "upstream" note, only possibly | move the special case onto a local type; narrow or drop one impl                      |
| E0116 | cannot define inherent `impl` for a type outside of the crate where the type is defined            | inherent methods on a foreign type                             | an extension trait, or a newtype with its own inherent impl                           |
| E0390 | cannot define inherent `impl` for primitive types                                                  | inherent methods on `str`, `u32`, slices, ...                  | an extension trait implemented for the primitive (`impl StrExt for str`)              |
| E0118 | no nominal type found for inherent implementation                                                  | `impl<T: Bound> T { .. }` names no type at all                 | an extension trait with a blanket impl                                                |
| E0034 | multiple applicable items in scope                                                                 | two traits in scope provide a method of that name              | fully qualified syntax, `IterExt::pairwise(it)`, or rename the method                 |

## Exercise Path

1. **coherence1** — E0117: `impl Display for Vec<Point>` is an orphan. Wrap the
   `Vec` in a local newtype that prints the same way and can be `collect()`ed
   into. Then an `impl Into<(i32, i32)> for Point`, written by someone who
   thought the `From` version was an orphan too, leaves `From::from` and a
   `From`-bounded helper failing with E0277. Replace it with the `From` impl
   the orphan rule does allow.
2. **coherence2** — E0119 with "upstream crates may add a new impl of trait
   `std::fmt::Display` for type `std::vec::Vec<u8>`": a blanket
   `impl<T: Display + ?Sized> Describe for T` leaves no room for an impl on
   `Vec<u8>`, because std could make `Vec<u8>` `Display` in any release. Keep
   the blanket impl and give the byte case a type that only this crate can
   make `Display`.
3. **coherence3** — E0118 and E0390: inherent impls for "every iterator" and
   for `str` are not allowed. Turn them into extension traits: one blanket impl
   gives every iterator (the adapters included) `pairwise` and `dedup_adjacent`,
   and one impl for `str` gives every string type `is_blank` and
   `truncate_ellipsis`. Each method keeps its own bound (`Clone` for
   `pairwise`, `PartialEq` for `dedup_adjacent`), and `34_iterators/iter2`'s
   `Pairwise` becomes a method.

## Further Reading

- [Orphan rules (The Reference)](https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules) and the glossary entries for [fundamental type constructors](https://doc.rust-lang.org/reference/glossary.html#fundamental-type-constructors), local types and uncovered types
- [Implementing External Traits with the Newtype Pattern (The Book)](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html#implementing-external-traits-with-the-newtype-pattern)
- [RFC 2451: re-rebalancing coherence](https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html), the current orphan rule and "adding a blanket impl is a major change"
- [RFC 1023: rebalancing coherence](https://rust-lang.github.io/rfcs/1023-rebalancing-coherence.html), where `#[fundamental]` comes from
- [Announcing Rust 1.41.0, "Relaxed restrictions when implementing traits"](https://blog.rust-lang.org/2020/01/30/Rust-1.41.0/)
- [Little Orphan Impls (Niko Matsakis)](https://smallcultfollowing.com/babysteps/blog/2015/01/14/little-orphan-impls/)
- [RFC 1210: impl specialization](https://rust-lang.github.io/rfcs/1210-impl-specialization.html) and [Shipping specialization: a story of soundness (Aaron Turon)](https://aturon.github.io/blog/2017/07/08/lifetime-dispatch/)
- [`ToString`](https://doc.rust-lang.org/std/string/trait.ToString.html) and [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) (why you implement `From`, not `Into`), plus Clippy's [`from_over_into`](https://rust-lang.github.io/rust-clippy/master/index.html#from_over_into)
- [SemVer Compatibility (The Cargo Book)](https://doc.rust-lang.org/cargo/reference/semver.html#trait-new-default-item), "Possibly-breaking: adding a defaulted trait item"
- [Rust API Guidelines: newtypes (C-NEWTYPE)](https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype) and [sealed traits (C-SEALED)](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-sealed)
- [The `Itertools` extension trait](https://docs.rs/itertools/latest/itertools/trait.Itertools.html)
- [Rust error code index](https://doc.rust-lang.org/error_codes/error-index.html): [E0117](https://doc.rust-lang.org/error_codes/E0117.html), [E0210](https://doc.rust-lang.org/error_codes/E0210.html), [E0119](https://doc.rust-lang.org/error_codes/E0119.html), [E0116](https://doc.rust-lang.org/error_codes/E0116.html), [E0390](https://doc.rust-lang.org/error_codes/E0390.html), [E0118](https://doc.rust-lang.org/error_codes/E0118.html)
