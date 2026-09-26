# Traits & Abstraction · Builders and Typestate: From Run-Time Checks to Compile-Time Guarantees

> Part of the "Traits & Abstraction" group, after `45_sized_deref` and before
> `quizzes/quiz4` and `48_macros_deep`. "Design this API" is the most common library and backend
> prompt in a Rust interview. This module drills the two answers reviewers look
> for: a builder that is pleasant to call and validates what it builds, and a
> typestate builder that turns misuse into a compile error. Entirely **std**,
> **100% safe**, **stable** Rust, edition 2024.
>
> The `// TODO` comments name the error and the requirements but **not** the
> fix, as in an interview. Read what rustc (or clippy) says first. Press `h`
> when you want the full answer.

## Core Ideas

- **Reach for `Default` and struct update first.** `impl Default` gives every
  field a value, and `Config { port: 9090, ..Config::default() }` names only
  what changes. It needs no extra code, but every field must be visible to the
  caller (E0451 otherwise), and it can neither validate nor convert anything.
  When the defaults are not the field types' own defaults, write
  `impl Default` by hand: `#[derive(Default)]` would give port 0.
- **Use a builder for required fields and invariants.** Private fields, one
  setter per setting, and a `build()` that returns `Result<T, BuildError>`.
  Unset optional fields fall back to `T::default()`, so the defaults are
  written down once, and `BuildError` is a real error type (see
  `35_error_design`).
- **Setters take `impl Into<String>`.** Callers pass `&str`, `String`,
  `&String` or `Cow<str>` without converting. An owned `String` moves in for
  free through the identity `impl<T> From<T> for T`, and a borrowed string is
  copied once, which is the copy the setter needed anyway. The cost is one
  monomorphized copy of the setter per argument type (see
  `23_conversions/conversions2` for `From` and `Into`).
- **Collection setters take `IntoIterator`.** With
  `I: IntoIterator, I::Item: Into<String>`, arrays, `Vec`s, borrowed `Vec`s,
  iterator adapters and `Option`s all work (see `34_iterators/iter4`). A second
  call should append, not replace.
- **Consuming builders are `#[must_use]`.** A builder chain that ends in `;`
  (`Config::builder().host(h).port(9000);`) builds a builder, drops it, and
  configures nothing. With `#[must_use]` on the builder TYPE, that statement
  is an `unused_must_use` warning, whichever method produced the value. std's
  `thread::Builder` does exactly this.
- **Typestate moves the check into the type.** `RequestBuilder<S>` records its
  state in a type parameter. A transition (`url()`) consumes `self` and
  returns a different type, and `send()` exists only on
  `RequestBuilder<HasUrl>`, so it returns a plain `Request` and calling it too
  early is E0599 in the caller's code.
- **`PhantomData<S>` and unit-struct markers cost nothing.** Both are
  zero-sized, so every state of the builder has the same size, and there is no
  run-time branch or error value left. The struct needs the `PhantomData`
  field because a type parameter that no field uses is E0392. With unit-struct
  states the choice of marker makes no difference, but in general it does:
  `PhantomData<T>` is `Send` only if `T` is, `PhantomData<fn() -> T>` is always
  `Send + Sync` (both are covariant), and `PhantomData<fn(T)>` is
  contravariant. `38_variance/variance1` drills that choice.
- **Seal the state trait.** `pub trait State: sealed::Sealed {}` with
  `Sealed` declared `pub` inside a private `mod sealed`. Other crates can name
  `State` but cannot implement it, because they cannot name its supertrait.
  The set of states stays closed, and the crate can add items to `State` later
  without a breaking change. `Sealed` itself must be `pub` ("pub in private"):
  a private `trait Sealed` is E0603 "trait `Sealed` is private" wherever it is
  named outside `mod sealed` (`State`'s own supertrait list and every impl),
  and `pub(crate)` trips the `private_bounds` warning "trait `Sealed` is more
  private than the item `State`".
- **Don't derive `Default` on the generic builder.** The derive generates
  `impl<S: Default> Default for RequestBuilder<S>`. It cannot pick the state on
  its own (E0283 "type annotations needed"), and once `HasUrl: Default` it
  hands out `HasUrl` builders with no URL. Implement `Default` for the
  `NoUrl` builder only.

## Builder, `Default`, or Typestate?

| | `Default` + `..Default::default()` | Builder, `build() -> Result` | Typestate builder |
| --- | --- | --- | --- |
| Extra code | one `impl Default` | a builder type and a setter per field | also markers, a sealed trait and one `impl` per state |
| Private fields | no (E0451) | yes | yes |
| Argument conversion | no | `impl Into<..>`, `IntoIterator` | same |
| Missing required field | not expressible | `Err` at run time | E0599 at compile time |
| Invalid values (`workers: 0`) | accepted | `Err` at run time | still `Err` at run time, unless the type rules it out (`NonZeroUsize`) |
| Adding a setting later | a new `pub` field breaks callers who wrote the struct out in full | a new setter breaks nobody | a new setter breaks nobody |

`#[non_exhaustive]` on the config struct makes adding a field non-breaking, but
it also forbids struct expressions for that type outside its crate,
functional update syntax included, which pushes users of the type to a
builder anyway.

## Consuming or `&mut self` Setters?

| | Consuming: `fn port(self, ..) -> Self` | Borrowing: `fn port(&mut self, ..) -> &mut Self` |
| --- | --- | --- |
| One-expression chain | yes | yes, but `let b = Builder::new().port(1);` keeps a borrow of a temporary that dies at the `;`, so using `b` later is E0716 (see `37_borrowck_errors/borrowck2`) |
| Configuring inside an `if` | rebind: `b = b.port(1);` | call it: `b.port(1);` |
| Finishing | `build(self)` moves the fields out, no clone | `build(&self)` clones what it hands out (or `build(&mut self)` uses `mem::take`), and the builder can be reused |
| In std | `thread::Builder` | `process::Command`, `fs::OpenOptions` |

Consuming setters are the usual choice for one-shot builders that own
`String`s and `Vec`s, and typestate needs them: a transition changes the
builder's TYPE, and a `&mut self` method can only change its value.

## What the Tests Cannot Check

A graded exercise is a binary plus its tests, so it can only prove that
correct code compiles. No `#[test]` can assert that wrong code fails to
compile, so the negative half of the typestate guarantee (`send()` without
`url()` is E0599, and so is a second `url()`) is not graded as a compile error
here. It belongs to the planned `api-surface-lab`
(`deep-dive/src/api_surface.rs`, exercise `compile_fail_guarantees`), as
`compile_fail` doctests for a typestate `RequestBuilder` and a `#[must_use]`
builder, each paired with a positive doctest. The pairing matters because
stable rustdoc does not check the error code of a `compile_fail` doctest, so a
doctest that fails for the wrong reason would still pass. Until that lab
exists, try `RequestBuilder::default().send();` in `typestate1`'s `main`.

`typestate1` gets as close as a test can. Method lookup tries a type's own
(inherent) methods before trait methods, so a test that defines a local trait
with a `send()` for `RequestBuilder<NoUrl>` calls the trait's method only when
the builder has no inherent `send()`. The test checks which one ran. This
catches a `send()` (or a second `url()`) that was put on every state, but it is
not a compile-time guarantee: any trait in scope could still add the method.

What the tests do check:

- **builder1** grades `#[must_use]` through clippy. Its `impl` enables the
  pedantic `return_self_not_must_use` lint as an error, and rustlings runs
  clippy after the tests. The lint only checks public methods, which is why the
  exercise's items are `pub`.
- **typestate1** checks that `RequestBuilder::default()` picks `NoUrl` with no
  annotation (so `Default` is not generic), that `header()` works in generic
  code over any `S: State`, that every `State` is a `sealed::Sealed` (what the
  supertrait says), that the markers are zero-sized and both builders have the
  same size, that the url and headers are moved, not copied, and that
  `send()` and `url()` are missing in the wrong state (above). It cannot
  check that `mod sealed` stays private: inside one crate, a private module
  and a `pub` one look the same.

## Exercise Path

1. **builder1** — `ServerConfig` has no `Default` (E0599), `host()` takes a
   `String` (E0308 at every `&str`), there is no `tags()` (E0599), and
   `build()` has an empty body (E0308). Hand-write `Default`, take
   `impl Into<String>`, add an `IntoIterator` setter that appends, return
   `MissingHost` / `ZeroWorkers` from `build()`, then mark the builder
   `#[must_use]` so clippy lets the exercise pass.
2. **typestate1** — The tests use `RequestBuilder<NoUrl>`, `RequestBuilder<HasUrl>`
   and a sealed `State` trait that don't exist yet (E0425, E0405, E0433),
   then hit the non-generic struct (E0107) and a `send()` that still returns
   a `Result` (E0308). Add zero-sized markers behind a sealed trait and a
   `PhantomData<S>` field, and put `url()` on `NoUrl` only, `header()` on every
   state, `send() -> Request` on `HasUrl` only, and `Default` on the `NoUrl`
   builder only. Two tests fail while `send()` or a second `url()` can still be
   called in the wrong state.

Related modules: `24_ownership_model/ownership5` drives a state machine at run
time with an enum and `mem::replace`; typestate is the compile-time version,
for states that are known statically. `41_memory_layout` has the `size_of`
drills behind "ZSTs are free", and `42_coherence` covers the other side of
"who may implement this trait": the orphan rule and blanket impls.

## Further Reading

- [Rust API Guidelines: builders (C-BUILDER)](https://rust-lang.github.io/api-guidelines/type-safety.html#builders-enable-construction-of-complex-values-c-builder)
- [Rust API Guidelines: sealed traits (C-SEALED)](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed)
- [Rust API Guidelines: generic parameters (C-GENERIC)](https://rust-lang.github.io/api-guidelines/flexibility.html#functions-minimize-assumptions-about-parameters-by-using-generics-c-generic)
- [Struct update syntax (The Book)](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#creating-instances-from-other-instances-with-struct-update-syntax) and [functional update syntax (The Reference)](https://doc.rust-lang.org/reference/expressions/struct-expr.html#functional-update-syntax)
- [The `must_use` attribute](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute) and [the `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute) (The Reference)
- [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html), [`Into`](https://doc.rust-lang.org/std/convert/trait.Into.html), [`IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html) and [`PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html)
- [`std::thread::Builder`](https://doc.rust-lang.org/std/thread/struct.Builder.html) (consuming) and [`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html) (`&mut self`)
- [Method-call expressions (The Reference)](https://doc.rust-lang.org/reference/expressions/method-call-expr.html): inherent methods are found before trait methods at each receiver step
- [Clippy: `return_self_not_must_use`](https://rust-lang.github.io/rust-clippy/master/index.html#return_self_not_must_use)
- [The builder pattern (Rust Design Patterns)](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)
- [The Typestate Pattern in Rust (Cliff L. Biffle)](https://cliffle.com/blog/rust-typestate/)
- [Newtype and type-state patterns (Microsoft Rust Training)](https://microsoft.github.io/RustTraining/rust-patterns-book/ch03-the-newtype-and-type-state-patterns.html)
- Builder generators: [`derive_builder`](https://docs.rs/derive_builder/latest/derive_builder/) (checked at run time), [`typed-builder`](https://docs.rs/typed-builder/latest/typed_builder/) and [`bon`](https://docs.rs/bon/latest/bon/) (typestate, checked at compile time)
