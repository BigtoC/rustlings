# Iterators

> Implementing `Iterator`, building custom adapters, and understanding laziness.
> This directory looks past the *using* of iterators (already covered in the
> introductory `18_iterators`) at how the machinery actually works, using
> entirely **std**, **100% safe**, **stable** code.

## Core Ideas

Rust's iterators are a zero-cost abstraction built from two small traits:

- **`Iterator`** has exactly ONE required method: `fn next(&mut self) ->
  Option<Self::Item>`. `Some(x)` means "here is the next element", `None` means
  "I am exhausted". Every other method - `take`, `map`, `filter`, `sum`,
  `collect`, and dozens more - is a **default method** implemented in terms of
  `next`. Implement `next` and you get all of them for free.
- **Adapters own their inner iterator.** `map`, `filter`, `zip`, ... are just
  structs that hold the iterator they wrap and compute their own `next` by
  pulling from it. Chaining them nests the structs
  (`v.iter().map(..).filter(..)` is a `Filter<Map<..>>`), and a custom adapter is
  nothing more than your own such struct.
- **Iterators are lazy.** Building an adapter does **zero** work; nothing runs
  until an **eager consumer** (`for_each`, `collect`, `count`, `sum`, a `for`
  loop, ...) starts calling `next` to pull values through. A `map` whose result
  is never consumed is a no-op - a real, silent bug that the `#[must_use]` std
  puts on its adapters, via rustc's `unused_must_use` lint, is designed to catch.
- **`IntoIterator` is what `for` loops require** - not `Iterator` directly.
  `for x in coll` desugars to `let mut it = IntoIterator::into_iter(coll); while
  let Some(x) = it.next() { .. }`. Implementing it is what makes your own type
  loopable.

## Exercise Path

1. **iter1** — Implement `Iterator` for an unbounded `Fibonacci` by writing only
   `next`. Then watch `take`, `map`, and `sum` - all default methods - work on
   top of it without any extra code. Nothing stops the sequence except the range
   of `u64`, and `checked_add` is how you report that honestly instead of
   panicking on overflow.
2. **iter2** — Write a custom lazy adapter, `Pairwise`, that owns an inner
   iterator and yields overlapping neighbour pairs. Uses `?` on `Option` inside
   `next` to report exhaustion.
3. **iter3** — Feel laziness bite: a `map` built purely for a side effect is
   never consumed, so the closure never runs and the count stays `0`. Fix it by
   driving the iterator with an eager consumer (`for_each`).
4. **iter4** — Implement `IntoIterator` (the consuming, by-value form) for a
   `Grid` so it works in a `for` loop, deferring to `Vec`'s own
   `std::vec::IntoIter`.

## Further Reading

- [The `Iterator` trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
- [Module `std::iter` (laziness and how iterators work)](https://doc.rust-lang.org/std/iter/index.html)
- [The `IntoIterator` trait](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html)
- [Processing a Series of Items with Iterators (The Book)](https://doc.rust-lang.org/book/ch13-02-iterators.html)
