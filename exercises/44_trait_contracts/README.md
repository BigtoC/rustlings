# Traits & Abstraction · Trait Contracts: `Eq`, `Hash`, `Ord` and Floats

> Part of the "Traits & Abstraction" group, after `42_coherence` and
> `43_assoc_types`. The comparison traits come with rules that rustc never
> checks: `Hash` must agree with `Eq`, `Ord` must agree with `Eq` and with
> `PartialOrd`, and `f64` keeps none of those rules. Break one and the code
> still compiles, and then a `HashMap`, `BTreeSet` or `BinaryHeap` quietly
> gives wrong answers. Entirely **std**, **100% safe**, **stable** Rust,
> edition 2024.
>
> As in an interview, the `// TODO` comments name the error (or the failing
> test) and the requirements but **not** the fix. Read what rustc or the test
> says first. Press `h` when you want the full answer.

## Core Ideas

Each comparison trait adds a promise, and each promise has customers:

| Trait        | Promise                                                                              | Who relies on it                                                   |
| ------------ | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `PartialEq`  | `==` is symmetric and transitive                                                     | `==`, `contains`, `dedup`                                          |
| `Eq`         | also reflexive: `a == a` for every value                                             | `HashMap` / `HashSet` keys, `Ord`                                  |
| `Hash`       | `a == b` implies `hash(a) == hash(b)`, and the bytes fed are prefix-free             | `HashMap`, `HashSet`                                               |
| `PartialOrd` | `partial_cmp` agrees with `==`; with `Ord`, `partial_cmp(a, b) == Some(cmp(a, b))`   | `<`, `<=`, `>`, `>=`, and (today) `slice::sort` and `BinaryHeap`   |
| `Ord`        | a total order: exactly one of `<`, `==`, `>` holds, transitively                     | `BTreeMap` / `BTreeSet`, `Iterator::max`, `binary_search`, `sort`  |

- **The converse of the `Hash` rule is not required.** Unequal keys may share
  a hash; a collision costs time, not correctness. A hash that looks at
  something `==` ignores is the bug.
- **Violations are logic errors, not undefined behavior.** The docs say the
  resulting behavior is unspecified but stays inside the collection that saw
  it: panics, wrong results, aborts, leaks or non-termination, never memory
  corruption. The traits are safe to implement, so `unsafe` code must not rely
  on them being correct.
- **Nobody promises which method a collection calls.** Today `slice::sort`
  compares with `<` and `BinaryHeap` with `<=`, while `BTreeSet` and
  `Iterator::max` call `Ord::cmp`. A `BTreeSet` built with `collect` does
  both: it sorts its input with `sort`, then searches with `cmp`. When
  `PartialOrd` and `Ord` disagree, one type sorts one way in a `Vec` and
  another way in a set, and a collected set can fail to find its own elements.
- **Contract violations are nondeterministic.** With the default
  `RandomState`, a key hashed inconsistently may be found on one run and missed
  on the next. Tests should compare hashes from a fixed hasher
  (`DefaultHasher::new()`), and check consistency with `Eq` rather than
  hard-coded hash values (the `Hash` docs, "Portability").

## Derive Together or Write Together

- `#[derive(PartialEq, Eq, Hash)]` hashes and compares the same fields in the
  same way, so the derives agree. The risk is in mixing: a hand-written
  `PartialEq` next to a derived `Hash` (or the reverse) is correct only if you
  check by hand that the hash never looks at anything `==` ignores.
- A derived `PartialOrd` / `Ord` is **lexicographic in field declaration
  order**: the first field decides and later fields only break ties. Enums
  order by discriminant (variant order unless you assign values), then by
  fields. Reordering struct fields changes how the type sorts.
- When you write the order by hand, put the logic in `Ord::cmp` and make
  `partial_cmp` return `Some(self.cmp(other))`. Compare every field that `==`
  compares, so `cmp` returns `Equal` exactly when `==` is true. Build tie-breaks
  with `Ordering::then_with`, or compare tuples of the fields, wrapping a field
  that sorts the other way in `std::cmp::Reverse`.
- A hand-written `Hash` feeds exactly the information `==` compares,
  normalized the same way, and ends each variable-length field with a marker
  that cannot occur inside it (like the `0xff` byte `str` appends; a zero byte
  would not do, since `'\0'` is a valid `char`) or starts it with its length.

rustc checks none of this, but clippy catches the common shapes:

| Clippy lint                      | Default | Fires on                                                                 |
| -------------------------------- | ------- | ------------------------------------------------------------------------ |
| `derived_hash_with_manual_eq`    | deny    | `#[derive(Hash)]` next to a hand-written `PartialEq`                     |
| `derive_ord_xor_partial_ord`     | deny    | a derived `PartialOrd` with a hand-written `Ord`, or the reverse         |
| `non_canonical_partial_ord_impl` | warn    | a `partial_cmp` that is not `Some(self.cmp(other))` on a type with `Ord` |

Nothing flags an `Ord` that is **coarser** than `==` (it calls two different
values `Equal`). A `BTreeSet` treats `Equal` as "already present": `insert`
drops the new value and `contains` finds values that were never inserted.

## Floats Have No Total Order

`f64` implements `PartialEq` and `PartialOrd` only:

| Expression              | Value   | Why it matters                                                          |
| ----------------------- | ------- | ----------------------------------------------------------------------- |
| `NaN == NaN`            | `false` | `Eq` requires `a == a` for every value                                  |
| `NaN.partial_cmp(&1.0)` | `None`  | `Ord` must always answer                                                |
| `-0.0 == 0.0`           | `true`  | equal, but different bits: hashing the bits would break the `Hash` rule |
| `f64::max(NaN, 1.0)`    | `1.0`   | no contract broken, but the NaN silently disappears                     |

So `v.sort()`, `BinaryHeap<f64>`, `BTreeSet<f64>` and `iter().max()` on floats
are E0277 "the trait bound `f64: Ord` is not satisfied". The fixes:

- **Sort** with `v.sort_by(f64::total_cmp)`. `total_cmp` (stable since 1.62)
  is IEEE 754's `totalOrder`: `-NaN < -inf < ... < -0.0 < 0.0 < ... < inf <
  NaN`. Two values are equal under it exactly when their bits are. Avoid
  `partial_cmp(..).unwrap()` (panics on NaN) and `.unwrap_or(Equal)` (not
  transitive, so the result is unspecified and `sort` may panic).
- **Pick a max** with `iter().copied().max_by(f64::total_cmp)`.
- **Use floats as keys** through a newtype whose `Eq`, `Ord` and `Hash` all
  follow `total_cmp` and `to_bits`. It has to be a newtype: `impl Ord for f64`
  is E0117 (the orphan rule, see `42_coherence`). The `ordered-float` crate's
  `OrderedFloat` and `NotNan` are the production versions, with slightly
  different rules for NaN and zeros.
- **Mind the NaN sign.** `f64::NAN`'s bit pattern is not guaranteed, and on
  x86 `0.0 / 0.0` computed at run time gives a *negative* NaN, which
  `total_cmp` sorts first. Build test NaNs with `f64::from_bits`.

## Min-Heaps

`BinaryHeap` is a **max-heap**: `pop` returns the greatest element. For the
smallest first (Dijkstra, k-way merge, scheduling by deadline), wrap the items
in `std::cmp::Reverse`, whose `Ord` is reversed. The alternative in the
`BinaryHeap` docs' Dijkstra example flips the comparison inside `cmp`
(`other.cost.cmp(&self.cost)`) and then compares `position` on a tie, "to
make implementations of `PartialEq` and `Ord` consistent". To keep only the
`k` smallest of `n` values, keep a max-heap of size `k` and evict its root:
O(n log k).

## Exercise Path

1. **contracts1** — A case-insensitive `Header(String)` has a hand-written
   `PartialEq` but a derived `Hash`, so `"Content-Type"` and `"content-type"`
   are equal and hash differently: lookups miss and a `HashSet` keeps both
   (clippy's `derived_hash_with_manual_eq` rejects it too). Write `Hash` by
   hand: feed exactly what `==` compares, prefix-free, without allocating and
   without changing how the name is stored.
2. **contracts2** — A `Job` has a derived `PartialOrd` (fields in the wrong
   order, so the heap runs jobs by name) and a hand-written `Ord` that compares
   only `priority` (so a `BTreeSet` drops jobs). Write one total order,
   priority first, then FIFO, then name, so that it agrees with the derived
   `Eq`, and make `PartialOrd` defer to it.
3. **contracts3** — Floats (E0277 `f64: Ord`): sort with `total_cmp`, build a
   `TotalF64` newtype that is a well-behaved `Eq + Ord + Hash` key, keep the
   `k` smallest values in a bounded max-heap, and turn the heap in a Dijkstra
   search into a min-heap without negating costs.

## Related Modules

- `42_coherence`: the orphan rule, and why a float key must be a newtype.
- `43_assoc_types`: operator traits; `PartialEq` and `PartialOrd` are the
  traits behind `==` and `<`.
- `45_sized_deref/borrow1` (the next module): the `Borrow` contract. A
  borrowed form must hash, compare and order exactly like the owned key, which
  is why a case-insensitive key like `Header` must not implement
  `Borrow<str>`.
- `27_data_structures/hashtable1` (later in the path): a hash map built by
  hand on top of `DefaultHasher`.
- `11_hashmaps`: derived `Hash` and `Eq` for map keys.

## Further Reading

- [`Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html): the sections ["`Hash` and `Eq`"](https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq), ["Prefix collisions"](https://doc.rust-lang.org/std/hash/trait.Hash.html#prefix-collisions) and ["Portability"](https://doc.rust-lang.org/std/hash/trait.Hash.html#portability)
- [`Ord`](https://doc.rust-lang.org/std/cmp/trait.Ord.html), including ["How can I implement `Ord`?"](https://doc.rust-lang.org/std/cmp/trait.Ord.html#how-can-i-implement-ord) and ["Examples of incorrect `Ord` implementations"](https://doc.rust-lang.org/std/cmp/trait.Ord.html#examples-of-incorrect-ord-implementations)
- [`PartialOrd`](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html) and [`Eq`](https://doc.rust-lang.org/std/cmp/trait.Eq.html)
- [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html) and [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) (the logic-error paragraphs)
- [`BinaryHeap`](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html#min-heap) (min-heap with `Reverse`) and the [Dijkstra example](https://doc.rust-lang.org/std/collections/binary_heap/index.html)
- [`f64::total_cmp`](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp) and [`slice::sort_by`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by)
- [`std::cmp::Reverse`](https://doc.rust-lang.org/std/cmp/struct.Reverse.html) and [`Ordering::then_with`](https://doc.rust-lang.org/std/cmp/enum.Ordering.html#method.then_with)
- Clippy: [`derived_hash_with_manual_eq`](https://rust-lang.github.io/rust-clippy/master/index.html#derived_hash_with_manual_eq), [`derive_ord_xor_partial_ord`](https://rust-lang.github.io/rust-clippy/master/index.html#derive_ord_xor_partial_ord), [`non_canonical_partial_ord_impl`](https://rust-lang.github.io/rust-clippy/master/index.html#non_canonical_partial_ord_impl)
- [Derivable traits (The Book, Appendix C)](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)
- [Rust API Guidelines: C-COMMON-TRAITS](https://rust-lang.github.io/api-guidelines/interoperability.html#types-eagerly-implement-common-traits-c-common-traits)
- [IEEE 754 total-ordering predicate (Wikipedia)](https://en.wikipedia.org/wiki/IEEE_754#Total-ordering_predicate)
- [The `ordered-float` crate](https://docs.rs/ordered-float/latest/ordered_float/)
