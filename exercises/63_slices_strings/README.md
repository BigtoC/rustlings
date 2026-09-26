# Module 5 · Slices and Strings: Two Pointers, Unicode Windows, Intervals, Binary Search

> The live-coding drills of Module 5, after `31_debugging`. Four problems
> that every interview loop reuses (two pointers, a sliding window, interval
> merging, binary search), each with the Rust-specific way it breaks: `usize`
> underflow, integer overflow, slicing a `&str` in the middle of a character,
> and `(lo + hi) / 2`. The module also covers what `09_strings` leaves out:
> bytes vs chars, char boundaries and `char_indices`. All **std**, **100%
> safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements but **not** the fix, as in an
> interview. Press `h` when you want the full answer.

## Core Ideas

- **Indices are `usize`, and a `usize` is never negative.** On an empty slice
  `len() - 1` panics with "attempt to subtract with overflow" in debug builds
  and tests, and wraps to `usize::MAX` in release. Handle the empty case
  first (`len().checked_sub(1)?`), or use half-open ranges: `lo..hi` is empty
  when `lo == hi`, its size is `hi - lo`, and no `- 1` ever appears.
- **Overflow checks are on in debug builds and off in release.** A sum that
  overflows panics in your tests and silently wraps in production. Widen
  before you add (`i64::from(a) + i64::from(b)` is exact for any two `i32`s)
  or use an operation that cannot overflow (`lo.midpoint(hi)`).
  `checked_*`, `saturating_*` and `wrapping_*` say what overflow *means*;
  they are not a way to make the panic go away (`31_debugging/debugging2`).
- **A `&str` is UTF-8 bytes.** `s.len()` counts bytes. `&s[a..b]` takes
  byte offsets and panics unless both fall on a char boundary, and `s[i]`
  does not compile at all (E0277). Take offsets from `char_indices()`, plus
  `c.len_utf8()` for the end of a char. To check or repair an offset, use
  `s.get(a..b)`, `is_char_boundary`, or `floor_char_boundary` /
  `ceil_char_boundary` (stable since Rust 1.91) when you truncate.
- **A `char` is a Unicode scalar value, not a letter on screen.** "é" is one
  `char` (U+00E9) or two (`e` followed by the combining U+0301), depending
  on how it was written. Splitting text into user-visible characters
  (grapheme clusters) needs the `unicode-segmentation` crate.
- **Return borrows, not copies.** When the answer is a piece of the input,
  return a `&str` or `&[T]` into it. Elision ties its lifetime to the input
  (see `25_lifetimes_deep`), and nothing is allocated.
- **Search for a boundary, not a match.** `partition_point` is C++'s
  `lower_bound` / `upper_bound`. `binary_search` returns *some* index of a
  matching element: with duplicates it may return any of them (on Rust 1.96,
  `[2; 8].binary_search(&2)` is `Ok(7)`, the last one). `binary_search`
  needs a slice sorted by a total order (`44_trait_contracts/contracts3`);
  `partition_point` needs a predicate that is true for a prefix, then false.

## Translating the Idioms

| You know it as                   | In Rust                                               | Watch out for                                                        |
| -------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------- |
| `lower_bound(x)`, `bisect_left`  | `v.partition_point(\|&e\| e < x)`                     | the predicate must be true for a prefix, then false                  |
| `upper_bound(x)`, `bisect_right` | `v.partition_point(\|&e\| e <= x)`                    | same                                                                 |
| Java's `Arrays.binarySearch`     | `v.binary_search(&x)`: `Ok(found)`, `Err(insert_at)`  | any one of several duplicates                                        |
| `mid = (lo + hi) / 2`            | `lo.midpoint(hi)` or `lo + (hi - lo) / 2`             | signed `midpoint` rounds toward zero: `(-3i32).midpoint(-2)` is `-2` |
| `s[i]` for a character           | `s.chars().nth(i)` (O(n)), `s.as_bytes()[i]` (a byte) | E0277 "the type `str` cannot be indexed by `{integer}`"              |
| `s.substring(a, b)`, `s[a:b]`    | `&s[a..b]` with offsets from `char_indices()`         | "byte index 1 is not a char boundary"                                |
| `s.length()`                     | `s.len()` (bytes) or `s.chars().count()` (O(n))       | they differ for anything but ASCII                                   |
| update `v.back()` if it exists   | `if let Some(last) = v.last_mut() && ..`              | let chains need edition 2024 (stable since Rust 1.88)                |
| `sort`                           | `sort_unstable_by_key` (in place, never allocates)    | the stable `sort_by_key` allocates a buffer for all but short slices |

## Exercise Path

1. **window1** — Two pointers. `two_sum_sorted` computes `nums.len() - 1`,
   which underflows on an empty slice, and adds two `i32`s, which overflows
   near `i32::MAX` and `i32::MIN`. `three_sum` is empty (E0308). Handle the
   empty case, widen the sums to `i64` (`checked_add` and `saturating_add`
   give wrong answers), and write three-sum as sort plus two pointers,
   skipping duplicates as you go.
2. **window2** — The longest substring without repeating characters,
   returned as a slice of the input. The byte-based `[usize; 256]` version
   panics on "éã" with "not a char boundary". Walk `char_indices()`, keep
   each char's last position in a `HashMap`, move the window's start with a
   let chain, and count the length in chars.
3. **window3** — Merge intervals (E0308, an empty body). Sort by start with
   `sort_unstable_by_key`, then either extend `out.last_mut()` or push, in
   one let chain. The intervals are closed, so touching ones merge, and an
   interval inside another one must not shrink it.
4. **window4** — Binary search. `count_in_range` (E0308) takes two
   `partition_point` calls and must return 0, not panic, for a reversed
   range. `first_true` computes `(lo + hi) / 2`, which overflows near
   `u64::MAX`: use `midpoint`. `min_ship_capacity` (E0308) is binary search
   on the answer, between the heaviest package and the total weight.

## Where This Leads

- `66_checked_math` keeps going with overflow: a 256-bit `a * b / c`,
  rounding direction and fixed-point decimals.
- `67_code_review/review1` hides a `&memo[..16]` that cuts a character in
  half among other planted bugs, with no TODO pointing at it.
- Planned (Tier 2 in `deep-dive/ROADMAP.md`): `64_parsing` (`parse1..3`),
  a lexer whose borrowed tokens are slices cut at byte offsets, and
  `61_trees` / `62_graphs`, more live-coding problems in the same style.

## Further Reading

- [`str`](https://doc.rust-lang.org/std/primitive.str.html): UTF-8, byte offsets and char boundaries, with [`char_indices`](https://doc.rust-lang.org/std/primitive.str.html#method.char_indices), [`is_char_boundary`](https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary) and [`floor_char_boundary`](https://doc.rust-lang.org/std/primitive.str.html#method.floor_char_boundary)
- [Storing UTF-8 Encoded Text with Strings (The Book)](https://doc.rust-lang.org/book/ch08-02-strings.html), especially "Indexing into Strings" and "Slicing Strings"
- [`slice::partition_point`](https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point) and [`slice::binary_search`](https://doc.rust-lang.org/std/primitive.slice.html#method.binary_search)
- [`u64::midpoint`](https://doc.rust-lang.org/std/primitive.u64.html#method.midpoint) and [`i32::midpoint`](https://doc.rust-lang.org/std/primitive.i32.html#method.midpoint) (note the rounding)
- [`slice::sort_unstable_by_key`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_unstable_by_key) and [`Vec::dedup_by`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.dedup_by)
- [Chains of conditions (The Reference)](https://doc.rust-lang.org/reference/expressions/if-expr.html#chains-of-conditions) and [Announcing Rust 1.88.0](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/) (let chains)
- [Nearly All Binary Searches and Mergesorts are Broken (Joshua Bloch, 2006)](https://research.google/blog/extra-extra-read-all-about-it-nearly-all-binary-searches-and-mergesorts-are-broken/)
- [The Absolute Minimum Every Software Developer Must Know About Unicode in 2023 (Nikita Prokopov)](https://tonsky.me/blog/unicode/)
- [`unicode-segmentation`](https://docs.rs/unicode-segmentation) (grapheme clusters, outside std)
- The LeetCode originals: [167](https://leetcode.com/problems/two-sum-ii-input-array-is-sorted/), [15](https://leetcode.com/problems/3sum/), [3](https://leetcode.com/problems/longest-substring-without-repeating-characters/), [56](https://leetcode.com/problems/merge-intervals/) and [1011](https://leetcode.com/problems/capacity-to-ship-packages-within-d-days/)
