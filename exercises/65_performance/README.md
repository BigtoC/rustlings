# Module 5 · Performance: Buffer Reuse, Borrowed Parameters and Returns, Bulk Slice Operations

> The performance round of Module 5, after `64_parsing` and before
> `66_checked_math`. "Why is this slow, and how do you make it stop
> allocating?" is the Rust performance question you are most likely to get,
> and the answer is usually about the allocator, not about a faster algorithm.
> The three exercises cover reusing a buffer the caller owns, borrowing
> instead of copying, and bulk slice operations with their length contracts.
> Each one is checked by behavior: heap pointers, capacities and panics, with
> Clippy as a second grader. All **std**, **100% safe**, **stable** Rust,
> edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error, the
> failing test or the Clippy lint, and the requirements, but **not** the fix.
> Press `h` when you want the full answer.

## Core Ideas

- **Count allocations first.** An allocation is a call into the allocator,
  usually a copy of the data, and a `free` later. A `String` that starts
  empty and grows line by line reallocates each time it fills up. Every
  `format!`, `to_string()`, `to_vec()` and `clone()` allocates. In a hot loop
  these calls often cost more than the work itself.
- **Let the caller own the buffer.** Take `&mut String` or `&mut Vec<T>`,
  `clear()` it and fill it. `clear()` keeps the capacity, and a `Vec` never
  shrinks on its own, so after the first call nothing allocates. std's own
  example is `BufRead::read_line(&mut buf)`, which appends to a buffer you
  reuse, while `lines()` hands you a new `String` for every line.
- **Format into the buffer.** `String` implements `std::fmt::Write`, so
  `write!(s, ..)` and `writeln!(s, ..)` format directly into it.
  `s.push_str(&format!(..))` builds a temporary `String` first and throws it
  away. `String`'s `write_str` never fails, so the `fmt::Result` can only be
  an error if a `Display` impl returns one.
- **Borrow what you only read.** `&str` and `&[T]` accept literals,
  sub-slices, arrays, `Box<str>` and (by deref coercion) `&String` and
  `&Vec<T>`. For a slice of strings, make the element type generic:
  `&[impl AsRef<str>]` takes both `Vec<String>` and `Vec<&str>`.
- **Return borrows into the input.** When the result is a piece of the
  input, return `&str`, `&[T]` or `Vec<&str>`. Elision ties the result to
  the input, and nothing is copied. Take ownership only of what you keep
  (`impl Into<String>`, `47_type_level/builder1`), and return a `Cow` when you
  only sometimes need to allocate (`45_sized_deref/cow1`).
- **Use the bulk operation, and keep its contract.** `copy_from_slice` is a
  `memcpy` and panics unless the lengths are equal. `extend_from_slice` is a
  `reserve` plus a `memcpy` for `Copy` elements. `zip` never goes out of
  bounds, because it stops at the shorter input without telling you. Decide
  what unequal lengths mean and `assert!` it before the loop. The same assert
  lets LLVM remove the bounds checks from an index loop; confirm that in the
  assembly instead of assuming it.
- **Size once when you know the total.** `Vec::with_capacity(n)` or
  `reserve(n)`, then fill. `collect()` and `extend` pre-size from the
  iterator's `size_hint`. That is exact for `map` over a slice, but after
  `filter` the lower bound is 0 and after `flat_map` it is too small, so the
  vector grows step by step.

## Where the Allocations Hide

| Pattern | What it costs | Allocation-aware version |
| --- | --- | --- |
| `fn render(..) -> String` called in a loop | a new buffer per call, grown from empty every time | `fn render_into(.., out: &mut String)` with `out.clear()` (perf1) |
| `s.push_str(&format!(..))`, `s += &format!(..)` | a temporary `String` per call | `write!(s, ..)` with `std::fmt::Write` in scope (perf1) |
| `for line in reader.lines()` | a new `String` per line | `read_line(&mut buf)` into one cleared buffer |
| `fn f(s: &String)`, `fn f(v: &Vec<T>)` | callers with a literal, a sub-slice or an array must build a `String` or `Vec` first | `&str`, `&[T]` (perf2) |
| `fn f(words: &[String])` | a caller with `&str`s must allocate a `String` for each | `&[impl AsRef<str>]` (perf2) |
| `-> Vec<String>` of pieces of the input | one allocation and one copy per piece | `-> Vec<&str>` into the input (perf2) |
| `fn set(&mut self, s: &str)` that stores `s.to_string()` | a copy even when the caller owned a `String` it no longer needs | `impl Into<String>` (`47_type_level/builder1`) |
| `.clone()` to get past the borrow checker | a full copy | move or borrow instead (`24_ownership_model`) |
| `push` in a loop, `collect()` after `filter` | reallocation each time the buffer fills, up to half of it unused | `reserve` the total, then `extend_from_slice` (perf3) |
| an element-by-element copy loop | a bounds check per element | `copy_from_slice` (perf3) |

## What `Vec` Promises About Capacity

The pointer and capacity checks in this module lean on the first two rows,
and on `reserve` doing nothing when the capacity already suffices (also in
the docs). Growth is only ever checked against a bound.

| Operation | Guaranteed by the docs | Not guaranteed |
| --- | --- | --- |
| `clear()`, `truncate(..)` | the capacity is kept; a `Vec` never shrinks on its own | |
| emptying, then refilling to the same length | "should incur no calls to the allocator" | |
| `push`, `insert` | never reallocate while the capacity is sufficient | the growth factor when they do; bulk methods "may reallocate, even when not necessary" |
| `Vec::with_capacity(n)`, `vec![x; n]` | request exactly `n` elements from the allocator | `capacity()` is only promised to be at least `n` |
| `reserve(n)` | room for at least `n` more elements | how much more: it may over-allocate |
| `reserve_exact(n)` | no deliberate over-allocation | the allocator may still hand back more |
| `shrink_to_fit()` | tries to drop the excess capacity | it may reallocate and move the data |

## Proving "No Copy" in a Test

- **Reuse:** after a warm-up call, `as_ptr()` and `capacity()` stay the same.
  A new buffer allocated while the old one is still alive cannot have the old
  address, so the check is deterministic.
- **Borrowing:** `input.as_bytes().as_ptr_range().contains(&piece.as_ptr())`
  holds for a view into `input`, never for a copy.
- **Sizing:** compare `capacity()` with a bound that growth by doubling
  cannot meet, instead of an exact number the docs do not promise.
- **Counting allocations** needs a custom `#[global_allocator]`, which means
  `unsafe impl GlobalAlloc`. That is the planned `alloc-perf-lab` deep-dive
  lab (ROADMAP), together with `black_box` micro-benchmarks and checking the
  assembly for bounds checks.

## Exercise Path

1. **perf1** — `render_into` replaces the caller's buffer with a new
   `String` on every frame, and `render` builds each line with
   `+= &format!(..)`. The heap pointer test fails, and Clippy's
   `format_push_string` lint (turned on at the top of the file) rejects the
   temporaries. Clear and reuse the caller's buffer, `writeln!` straight into
   it, and keep the layout code in one place. This exercise runs Clippy with
   `-D warnings`, so `write!(out, "{}", format!(..))` fails too
   (`format_in_format_args`).
2. **perf2** — `tokens(&String) -> Vec<String>` and
   `count_long(&Vec<String>)`. The tests call them with literals, sub-slices,
   arrays and a `Vec<&str>` (E0308), then check that every token points into
   the line. Take `&str` and `&[impl AsRef<str>]`, and return `Vec<&str>`.
3. **perf3** — `dot` indexes past the end of `b` or silently ignores its
   tail, `copy_prefix` indexes past the end of `dst`, and `append_chunks`
   pushes byte by byte (capacity 2048 for 1025 bytes). Assert the length
   contract before `zip`, copy with one `copy_from_slice`, and `reserve` the
   total before `extend_from_slice`. This exercise runs Clippy with
   `-D warnings`, so a hand-written copy loop fails `manual_memcpy` even once
   the tests pass.

## Related Modules

- `24_ownership_model` (`ownership4` to `ownership6`) — moving a `Vec` or a
  `String` copies only its three-word header, proven with `as_ptr()`, the
  same technique as here.
- `45_sized_deref` — its README explains the deref coercion that lets a
  `&String` argument reach a `&str` parameter (`deref1` drills it), and
  `cow1` returns borrowed data when it can and owned data when it must.
- `47_type_level/builder1` — setters that take `impl Into<String>`, the other
  half of "borrow what you only read".
- `63_slices_strings` — slices returned into the input, and bytes vs chars;
  `64_parsing/parse1` builds a lexer whose `Token<'a>` values borrow the
  source.
- `67_code_review/review2` — `ptr_arg` and `needless_range_loop` as Clippy
  lints in a review, with the reasons behind them.
- Planned deep-dive lab `alloc-perf-lab` (ROADMAP) — a counting global
  allocator to prove a hot path makes zero allocations, `black_box`
  micro-benchmarks, and bounds-check elimination confirmed in the assembly.

## Further Reading

- [`Vec`: Guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees) and [Capacity and reallocation](https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation)
- [`String::clear`](https://doc.rust-lang.org/std/string/struct.String.html#method.clear), [`std::fmt::Write`](https://doc.rust-lang.org/std/fmt/trait.Write.html) and [`write!`](https://doc.rust-lang.org/std/macro.write.html)
- [`BufRead::read_line`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line) and [`BufRead::lines`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.lines)
- [`slice::copy_from_slice`](https://doc.rust-lang.org/std/primitive.slice.html#method.copy_from_slice), [`Vec::extend_from_slice`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.extend_from_slice), [`Vec::reserve`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.reserve), [`Iterator::zip`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.zip) and [`Iterator::size_hint`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.size_hint)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/) (Nicholas Nethercote), especially [Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html) and [Bounds Checks](https://nnethercote.github.io/perf-book/bounds-checks.html)
- [How to avoid bounds checks in Rust (without unsafe!)](https://shnatsel.medium.com/how-to-avoid-bounds-checks-in-rust-without-unsafe-f65e618b4c1e) (Sergey "Shnatsel" Davidoff)
- [Rust API Guidelines: the caller decides where to copy and place data (C-CALLER-CONTROL)](https://rust-lang.github.io/api-guidelines/flexibility.html#caller-decides-where-to-copy-and-place-data-c-caller-control)
- [Use borrowed types for arguments (Rust Design Patterns)](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html)
- Clippy's [`format_push_string`](https://rust-lang.github.io/rust-clippy/master/index.html#format_push_string), [`manual_memcpy`](https://rust-lang.github.io/rust-clippy/master/index.html#manual_memcpy) and [`ptr_arg`](https://rust-lang.github.io/rust-clippy/master/index.html#ptr_arg)
- [Compiler Explorer](https://godbolt.org/), to look for `panic_bounds_check` in the assembly
