//! Lab · testing a library from outside: `compile_fail`, proptest, semver
//!
//! A library's public API is a contract with code in OTHER crates, and several
//! of its guarantees only exist over there. Inside the defining crate a sealed
//! trait can be implemented, a `#[non_exhaustive]` enum can be matched without
//! a `_` arm, and a private module is visible everywhere. So the tests that
//! check the contract have to stand outside it:
//!
//! | Test | Lives in | Compiled as | Sees |
//! | --- | --- | --- | --- |
//! | unit | `#[cfg(test)] mod tests` in `src/` | part of the library crate | private items too; seals and `#[non_exhaustive]` do not apply |
//! | integration | `tests/*.rs` | one crate per file, linking the library | the `pub` API only, like a user |
//! | doctest | a code block in a `///` or `//!` comment | code outside the library that links it | the `pub` API only |
//!
//! Doctests are also the only std tool that can check that code does NOT
//! compile. rustdoc wraps every example in `#![allow(unused)]`; in edition
//! 2024 it merges the examples that should compile into one bundle crate (one
//! module each) and compiles every `compile_fail` example as a crate of its
//! own.
//!
//! This module is a small library tested the way a published crate should be:
//! the doctests on its items, the integration tests in `tests/api_surface.rs`
//! (with helpers in `tests/common/mod.rs`), property tests with proptest, and a
//! semver table that links each rule to the doctest that shows it.
//!
//! Compare with the graded exercises that could not test these guarantees:
//! `47_type_level/builder1` and `typestate1` (their tests can only show that
//! correct code compiles), `38_variance/variance1` and `quizzes/quiz5` (they
//! test auto traits with a `Probe` whose inherent associated const shadows a
//! trait's, here replaced by `compile_fail` doctests),
//! `45_sized_deref/deref1` (the missing `DerefMut`), `48_macros_deep/macros6`
//! (a `max!` misuse), `50_testing_seams` (the recording double behind
//! [`Recorder`]), `42_coherence` (sealing, and the E0034 collisions that make
//! even a defaulted trait method possibly-breaking) and
//! `66_checked_math/checkedmath1` (a hand-rolled property test with a
//! fixed-seed generator and no shrinking).
//!
//! # Invariants
//!
//! - `decode::<F>(&encode::<F, _>(&fields)) == Ok(fields)` for every `F` and
//!   every non-empty `fields` (an empty record and one empty field both
//!   encode to the empty line, which decodes to one empty field).
//! - `encode::<F, _>(&decode::<F>(line)?) == line` for every line that decodes:
//!   each record has exactly one encoding.
//! - After [`insert_sorted`] on a sorted `Vec`, it is still sorted, one longer,
//!   and holds the old elements plus the new one, after any equal elements.
//! - A `RequestBuilder<NoUrl>` has no `send()`, a `RequestBuilder<HasUrl>` has
//!   no `url()`, and no other crate can implement [`Format`] or [`State`].
//!
//! # Part 1 · `compile_fail` guarantees
//!
//! Each guarantee below has a `compile_fail` doctest (the misuse) and a
//! positive control (a doctest that must compile and differs only in the
//! misuse), on the item named in the last column:
//!
//! | Guarantee | Misuse | Error | Doctests on |
//! | --- | --- | --- | --- |
//! | typestate | `send()` before `url()`; `url()` twice | E0599 | [`RequestBuilder`] |
//! | `#[must_use]` builder | a builder chain that ends in `;` | `unused_must_use` lint | [`RequestBuilder`] |
//! | sealed trait | `impl Format for Pipe`; naming the seal | E0277; E0603 | [`Format`] |
//! | `#[non_exhaustive]` enum | a `match` without `_` | E0004 | [`Status`] |
//! | `#[non_exhaustive]` struct | a struct expression; a pattern without `..` | E0639; E0638 | [`Opts`] |
//! | variance and markers (`38_variance`) | a short-lived `&str` in a `Recorder<&'static str>`; an id whose marker is `PhantomData<T>` | E0597; E0277 | [`Recorder`], [`Id`] |
//! | Send / Sync (`quizzes/quiz5`) | `RwLock<Cell<_>>: Sync`, `MutexGuard: Send`, ... | E0277 | [`assert_send`], [`assert_sync`], [`Recorder`] |
//! | no `DerefMut` (`deref1`) | `name.make_ascii_uppercase()` | E0596 | [`Username`] |
//! | macro arity (`macros6`) | `max_of!()` | none (no code) | [`max_of!`](crate::max_of) |
//!
//! Three rules make a `compile_fail` doctest trustworthy:
//!
//! - **Pair it with a positive control.** A `compile_fail` example passes when
//!   it fails to compile for ANY reason: a typo, a missing `use`, a renamed
//!   item. The control proves that everything except the misuse compiles.
//! - **Pin the error code**, as in ```` ```compile_fail,E0599 ````. Stable
//!   rustdoc ignores the code (a wrong one passes on 1.96). rustdoc checks it
//!   only when it runs as a nightly build, and then reports a mismatch as
//!   "Some expected error codes were not found". So check the codes with
//!   `cargo +nightly test --doc`; in CI the `deep-dive-miri` job checks them
//!   as well, since `cargo +nightly miri test` runs the doctests with nightly
//!   rustdoc. The codes can change between compiler versions (E0412 became
//!   E0425 for "cannot find type"), so a nightly failure can also mean the
//!   code needs updating. For full compiler output, use snapshot tests with
//!   the `trybuild` crate.
//! - **A lint has no error code.** `#[must_use]` only warns, and rustdoc's
//!   `#![allow(unused)]` would even silence the warning, so the doctest must
//!   `#![deny(unused_must_use)]` itself; the code-less `max_of!()` misuse is
//!   in the same position. There the positive control is the only evidence of
//!   why the example fails.
//!
//! # Part 2 · integration tests and proptest
//!
//! `tests/api_surface.rs` uses the API the way a dependent crate does: it
//! needs a `_` arm to match a [`Status`], starts an [`Opts`] from `Default`,
//! implements the unsealed [`ToFields`] for its own type, uses a [`Recorder`]
//! as a test double, and counts drops to show that [`insert_sorted`] drops
//! each element exactly once. `tests/common/mod.rs` holds the helpers: a
//! `common/mod.rs` file is not a test crate of its own, as a `common.rs` would
//! be, so it is compiled only into the test files that declare `mod common;`.
//!
//! The `props` module of `tests/api_surface.rs` (compiled only under
//! `--cfg proptest`, like `loom_lab` under `--cfg loom`, so a plain
//! `cargo test` never builds proptest) states the invariants above as
//! properties: two round trips for [`encode`] and [`decode`], and a
//! sortedness check for [`insert_sorted`]. [`planted`] holds a twin of each
//! function with a one-line bug. Two tests run the properties
//! against those twins with a fixed seed and assert the counterexample that
//! proptest shrinks the random failure down to: a record holding a lone
//! backslash, and `v = [0]`, `x = 1`. They pass because they find the bug.
//! The property tests use no failure persistence, so a failing run never
//! writes a `proptest-regressions/` file into the repository.
//!
//! # Part 3 · semver drill
//!
//! Which changes need a new major version (a breaking change) and which fit
//! in a minor release? A change is major when some downstream code that
//! compiles today would stop compiling. Each row names the doctest that shows
//! the downstream code in question.
//!
//! | Change in the next release | Semver | Downstream code that breaks | Doctest |
//! | --- | --- | --- | --- |
//! | add a variant to an exhaustive enum (`Method::Put`) | major | an exhaustive `match` (E0004) | [`Method`] |
//! | add a variant to a `#[non_exhaustive]` enum (`Status::Informational`) | minor | none: a `match` outside the crate already needs `_` | [`Status`] |
//! | add a `pub` field to a struct whose fields are all `pub` | major | struct expressions (E0063) and patterns without `..` (E0027) | [`Request`] |
//! | add a field to a `#[non_exhaustive]` struct | minor | none: no struct expressions (E0639) or exhaustive patterns (E0638) outside the crate | [`Opts`] |
//! | add `#[non_exhaustive]` to an existing enum, or to a struct with no private fields | major | every exhaustive `match`, struct expression and pattern of it | [`Status`], [`Opts`] |
//! | add a method without a default to an unsealed trait | major | every downstream impl (E0046) | [`ToFields`] |
//! | add a method with a default to an unsealed trait | minor, possibly breaking (see below) | a call becomes ambiguous if a downstream trait has a method of that name (E0034) | [`ToFields`] |
//! | add an item, even without a default, to a sealed trait | minor, if the trait stays dyn-compatible (see below) | no impl: none can exist downstream (a new name can still collide, as in the row above) | [`Format`] |
//! | tighten a bound (`insert_sorted<T: Ord + Clone>`) | major | callers whose type misses the new bound (E0277) | [`insert_sorted`] |
//! | loosen a bound on a function | minor | none: every type that met the old bound meets the new one | [`insert_sorted`] |
//! | lose an auto trait (an `Rc` field in the builder) | major | code that sends it to another thread (E0277) | [`RequestBuilder`] |
//! | move `send()` to every state | minor, yet the typestate guarantee is gone | none; only the `compile_fail` doctest notices | [`RequestBuilder`] |
//!
//! Traits add two more rules. Loosening a bound on a trait method is major
//! for an unsealed trait: an impl may not demand more than its trait, so
//! implementors that kept the old bound stop compiling (E0276). (A sealed
//! trait's impls all live in its crate and change with it.) And, sealed or
//! not, a new item that costs a trait its dyn compatibility (a generic method
//! without `where Self: Sized`, an associated const) is major: every
//! `dyn Trait` downstream stops compiling (E0038). [`ToFields`] is
//! dyn-compatible, so that applies to it; [`Format`] never was, because of
//! its associated const. The rules are in the Cargo book's "SemVer
//! compatibility" chapter. `cargo-semver-checks` automates part of the table:
//! it compares the rustdoc JSON of the old and the new version and reports,
//! for example, `enum_variant_added`, `constructible_struct_adds_field`,
//! `trait_method_added` and `auto_trait_impl_removed` as major. It checks
//! API shapes, not behavior, and not every shape: version 0.50 reports
//! nothing for the tightened bound on `insert_sorted`. The doctests check
//! the guarantees behind them.
//!
//! # Sharpest question
//!
//! *How do you test that misuse of your API fails to compile, and which of
//! these changes are semver-major: adding a variant to a
//! non-`#[non_exhaustive]` enum, adding a method to a sealed vs an unsealed
//! trait?*
//!
//! A `#[test]` cannot say "this must not compile": a test that does not
//! compile fails the whole build. Write a `compile_fail` doctest (or a
//! `trybuild` test), pair it with a positive control that differs only in the
//! misuse, and pin the error code, which only nightly rustdoc checks. Adding a
//! variant to an exhaustive enum is major, because a downstream exhaustive
//! `match` stops compiling (E0004); with `#[non_exhaustive]` it is minor.
//! Adding a method without a default to an unsealed trait is major, because
//! every downstream impl lacks it (E0046); with a default it is minor but can
//! make calls ambiguous. On a sealed trait a new method is minor even without
//! a default: no impl outside the crate can exist to break. (Sealed or not, a
//! method that costs the trait its dyn compatibility is major.)
//!
//! # Run it
//!
//! ```text
//! # This module's unit tests, then its doctests (a bare `cargo test NAME`
//! # skips the doctests; `cargo test -- NAME` runs every kind):
//! cargo test --manifest-path deep-dive/Cargo.toml --lib api_surface
//! cargo test --manifest-path deep-dive/Cargo.toml --doc api_surface
//! # The integration tests:
//! cargo test --manifest-path deep-dive/Cargo.toml --test api_surface
//! # The doctests with their error codes checked (nightly rustdoc):
//! cargo +nightly test --manifest-path deep-dive/Cargo.toml --doc api_surface
//! # The integration tests plus the property tests:
//! RUSTFLAGS="--cfg proptest" cargo test --manifest-path deep-dive/Cargo.toml --test api_surface
//! # Semver drill: classify an uncommitted edit against the last commit
//! # (cargo install cargo-semver-checks --locked), from the repository root:
//! cargo semver-checks --manifest-path deep-dive/Cargo.toml --baseline-rev HEAD
//! ```
//!
//! # Try it
//!
//! - Remove a guarantee and watch its `compile_fail` doctest go red with
//!   "Test compiled successfully, but it's marked `compile_fail`": delete
//!   `#[non_exhaustive]` from [`Status`], drop the `sealed::FormatSeal`
//!   supertrait from [`Format`], or move `send()` into the
//!   `impl<S: State> RequestBuilder<S>` block.
//! - Break a `compile_fail` doctest for the wrong reason: in the first one on
//!   [`RequestBuilder`], misspell the import as `RequestBulder`. Stable
//!   `cargo test --doc` still passes; `cargo +nightly test --doc` reports
//!   "Some expected error codes were not found: [\"E0599\"]". Changing the
//!   code to `E0999` does the same. (Misspelling `send` would not: a typo in
//!   a method name is E0599 too. A code narrows the reason down; the positive
//!   control covers the rest.)
//! - Change [`Id`]'s marker from `PhantomData<fn() -> T>` to `PhantomData<T>`:
//!   the first doctest on `Id` fails with E0277, since `Id<Rc<String>>` is no
//!   longer `Send` (the local `RowId` pair below it shows the same rule).
//! - Semver drill: add `Put` to [`Method`] and the doctest on `Method` fails
//!   with E0004 (major). Add `Informational` to [`Status`]: no doctest or
//!   integration test breaks (minor); only this module's own unit test, whose
//!   `match` needs no `_` inside the crate, wants the new arm. Add a method
//!   without a default to [`ToFields`] and both the doctest on `ToFields` and
//!   `tests/api_surface.rs` fail with E0046; add one to [`Format`] (and to
//!   the impls in this file) and nothing outside this file notices.
//! - Let cargo-semver-checks classify the same edits: leave one uncommitted
//!   and run the `cargo semver-checks` command under "Run it", which compares
//!   the working tree with the last commit. Version 0.50 fails `Put` with
//!   `enum_variant_added` and the required `ToFields` method with
//!   `trait_method_added` ("semver requires new major version"), and passes
//!   `Informational` and the `Format` method. It also passes
//!   `T: Ord + Clone` on [`insert_sorted`], which is major all the same:
//!   there only the doctest on `insert_sorted` notices (E0277).
//! - In `tests/api_surface.rs`, run the `props::watch_*` tests with
//!   `--ignored` to read proptest's own report ("minimal failing input: ...")
//!   for each planted bug. Widen the record strategy in `tests/common/mod.rs`
//!   from `1..8` fields to `0..8` and the round trip fails on `[]`, the empty
//!   record this format cannot tell apart from one empty field. The escaping
//!   demonstration fails too: its counterexample now shrinks to `[]`, a
//!   smaller input that fails for that other reason. Shrinking minimizes the
//!   input, not the bug.
//! - Explore beyond the fixed seed: `PROPTEST_RNG_SEED=12345` picks another
//!   one, `PROPTEST_CASES=10000` runs more cases (a few seconds in a debug
//!   build). Both change the property tests only; the two demonstrations pin
//!   their config.

use std::cell::{Ref, RefCell};
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

// ===================================================================
// Sealing
// ===================================================================

// A private module with `pub` traits: other crates cannot name them, so they
// cannot implement them, and as supertraits they seal `Format` and `State`.
// Inside this crate the module is visible everywhere, so the seal protects
// the crate from OTHER crates only (a unit test below implements `Format`).
mod sealed {
    pub trait FormatSeal {}
    pub trait StateSeal {}
}

// ===================================================================
// The record codec: a sealed trait, an unsealed trait, a round trip
// ===================================================================

/// The escape character of every [`Format`]: it protects a separator or
/// another escape inside a field.
pub const ESCAPE: char = '\\';

/// A line format for [`encode`] and [`decode`]. **Sealed**: other crates can
/// name it, bound on it and call it, but cannot implement it.
///
/// The seal lets [`encode`] rely on a fact that no signature states: the
/// separator is never [`ESCAPE`]. An outside `Format` with `SEPARATOR = '\\'`
/// would break the round trip, and it cannot exist. It also makes adding an
/// item to `Format` a minor change.
///
/// Using the trait from another crate compiles (the positive control):
///
/// ```
/// use deep_dive::api_surface::{Csv, Format, Tsv, encode};
///
/// struct Pipe;
///
/// fn separator_of<F: Format>() -> char {
///     F::SEPARATOR
/// }
/// assert_eq!(separator_of::<Csv>(), ',');
/// assert_eq!(encode::<Tsv, _>(&["a", "b"]), "a\tb");
/// ```
///
/// Implementing it does not: E0277, "the trait bound
/// `Pipe: api_surface::sealed::FormatSeal` is not satisfied", with the note
/// "`Format` is a \"sealed trait\"":
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::{Csv, Format, Tsv, encode};
///
/// struct Pipe;
///
/// impl Format for Pipe {
///     const SEPARATOR: char = '|';
/// }
/// ```
///
/// Nor can the seal itself be named: E0603, "module `sealed` is private":
///
/// ```compile_fail,E0603
/// use deep_dive::api_surface::{Csv, Format, Tsv, encode};
///
/// struct Pipe;
///
/// impl deep_dive::api_surface::sealed::FormatSeal for Pipe {}
/// ```
pub trait Format: sealed::FormatSeal {
    /// The character between two fields. Never [`ESCAPE`].
    const SEPARATOR: char;
}

/// Comma-separated fields, escaped with a backslash. (RFC 4180 CSV quotes
/// fields instead; this is the same idea with a simpler escape.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Csv;

/// Tab-separated fields, escaped with a backslash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tsv;

impl sealed::FormatSeal for Csv {}
impl Format for Csv {
    const SEPARATOR: char = ',';
}

impl sealed::FormatSeal for Tsv {}
impl Format for Tsv {
    const SEPARATOR: char = '\t';
}

/// Joins `fields` with `F::SEPARATOR`, putting an [`ESCAPE`] before every
/// separator and every escape inside a field.
///
/// ```
/// use deep_dive::api_surface::{Csv, decode, encode};
///
/// let line = encode::<Csv, _>(&["a,b", r"c\d", ""]);
/// assert_eq!(line, r"a\,b,c\\d,");
/// assert_eq!(decode::<Csv>(&line).unwrap(), ["a,b", r"c\d", ""]);
/// ```
///
/// An empty slice encodes to the empty line, which decodes to one empty field:
/// like CSV, the format cannot tell `[]` from `[""]`.
pub fn encode<F: Format, S: AsRef<str>>(fields: &[S]) -> String {
    let mut line = String::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            line.push(F::SEPARATOR);
        }
        for c in field.as_ref().chars() {
            if c == F::SEPARATOR || c == ESCAPE {
                line.push(ESCAPE);
            }
            line.push(c);
        }
    }
    line
}

/// Splits a line written by [`encode`] back into its fields. Strict: an
/// [`ESCAPE`] must be followed by the separator or another escape, so every
/// record has exactly one encoding.
///
/// ```
/// use deep_dive::api_surface::{DecodeError, Tsv, decode};
///
/// assert_eq!(decode::<Tsv>("a,b\tc").unwrap(), ["a,b", "c"]);
/// let error = decode::<Tsv>(r"a\").unwrap_err();
/// assert_eq!(error, DecodeError::DanglingEscape { at: 1 });
/// ```
pub fn decode<F: Format>(line: &str) -> Result<Vec<String>, DecodeError> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.char_indices();
    while let Some((at, c)) = chars.next() {
        if c == ESCAPE {
            match chars.next() {
                Some((_, next)) if next == F::SEPARATOR || next == ESCAPE => field.push(next),
                Some((_, found)) => return Err(DecodeError::UnknownEscape { at, found }),
                None => return Err(DecodeError::DanglingEscape { at }),
            }
        } else if c == F::SEPARATOR {
            fields.push(mem::take(&mut field));
        } else {
            field.push(c);
        }
    }
    fields.push(field);
    Ok(fields)
}

/// Why [`decode`] rejected a line. `#[non_exhaustive]`, so a later release may
/// add a variant as a minor change; callers match it with a `_` arm.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The line ends with a lone [`ESCAPE`] at byte offset `at`.
    DanglingEscape { at: usize },
    /// The [`ESCAPE`] at byte offset `at` is followed by `found`, which needs
    /// no escaping.
    UnknownEscape { at: usize, found: char },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::DanglingEscape { at } => {
                write!(f, "the line ends with a lone escape at byte {at}")
            }
            DecodeError::UnknownEscape { at, found } => {
                write!(f, "the escape at byte {at} is followed by {found:?}")
            }
        }
    }
}

impl Error for DecodeError {}

/// A type that can be written as one record. **Not sealed**: implement it for
/// your own types. That freedom has a price: a new method without a default
/// would break every downstream impl (E0046), so it could only come in a major
/// release. The example below is such an impl.
///
/// ```
/// use deep_dive::api_surface::{Csv, ToFields, encode_record};
///
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// impl ToFields for Point {
///     fn to_fields(&self) -> Vec<String> {
///         vec![self.x.to_string(), self.y.to_string()]
///     }
/// }
///
/// assert_eq!(encode_record::<Csv>(&Point { x: 3, y: -4 }), "3,-4");
/// ```
pub trait ToFields {
    /// The record's fields, in order.
    fn to_fields(&self) -> Vec<String>;
}

/// [`encode`]s the fields of any [`ToFields`] type.
pub fn encode_record<F: Format>(record: &impl ToFields) -> String {
    encode::<F, _>(&record.to_fields())
}

/// Inserts `x` into the sorted `v`, after any elements equal to it, so `v`
/// stays sorted: O(log n) comparisons plus the O(n) shift.
///
/// The bound is only `T: Ord`. Tightening it (to `T: Ord + Clone`, say) would
/// be a major change, because code like this would stop compiling (E0277):
///
/// ```
/// use deep_dive::api_surface::insert_sorted;
///
/// // `Ord`, but not `Clone`.
/// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
/// struct Job(u32);
///
/// let mut queue = vec![Job(1), Job(5)];
/// insert_sorted(&mut queue, Job(3));
/// assert_eq!(queue, [Job(1), Job(3), Job(5)]);
/// ```
pub fn insert_sorted<T: Ord>(v: &mut Vec<T>, x: T) {
    // The first index whose element is greater than `x`: after every equal
    // element, and `v.len()` when there is none.
    let at = v.partition_point(|y| *y <= x);
    v.insert(at, x);
}

/// Deliberately broken twins of [`encode`] and [`insert_sorted`], kept for the
/// property-test demonstrations in `tests/api_surface.rs`. Each bug is one
/// line. Never use them outside a test.
pub mod planted {
    use super::{ESCAPE, Format};

    /// PLANTED BUG: escapes the separator but not [`ESCAPE`] itself, so a
    /// field that contains a backslash decodes to something else, or not at
    /// all. The minimal counterexample is one field holding a lone backslash.
    pub fn encode_without_escaping_the_escape<F: Format, S: AsRef<str>>(fields: &[S]) -> String {
        let mut line = String::new();
        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                line.push(F::SEPARATOR);
            }
            for c in field.as_ref().chars() {
                // BUG: the fix is `c == F::SEPARATOR || c == ESCAPE`.
                if c == F::SEPARATOR {
                    line.push(ESCAPE);
                }
                line.push(c);
            }
        }
        line
    }

    /// PLANTED BUG: when no element is greater than `x`, `position` returns
    /// `None`, and `unwrap_or(0)` puts `x` at the front instead of the end.
    /// The minimal counterexample is `v = [0]`, `x = 1`.
    pub fn insert_sorted_largest_at_front<T: Ord>(v: &mut Vec<T>, x: T) {
        // BUG: the fix is `unwrap_or(v.len())`.
        let at = v.iter().position(|y| *y > x).unwrap_or(0);
        v.insert(at, x);
    }
}

// ===================================================================
// The request builder: typestate, #[must_use], #[non_exhaustive]
// ===================================================================

/// The HTTP method of a [`Request`]. An **exhaustive** enum: other crates may
/// match it without a `_` arm, so adding a variant is a major change.
///
/// ```
/// use deep_dive::api_surface::Method;
///
/// // Compiles today. Add `Method::Put` and this `match` is E0004,
/// // "non-exhaustive patterns": the change is semver-major.
/// fn is_safe(method: Method) -> bool {
///     match method {
///         Method::Get => true,
///         Method::Post => false,
///     }
/// }
/// assert!(is_safe(Method::default()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    #[default]
    Get,
    Post,
}

/// The class of an HTTP status code. `#[non_exhaustive]`: 1xx codes are not
/// classified yet ([`Status::from_code`] returns `None`), and adding an
/// `Informational` variant later must not break anyone.
///
/// Outside this crate a `match` needs a `_` arm (the positive control):
///
/// ```
/// use deep_dive::api_surface::Status;
///
/// fn retryable(status: Status) -> bool {
///     match status {
///         Status::ServerError => true,
///         Status::Success | Status::Redirect | Status::ClientError => false,
///         _ => false,
///     }
/// }
/// assert!(retryable(Status::from_code(503).unwrap()));
/// ```
///
/// Without it the `match` is E0004, "non-exhaustive patterns: `_` not
/// covered", although it lists every variant that exists today:
///
/// ```compile_fail,E0004
/// use deep_dive::api_surface::Status;
///
/// fn retryable(status: Status) -> bool {
///     match status {
///         Status::ServerError => true,
///         Status::Success | Status::Redirect | Status::ClientError => false,
///     }
/// }
/// assert!(retryable(Status::from_code(503).unwrap()));
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// 2xx.
    Success,
    /// 3xx.
    Redirect,
    /// 4xx.
    ClientError,
    /// 5xx.
    ServerError,
}

impl Status {
    /// The class of `code`, or `None` for a code this version does not
    /// classify (1xx, or outside 100..=599).
    pub fn from_code(code: u16) -> Option<Status> {
        match code {
            200..=299 => Some(Status::Success),
            300..=399 => Some(Status::Redirect),
            400..=499 => Some(Status::ClientError),
            500..=599 => Some(Status::ServerError),
            _ => None,
        }
    }
}

/// Transport options. `#[non_exhaustive]` with `pub` fields: other crates can
/// read and assign the fields but cannot write a struct expression, so a
/// later release may add a field as a minor change.
///
/// Outside this crate, start from `Default` and assign, and write patterns
/// with `..` (the positive control):
///
/// ```
/// use deep_dive::api_surface::Opts;
///
/// let mut opts = Opts::default();
/// opts.retries = 5;
/// let Opts { timeout_ms, retries, .. } = opts;
/// assert_eq!((timeout_ms, retries), (30_000, 5));
/// ```
///
/// A struct expression is E0639, "cannot create non-exhaustive struct using
/// struct expression", even with functional update syntax:
///
/// ```compile_fail,E0639
/// use deep_dive::api_surface::Opts;
///
/// let opts = Opts { retries: 5, ..Opts::default() };
/// let Opts { timeout_ms, retries, .. } = opts;
/// assert_eq!((timeout_ms, retries), (30_000, 5));
/// ```
///
/// A pattern without `..` is E0638, "`..` required with struct marked as
/// non-exhaustive":
///
/// ```compile_fail,E0638
/// use deep_dive::api_surface::Opts;
///
/// let mut opts = Opts::default();
/// opts.retries = 5;
/// let Opts { timeout_ms, retries } = opts;
/// assert_eq!((timeout_ms, retries), (30_000, 5));
/// ```
///
/// proptest's `Config` takes the other route, which keeps functional update
/// syntax: it is not `#[non_exhaustive]` but has a hidden
/// `pub _non_exhaustive: ()` field ("Needs to be public so FRU syntax can be
/// used", says its source). Users can write
/// `Config { cases: 42, ..Config::default() }`, and only code that names the
/// hidden field can list every field.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opts {
    /// How long to wait for a response, in milliseconds.
    pub timeout_ms: u32,
    /// How many times to retry a failed request.
    pub retries: u8,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            timeout_ms: 30_000,
            retries: 3,
        }
    }
}

/// A finished request: plain data with every field `pub` and no
/// `#[non_exhaustive]`, so other crates may build and destructure it with
/// struct syntax, and adding a field is a major change.
///
/// ```
/// use deep_dive::api_surface::{Method, Opts, Request};
///
/// // Both statements stop compiling when a field is added: the struct
/// // expression with E0063 "missing field", the pattern with E0027.
/// let request = Request {
///     method: Method::Get,
///     url: "https://example.com/".to_string(),
///     headers: Vec::new(),
///     opts: Opts::default(),
/// };
/// let Request { method, url, headers, opts } = request;
/// assert_eq!((method, url.as_str()), (Method::Get, "https://example.com/"));
/// assert!(headers.is_empty() && opts == Opts::default());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub opts: Opts,
}

/// The typestate of a [`RequestBuilder`]: [`NoUrl`] or [`HasUrl`]. Sealed like
/// [`Format`], so generic code over `S: State` knows it sees only these two.
pub trait State: sealed::StateSeal {}

/// The state before `url()`: a builder with no URL, and no `send()`.
#[derive(Debug)]
pub struct NoUrl;

/// The state after `url()`: `send()` is available, `url()` is not.
#[derive(Debug)]
pub struct HasUrl;

impl sealed::StateSeal for NoUrl {}
impl sealed::StateSeal for HasUrl {}
impl State for NoUrl {}
impl State for HasUrl {}

/// A typestate request builder, as in `47_type_level/typestate1`: `url()`
/// exists only on `RequestBuilder<NoUrl>`, `send()` only on
/// `RequestBuilder<HasUrl>`, and the state costs no memory.
///
/// The positive control:
///
/// ```
/// use deep_dive::api_surface::{Method, RequestBuilder};
///
/// let request = RequestBuilder::default()
///     .header("accept", "text/csv")
///     .url("https://example.com/export")
///     .method(Method::Post)
///     .send();
/// assert_eq!(request.url, "https://example.com/export");
/// ```
///
/// `send()` before `url()` is E0599, "no method named `send` found for struct
/// `RequestBuilder<NoUrl>`":
///
/// ```compile_fail,E0599
/// use deep_dive::api_surface::{Method, RequestBuilder};
///
/// let request = RequestBuilder::default()
///     .header("accept", "text/csv")
///     .method(Method::Post)
///     .send();
/// assert_eq!(request.url, "https://example.com/export");
/// ```
///
/// So is a second `url()`, "no method named `url` found for struct
/// `RequestBuilder<HasUrl>`":
///
/// ```compile_fail,E0599
/// use deep_dive::api_surface::{Method, RequestBuilder};
///
/// let request = RequestBuilder::default()
///     .header("accept", "text/csv")
///     .url("https://example.com/export")
///     .url("https://example.com/other")
///     .method(Method::Post)
///     .send();
/// assert_eq!(request.url, "https://example.com/export");
/// ```
///
/// The builder is `#[must_use]`: a chain that ends in `;` builds a builder and
/// drops it, configuring nothing. That is only a warning, and rustdoc compiles
/// examples under `#![allow(unused)]`, so each of these two examples denies the
/// lint itself. A lint has no error code, so the positive control is all that
/// shows the second one fails for the right reason:
///
/// ```
/// #![deny(unused_must_use)]
/// use deep_dive::api_surface::RequestBuilder;
///
/// let builder = RequestBuilder::default().header("accept", "text/csv");
/// ```
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use deep_dive::api_surface::RequestBuilder;
///
/// RequestBuilder::default().header("accept", "text/csv");
/// ```
///
/// Auto traits are part of the API too. Nobody wrote `impl Send` for the
/// builder, yet callers may rely on it, so a new field that is not `Send` (an
/// `Rc`, say) would be a major change. A positive doctest pins it:
///
/// ```
/// use deep_dive::api_surface::{HasUrl, NoUrl, RequestBuilder};
/// use deep_dive::api_surface::{assert_send, assert_sync};
///
/// assert_send::<RequestBuilder<NoUrl>>();
/// assert_sync::<RequestBuilder<HasUrl>>();
/// ```
#[must_use = "a builder does nothing until `send()` is called"]
#[derive(Debug)]
pub struct RequestBuilder<S> {
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    opts: Opts,
    state: PhantomData<S>,
}

// Written by hand for `NoUrl` only: `#[derive(Default)]` would generate
// `impl<S: Default> Default for RequestBuilder<S>`, which cannot pick the
// state by itself (see `typestate1`).
impl Default for RequestBuilder<NoUrl> {
    fn default() -> Self {
        RequestBuilder {
            method: Method::default(),
            url: String::new(),
            headers: Vec::new(),
            opts: Opts::default(),
            state: PhantomData,
        }
    }
}

// Every state has these. Each returns `Self`, so the state stays the same.
impl<S: State> RequestBuilder<S> {
    /// Adds a header; the same name may appear more than once.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets the method (`GET` unless set).
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Sets the transport options ([`Opts::default`] unless set).
    pub fn opts(mut self, opts: Opts) -> Self {
        self.opts = opts;
        self
    }
}

impl RequestBuilder<NoUrl> {
    /// The transition: consumes the `NoUrl` builder and returns one of a
    /// different type. There is no `url()` on `RequestBuilder<HasUrl>`.
    pub fn url(self, url: impl Into<String>) -> RequestBuilder<HasUrl> {
        RequestBuilder {
            method: self.method,
            url: url.into(),
            headers: self.headers,
            opts: self.opts,
            state: PhantomData,
        }
    }
}

impl RequestBuilder<HasUrl> {
    /// Builds the request. Only a `HasUrl` builder has `send()`, so it cannot
    /// fail and returns a plain [`Request`].
    pub fn send(self) -> Request {
        Request {
            method: self.method,
            url: self.url,
            headers: self.headers,
            opts: self.opts,
        }
    }
}

// ===================================================================
// Type contracts: auto traits, variance, Deref, a macro
// ===================================================================

/// Compiles only if `T: Send`. A call proves that a type IS `Send`; the proof
/// that one is NOT is a `compile_fail` doctest. Each pair below asks both
/// questions about one type (`quizzes/quiz5` has the rules).
///
/// A `MutexGuard` is `Sync` when `T` is, but never `Send`: dropping it unlocks
/// the mutex, which must happen on the thread that locked it (quiz5 Q9):
///
/// ```
/// use deep_dive::api_surface::{assert_send, assert_sync};
/// use std::sync::MutexGuard;
///
/// assert_sync::<MutexGuard<'static, u32>>();
/// ```
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::{assert_send, assert_sync};
/// use std::sync::MutexGuard;
///
/// assert_send::<MutexGuard<'static, u32>>();
/// ```
///
/// A trait object has only the auto traits it names: `dyn Fn() + Send` is
/// `Send`, not `Sync` (Q14):
///
/// ```
/// use deep_dive::api_surface::{assert_send, assert_sync};
///
/// assert_send::<Box<dyn Fn() + Send>>();
/// ```
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::{assert_send, assert_sync};
///
/// assert_sync::<Box<dyn Fn() + Send>>();
/// ```
pub fn assert_send<T: ?Sized + Send>() {}

/// Compiles only if `T: Sync`. See [`assert_send`].
///
/// A `Mutex<T>` hands the `T` to one thread at a time, so it is `Sync` when
/// `T` is only `Send`. An `RwLock<T>` lets several readers hold `&T` at once,
/// so it also needs `T: Sync`, which `Cell` is not (Q2, Q3):
///
/// ```
/// use deep_dive::api_surface::assert_sync;
/// use std::cell::Cell;
/// use std::sync::{Mutex, RwLock};
///
/// assert_sync::<Mutex<Cell<i32>>>();
/// ```
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::assert_sync;
/// use std::cell::Cell;
/// use std::sync::{Mutex, RwLock};
///
/// assert_sync::<RwLock<Cell<i32>>>();
/// ```
///
/// A `Receiver` can move to another thread, but only one thread may receive
/// at a time (Q12):
///
/// ```
/// use deep_dive::api_surface::{assert_send, assert_sync};
/// use std::sync::mpsc::Receiver;
///
/// assert_send::<Receiver<String>>();
/// ```
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::{assert_send, assert_sync};
/// use std::sync::mpsc::Receiver;
///
/// assert_sync::<Receiver<String>>();
/// ```
pub fn assert_sync<T: ?Sized + Sync>() {}

/// A typed row id, as in `38_variance/variance1`: an `Id<User>` is not an
/// `Id<Order>`, yet at run time it is only a `u64`. The marker is
/// `PhantomData<fn() -> T>`, "can produce a `T`": covariant, and `Send + Sync`
/// whatever `T` is, because a function pointer is.
///
/// ```
/// use deep_dive::api_surface::{Id, assert_send};
/// use std::rc::Rc;
///
/// // `Rc` is not `Send`, but an id into a table of `Rc`s is.
/// assert_send::<Id<Rc<String>>>();
///
/// // Covariant: an id of `&'static str` rows can stand in for an id of
/// // shorter-lived rows.
/// fn shorten<'a>(id: Id<&'static str>) -> Id<&'a str> {
///     id
/// }
/// assert_eq!(shorten(Id::new(7)).raw(), 7);
/// ```
///
/// That the marker decides `Send` is itself a guarantee worth pinning, and
/// proving that a type is NOT `Send` takes a `compile_fail` doctest (the
/// graded `variance1` settles for a `Probe` const that its tests compare at
/// run time). Two local copies of `Id` that differ only in the marker: with
/// `PhantomData<fn() -> T>` the id is `Send` for every `T` (the positive
/// control),
///
/// ```
/// use deep_dive::api_surface::assert_send;
/// use std::marker::PhantomData;
/// use std::rc::Rc;
///
/// struct RowId<T> {
///     raw: u64,
///     _row: PhantomData<fn() -> T>,
/// }
/// assert_send::<RowId<Rc<String>>>();
/// ```
///
/// and with `PhantomData<T>`, "owns a `T`", it inherits the `Rc`'s `!Send`:
/// E0277, "`Rc<String>` cannot be sent between threads safely":
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::assert_send;
/// use std::marker::PhantomData;
/// use std::rc::Rc;
///
/// struct RowId<T> {
///     raw: u64,
///     _row: PhantomData<T>,
/// }
/// assert_send::<RowId<Rc<String>>>();
/// ```
pub struct Id<T> {
    raw: u64,
    _row: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// The id with the given raw value.
    pub const fn new(raw: u64) -> Self {
        Id {
            raw,
            _row: PhantomData,
        }
    }

    /// The raw value.
    pub const fn raw(&self) -> u64 {
        self.raw
    }
}

/// A recording test double, as in `50_testing_seams`: it records calls through
/// `&self`, so it keeps its log in a `RefCell<Vec<T>>`. That one field decides
/// two properties nobody wrote down, and the doctests pin both.
///
/// **Variance.** `RefCell<T>` is invariant in `T`, so `Recorder<T>` is too: a
/// `Recorder<&'static str>` cannot be used as a `Recorder<&'a str>`, so a
/// short-lived `&str` cannot be recorded into it. Were it covariant, like a
/// raw-pointer type with a `PhantomData<T>` marker, `record(&local)` would
/// compile, and the recorder, still typed `Recorder<&'static str>`, would hold
/// a dangling reference once `local` is dropped. Safe code gets the variance
/// right by itself; unsafe code picks it with its marker (`38_variance`).
///
/// ```
/// use deep_dive::api_surface::Recorder;
///
/// let recorder: Recorder<&'static str> = Recorder::new();
/// let local = String::from("short-lived");
/// recorder.record("static");
/// assert_eq!(recorder.calls().len(), 1);
/// ```
///
/// E0597, "`local` does not live long enough":
///
/// ```compile_fail,E0597
/// use deep_dive::api_surface::Recorder;
///
/// let recorder: Recorder<&'static str> = Recorder::new();
/// let local = String::from("short-lived");
/// recorder.record(&local);
/// assert_eq!(recorder.calls().len(), 1);
/// ```
///
/// **Auto traits.** `RefCell` is `Send` but not `Sync`, so a recorder can move
/// to another thread but not be shared between threads (a double for code
/// that shares it would keep a `Mutex<Vec<T>>` instead):
///
/// ```
/// use deep_dive::api_surface::{Recorder, assert_send, assert_sync};
///
/// assert_send::<Recorder<String>>();
/// ```
///
/// ```compile_fail,E0277
/// use deep_dive::api_surface::{Recorder, assert_send, assert_sync};
///
/// assert_sync::<Recorder<String>>();
/// ```
#[derive(Debug)]
pub struct Recorder<T> {
    calls: RefCell<Vec<T>>,
}

impl<T> Recorder<T> {
    /// An empty recorder.
    pub fn new() -> Self {
        Recorder {
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Appends one call to the log.
    pub fn record(&self, call: T) {
        self.calls.borrow_mut().push(call);
    }

    /// The calls so far, oldest first. Drop the guard before the next
    /// `record`, which would otherwise panic with "RefCell already borrowed".
    pub fn calls(&self) -> Ref<'_, [T]> {
        Ref::map(self.calls.borrow(), Vec::as_slice)
    }
}

impl<T> Default for Recorder<T> {
    fn default() -> Self {
        Recorder::new()
    }
}

/// Why [`Username::new`] rejected a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidUsername;

/// A validated login name, as in `45_sized_deref/deref1`: 3 to 16 characters
/// from `a-z`, `0-9` and `_`, starting with a letter. `Deref<Target = str>`
/// lends callers every `&self` method of `str`, and the missing `DerefMut`
/// keeps them from changing the text behind `Username::new`'s back.
///
/// ```
/// use deep_dive::api_surface::Username;
///
/// let mut name = Username::new("ferris").unwrap();
/// let _ = name.to_ascii_uppercase(); // returns a new `String`
/// assert_eq!(name.len(), 6);
/// ```
///
/// Changing it in place needs a `&mut str`: E0596, "cannot borrow data in
/// dereference of `Username` as mutable":
///
/// ```compile_fail,E0596
/// use deep_dive::api_surface::Username;
///
/// let mut name = Username::new("ferris").unwrap();
/// let _ = name.make_ascii_uppercase(); // changes the text in place
/// assert_eq!(name.len(), 6);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    /// Checks `raw` and wraps a copy of it.
    pub fn new(raw: &str) -> Result<Username, InvalidUsername> {
        let valid_length = (3..=16).contains(&raw.len());
        let valid_chars = raw.chars().enumerate().all(|(i, c)| match c {
            'a'..='z' => true,
            '0'..='9' | '_' => i > 0,
            _ => false,
        });
        if valid_length && valid_chars {
            Ok(Username(raw.to_string()))
        } else {
            Err(InvalidUsername)
        }
    }
}

impl Deref for Username {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// The largest of one or more expressions, each evaluated exactly once: the
/// fixed `max!` of `48_macros_deep/macros6`. `#[macro_export]` puts it at the
/// crate root, `deep_dive::max_of!`, whatever module defines it, and the
/// recursive call goes through `$crate::` so that it works from other crates.
///
/// ```
/// assert_eq!(deep_dive::max_of!(3, 9, 4), 9);
/// ```
///
/// With no argument no rule matches, and rustc says "unexpected end of macro
/// invocation", an error without a code, so only the positive control above
/// shows this example fails for the right reason:
///
/// ```compile_fail
/// assert_eq!(deep_dive::max_of!(), 9);
/// ```
#[macro_export]
macro_rules! max_of {
    ($x:expr $(,)?) => {
        $x
    };
    ($x:expr, $($rest:expr),+ $(,)?) => {{
        let first = $x;
        let rest = $crate::max_of!($($rest),+);
        if first > rest { first } else { rest }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inside the defining crate `#[non_exhaustive]` has no effect: this match
    // needs no `_` arm (one would even be an `unreachable_patterns` warning),
    // and a struct expression for `Opts` compiles. The doctests on `Status`
    // and `Opts` show the rules that apply everywhere else.
    #[test]
    fn non_exhaustive_does_not_apply_inside_the_crate() {
        let class = |status: Status| match status {
            Status::Success => "2xx",
            Status::Redirect => "3xx",
            Status::ClientError => "4xx",
            Status::ServerError => "5xx",
        };
        assert_eq!(Status::from_code(404).map(class), Some("4xx"));

        let opts = Opts {
            timeout_ms: 1_000,
            retries: 0,
        };
        assert_ne!(opts, Opts::default());
    }

    // The seal is only a seal for other crates: the private module is visible
    // here, so a test can add a `Format`. (With `SEPARATOR = ESCAPE` this impl
    // would break the round trip, which is exactly what sealing prevents
    // outside the crate.)
    #[test]
    fn the_seal_is_open_inside_the_crate() {
        struct Pipe;
        impl sealed::FormatSeal for Pipe {}
        impl Format for Pipe {
            const SEPARATOR: char = '|';
        }
        let line = encode::<Pipe, _>(&["a|b", "c"]);
        assert_eq!(line, r"a\|b|c");
        assert_eq!(decode::<Pipe>(&line).unwrap(), ["a|b", "c"]);
    }

    #[test]
    fn each_format_escapes_only_its_own_separator() {
        let fields = ["a,b", "c\td", r"e\f"];
        assert_eq!(encode::<Csv, _>(&fields), "a\\,b,c\td,e\\\\f");
        assert_eq!(encode::<Tsv, _>(&fields), "a,b\tc\\\td\te\\\\f");
        assert_eq!(decode::<Csv>(&encode::<Csv, _>(&fields)).unwrap(), fields);
        assert_eq!(decode::<Tsv>(&encode::<Tsv, _>(&fields)).unwrap(), fields);
    }

    #[test]
    fn the_empty_line_is_one_empty_field() {
        assert_eq!(encode::<Csv, &str>(&[]), "");
        assert_eq!(encode::<Csv, _>(&[""]), "");
        assert_eq!(decode::<Csv>("").unwrap(), [""]);
        assert_eq!(decode::<Csv>(",").unwrap(), ["", ""]);
    }

    #[test]
    fn decode_reports_bad_escapes_by_byte_offset() {
        // `é` is two bytes, so the escape after it is at byte 3.
        assert_eq!(
            decode::<Csv>(r"aé\"),
            Err(DecodeError::DanglingEscape { at: 3 })
        );
        assert_eq!(
            decode::<Csv>(r"a\tb"),
            Err(DecodeError::UnknownEscape { at: 1, found: 't' })
        );
        // A tab needs no escape in CSV, but it does in TSV.
        assert_eq!(decode::<Tsv>("a\\\tb").unwrap(), ["a\tb"]);
        let message = decode::<Csv>(r"a\").unwrap_err().to_string();
        assert_eq!(message, "the line ends with a lone escape at byte 1");
    }

    #[test]
    fn insert_sorted_puts_equal_elements_after_existing_ones() {
        // Compared by the first element only (`Eq` and `Ord` agree), so the
        // second element tells equal keys apart.
        struct Keyed(u32, char);
        impl PartialEq for Keyed {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
        impl Eq for Keyed {}
        impl PartialOrd for Keyed {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Keyed {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }
        let mut v = Vec::new();
        for item in [Keyed(2, 'a'), Keyed(1, 'b'), Keyed(2, 'c'), Keyed(3, 'd')] {
            insert_sorted(&mut v, item);
        }
        let order: String = v.iter().map(|k| k.1).collect();
        assert_eq!(order, "bacd");
    }

    // Deterministic demonstrations of the planted bugs, on the minimal
    // counterexamples that proptest shrinks to (`tests/api_surface.rs` finds
    // them from random inputs under `--cfg proptest`).
    #[test]
    fn the_planted_encoder_loses_a_lone_backslash() {
        let line = planted::encode_without_escaping_the_escape::<Csv, _>(&[r"\"]);
        assert_eq!(line, r"\");
        assert_eq!(
            decode::<Csv>(&line),
            Err(DecodeError::DanglingEscape { at: 0 })
        );
        // Without a backslash, the planted encoder agrees with the real one.
        let fields = ["a,b", "c"];
        assert_eq!(
            planted::encode_without_escaping_the_escape::<Csv, _>(&fields),
            encode::<Csv, _>(&fields)
        );
    }

    #[test]
    fn the_planted_insert_puts_the_largest_element_first() {
        let mut v = vec![0];
        planted::insert_sorted_largest_at_front(&mut v, 1);
        assert_eq!(v, [1, 0]);
        assert!(!v.is_sorted());
        // Anything that is not the largest lands in the right place.
        let mut w = vec![0, 2];
        planted::insert_sorted_largest_at_front(&mut w, 1);
        assert_eq!(w, [0, 1, 2]);
    }

    #[test]
    fn status_classes_follow_the_first_digit() {
        assert_eq!(Status::from_code(200), Some(Status::Success));
        assert_eq!(Status::from_code(301), Some(Status::Redirect));
        assert_eq!(Status::from_code(499), Some(Status::ClientError));
        assert_eq!(Status::from_code(503), Some(Status::ServerError));
        assert_eq!(Status::from_code(101), None);
        assert_eq!(Status::from_code(600), None);
    }

    #[test]
    fn username_checks_length_and_characters() {
        assert!(Username::new("ferris_42").is_ok());
        assert_eq!(Username::new("ab"), Err(InvalidUsername));
        assert_eq!(Username::new("Ferris"), Err(InvalidUsername));
        assert_eq!(Username::new("4ferris"), Err(InvalidUsername));
        assert_eq!(Username::new("a_very_long_username"), Err(InvalidUsername));
        let name = Username::new("ferris").unwrap();
        assert!(name.starts_with("fer"));
    }

    #[test]
    fn max_of_evaluates_each_argument_once() {
        let mut calls = 0;
        let mut next = |n: u32| {
            calls += 1;
            n
        };
        assert_eq!(crate::max_of!(next(3), next(9), next(4)), 9);
        assert_eq!(calls, 3);
    }
}
