# Module 1 · Lifetimes (In Depth): Parameters, Elision Rules, and Why Explicit Annotations Are Needed

> Building on the basics from `16_lifetimes`, this directory digs into lifetime
> **parameters**: structs that hold references, how to pick which reference a
> return value borrows from when there are several reference parameters, and
> annotations in `impl` blocks and methods. It ends with what `'static` really
> means: `T: 'static` vs `&'static T`, the hidden `+ 'static` in
> `Box<dyn Trait>`, and leaking on purpose. All **std**, **100% safe**.
>
> From `lifetimes7` on, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Press `h` when you want
> the full answer.

## Core Ideas

Lifetime annotations **do not change how long anything lives**. They simply spell
out "the survival relationships between references" for the compiler, so the
borrow checker can prove that "there will be no dangling references."

- **Structs that hold references** must carry a lifetime parameter:
  `struct Excerpt<'a> { part: &'a str }` reads as "an `Excerpt<'a>` must not
  outlive the `&'a str` it borrows."
- **Elision rules** let most functions skip writing lifetimes by hand, but they
  only kick in when the inference is unambiguous. When there are multiple
  reference parameters and the return value is a reference, the compiler cannot
  guess "whose slice is being returned," so an explicit annotation is required —
  and **not all parameters need to share the same lifetime**; annotate only the
  one that is actually borrowed.
- **Elision in methods** binds a returned reference to `&self` by default. If what
  you return is actually "longer-lived" underlying data, you must write it
  explicitly as `-> &'a str` to override this overly short default.
- **`T: 'static` is a bound on a type, not the lifetime of a value.** It means
  "`T` holds no borrow that could expire": `T` owns its data, or borrows only
  `'static` data. A `String` or an `Arc<str>` qualifies, and is still freed as
  soon as its owner drops it. `&'static T` is something else: a reference whose
  target is never freed. `thread::spawn` asks for the bound, because the new
  thread may outlive the function that started it.
- **Every trait object has a lifetime bound, and it has a default.**
  `Box<dyn Trait>` means `Box<dyn Trait + 'static>`, while `&'a dyn Trait` means
  `&'a (dyn Trait + 'a)`. To store closures that borrow locals, write the bound
  out, as in `Box<dyn Fn(&str) + 'a>` inside a `struct EventBus<'a>`. rustc's
  "add `+ 'static`" suggestion is right only when the value really has to own
  everything it uses.
- **Leaking gives you a genuine `&'static`.** `Box::leak` and `String::leak`
  are safe: the memory is simply never freed. That is fine for a bounded amount
  of once-per-process data, and a bug when it happens on every call.

## `'static` at a Glance

| You write            | It means                                                   | Satisfied by                                                         |
| -------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------- |
| `&'static str`       | a reference that stays valid until the program exits       | string literals, `static` items, leaked strings                      |
| `T: 'static`         | `T` contains no borrow shorter than `'static`              | `String`, `Arc<str>`, `u32`, `&'static str`, but not `&'a str`       |
| `Box<dyn Fn()>`      | `Box<dyn Fn() + 'static>`                                  | closures that own their captures (or borrow only `'static` data)     |
| `Box<dyn Fn() + 'a>` | the boxed closure may borrow data that lives at least `'a` | any closure whose borrows outlive `'a`, including `'static` closures |
| `&'a dyn Trait`      | `&'a (dyn Trait + 'a)`                                     | any value that outlives the borrow                                   |

The default object lifetime applies to types written in signatures, struct
fields and type aliases. A type written inside a function body, such as a `let`
annotation, gets an inferred lifetime instead. That is why the same
`Box<dyn Fn()>` can accept a borrowing closure in one place and reject it in
another.

## Leaking on Purpose

`dev/Cargo.toml` denies clippy's `mem_forget` lint with the comment "You
shouldn't leak memory while still learning Rust!". That lint only matches calls
to `std::mem::forget` (its own documentation says it cannot detect every way of
creating a leak), so it does not fire on `Box::leak`, and `lifetimes9` needs no
exception. The advice still stands for *accidental* leaks. `lifetimes9` is a
deliberate one, and in an interview you should be able to defend it:

- It is **safe**. Nothing dangles, and Rust never promised that memory gets
  freed: `mem::forget` is a safe function, and an `Rc` cycle leaks without any
  `unsafe`.
- It is **bounded**. A config value is made once, at startup. The OS reclaims
  the memory at exit, which is when it would have been freed anyway.
- The value's `Drop` **never runs**. That is fine for a `String`, and wrong for
  a `BufWriter` that still has to flush.

Leaking once per call is the wrong answer: memory then grows without limit.
That is why `37_borrowck_errors/borrowck2`, in the next module, rejects it as
a way out of E0515.

When there is exactly **one** global value, a `static` is usually a better home
than a leaked `Box`:

```rust
use std::sync::{LazyLock, OnceLock};

// Set once at run time, e.g. by `main` after reading the config file.
static CONFIG: OnceLock<String> = OnceLock::new();

// Computed on first access, from whichever thread gets there first.
static BANNER: LazyLock<String> = LazyLock::new(|| format!("pid {}", std::process::id()));

fn config() -> &'static str {
    CONFIG.get().map_or("default", String::as_str)
}
```

These do not free anything either (static items are never dropped), but the
slot has a name, can be set only once, and is safe to share between threads.
`Box::leak` is for values you create at run time (any fixed number of them,
not one per request) and pass around explicitly.

## Exercise Path

1. **lifetimes4** — A struct holding `&str`: add `<'a>` and `part: &'a str`, or
   compilation fails with "missing lifetime specifier."
2. **lifetimes5** — Multiple reference parameters where the return value borrows
   only one of them: give `prefix` and the return value the same `'a`, and give
   `separator` an unrelated lifetime. The test proves the return value is bound
   only to `prefix` by "dropping `separator` first."
3. **lifetimes6** — `impl<'a>` and methods: change the method's return type from
   the elided `&self` to an explicit `-> &'a str`, so the returned slice can
   outlive the `Parser` itself.
4. **lifetimes7** — `T: 'static` vs `&'static T`: a background logger that takes
   `msg: &'static str` rejects a `format!`-built `String` and an `Arc<str>`
   (E0308). Make it generic over any `Display + Send + 'static` message (no
   `Sync`: the message is moved, not shared), and move the message to the logger
   thread, where the tests watch it being formatted and dropped.
5. **lifetimes8** — `Box<dyn Fn(&str)>` means `+ 'static`, so an event bus cannot
   store a handler that borrows a local log (E0310). rustc's `+ 'static`
   suggestion only moves the error into the tests (E0373). Give the bus a
   lifetime parameter and box the handlers as `dyn Fn(&str) + 'a`.
6. **lifetimes9** — `&s` of a `String` parameter can never be a `&'static str`
   (E0515). Leak the buffer on purpose with `Box::leak(s.into_boxed_str())`,
   without copying the text, and hand the result to threads with no `Arc`.

## Further Reading

- [Validating References with Lifetimes (The Book)](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)
- [Lifetime elision (Reference)](https://doc.rust-lang.org/reference/lifetime-elision.html), including [default trait object lifetimes](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes)
- [lifetimekata](https://tfpk.github.io/lifetimekata/)
- [Common Rust Lifetime Misconceptions](https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md), especially 2 ("if `T: 'static` then `T` must be valid for the entire program"), 6 (boxed trait objects) and 7 (compiler suggestions)
- [`'static` (Rust by Example)](https://doc.rust-lang.org/rust-by-example/scope/lifetime/static_lifetime.html)
- [`std::thread::spawn`](https://doc.rust-lang.org/std/thread/fn.spawn.html) and [`std::thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html)
- [`Box::leak`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.leak) and [`String::leak`](https://doc.rust-lang.org/std/string/struct.String.html#method.leak)
- [`OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) and [`LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html)
- [RFC 599: default object lifetime bounds](https://rust-lang.github.io/rfcs/0599-default-object-bound.html)
- [RFC 1066: make `mem::forget` safe](https://rust-lang.github.io/rfcs/1066-safe-mem-forget.html)
