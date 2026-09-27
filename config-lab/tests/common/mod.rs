//! Proptest generators shared by the integration tests.
//!
//! Each file in `tests/` is its own crate and compiles its own copy of this
//! module, using only some of it; hence the `dead_code` allowance.
#![allow(dead_code)]

use config_lab::{Email, LogLevel, ServerConfig, ServiceConfig};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;

/// The config every property test here uses (the unit tests in
/// `src/features_and_cfg.rs` repeat it).
pub fn proptest_config() -> ProptestConfig {
    // `default()` already holds any PROPTEST_* environment overrides.
    let default = ProptestConfig::default();
    ProptestConfig {
        // Never write proptest-regressions/ files into the source tree; a
        // failure prints the shrunk input instead.
        failure_persistence: None,
        // The same 256 cases on every run, so a test that passed in CI never
        // fails later on inputs nobody can reproduce. Set PROPTEST_RNG_SEED
        // to another number to explore other inputs.
        rng_seed: match default.rng_seed {
            RngSeed::Random => RngSeed::Fixed(0x5EED),
            chosen => chosen,
        },
        ..default
    }
}

/// Strings that follow `Email`'s rules, with mixed-case domains.
pub fn address() -> impl Strategy<Value = String> {
    // No dot first or last, never two in a row: each dot comes with the
    // letter or digit that follows it. At most 1 + 2 * 20 = 41 bytes.
    let local = "[A-Za-z0-9]([A-Za-z0-9_%+-]|\\.[A-Za-z0-9]){0,20}";
    let label = "[A-Za-z0-9]([A-Za-z0-9-]{0,10}[A-Za-z0-9])?";
    let tld = "[A-Za-z]{2,6}";
    (local, prop::collection::vec(label, 1..3), tld)
        .prop_map(|(local, labels, tld)| format!("{local}@{}.{tld}", labels.join(".")))
}

pub fn email() -> impl Strategy<Value = Email> {
    address().prop_map(|s| Email::try_from(s).expect("address() makes valid addresses"))
}

pub fn log_level() -> impl Strategy<Value = LogLevel> {
    prop_oneof![
        Just(LogLevel::Debug),
        Just(LogLevel::Info),
        Just(LogLevel::Warn),
        Just(LogLevel::Error),
    ]
}

/// Any config the types can hold. Names and hosts are arbitrary Unicode
/// (quotes, newlines, control characters), which TOML has to escape.
pub fn service_config() -> impl Strategy<Value = ServiceConfig> {
    let server = (any::<String>(), any::<u16>(), any::<u32>(), log_level()).prop_map(
        |(host, port, max_connections, log_level)| ServerConfig {
            host,
            port,
            max_connections,
            log_level,
        },
    );
    (
        any::<String>(),
        email(),
        prop::collection::vec(email(), 0..4),
        server,
    )
        .prop_map(|(name, admin_email, alert_emails, server)| ServiceConfig {
            name,
            admin_email,
            alert_emails,
            server,
        })
}
