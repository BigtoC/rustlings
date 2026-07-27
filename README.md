# [Rustlings](https://rustlings.rust-lang.org) 🦀

Small exercises to get you used to reading and writing [Rust](https://www.rust-lang.org) code - _Recommended in parallel to reading [the official Rust book](https://doc.rust-lang.org/book) 📚️_

Visit the **website** for a demo, info about setup and more:

## ➡️ [rustlings.rust-lang.org](https://rustlings.rust-lang.org) ⬅️

## Deep-dive track (this fork)

This fork adds an advanced track for people who already _use_ Rust and want to
understand _why_ it works: modules `24_ownership_model` through `36_atomics` in
`exercises/`, plus a separate [`deep-dive/`](deep-dive/) crate holding the labs
that require `unsafe`.

Start with the course map: **[deep-dive/COURSE.md](deep-dive/COURSE.md)**.

```bash
cargo run                                          # work through the exercises
cargo test --manifest-path deep-dive/Cargo.toml    # run the unsafe labs
```
