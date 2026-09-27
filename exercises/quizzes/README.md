# Quizzes

After every couple of sections, there will be a quiz in this directory that'll test your knowledge on a bunch of sections at once.

## Deep-Dive Quizzes

> `quiz1` to `quiz3` are the upstream rustlings quizzes. `quiz4` and `quiz5`
> belong to the interview-prep course (`deep-dive/COURSE.md`): rapid-fire
> "which impl runs?", "does this compile?" and "is this type Send?" drills,
> graded automatically. All **std**, **100% safe**, **stable** Rust, edition
> 2024. The TODOs name the error but not the fix, and a wrong answer fails its
> test without showing the right one; press `h` for every answer and the rule
> behind it.

1. **quiz4** (after `47_type_level`) — Method resolution and borrowck
   verdicts. Part A: two calls fail with E0034 "multiple applicable items in
   scope" (two traits both provide `fly`) and E0790 "cannot call associated
   function on trait without specifying the corresponding `impl` type" (inside
   a generic function, where the impls rustc suggests are the wrong answer).
   Name the impl you mean with fully qualified syntax. Part B: ten snippets
   hidden behind `#[cfg(any())]`; predict whether each compiles or which error
   rustc reports. Six are about two-phase borrows, and the others revisit
   `37_borrowck_errors`, `39_drop_raii` and `33_closures`. The answers are
   checked against fingerprints, and flipping a snippet to `#[cfg(all())]`
   lets rustc explain the ones you missed.
2. **quiz5** (after `36_atomics`) — Send and Sync. Classify fourteen std types
   (`Mutex<Cell<i32>>`, `RwLock<Cell<i32>>`, `&mut Cell<i32>`, `MutexGuard`,
   the `mpsc` channel ends, `*const u8`, `Box<dyn Fn() + Send>` and more) as
   Send and Sync, Send only, Sync only or neither. The tests ask the compiler
   itself, through an inherent-impl probe. The store-buffering and IRIW
   litmus questions once planned for this quiz are in the
   `deep-dive/src/ordering_lab.rs` lab, and `compile_fail` checks of Send and
   Sync in the `deep-dive/src/api_surface.rs` lab.

### Further Reading

- [Disambiguating between identically named methods (The Book)](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html#disambiguating-between-identically-named-methods)
- [Method call expressions (The Reference)](https://doc.rust-lang.org/reference/expressions/method-call-expr.html)
- [E0034](https://doc.rust-lang.org/error_codes/E0034.html) and [E0790](https://doc.rust-lang.org/error_codes/E0790.html) in the error code index
- [Two-phase borrows (rustc dev guide)](https://rustc-dev-guide.rust-lang.org/borrow-check/two-phase-borrows.html) and [RFC 2025: nested method calls](https://rust-lang.github.io/rfcs/2025-nested-method-calls.html)
- [`std::marker::Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) and [`std::marker::Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html)
- [Send and Sync (The Rustonomicon)](https://doc.rust-lang.org/nomicon/send-and-sync.html)
