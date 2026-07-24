# Module 1 · Lifetimes (In Depth): Parameters, Elision Rules, and Why Explicit Annotations Are Needed

> Building on the basics from `16_lifetimes`, this directory digs into lifetime
> **parameters**: structs that hold references, how to pick which reference a
> return value borrows from when there are several reference parameters, and
> annotations in `impl` blocks and methods. All **std**, **100% safe**.

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

## Further Reading

- [Validating References with Lifetimes (The Book)](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)
- [Lifetime elision (Reference)](https://doc.rust-lang.org/reference/lifetime-elision.html)
- [lifetimekata](https://tfpk.github.io/lifetimekata/)
