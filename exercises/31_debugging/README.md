# Module 5 · Debugging and Interview Practice

> Corresponds to "Module 5: Practical Debugging and the Debug/Display Mechanism" in the syllabus.
> This directory uses only **std** and **100% safe** code to practice the skill of "locating and fixing problems" itself.

## Core idea

Debugging isn't about lucky guesses — it's about **information**. Rust gives you a few handy tools:

- `{:?}` / `{:#?}` — `Debug` formatting. `{:?}` is compact and single-line, while `{:#?}` is the indented, multi-line "pretty" output that's great for printing complex nested structures. Most types get this for free just by adding `#[derive(Debug)]`.
- `dbg!(x)` — a debugging powerhouse: it dumps `file:line` plus the `{:#?}` representation to **stderr** and **returns `x` unchanged**, so you can drop it inline into an expression as a probe.
  > Note: `dbg!` is only a temporary probe — don't leave it in finished code (this course's clippy will reject any `dbg!` in a solution).
- `assert_eq!(a, b)` — on failure it prints **both values** ("expected 15, got 10"), letting you reason backward about the logic.
- Reading **compiler errors** and **assertion messages**: the compiler often tells you outright that "`X` doesn't implement `Debug`" or "the return type doesn't match" — read the message first, then act.

## Exercise progression

1. **debugging1** — Use `#[derive(Debug)]` to make a type support `{:?}` / `{:#?}`. The unfinished version fails to compile because "`Config` doesn't implement `Debug`"; just add the one derive line.
2. **debugging2** — **Find the bug**: code that compiles but fails its test, hiding an off-by-one error. Use the `assert_eq!` failure message (temporarily adding `dbg!(sum)` if needed) to locate it, and change `1..n` to `1..=n`.
3. **debugging3** — Hand-implement `std::fmt::Display` to format a color as `#ff00aa`. The unfinished version's `fmt` body is empty and fails to compile because `()` doesn't match `fmt::Result`; complete it with `write!` and `{:02x}`.

## `Debug` vs `Display`

| trait     | format specifier | who writes it        | audience         |
| --------- | ---------------- | -------------------- | ---------------- |
| `Debug`   | `{:?}`           | usually `#[derive]`  | programmers/logs |
| `Display` | `{}`             | must be hand-written | end users        |

`Display` can **never** be derived — it represents the wording you present "to the outside world", and only you can decide that. Any type that implements `Display` also gets `.to_string()` for free.

## Further reading

- [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) (formatting syntax and all format specifiers)
- [the `dbg!` macro](https://doc.rust-lang.org/std/macro.dbg.html)
- [`std::fmt::Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html)
