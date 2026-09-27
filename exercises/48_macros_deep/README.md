# Traits & Abstraction · Declarative Macros: Repetition, Single Evaluation, `$crate` and Code Generation

> Part of the "Traits & Abstraction" group, after `quizzes/quiz4` and before
> `49_panics`. Upstream `21_macros` shows how to call a macro and where to
> define it. This module is the part interviews probe: "write a `hashmap!`
> macro that accepts a trailing comma", "what is wrong with this `max!`",
> "why `$crate`?", "how would you implement this trait for ten types?".
> Entirely **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in an interview, the `// TODO` comments name the error (or the failing
> test) and the requirements but **not** the fix. Read what rustc or the test
> says first. Press `h` when you want the full answer.

## Core Ideas

- **Matching is on token trees, arm by arm.** rustc tries the arms of a
  `macro_rules!` macro from top to bottom and expands the first one whose
  matcher accepts the whole call. It never looks ahead, so a matcher that
  needs lookahead is an error ("local ambiguity"). When no arm matches, the
  error has no code: "no rules expected `x`", pointing at the first token no
  arm could accept.
- **A captured fragment is parsed once and stays opaque.** `$e:expr` holds a
  parsed expression. Pasted into the transcriber, it keeps its grouping:
  `$e * 2` with `1 + 1` gives 4, where a C macro (or a `$($t:tt)*` that
  pastes raw tokens) gives 3. Passed on to another macro, it arrives as ONE
  token tree, which literal tokens can no longer match: only a fragment of
  the same kind, or `tt`.
- **Pasting is evaluating.** Every `$x` in the transcriber is a fresh copy of
  the caller's expression, so an argument pasted twice runs twice. Bind each
  argument once (`{{ let x = $x; .. }}`, or `match $x { x => .. }` as std's
  `dbg!` does) and use the binding.
- **Hygiene is mixed-site.** Local variables and labels resolve where the
  macro is defined; everything else resolves where it is called. See the
  table below.
- **Macros have two scopes.** Textual scope follows the source order: a
  macro exists from its definition to the end of the enclosing module (that
  is `21_macros/macros2`), and `#[macro_use]` on a module extends it past the
  module's end (`macros3`). Path-based scope makes a macro nameable by path:
  `#[macro_export]` puts it at the crate root as `crate::name!` for this
  crate and `your_crate::name!` for others, and `pub(crate) use name;` right
  after the definition gives it a module path without exporting it.
- **Generated items are not hygienic.** A `struct $name` or `fn $name` is
  visible to the caller under exactly that name, and two expansions that
  define the same fixed name clash with E0428.

## Fragments and What May Follow Them

A metavariable is `$name:fragment`. The fragment decides what it captures,
and, so that future syntax cannot make a matcher ambiguous, which tokens may
come right after it:

| Fragment | Captures | May be followed by |
| --- | --- | --- |
| `expr`, `stmt` | an expression; a statement without its `;` | `=>` `,` `;` |
| `pat` | a pattern, top-level or-patterns included (edition 2021 and later) | `=>` `,` `=` `if` `in` |
| `pat_param` | a pattern without top-level or-patterns | the same, plus the or-pattern pipe |
| `path`, `ty` | a type-style path; a type | `=>` `,` `=` `;` `:` `>` `>>` `[` `{` `as` `where`, the pipe, or a `block` metavariable |
| `vis` | a visibility, possibly empty | `,`, an identifier other than `priv`, anything that can start a type, an `ident` / `ty` / `path` metavariable |
| `ident`, `lifetime`, `literal`, `block`, `item`, `meta`, `tt` | one identifier, lifetime, literal (with an optional `-`), block, item, attribute body, or token tree | anything |

- `tt` is the most flexible: a single token, or a whole `( .. )`, `[ .. ]`
  or `{ .. }` group, whatever is inside it. Recursive macros that eat their
  input a few tokens at a time ("tt munchers") are built on it.
- In edition 2024, `expr` also matches `_` and `const { .. }` at the top
  level. `expr_2021` keeps the old meaning for matchers that relied on it.
- Repetition is `$( .. ) sep op`, where `op` is `*` (zero or more), `+` (one
  or more) or `?` (at most one, no separator allowed). `$(,)?` is the usual
  optional trailing comma. In the transcriber, a metavariable must sit at
  the same repetition depth as in the matcher, or rustc says "variable `x`
  is still repeating at this depth".

## Counting

There is no stable "how many?" in a transcriber: the metavariable expression
`${count($x)}` is unstable (E0658, tracking issue
[#83527](https://github.com/rust-lang/rust/issues/83527)). Two stable ways:

```rust
// Recursive: one level of expansion per token. Simple, but it hits the
// recursion limit (128 by default) at around 128 tokens, sooner when it is
// called from inside other macros. `#![recursion_limit = "256"]` raises it.
macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

// The slice-length trick: constant depth, still a constant expression.
// The internal `@unit` rule must come FIRST: `@unit x` also matches
// `$($x:tt)*`, so with the arms swapped the macro recurses until the limit.
macro_rules! count_all {
    (@unit $x:tt) => { () };
    ($($x:tt)*) => { <[()]>::len(&[$(count_all!(@unit $x)),*]) };
}
```

The `maplit` crate's `hashmap!` pre-sizes its map with exactly this trick.
Its `$( .. ),*` arm also accepts an empty call, and it names the map `_map`.
`unused_mut` ignores variables whose names start with `_`, so that is the
other way around the warning an empty call would trigger; the exercise keeps
a separate `()` arm, as std's `vec!` does.

## Hygiene and Paths

| Name in the expansion | Resolved at | What it means for the macro author |
| --- | --- | --- |
| local variables, loop and block labels | the definition | the macro's `let x` never clashes with the caller's `x`, and the macro cannot use a caller's local unless it is passed in |
| functions, types, traits, modules, other macros, and paths starting with `self::` or `super::` | the call site | name them by absolute path (`$crate::..`, `::std::..`), or a missing import breaks the call and a same-named item in the caller hijacks it |
| `$crate` | the crate that defines the macro | `crate` inside that crate, `::your_crate` in all others |
| privacy | the call site | everything the expansion names must be visible to callers: `pub`, often `#[doc(hidden)]` in a `__private` module |

The first two rows pull in opposite directions. A caller's local is
invisible to the expansion, so this does not compile, even though `factor`
is in scope where the macro is called:

```rust
macro_rules! scaled {
    ($e:expr) => { $e * factor };
}

fn main() {
    let factor = 2;
    let _ = scaled!(3); // E0425: cannot find value `factor` in this scope
}
```

while a call such as `helper()` in an expansion quietly means whatever
`helper` is in scope where the macro is called. `crate::helper()` looks like the fix and is a
trap: `crate` means the calling crate, so it passes every test written inside
the defining crate and breaks for everyone else. Clippy's warn-by-default
`crate_in_macro_def` catches it, and `macros7` is graded with
`clippy -D warnings` for that reason. `#[macro_export(local_inner_macros)]`
is a pre-`$crate` migration tool; the Reference discourages it in new code.

## Generating Items

- An `ident` can name what the macro defines, `stringify!($name)` turns it
  into a `&'static str` at compile time, and `concat!` joins literals.
- `$(#[$attr:meta])*` in the matcher and `$(#[$attr])*` in the transcriber
  pass the caller's attributes (doc comments included) through to the
  generated item.
- `macro_rules!` cannot build a new identifier out of pieces
  (`get_` + `name`) on stable: `${concat(..)}` is unstable (E0658) and the
  old `concat_idents!` is gone. The `paste` crate does it with a procedural
  macro.
- **A macro or a blanket impl?** A blanket impl
  (`impl<T: Display> Describe for T`) covers an open set of types, including
  ones that do not exist yet, but it claims them all (see
  `42_coherence/coherence2`). A macro stamps out one impl per type in a
  closed list, each of which can differ, which is why core implements the
  operator traits for the primitive numbers with macros like `add_impl!`.

## Declarative vs Procedural Macros

- **When do you need a procedural macro?** On stable, `macro_rules!` macros
  are function-like only (`name!(..)`), so derives (`#[derive(Builder)]`)
  and attribute macros (`#[tokio::main]`) are procedural. So is anything that
  has to look inside a struct's fields and types, invent identifiers, or
  parse syntax too irregular for matchers.
- **Why must it live in its own crate?** A procedural macro is a compiler
  plugin: a `proc-macro = true` crate is compiled for the host, loaded by
  rustc and run during compilation, and it can export nothing but macros.
  That is why `serde` re-exports its derives from `serde_derive`, and why
  proc-macro crates lean on `syn` (parsing), `quote` (generating) and
  `proc-macro2`.
- **How do you emit an error that points at the right code?** In a
  procedural macro, build a `syn::Error::new_spanned(&node, "message")` and
  return `.to_compile_error()`: the error carries the span of `node`, so rustc
  underlines the offending tokens. In `macro_rules!`, a dedicated arm can
  expand to `compile_error!("max! needs at least one argument")`. rustc
  reports that message at the `compile_error!` inside the macro and marks
  the caller's line "in this macro invocation".
- **What about hygiene?** `proc_macro::Span::call_site()` resolves as if the
  caller wrote the code, `Span::mixed_site()` (stable since 1.45) behaves
  like `macro_rules!`, and `Span::def_site()` is still unstable.
- **Practice:** dtolnay's
  [proc-macro-workshop](https://github.com/dtolnay/proc-macro-workshop)
  builds a derive `Builder`, an attribute macro and more, with staged tests.
  It replaces the procedural-macro lab this course once planned.

## Debugging a Macro

- [`cargo expand`](https://github.com/dtolnay/cargo-expand) prints the code
  after expansion (it needs a nightly toolchain installed, not as the
  default). rust-analyzer's "Expand macro recursively" does the same for the
  call under the cursor.
- rustc's note "in Nightly builds, run with -Z macro-backtrace" is literal:
  that flag, and `trace_macros!`, are nightly-only.
- Testing that a misuse (like `max!()`) fails to compile needs a
  `compile_fail` doctest or `trybuild`, which single-file exercises cannot
  host; the `deep-dive/src/api_surface.rs` lab has such a doctest.

## Exercise Path

1. **macros5** — `count!` and `hashmap!` accept only an empty call, so every
   use fails with "no rules expected `a`" (no error code). Make `count!`
   count token trees recursively, as a `usize` constant, and give `hashmap!`
   an arm for `key => value` pairs with an optional trailing comma, pre-sized
   with `count!`, evaluating each expression once, next to its empty arm.
2. **macros6** — A recursive `max!` pastes its first argument and the
   recursive call twice, so three arguments make seven calls and ten make
   1023. Bind every argument once, left to right, keeping the `PartialOrd`
   comparison and non-`Copy` arguments working.
3. **macros7** — A `#[macro_export]` `percent!` defined in `mod util`
   expands to a bare `clamp_percent($e)`, which resolves at every call site:
   E0425 in other modules, and the wrong function in a module that has its
   own `clamp_percent`. Name the helper with `$crate`; the exercise runs
   clippy with `-D warnings`, so `crate::` fails `crate_in_macro_def`.
4. **macros8** — Two stubs fail with "no rules expected": generate one `f64`
   newtype per unit name (derives, `Add`, a `Display` built with
   `stringify!`), and turn a `test_cases!` table into one `#[test]` per row,
   passing each row's attributes through.

## Related Modules

- `21_macros`: calling and defining macros, textual order and `#[macro_use]`.
- `42_coherence/coherence2`: blanket impls, the alternative to stamping out
  impls with a macro. `43_assoc_types/assoc2`: `Add` and its `Output`.
- `45_sized_deref/deref1`: a validated newtype with a private field, like
  `macros7`'s `Percent`. `41_memory_layout`: why a newtype costs nothing.
- `17_tests`: `#[should_panic]`. `49_panics` (next): `catch_unwind`, which
  `macros8` uses to check that a generated test really fails.
- `50_testing_seams`: designing code so that it can be tested at all.

## Further Reading

- [Macros By Example (The Reference)](https://doc.rust-lang.org/reference/macros-by-example.html): [metavariables](https://doc.rust-lang.org/reference/macros-by-example.html#metavariables), [repetitions](https://doc.rust-lang.org/reference/macros-by-example.html#repetitions), [scoping and `#[macro_export]`](https://doc.rust-lang.org/reference/macros-by-example.html#scoping-exporting-and-importing), [hygiene and `$crate`](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene), [follow-set restrictions](https://doc.rust-lang.org/reference/macros-by-example.html#follow-set-ambiguity-restrictions) and [forwarding a matched fragment](https://doc.rust-lang.org/reference/macros-by-example.html#forwarding-a-matched-fragment)
- [Procedural Macros (The Reference)](https://doc.rust-lang.org/reference/procedural-macros.html)
- [Macros (The Book)](https://doc.rust-lang.org/book/ch20-05-macros.html)
- [The Little Book of Rust Macros](https://veykril.github.io/tlborm/): [counting](https://veykril.github.io/tlborm/decl-macros/building-blocks/counting.html), [tt munchers](https://veykril.github.io/tlborm/decl-macros/patterns/tt-muncher.html), [internal rules](https://veykril.github.io/tlborm/decl-macros/patterns/internal-rules.html) and [hygiene](https://veykril.github.io/tlborm/decl-macros/minutiae/hygiene.html)
- [Rust 2024: `expr` fragment specifier changes](https://doc.rust-lang.org/edition-guide/rust-2024/macro-fragment-specifiers.html)
- [`stringify!`](https://doc.rust-lang.org/std/macro.stringify.html), [`concat!`](https://doc.rust-lang.org/std/macro.concat.html) and [`compile_error!`](https://doc.rust-lang.org/std/macro.compile_error.html)
- [Clippy: `crate_in_macro_def`](https://rust-lang.github.io/rust-clippy/master/index.html#crate_in_macro_def)
- [dtolnay's proc-macro-workshop](https://github.com/dtolnay/proc-macro-workshop) and [macrokata](https://github.com/tfpk/macrokata) (`macro_rules!` exercises in the rustlings style)
- Crates worth reading: [`maplit`](https://docs.rs/maplit/latest/maplit/) (`hashmap!`), [`paste`](https://docs.rs/paste/latest/paste/) (building identifiers), [`test-case`](https://docs.rs/test-case/latest/test_case/) and [`rstest`](https://docs.rs/rstest/latest/rstest/) (table-driven tests)
