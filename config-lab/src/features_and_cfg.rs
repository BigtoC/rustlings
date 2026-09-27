//! Part 2 · `features_and_cfg`: what this copy of the crate was compiled with.
//!
//! Three kinds of conditional compilation meet in this crate:
//!
//! - `#[cfg(feature = "serde")]` keeps or removes a whole item. Cargo turns
//!   every enabled feature into `--cfg feature="..."` and declares the
//!   feature names for the `unexpected_cfgs` lint, so a typo such as
//!   `feature = "serd"` is a warning (and a CI failure under `-D warnings`).
//! - `#[cfg_attr(feature = "serde", derive(Serialize))]` keeps the item and
//!   adds an attribute only when the feature is on. That is how
//!   `serde_boundary`'s types exist in every build but implement the serde
//!   traits only when serde is compiled in. A plain `#[derive(Serialize)]`
//!   would break `cargo build --no-default-features`, because the `serde`
//!   crate is not there to name.
//! - `#[cfg(config_lab_has_floor_char_boundary)]` is a custom cfg that
//!   `build.rs` sets after checking the compiler version. Features are what
//!   the crates depending on this one ask for; a build-script cfg is a fact
//!   this crate finds out about its own build, and nobody else sees it.
//!
//! `cfg!(..)` is the expression form of the same test: it becomes the literal
//! `true` or `false` at compile time, and both branches still have to compile.

/// The Cargo features this copy of the crate was compiled with, in the order
/// `Cargo.toml` declares them. `default` is left out: it only turns on
/// `serde`.
///
/// Feature unification means there is one answer per build: if any crate in
/// the dependency graph turns `lenient` on, every crate that uses
/// `config-lab` in that build sees `"lenient"` here.
pub const ENABLED_FEATURES: &[&str] = &[
    #[cfg(feature = "serde")]
    "serde",
    #[cfg(feature = "lenient")]
    "lenient",
];

/// Whether [`truncate_on_char_boundary`] uses std's `str::floor_char_boundary`
/// (`build.rs` found rustc 1.91 or newer) or the hand-written fallback.
pub const USES_STD_FLOOR_CHAR_BOUNDARY: bool = cfg!(config_lab_has_floor_char_boundary);

/// `s` cut to at most `max` bytes, without splitting a UTF-8 character.
///
/// `&s[..max]` panics when `max` falls inside a multi-byte character, which is
/// how a "harmless" error message can crash on user input. `EmailError` uses
/// this to show at most 64 bytes of a rejected address.
///
/// ```
/// use config_lab::features_and_cfg::truncate_on_char_boundary;
///
/// // 'é' is 2 bytes (0xC3 0xA9): byte 2 is inside it, so the cut moves back.
/// assert_eq!(truncate_on_char_boundary("café", 4), "caf");
/// assert_eq!(truncate_on_char_boundary("café", 5), "café");
/// ```
pub fn truncate_on_char_boundary(s: &str, max: usize) -> &str {
    &s[..floor_char_boundary(s, max)]
}

// Rust 1.91+: use std. The cfg guards the call, but clippy cannot know that:
// with `rust-version = "1.85"` its `incompatible_msrv` lint flags every use of
// a newer std API. `#[clippy::msrv]` raises the MSRV clippy assumes for this
// one function, which is exactly what the cfg guarantees.
#[cfg(config_lab_has_floor_char_boundary)]
#[clippy::msrv = "1.91"]
fn floor_char_boundary(s: &str, index: usize) -> usize {
    s.floor_char_boundary(index)
}

// Older compilers, or `CONFIG_LAB_FORCE_FALLBACK=1`.
#[cfg(not(config_lab_has_floor_char_boundary))]
fn floor_char_boundary(s: &str, index: usize) -> usize {
    fallback_floor_char_boundary(s, index)
}

/// The largest char boundary `<= index` (clamped to `s.len()`), the same
/// contract as `str::floor_char_boundary`. Also compiled for tests on new
/// compilers, so the tests can compare it with std's version.
#[cfg(any(test, not(config_lab_has_floor_char_boundary)))]
fn fallback_floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    // 0 is always a boundary, and a char is at most 4 bytes long, so this
    // looks at no more than 4 positions.
    (0..=index)
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::RngSeed;

    #[test]
    fn lenient_turns_on_serde() {
        // `lenient = ["serde"]` in Cargo.toml: a feature can enable others.
        if cfg!(feature = "lenient") {
            assert!(ENABLED_FEATURES.contains(&"serde"));
        }
        assert_eq!(ENABLED_FEATURES.contains(&"serde"), cfg!(feature = "serde"));
    }

    #[test]
    fn truncation_never_splits_a_character() {
        // "aé€🦀": characters of 1, 2, 3 and 4 bytes.
        let s = "a\u{e9}\u{20ac}\u{1f980}";
        assert_eq!(s.len(), 10);
        let (a, ae, aee) = ("a", "a\u{e9}", "a\u{e9}\u{20ac}");
        let cuts: Vec<&str> = (0..=11)
            .map(|max| truncate_on_char_boundary(s, max))
            .collect();
        assert_eq!(cuts, ["", a, a, ae, ae, ae, aee, aee, aee, aee, s, s]);
    }

    #[test]
    fn fallback_handles_the_edges() {
        assert_eq!(fallback_floor_char_boundary("", 0), 0);
        assert_eq!(fallback_floor_char_boundary("", 3), 0);
        assert_eq!(fallback_floor_char_boundary("\u{1f980}", 3), 0);
        assert_eq!(fallback_floor_char_boundary("\u{1f980}", 4), 4);
        assert_eq!(fallback_floor_char_boundary("abc", usize::MAX), 3);
    }

    // The same config as `proptest_config` in `tests/common/mod.rs`: no
    // proptest-regressions/ files, and a fixed seed unless PROPTEST_RNG_SEED
    // picks another.
    fn config() -> ProptestConfig {
        let default = ProptestConfig::default();
        ProptestConfig {
            failure_persistence: None,
            rng_seed: match default.rng_seed {
                RngSeed::Random => RngSeed::Fixed(0x5EED),
                chosen => chosen,
            },
            ..default
        }
    }

    proptest! {
        #![proptest_config(config())]

        #[test]
        fn truncation_is_the_longest_prefix_that_fits(
            s in any::<String>(),
            max in 0usize..64,
        ) {
            let cut = truncate_on_char_boundary(&s, max);
            prop_assert!(s.starts_with(cut));
            prop_assert!(cut.len() <= max);
            // Longest: the next character (if any) would not have fitted.
            if let Some(next) = s[cut.len()..].chars().next() {
                prop_assert!(cut.len() + next.len_utf8() > max);
            }
        }

        // Only on compilers that have std's version to compare with.
        #[cfg(config_lab_has_floor_char_boundary)]
        #[test]
        fn fallback_agrees_with_std(s in any::<String>(), index in 0usize..80) {
            prop_assert_eq!(
                fallback_floor_char_boundary(&s, index),
                floor_char_boundary(&s, index)
            );
        }
    }
}
