//! Helpers shared by the integration tests in `tests/`.
//!
//! Cargo turns each `tests/*.rs` file (and each `tests/*/main.rs`) into a test
//! crate of its own. This file is neither, so it is compiled only into the
//! test crates that declare `mod common;`. (As `tests/common.rs` it would
//! become a test crate too, one that runs zero tests.) Every such crate gets
//! its own copy of the module, so a helper that one of them never uses is
//! `dead_code` there: keep each helper used by every file that declares it.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

/// Owned copies of `items`.
pub fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// The ids of the [`Tracked`] values dropped so far, in drop order.
pub type DropLog = Rc<RefCell<Vec<u32>>>;

/// A payload that orders by `key` alone and logs its `id` when dropped, so a
/// test can check where equal keys end up and that every value is dropped
/// exactly once. `Eq` and `Ord` both look at `key` only, so they agree
/// (`44_trait_contracts`).
#[derive(Debug)]
pub struct Tracked {
    pub key: u32,
    pub id: u32,
    log: DropLog,
}

impl Tracked {
    pub fn new(key: u32, id: u32, log: &DropLog) -> Self {
        Tracked {
            key,
            id,
            log: Rc::clone(log),
        }
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.log.borrow_mut().push(self.id);
    }
}

impl PartialEq for Tracked {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Tracked {}

impl PartialOrd for Tracked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tracked {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

/// Configs and input strategies for the property tests. proptest is a
/// dev-dependency only under `--cfg proptest`, so this module exists only
/// then (see `deep-dive/Cargo.toml`).
#[cfg(proptest)]
pub mod strategies {
    use proptest::prelude::*;
    use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

    /// The seed of every run unless `PROPTEST_RNG_SEED` picks another one.
    pub const SEED: u64 = 20_260_927;

    /// The config of the property tests: proptest's defaults (256 cases,
    /// overridable with `PROPTEST_CASES`), a fixed seed so that every run
    /// tests the same cases, and no failure persistence, so a failing run
    /// never writes a `proptest-regressions/` file into the repository.
    pub fn config() -> Config {
        // `Config::default()` has already applied the `PROPTEST_*` variables.
        let base = Config::default();
        let rng_seed = match base.rng_seed {
            RngSeed::Random => RngSeed::Fixed(SEED),
            chosen => chosen,
        };
        // Functional update syntax works on `Config` from outside proptest:
        // it is not `#[non_exhaustive]`, it has a hidden
        // `pub _non_exhaustive: ()` field instead (see the doctests on
        // `api_surface::Opts`).
        Config {
            rng_seed,
            failure_persistence: None,
            ..base
        }
    }

    /// The config of the planted-bug demonstrations. Everything that decides
    /// which counterexample they reach is pinned, whatever the `PROPTEST_*`
    /// variables say: the RNG algorithm and seed, the case count, and the
    /// shrink limits (with `PROPTEST_MAX_SHRINK_ITERS=3`, shrinking would stop
    /// early and leave a bigger counterexample than the one they assert).
    pub fn demo_config() -> Config {
        Config {
            cases: 256,
            rng_algorithm: RngAlgorithm::ChaCha,
            rng_seed: RngSeed::Fixed(SEED),
            max_shrink_iters: u32::MAX,
            max_shrink_time: 0,
            failure_persistence: None,
            ..Config::default()
        }
    }

    /// One field made of the characters that matter to the codec: both
    /// separators, the escape and one ordinary letter, so that every case
    /// exercises the escaping rules.
    pub fn field() -> impl Strategy<Value = String> {
        "[a,\t\\\\]{0,6}"
    }

    /// One to seven fields. (With `0..8`, the round trip fails on `[]`: the
    /// empty record encodes to the same line as one empty field.)
    pub fn record() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(field(), 1..8)
    }

    /// A line of codec characters, most of which do not decode.
    pub fn line() -> impl Strategy<Value = String> {
        "[a,\t\\\\]{0,12}"
    }

    /// A sorted vector of small numbers and one more number to insert.
    pub fn sorted_and_value() -> impl Strategy<Value = (Vec<u32>, u32)> {
        let sorted = prop::collection::vec(0u32..100, 0..16).prop_map(|mut v| {
            v.sort_unstable();
            v
        });
        (sorted, 0u32..100)
    }
}
