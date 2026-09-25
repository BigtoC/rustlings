# Traits & Abstraction · Error Design: Custom Errors, `?`, `From` / `TryFrom`, the `Error` Trait, and Application Reports

> Part of the "Traits & Abstraction" group (the trait-system prerequisites the
> async and concurrency modules assume).
> This directory is about **designing your own error types** the way real
> libraries do — how `?` really works, what makes a type "a real error," and
> when to reach for `From` versus `TryFrom`. Parts 4-6 then switch to the
> **application** side: context chains, errors that cross threads,
> downcasting, and why an anyhow-style report cannot itself be an `Error`.
> Entirely **std**, **100% safe**, **stable** code.
>
> From part 4 on, the `// TODO` comments name the error and the requirements
> but **not** the fix, as in an interview. Read what rustc says first. Press
> `h` when you want the full answer.

## Core Ideas

Rust has no exceptions. Fallible operations return `Result<T, E>`, and you get
to design `E`. A good error type is ergonomic to propagate, informative to
display, and safe to build.

- **`?` is `From`-powered.** `expr?` on an `Err(e)` desugars to
  `return Err(From::from(e))`. A single `impl From<LowLevel> for MyError` lets
  `?` convert a foreign error into your domain error everywhere — no
  `.map_err(...)` boilerplate.
- **A "real error" implements `std::error::Error`.** Its supertraits are
  `Debug` + `Display`, and it may override `source()` to return the underlying
  cause. `Display` is written by hand (never derived); `source()` returning
  `Some(&cause)` builds the chain that `Box<dyn Error>` and "caused by"
  reporters walk.
- **`From` is infallible; `TryFrom` is fallible.** Use `From` for conversions
  that cannot fail and lose nothing (you get `Into` for free). Use `TryFrom`
  when the conversion must be validated — it carries an associated `Error` type
  and returns `Result`, so an invalid value can never be constructed (and you
  get `TryInto` for free).

## Libraries and Applications (parts 4-6)

- **Libraries return typed errors; applications return reports.** A library's
  callers branch on the cause, so it exposes a matchable enum (often
  `#[non_exhaustive]`, so a new variant is not a breaking change). An
  application mostly adds context, logs and exits, so it uses one opaque
  report type that can carry any error (`anyhow::Error`, `eyre::Report`).
- **Wrap, don't stringify.** Context goes on top of the cause, not in its
  place: each layer's `source()` is the layer below, so the whole chain can be
  printed and the root cause is still a real value. `format!("...: {e}")`
  keeps the text and throws the value away.
- **`Box<dyn Error + Send + Sync + 'static>`.** A trait object has only the
  auto traits written in its type, so `Box<dyn Error>` can't leave its thread
  even when the value inside could. `Send` lets it move, `Sync` lets threads
  share `&` to it, `'static` means it owns everything (and can be downcast).
  std's `From` impls, which `?` and `.into()` use, exist only for
  `Box<dyn Error>` and `Box<dyn Error + Send + Sync>`.
- **Downcasting gets the type back.** `downcast_ref::<T>()`, `is::<T>()` and
  (by value) `downcast::<T>()` compare `TypeId`s. Branch on types, never on
  messages.
- **Coherence decides what a report can be.** The blanket
  `impl<E: Error + ..> From<E> for Report` that lets `?` accept any error
  overlaps core's `impl<T> From<T> for T` as soon as `Report: Error` (E0119).
  anyhow keeps the `From` and gives up `Error`, then restores access with
  `AsRef` / `Deref` and `From<Report> for Box<dyn Error + Send + Sync>`.
  `Box<dyn Error>` does not implement `Error` for the same reason.

The errors you will meet in parts 4-6:

| Code  | rustc says                                                            | What it means                                                         | Idiomatic fix                                                  |
| ----- | --------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------- |
| E0599 | no method named `context` found for enum `Result<T, E>`               | an extension trait's method exists only where an impl covers the type | implement the trait for `Result<T, E>` (a blanket impl)        |
| E0277 | `dyn std::error::Error` cannot be sent between threads safely         | the boxed error type has no `Send` in it                              | `Box<dyn Error + Send + Sync + 'static>`                       |
| E0277 | `dyn std::error::Error` cannot be shared between threads safely       | the boxed error type has no `Sync` in it                              | the same alias; `Send` alone is not enough                     |
| E0119 | conflicting implementations of trait `From<Report>` for type `Report` | a blanket `From<E: Error>` meets core's `From<T> for T`               | don't implement `Error` for the report; add `AsRef` and `From` |

## Exercise Path

1. **err1** — `From` powers `?`: a `From<ParseIntError> for ConfigError` impl
   has an empty body, so it returns `()` instead of `Self` and won't compile.
   Return `ConfigError::Parse(e)` and watch `?` bridge the low-level error into
   your domain error inside `parse_port`.
2. **err2** — `Display` + `Error` + `source()`: implement the hand-written
   `Display` (`write!` the message, no trailing semicolon) and return
   `Some(&self.source)` from `source()` so the cause chain is walkable. Empty
   bodies return `()` and mismatch the required `fmt::Result` / `Option<...>`.
3. **err3** — `TryFrom` for validated construction: implement
   `TryFrom<i32> for Percentage`, accepting only `0..=100` and returning the
   provided `OutOfRange` error otherwise, so an out-of-range `Percentage` cannot
   exist. Note that the associated `Error` is a real error type (`Debug` +
   `Display` + `Error`, as in part 2) rather than a `String`. The free `TryInto`
   falls out of the same impl.
4. **err4** — Context chains (E0599, E0308): `Report { msg, source }` and the
   `Context<T>` extension trait have no impls, so every `.context(...)` is
   E0599, and `chain()` is an empty body. Add a blanket impl for any
   `Result<T, E>` with `E: Error + Send + Sync + 'static` and a second impl for
   `Result<T, Report>` (legal only because `Report` is not an `Error`), then walk
   `source()` to list every message, outermost first. The root cause must still
   downcast to the original `ParseIntError`.
5. **err5** — Errors that cross threads (E0277): a worker returns
   `Result<u32, Box<dyn Error>>` through `thread::spawn`, and a test reads one
   error from several threads. Put `Send + Sync + 'static` into the alias, then
   classify the joined error by its type with `downcast_ref` / `is`. A
   plain-text error with the same message must not match.
6. **err6** — Why `anyhow::Error` isn't an `Error` (E0119): a `Report` that
   implements `Error` plus a blanket `From<E: Error + ..>` conflicts with core's
   `From<T> for T`. Keep the blanket `From`, drop `impl Error`, and restore what
   callers need with `AsRef<dyn Error + Send + Sync>` and a
   `From<Report> for BoxError` that hands over the box the report already owns.

## Further Reading

- [Recoverable Errors with `Result` (The Book)](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [The `?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator)
- [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html), including [`downcast_ref` on `dyn Error`](https://doc.rust-lang.org/std/error/trait.Error.html#method.downcast_ref)
- [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) and [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [`anyhow::Error`](https://docs.rs/anyhow/latest/anyhow/struct.Error.html) (note the `AsRef`, `Deref` and `From<Error> for Box<dyn Error ...>` impls, and no `impl Error`) and [`anyhow::Context`](https://docs.rs/anyhow/latest/anyhow/trait.Context.html)
- [`thiserror`](https://docs.rs/thiserror/latest/thiserror/), the derive most libraries use for their typed errors
- [`io::Error::other`](https://doc.rust-lang.org/std/io/struct.Error.html#method.other), which takes any `Into<Box<dyn Error + Send + Sync>>`
- [Trait implementation coherence (The Reference)](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)
- [Default trait object lifetimes (The Reference)](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes)
- [The `non_exhaustive` attribute (The Reference)](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute)
