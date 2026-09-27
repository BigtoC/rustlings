//! Lab · serde at the boundary and additive Cargo features
//!
//! A service reads its config from a TOML file. This crate is the part that
//! turns that untrusted text into types the rest of the program can trust,
//! and it is also a small study of how one crate compiles differently under
//! different Cargo features. `README.md` is the full lab text; this is the
//! short version.
//!
//! | Module | Part | What it reveals |
//! | --- | --- | --- |
//! | [`serde_boundary`] | `serde_boundary` | An `Email` newtype that serde can only build through `TryFrom<String>`; `deny_unknown_fields`, `rename_all` and defaults |
//! | [`features_and_cfg`] | `features_and_cfg` | An optional `serde` feature gated with `cfg_attr`, and a `build.rs` cfg with `rustc-check-cfg` and `rerun-if-env-changed` |
//!
//! Compare with `35_error_design/err3` (a `TryFrom` newtype that can't hold
//! an invalid value: `try_from = "String"` makes serde use exactly that) and
//! `47_type_level/builder1` (a hand-written `Default` and a validating
//! builder: the same "one checked way in" idea, for values built in code).
//!
//! The sharpest question, "How do you guarantee that a deserialized `Email`
//! field is always valid, and why must Cargo features be additive?":
//!
//! - Give `Email` a private field and one checked constructor,
//!   `TryFrom<String>`, and tell serde to use it with
//!   `#[serde(try_from = "String")]`. A plain `#[derive(Deserialize)]` would
//!   build the struct directly and skip the check (`UncheckedEmail` shows
//!   it). Every `Email` in the program then went through the check once.
//! - Cargo builds one copy of a dependency for all the crates in the build
//!   that use it, with the union of the features they asked for. A feature
//!   that takes something away (the `lenient` feature here drops
//!   `deny_unknown_fields`) is therefore imposed on every other user of the
//!   crate in that build, including ones that relied on the check.
//!
//! Run it:
//!
//! ```text
//! cargo test --manifest-path config-lab/Cargo.toml
//! cargo test --manifest-path config-lab/Cargo.toml --no-default-features
//! cargo test --manifest-path config-lab/Cargo.toml --all-features
//! cargo run --manifest-path config-lab/Cargo.toml --example report
//! ```
//!
//! Try it (the "Try it" section of `README.md` has nine experiments, each
//! with what to expect). Three of them, from `config-lab/`:
//!
//! ```text
//! # The two typo tests that `lenient` switches off fail: the typo is ignored.
//! cargo test --features lenient -- --ignored
//! # The fallback path, as on a compiler older than 1.91.
//! CONFIG_LAB_FORCE_FALLBACK=1 cargo run --example report
//! # After replacing a `cfg_attr(feature = "serde", derive(..))` with a plain
//! # derive: `cargo test` passes, this fails with E0433.
//! cargo build --no-default-features
//! ```

pub mod features_and_cfg;
pub mod serde_boundary;

#[cfg(feature = "serde")]
pub use serde_boundary::UncheckedEmail;
pub use serde_boundary::{
    Email, EmailError, EmailErrorKind, LogLevel, ServerConfig, ServiceConfig,
};
