# Traits & Abstraction · Associated Types: Outputs, Inputs, Operators and GATs

> Part of the "Traits & Abstraction" group, after `32_dispatch` through
> `35_error_design` and `42_coherence`. This directory is about the types a
> trait talks about: when a type should be an **associated type** (fixed by
> the impl) and when it should be a **type parameter** (chosen by the user),
> how the operator traits use both, and how a **generic associated type**
> (GAT) lets an iterator lend items that borrow from itself. Entirely
> **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in an interview, the `// TODO` comments name the error and the
> requirements but **not** the fix. Read what rustc says first. Press `h` when
> you want the full answer.

## Core Ideas

A trait's type parameters are **inputs** and its associated types are
**outputs**. Coherence allows at most one impl for each combination of
`Self` and the trait's parameters, and that impl then fixes every associated
type. Everything else follows from that rule:

- **Associated type**: `trait Graph { type Node; }` can be implemented once
  per type, so `G::Node` means exactly one type. Generic code names it
  instead of carrying extra parameters, `x.method()` needs no annotations, and
  bounds can talk about it: `G: Graph<Edge = u32>` or `where G::Edge: Ord`.
- **Type parameter**: `trait ConvertTo<T>` can be implemented once per `T`,
  so one type can have several impls (E0119 "conflicting implementations"
  is what you get when you try that with an associated type). The price is
  inference: when nothing at the call site picks `T`, rustc stops with E0283
  "type annotations needed" ("multiple `impl`s satisfying `Celsius:
  ConvertTo<_>` found"), or E0282 when it needs the type right away (a field
  access, say).
- **Bounds.** A bound on an associated type, `type Node: PartialEq;`, is
  implied wherever `G: Graph` holds. A bound on a trait parameter,
  `trait Graph<N: PartialEq>`, is not: every `G: Graph<N>` must restate
  `N: PartialEq` or get E0277.

| Question                          | Associated type (`trait Graph { type Node; }`)                                    | Type parameter (`trait ConvertTo<T>`)                                   |
| --------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Impls per type                    | at most one                                                                       | one per `T`                                                             |
| Who picks the type                | the impl                                                                          | the caller, or the bound that names the trait                           |
| Call sites                        | `g.neighbors(n)` just works                                                       | may need `let f: Fahrenheit = ..` or `<C as ConvertTo<K>>::convert(..)` |
| In a generic signature            | `fn f<G: Graph>(g: &G, n: &G::Node)`                                              | `fn f<T, U: ConvertTo<T>>(u: &U) -> T`                                  |
| Is a bound on it implied?         | yes                                                                               | no, restate it (E0277)                                                  |
| Examples in std                   | `Iterator::Item`, `Deref::Target`, `Future::Output`, `FromStr::Err`               | `From<T>`, `AsRef<T>`, `PartialEq<Rhs>`, `FromIterator<A>`, `Extend<A>` |

Rule of thumb: if, for a given `Self`, there is one natural answer, make it an
associated type. If a type should be able to relate to many others, make it a
parameter.

## Operators Use Both

```rust
trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}
```

- **`Rhs` is an input** with a default: `impl Add for T` means `T + T`, and
  one type may accept several right-hand sides (`i32 + i32`, `i32 + &i32`) or
  one that is not `Self` (`String + &str`, `Instant + Duration`).
  `String + String` is E0308, because std only has `String + &str`.
- **`Output` is an output**: once both operand types are known the result
  type is fixed, so `a + b + c` needs no annotations.
- **`self` by value.** `a + b` is `Add::add(a, b)` and moves non-`Copy`
  operands. That lets an owned left operand hand its buffer to the result.
  For borrowed operands, implement the trait for references too:
  `impl Add<&Vector> for &Vector`. std implements all four combinations
  (`T + T`, `T + &T`, `&T + T`, `&T + &T`) for its numeric types.
- **Separate traits for separate operators.** `+=` is `AddAssign` (E0368
  without it), unary `-` is `Neg` (E0600), and `.sum()` needs `Sum` (E0277).
  A `Sum` impl for a vector type has to pick a result for an empty iterator
  without knowing the dimension: it can panic, or start from the first
  element and return the zero-dimensional `Vector(Vec::new())` when there is
  none (which then panics with "dimension mismatch" as soon as it meets a
  real vector). `assoc2` sidesteps the question: its tests fold from
  `Vector::zeros(dim)`, which states the dimension up front.
- **The orphan rule still applies** (`42_coherence`): `impl Mul<Vector> for
  f64` is allowed because the local type `Vector` appears among the trait's
  type parameters, and that impl is what makes `2.0 * v` possible.
- **Generic numeric code.** `T: Add` only says that `T + T` is *some* type,
  `<T as Add>::Output`, so an accumulator of type `T` is E0308. Say what the
  result is: `T: Add<Output = T> + Mul<Output = T> + Copy + Default`. For
  types that are not `Copy`, bound the borrowed form with a higher-ranked
  bound (see `33_closures`): `for<'a> T: Add<&'a T, Output = T>` accepts both
  `i32` and a `Vector` that implements `Vector + &Vector`.

## Generic Associated Types and Lending Iterators

std's `Iterator` has one `type Item` for the whole iteration, so an item
cannot borrow from the `&mut self` of the `next` call that produced it. Every
item has to stay valid while the iterator goes on, which is what `collect()`
relies on. `iter_mut()` fits that model because its items never overlap.
Overlapping mutable windows do not: two of them alive at once would be two
`&mut` to the same elements. That is why std has `windows` but no
`windows_mut`.

A GAT (stable since Rust 1.65) gives the item a lifetime parameter:

```rust
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}
```

- `next` returns the item **for this borrow of `self`**. While the item is
  alive the iterator stays mutably borrowed, so a second `it.next()` before
  the first item is dead is E0499.
- `where Self: 'a` is required: without it rustc reports "missing required
  bound on `Item`" ([#87479](https://github.com/rust-lang/rust/issues/87479)).
  An impl whose item borrows, like `&'a mut [T]`, has to repeat it, or the
  item type is not known to be well-formed (E0477, E0309).
- **Why there is no `collect`.** Collecting means holding every item at once,
  and each item holds the one mutable borrow of the iterator. There is no
  `for` loop either (that needs `IntoIterator`), and none of `Iterator`'s
  adapters. You drive it with `while let Some(x) = it.next() { .. }`, and you
  may collect data *computed from* each item (`window.to_vec()`, a sum).
- **Closures hit a limitation.** A helper bounded by
  `F: for<'a> FnMut(L::Item<'a>)` (or `FnMut(Self::Item<'_>)` in a provided
  method) compiles, but calling it on an iterator that borrows a local fails
  with E0597 and the note "due to a current limitation of the type system,
  this implies a `'static` lifetime". The `for<'a>` also covers lifetimes
  longer than the iterator's own, and `where Self: 'a` then demands that the
  iterator outlive all of them. Sabrina Jewson's post (below) explains the
  problem and a workaround that replaces the GAT with a supertrait that has a
  lifetime parameter.
- **Not dyn-compatible.** `dyn LendingIterator` is E0038 ("it contains
  generic associated type `Item`"). Gating the GAT with `where Self: Sized`
  restores dyn compatibility only by making it, and `next`, unusable through
  `dyn` (see the table in `32_dispatch`).
- For `Copy` elements, std has a lending-free alternative:
  `Cell::from_mut(slice).as_slice_of_cells().windows(2)` gives overlapping
  `&[Cell<T>]` windows through a plain `Iterator`, because `Cell` allows
  mutation through shared references (`40_interior_mutability`).

## Exercise Path

1. **assoc1** — `trait Graph<N, E>` is implemented for an adjacency list and
   a grid, but the functions and tests use `G::Node` and `Graph<Edge = u32>`
   (E0107, E0220, E0576). Turn the parameters into associated types, with a
   bound on `Node` that every generic user gets for free.
2. **assoc2** — Three parts. `Celsius` needs two `ConvertTo` impls, but the
   trait has `type Target` (E0119): make the target a type parameter, then
   pay the inference cost in `report` (E0283). Add `&Vector + &Vector` and
   `Vector + &Vector` (E0369, E0308) without making `Vector` `Clone`; the
   owned form reuses its buffer, and different dimensions panic instead of
   truncating. Fix the bounds of a generic `dot<T>` so the operators produce
   `T` (E0308).
3. **gat1** — `WindowsMut` tries to be a std `Iterator` over overlapping
   `&mut [T]` windows ("lifetime may not live long enough"). Implement the
   provided GAT-based `LendingIterator` instead, and drive it with
   `while let` in `prefix_sums_in_place` (E0277 for the old `for` loop).

## Related Modules

- `34_iterators` — implementing `Iterator` and `IntoIterator`, the model that
  `gat1` has to leave behind.
- `33_closures` — higher-ranked `for<'a>` bounds, behind both
  `for<'a> T: Add<&'a T, Output = T>` and the lending-iterator closure
  limitation.
- `32_dispatch` — dyn compatibility, and why a GAT rules out `dyn`.
- `42_coherence` — the overlap check behind E0119, and the orphan rule that
  allows `impl Mul<Vector> for f64`.
- `44_trait_contracts` — what `PartialEq`, `Eq`, `Hash` and `Ord` promise,
  the kind of bound that `assoc1` puts on `Node` and `G::Edge`.

## Further Reading

- [Advanced Traits: associated types (The Book)](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html#defining-traits-with-associated-types)
- [Advanced Traits: default type parameters and operator overloading (The Book)](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html#using-default-generic-parameters-and-operator-overloading)
- [Associated types (The Reference)](https://doc.rust-lang.org/reference/items/associated-items.html#associated-types), including [required where clauses on GATs](https://doc.rust-lang.org/reference/items/associated-items.html#required-where-clauses-on-generic-associated-types)
- [RFC 195: associated items](https://rust-lang.github.io/rfcs/0195-associated-items.html), the design of input and output types
- [Module `std::ops`](https://doc.rust-lang.org/std/ops/index.html) and the [`Add` trait](https://doc.rust-lang.org/std/ops/trait.Add.html)
- [RFC 1598: generic associated types](https://rust-lang.github.io/rfcs/1598-generic_associated_types.html)
- [Generic associated types to be stable in Rust 1.65 (Rust Blog)](https://blog.rust-lang.org/2022/10/28/gats-stabilization/)
- [The Better Alternative to Lifetime GATs (Sabrina Jewson)](https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats)
- [`Cell::as_slice_of_cells`](https://doc.rust-lang.org/std/cell/struct.Cell.html#method.as_slice_of_cells)
