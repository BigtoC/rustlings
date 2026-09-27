# [Rustlings](https://rustlings.rust-lang.org) 🦀

Small exercises to get you used to reading and writing [Rust](https://www.rust-lang.org) code - _Recommended in parallel to reading [the official Rust book](https://doc.rust-lang.org/book) 📚️_

Visit the **website** for a demo, info about setup and more:

## ➡️ [rustlings.rust-lang.org](https://rustlings.rust-lang.org) ⬅️

## Deep-dive track (this fork)

This fork adds an advanced track for people who already _use_ Rust and want to
understand _why_ it works: modules `24_ownership_model` through
`68_mock_interviews` in `exercises/` (the two gaps in the numbering, 46 and 55,
belong to modules the roadmap cut; the learner order comes from `info.toml`),
plus a separate [`deep-dive/`](deep-dive/) crate holding the labs that require
`unsafe`, and two lab crates with crates.io dependencies:
[`backend-lab/`](backend-lab/) (tokio, axum, tower) and
[`config-lab/`](config-lab/) (serde at the boundary, additive Cargo features).

Start with the course map: **[deep-dive/COURSE.md](deep-dive/COURSE.md)**.
The plan, as-built notes and still-open topics: **[deep-dive/ROADMAP.md](deep-dive/ROADMAP.md)**.

```bash
cargo run                                          # work through the exercises
cargo test --manifest-path deep-dive/Cargo.toml    # run the unsafe labs
cargo test --manifest-path backend-lab/Cargo.toml  # run the tokio / axum / tower lab
cargo test --manifest-path config-lab/Cargo.toml   # run the serde + Cargo features lab
```
