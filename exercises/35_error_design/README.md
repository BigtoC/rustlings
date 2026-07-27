# Traits & Abstraction · Error Design: Custom Errors, `?`, `From` / `TryFrom`, and the `Error` Trait

> Part of the "Traits & Abstraction" group (the trait-system prerequisites the
> async and concurrency modules assume).
> This directory is about **designing your own error types** the way real
> libraries do — how `?` really works, what makes a type "a real error," and
> when to reach for `From` versus `TryFrom`. Entirely **std**, **100% safe**,
> **stable** code.

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

## Further Reading

- [Recoverable Errors with `Result` (The Book)](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [The `?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator)
- [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
- [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) and [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
