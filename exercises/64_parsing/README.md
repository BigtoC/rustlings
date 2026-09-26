# Module 5 · Parsing: Borrowed Tokens, Recursive Descent, Never-Panicking Evaluation, an AST

> The parsing drills of Module 5, after `63_slices_strings`. "Write an
> expression evaluator" is a common take-home and live-coding problem, and
> in Rust it tests more than the algorithm: tokens that borrow the input,
> byte offsets, left associativity, checked arithmetic, bounded recursion,
> and an AST that `FromStr` forces to own its data. You build it in three
> steps: a lexer, an evaluator and a tree. All **std**, **100% safe**,
> **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements but **not** the fix, as in an
> interview. Press `h` when you want the full answer.

## Core Ideas

- **Lex first, then parse.** The lexer turns characters into tokens, so the
  parser never deals with whitespace, digits or UTF-8. Both look one item
  ahead: `Peekable` adds `peek()` and `next_if(pred)` to any iterator.
- **Tokens borrow the source.** `Token<'a>` holds `Ident(&'a str)`, a slice
  of the input, so lexing allocates nothing. The `'a` is the source's
  lifetime, not the lexer's: the tokens outlive the lexer, and that is why a
  std `Iterator` can yield them. An item that borrows from the iterator
  itself needs a lending iterator (`43_assoc_types/gat1`).
- **Positions are byte offsets.** `char_indices()` gives each character's
  offset, and a token ends where the next character starts, or at
  `src.len()`. A byte offset is what a caller can slice with (rustc's own
  spans are byte offsets too); counting characters puts every error after
  an "é" in the wrong place (`63_slices_strings/window2`). This course's lexer yields
  `(start, token)` pairs; LALRPOP's custom lexers yield `(start, token,
  end)` triples.
- **One function per precedence level, one loop per function.** `expr`
  reads `term`s, and `term` reads `factor`s. Inside a level, a loop folds
  each operand into the running result, which makes the operators
  left-associative.
- **Bound the recursion.** Parentheses and unary minus still recurse, one
  level per nesting. Running out of stack is not a panic: the process
  aborts with "has overflowed its stack", and `catch_unwind` cannot stop it.
  Count the depth and return an error; serde_json's default limit is 128.
- **Evaluate without panicking.** Every operator is checked. Division fails
  in two ways, and `checked_div` returns `None` for both, so test for a zero
  divisor first.
- **`FromStr` output cannot borrow its input.** `fn from_str(s: &str) ->
  Result<Self, Self::Err>` has no lifetime that could tie `Self` to `s`, so
  a `FromStr` AST owns its names (`Var(String)`). A zero-copy AST
  (`Expr<'a>`) needs an inherent `fn parse(src: &'a str) -> Result<Expr<'a>,
  _>` instead, which is what serde's `Deserialize<'de>` lifetime is for.
- **Test a parser with round trips.** Print the tree fully parenthesized,
  parse the text again and compare the trees. Random trees printed with as
  few parentheses as possible check precedence and associativity at the
  same time.

## Three Ways to Write `8 - 3 - 2`

| Rule for `expr`                      | Reads `8 - 3 - 2` as | What goes wrong                                                                     |
| ------------------------------------ | -------------------- | ----------------------------------------------------------------------------------- |
| `expr '-' term` (left recursion)     | nothing              | `expr` calls itself before consuming a token, until the stack overflows             |
| `term ('-' expr)?` (right recursion) | `8 - (3 - 2)` = 7    | the wrong associativity, and one more level of recursion per operator               |
| `term ('-' term)*` (a loop)          | `(8 - 3) - 2` = 3    | nothing: a flat chain of any length runs at a constant depth                        |

Precedence climbing and Pratt parsing generalize the third row: one loop,
with each operator's binding power in a table instead of one function per
level.

## What Can Panic in a Calculator

| Input                             | Naive code         | What happens                                                                    | Never-panicking version                          |
| --------------------------------- | ------------------ | ------------------------------------------------------------------------------- | ------------------------------------------------ |
| `99999999999999999999`            | `parse().unwrap()` | a panic on user input                                                           | `IntErrorKind::PosOverflow` becomes an error     |
| `9223372036854775807 + 1`         | `a + b`            | "attempt to add with overflow" in debug builds, wraps to `i64::MIN` in release  | `checked_add`                                    |
| `1 / 0`                           | `a / b`            | "attempt to divide by zero", in every build                                     | test `b == 0` first, then `DivByZero`            |
| `(-9223372036854775807 - 1) / -1` | `a / b`            | "attempt to divide with overflow", in every build                               | `checked_div`, then `Overflow`                   |
| `-(-9223372036854775807 - 1)`     | `-a`               | "attempt to negate with overflow" in debug builds, `i64::MIN` again in release  | `checked_neg`                                    |
| 100_000 `(` in a row              | plain recursion    | "has overflowed its stack", and the process aborts                              | a depth counter, then `TooDeep`                  |

`i64::MIN` has no literal in this calculator: 9223372036854775808 does not
fit in an `i64`, so the lexer rejects it before any minus sign can apply.
Rust itself exempts a negated literal from the overflow rules (see the
Reference below), which is why `-9223372036854775808i64` compiles.

## Exercise Path

1. **parse1** — A `Lexer<'a>` whose `next` is empty (E0308). Walk a
   `Peekable<CharIndices>`, skip whitespace, slice identifiers out of the
   source at byte offsets, report every error with its byte offset, turn a
   literal above `i64::MAX` into `NumberTooLarge` (not a panic or a wrapped
   value), and carry on after an error.
2. **parse2** — A recursive-descent evaluator whose rules recurse on the
   right: `8 - 3 - 2` gives 7, `8 / 4 / 2` gives 4, a long flat sum fails
   with `TooDeep`, and overflow and division panic. Rewrite `expr` and
   `term` as loops, add unary minus behind the depth guard, and check every
   operation, with `DivByZero` and `Overflow` told apart.
3. **parse3** — An `enum Expr` AST whose `FromStr` and `Display` are empty
   (two E0308s). Build left-deep trees with the same loops, with owned
   variable names; print every operator inside its own parentheses; and pass
   the round trip on hand-picked and random trees.

## Deep Trees

Bounding the parser's recursion does not bound the tree. A flat `1 - 1 -
... - 1` parses in a loop, but its AST is a left-deep tree with one level
per operator, and everything that walks the tree recursively (`eval`,
`Display`, `Debug` and the compiler-generated drop glue) needs one stack
frame per level. Dropping a 100_000-level tree overflows a 2 MiB test
thread's stack in a debug build. serde_json's documentation warns about the
same thing for its `Value` ("Display and Debug and Drop impls"). The fixes:
limit the input size, walk the tree with an explicit stack (like the
iterative `Drop` in `27_data_structures/linkedlist1`), or keep the nodes in
a `Vec` and link them by index (`59_arena`).

## Related Modules

- `25_lifetimes_deep/lifetimes6` — `impl<'a>`, and returning `&'a str`
  instead of a borrow of `&self`.
- `34_iterators/iter1` — implementing `Iterator` by writing only `next`.
- `35_error_design/err1` — the `From` impl behind `?`, as in
  `From<LexError> for EvalError`.
- `23_conversions/conversions3` — `FromStr` and `str::parse`.
- `26_smart_pointers_deep/smartptr1` — `Box` for recursive types (E0072).
- `31_debugging/debugging2` — overflow panics in debug builds and wraps in
  release builds.
- `66_checked_math/checkedmath3` — a strict `FromStr` and a canonical
  `Display` for fixed-point decimals, with the same round-trip test.
- `67_code_review/review1` — a `parse().unwrap()` on user input, among
  other planted bugs.

## Further Reading

- [`Peekable`](https://doc.rust-lang.org/std/iter/struct.Peekable.html) (see [`next_if`](https://doc.rust-lang.org/std/iter/struct.Peekable.html#method.next_if)) and [`str::char_indices`](https://doc.rust-lang.org/std/primitive.str.html#method.char_indices)
- [`IntErrorKind`](https://doc.rust-lang.org/std/num/enum.IntErrorKind.html) and [`ParseIntError::kind`](https://doc.rust-lang.org/std/num/struct.ParseIntError.html#method.kind)
- [`i64::checked_div`](https://doc.rust-lang.org/std/primitive.i64.html#method.checked_div), [`i64::checked_neg`](https://doc.rust-lang.org/std/primitive.i64.html#method.checked_neg) and [Overflow (The Reference)](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow), with the negated-literal exception and the `MIN / -1` rule
- [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) and [Understanding deserializer lifetimes (serde)](https://serde.rs/lifetimes.html)
- [Using `Box<T>` to Point to Data on the Heap (The Book)](https://doc.rust-lang.org/book/ch15-01-box.html), including recursive types
- Robert Nystrom, [Parsing Expressions (Crafting Interpreters)](https://craftinginterpreters.com/parsing-expressions.html): recursive descent with one function per precedence level
- Aleksey Kladov, [Simple but Powerful Pratt Parsing](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html), in Rust
- Eli Bendersky, [Parsing expressions by precedence climbing](https://eli.thegreenplace.net/2012/08/02/parsing-expressions-by-precedence-climbing)
- [Writing a custom lexer (LALRPOP book)](https://lalrpop.github.io/lalrpop/lexer_tutorial/003_writing_custom_lexer.html): a lexer that borrows its input and yields `(start, token, end)` spans
- [`serde_json::Deserializer::disable_recursion_limit`](https://docs.rs/serde_json/latest/serde_json/struct.Deserializer.html#method.disable_recursion_limit): what the default depth limit protects, and why deep values are risky to drop
