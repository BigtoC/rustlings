# Module 5 · Code Review: Find the Bugs, Then Fix the Idioms

> The review round of Module 5, after the debugging drills of `31_debugging`
> and the live-coding drills of `63_slices_strings` to `66_checked_math`.
> Many interview loops hand you a pull request instead of a blank editor and
> ask what you would flag before approving it. `review1` is that round with
> real bugs, `review2` with correct but unidiomatic code. All **std**, **100%
> safe**, **stable** Rust, edition 2024.
>
> This module is more unguided than the others. `review1` has a single
> `// TODO` for the whole file: it names neither the bugs nor where they
> are, only the failing tests do. `review2`'s TODO names the clippy lints, as
> clippy itself does. Neither TODO gives the fix. Press `h` when you want it:
> `review1`'s hint reveals the bugs a little at a time.

## Core Ideas

- **Compiling is where the review starts.** In safe Rust, the compiler rules
  out memory errors and data races. It does not check that a function does
  what its comment promises: arithmetic, conversions, run-time checks,
  ordering and plain logic are still the reviewer's job. The bugs that reach
  a Rust code review are almost all of that kind.
- **The spec is the comment, the evidence is a failing input.** Read each
  function against what it promises. A finding is only useful with an input
  that breaks it and a statement of what happens then: a panic, a wrong
  number, an order that changes between runs.
- **Rank what you find.** A crash that user input can trigger, or a number
  that is silently wrong, blocks the merge. A contract violation that users
  will hit is major. Style is a nit, and a review that spends its time on
  nits while missing a blocker fails the round.
- **Fix the bug, not the symptom.** The smallest change that meets the spec
  wins. Silencing a panic by returning a default (`unwrap_or(0)`, a
  saturating cast) swaps a loud bug for a quiet one.
- **An idiom is a reason, not a rule.** Clippy's default lints mostly remove
  a way for the NEXT edit to go wrong, or let more callers use a function.
  Interviewers ask what each idiomatic version buys, and whether your
  refactor kept the behavior.
- **Say how CI would have caught it.** A boundary test (the limit itself, the
  largest value, one past it, non-ASCII input), a reentrant test double, or
  an opt-in clippy lint. That answer is the follow-up question in almost
  every review round.

## How to Review

Ask these of every function, in this order. Not every question finds
something, which is the point of asking all of them.

1. Does it do what its comment says, including at the edges: empty input,
   one element, the boundary value itself, the maximum?
2. Can any arithmetic overflow, underflow or divide by zero? What does it do
   then in a debug build, and what in a release build?
3. Can any conversion lose information (`as`, a float, a narrower type)?
4. Can any indexing, slicing, `unwrap` or `expect` panic, and can user input
   reach it?
5. Does anything that leaves the function (output, a hash, a file) depend on
   an order nobody chose?
6. Is a borrow, guard or lock held across a call into code you don't
   control, such as a callback or a trait object?
7. On an error path, is the state left half-updated?
8. Does it allocate or clone where a borrow would do?
9. Are errors values with enough context, or panics and strings?
10. Which test is missing?

## The Lints in review2

All seven are warn-by-default; `strict_clippy` turns them into errors.

| Lint                  | Flags                             | Idiomatic form                                         | What it buys                                                        |
| --------------------- | --------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------- |
| `ptr_arg`             | a `&Vec<T>` or `&String` param    | `&[T]` / `&str`                                        | more callers (arrays, sub-slices, literals), one pointer hop fewer  |
| `needless_range_loop` | `for i in 0..v.len()` plus `v[i]` | iterate the elements (`.iter().skip(1)`, `enumerate`)  | no off-by-one in the range, no bounds check to hope away            |
| `manual_map`          | `match` that rebuilds an `Option` | `Option::map`                                          | one call that says "transform if present"                           |
| `unnecessary_unwrap`  | `is_some()`, then `unwrap()`      | `if let`, `match`, `unwrap_or`                         | the check and the extraction cannot drift apart                     |
| `len_zero`            | `len() == 0`                      | `is_empty()`                                           | states intent; the convention `len_without_is_empty` enforces       |
| `needless_return`     | a trailing `return x;`            | end the block with `x`                                 | `return` stays reserved for early exits                             |
| `manual_clamp`        | an `if`/`else` clamp              | `Ord::clamp` (it panics if `min > max`)                | one word; clippy only suggests it when the bounds are constants     |

The trap in `review2` is a refactor that changes behavior. The obvious
iterator method for "the maximum" has a documented tie rule, and it is the
opposite of the loop's. A shorter sum can overflow. The tests pin both.

## Exercise Path

1. **review1** — A single-threaded `Ledger` pull request of about 180 lines
   of code (fees, transfer limits, observers, a history, a statement, a
   legacy export and a text console) with seven planted bugs. It compiles,
   and eight happy-path tests pass. Seven bug reports fail, each named after
   what the user saw: four of them panic and three get a wrong answer.
   Review first, then run the tests, then fix each bug where it lives
   without changing any signature or test.
2. **review2** — A leaderboard helper that is correct and fully tested, but
   `strict_clippy` rejects it with eight errors from seven lints (`ptr_arg`
   twice, `needless_range_loop`, `manual_map`, `unnecessary_unwrap`,
   `len_zero`, `needless_return`, `manual_clamp`). Refactor until clippy is
   clean, without any `#[allow]`, and keep the tests passing unchanged: they
   pin a tie, some very large scores and ASCII-only case folding.

## Related Modules

Each bug in `review1` comes from a topic that an earlier module drills on its
own: `31_debugging`, `63_slices_strings`, `66_checked_math` and
`35_error_design`. Go back to them after the review, not before; the hint for
`review1` says which exercise goes with which bug.

- `50_testing_seams` — test doubles (stubs, fakes, spies) behind trait seams.
  The audit-log observer in `review1`'s tests is a spy.
- `22_clippy` — the first clippy exercises; `review2` asks for the reasons
  behind the lints.
- `65_performance` — the allocation side of `ptr_arg`: buffer reuse and
  borrowed returns checked by pointer identity, and `&str` / `&[T]`
  parameters that accept every caller.
- `68_mock_interviews` (next) — timed, statement-first interview sets.
- Planned deep-dive lab `api-surface-lab` (ROADMAP) — integration tests and
  property tests with proptest, which find boundary inputs like the ones in
  `review1`'s bug reports without you having to guess them.

## Further Reading

- [Google's code review guide: what to look for](https://google.github.io/eng-practices/review/reviewer/looking-for.html) and [how to write review comments](https://google.github.io/eng-practices/review/reviewer/comments.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy's lint list](https://rust-lang.github.io/rust-clippy/master/index.html) and [the Clippy book](https://doc.rust-lang.org/clippy/) (lint groups, and how to turn on `restriction` lints such as `arithmetic_side_effects`, `string_slice` and `unwrap_used` in CI)
- [Integer overflow (The Reference)](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow) and [numeric casts with `as` (The Reference)](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast)
- [Slicing strings (The Book, ch. 8.2)](https://doc.rust-lang.org/book/ch08-02-strings.html#slicing-strings) and [`str::char_indices`](https://doc.rust-lang.org/std/primitive.str.html#method.char_indices)
- [`HashMap::iter`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.iter) ("in arbitrary order") and [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)
- [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html) (`borrow` and `borrow_mut` panic on a conflicting borrow)
- [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [Use borrowed types for arguments (Rust Design Patterns)](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html)
- [`Iterator::max_by_key`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.max_by_key) and [`Ord::clamp`](https://doc.rust-lang.org/std/cmp/trait.Ord.html#method.clamp)
