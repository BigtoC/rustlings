# Module 5 · Debugging and Interview Practice

> Corresponds to "Module 5: Practical Debugging and the Debug/Display Mechanism" in the syllabus.
> This directory uses only **std** and **100% safe** code to practice the skill of "locating and fixing problems" itself.

## Core idea

Debugging isn't about lucky guesses — it's about **information**. Rust gives you a few handy tools:

- `{:?}` / `{:#?}` — `Debug` formatting. `{:?}` is compact and single-line, while `{:#?}` is the indented, multi-line "pretty" output that's great for printing complex nested structures. Most types get this for free just by adding `#[derive(Debug)]`.
- `dbg!(x)` — a debugging powerhouse: it dumps `file:line` plus the `{:#?}` representation to **stderr** and **returns `x` unchanged**, so you can drop it inline into an expression as a probe.
  > Note: `dbg!` is only a temporary probe — don't leave it in finished code. Nothing will stop you here, though: `clippy::dbg_macro` is allow-by-default, and while rustlings' own source opts into it (`[workspace.lints.clippy]` in the repo root), the exercise crate you are editing deliberately does not. Stripping your probes is on you. (Add `dbg_macro = "deny"` to `[lints.clippy]` in your own projects if you want the gate.)
- `assert_eq!(a, b)` — on failure it prints **both values** ("expected 15, got 10"), letting you reason backward about the logic.
- Reading **compiler errors** and **assertion messages**: the compiler often tells you outright that "`X` doesn't implement `Debug`" or "the return type doesn't match" — read the message first, then act.

Beyond formatting, this module drills the runtime and compile-time failures that trip people up in interviews and production:

- **Integer overflow** is checked in debug builds (it PANICS) and wrapping in release builds (it silently wraps modulo 2^bits). Neither is a bug in Rust — you are expected to choose behavior on purpose with `checked_add` / `saturating_add` / `wrapping_add`.
- **`RefCell` borrow panics**: interior mutability moves "aliasing XOR mutability" from compile time to run time. Overlapping `borrow()` and `borrow_mut()` panics with `BorrowMutError` — scope your borrows tightly.
- **Iterator invalidation**: mutating a container while iterating is a runtime footgun in other languages, but a plain **compile error** (E0502) in Rust.

## Exercise progression

1. **debugging1** — Use `#[derive(Debug)]` to make a type support `{:?}` / `{:#?}`. The unfinished version fails to compile because "`Config` doesn't implement `Debug`"; just add the one derive line.
2. **debugging2** — **Integer overflow**: `checksum` adds `u32`s with `+=`, which PANICS on overflow in debug builds (and would silently wrap in release). The large-input test fails. Decide the intended behavior — here, clamp — and replace `acc += v;` with `acc = acc.saturating_add(v);` (`checked_add`/`wrapping_add` are the other deliberate choices).
3. **debugging3** — Hand-implement `std::fmt::Display` to format a color as `#ff00aa`. The unfinished version's `fmt` body is empty and fails to compile because `()` doesn't match `fmt::Result`; complete it with `write!` and `{:02x}`.
4. **debugging4** — **`RefCell` runtime borrow panic**: a `borrow()` guard is still alive when `borrow_mut()` is taken, so the code compiles but panics with `BorrowMutError`. Copy the value out (`let current = *cell.borrow();`) so the shared borrow ends immediately, then mutate.
5. **debugging5** — **Iterator invalidation**: removing from a `Vec` while `v.iter()` still borrows it is rejected at compile time (E0502). Replace the whole index-removing loop with one safe pass: `v.retain(|&x| x % 2 != 0);`.

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
- [`u32::saturating_add`](https://doc.rust-lang.org/std/primitive.u32.html#method.saturating_add) (and `checked_add` / `wrapping_add` next to it)
- [`std::cell::RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html)
- [`Vec::retain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.retain)
