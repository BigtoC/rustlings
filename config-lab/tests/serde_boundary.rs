//! The TOML boundary, tested from outside the crate, the way a service would
//! use it. Needs the `serde` feature (`required-features` in Cargo.toml).

mod common;

use config_lab::{Email, LogLevel, ServerConfig, ServiceConfig, UncheckedEmail};
use proptest::prelude::*;

const MINIMAL: &str = r#"
name = "billing"
admin-email = "ops@example.com"
"#;

/// Parses `input`, expecting an error, and returns its message plus the text
/// its span points at.
fn rejection(input: &str) -> (String, String) {
    let err = ServiceConfig::from_toml_str(input).expect_err("the input should be rejected");
    let span = err.span().expect("toml reports where the error is");
    (err.message().to_string(), input[span].to_string())
}

#[test]
fn missing_optional_keys_take_their_defaults() {
    let config = ServiceConfig::from_toml_str(MINIMAL).unwrap();
    assert_eq!(config.name, "billing");
    assert_eq!(config.admin_email.as_str(), "ops@example.com");
    assert!(config.alert_emails.is_empty());
    assert_eq!(config.server, ServerConfig::default());
}

#[test]
fn a_partial_table_keeps_the_other_defaults() {
    let input = format!("{MINIMAL}\n[server]\nport = 9000\nlog-level = \"warn\"\n");
    let server = ServiceConfig::from_toml_str(&input).unwrap().server;
    assert_eq!(
        server,
        ServerConfig {
            port: 9000,
            log_level: LogLevel::Warn,
            ..ServerConfig::default()
        }
    );
}

#[test]
fn an_invalid_email_is_rejected_with_its_message() {
    let input = "name = \"billing\"\nadmin-email = \"ops.example.com\"\n";
    let (message, at) = rejection(input);
    // serde passes `EmailError`'s `Display` through unchanged.
    assert_eq!(message, r#"invalid email "ops.example.com": missing '@'"#);
    assert_eq!(at, r#""ops.example.com""#);
}

#[test]
fn an_invalid_email_in_a_list_is_rejected_too() {
    let input = format!("{MINIMAL}alert-emails = [\"oncall@example.com\", \"oncall@\"]\n");
    let (message, at) = rejection(&input);
    assert_eq!(
        message,
        r#"invalid email "oncall@": bad domain (after the '@')"#
    );
    // toml 1.x points at the whole array, not at the bad element.
    assert_eq!(at, r#"["oncall@example.com", "oncall@"]"#);
}

#[test]
fn the_rendered_error_shows_line_column_and_message() {
    let input = "name = \"billing\"\nadmin-email = \"ops.example.com\"\n";
    let rendered = ServiceConfig::from_toml_str(input).unwrap_err().to_string();
    assert!(
        rendered.starts_with("TOML parse error at line 2, column 15"),
        "{rendered}"
    );
    assert!(
        rendered.ends_with("invalid email \"ops.example.com\": missing '@'\n"),
        "{rendered}"
    );
}

/// The typo the lab is about: `prot` instead of `port`. Without
/// `deny_unknown_fields` serde skips the unknown key, `port` is missing, the
/// default 8080 applies, and the service quietly listens on the wrong port.
#[test]
#[cfg_attr(
    feature = "lenient",
    ignore = "the non-additive `lenient` feature removes this check; \
              run it with --ignored to watch it fail"
)]
fn a_misspelled_key_is_rejected() {
    let input = format!("{MINIMAL}\n[server]\nprot = 9000\n");
    let (message, at) = rejection(&input);
    assert_eq!(
        message,
        "unknown field `prot`, expected one of `host`, `port`, `max-connections`, `log-level`"
    );
    assert_eq!(at, "prot");
}

/// The same at the top level: `alert-email` for `alert-emails`. Without the
/// check the list is silently empty and nobody gets paged.
#[test]
#[cfg_attr(
    feature = "lenient",
    ignore = "the non-additive `lenient` feature removes this check; \
              run it with --ignored to watch it fail"
)]
fn a_misspelled_top_level_key_is_rejected() {
    let input = format!("{MINIMAL}alert-email = [\"oncall@example.com\"]\n");
    let (message, _) = rejection(&input);
    assert!(
        message.starts_with("unknown field `alert-email`, expected one of"),
        "{message}"
    );
}

/// DEMONSTRATION of the `lenient` anti-pattern: with the feature on, both
/// typos above parse without a word, and the defaults silently win. Any crate
/// in the build can turn the feature on for all the others.
#[test]
#[cfg(feature = "lenient")]
fn lenient_feature_silently_accepts_the_typos() {
    let input =
        format!("{MINIMAL}alert-email = [\"oncall@example.com\"]\n\n[server]\nprot = 9000\n");
    let config = ServiceConfig::from_toml_str(&input).unwrap();
    assert_eq!(config.server.port, 8080);
    assert!(config.alert_emails.is_empty());
}

#[test]
fn a_missing_required_key_is_an_error() {
    let (message, _) = rejection("name = \"billing\"\n");
    assert_eq!(message, "missing field `admin-email`");
}

#[test]
fn an_unknown_log_level_lists_the_choices() {
    let input = format!("{MINIMAL}\n[server]\nlog-level = \"verbose\"\n");
    let (message, at) = rejection(&input);
    assert_eq!(
        message,
        "unknown variant `verbose`, expected one of `debug`, `info`, `warn`, `error`"
    );
    assert_eq!(at, "\"verbose\"");
}

#[test]
fn an_out_of_range_port_is_rejected_by_its_type() {
    let input = format!("{MINIMAL}\n[server]\nport = 70000\n");
    let (message, _) = rejection(&input);
    assert_eq!(message, "invalid value: integer `70000`, expected u16");
}

#[test]
fn a_wrong_type_is_rejected() {
    let (message, _) = rejection("name = \"billing\"\nadmin-email = 5\n");
    assert_eq!(message, "invalid type: integer `5`, expected a string");
}

#[test]
fn round_trip_writes_kebab_case_keys_and_plain_strings() {
    let config = ServiceConfig {
        name: "billing".to_string(),
        admin_email: "Ops@Example.COM".parse().unwrap(),
        alert_emails: vec!["oncall@example.com".parse().unwrap()],
        server: ServerConfig {
            max_connections: 64,
            log_level: LogLevel::Debug,
            ..ServerConfig::default()
        },
    };
    let text = config.to_toml_string().unwrap();
    assert_eq!(
        text,
        r#"name = "billing"
admin-email = "Ops@example.com"
alert-emails = ["oncall@example.com"]

[server]
host = "127.0.0.1"
port = 8080
max-connections = 64
log-level = "debug"
"#
    );
    assert_eq!(ServiceConfig::from_toml_str(&text).unwrap(), config);
}

#[test]
fn the_domain_is_normalized_on_the_way_in() {
    let input = "name = \"billing\"\nadmin-email = \"Ops@Example.COM\"\n";
    let config = ServiceConfig::from_toml_str(input).unwrap();
    assert_eq!(config.admin_email.as_str(), "Ops@example.com");
}

/// DEMONSTRATION of the broken variant: a derived `Deserialize` on the
/// newtype builds it without calling `Email::try_from`.
#[test]
fn derived_deserialize_skips_validation() {
    #[derive(serde::Deserialize)]
    struct Unchecked {
        email: UncheckedEmail,
    }
    #[derive(Debug, serde::Deserialize)]
    struct Checked {
        #[allow(dead_code)]
        email: Email,
    }

    let input = "email = \"not an email\"\n";
    let unchecked: Unchecked = toml::from_str(input).unwrap();
    assert_eq!(unchecked.email.as_str(), "not an email");

    let err = toml::from_str::<Checked>(input).unwrap_err();
    assert_eq!(
        err.message(),
        r#"invalid email "not an email": missing '@'"#
    );
}

proptest! {
    #![proptest_config(common::proptest_config())]

    /// Serialize, parse back, compare. Names and hosts are arbitrary Unicode,
    /// so this also checks toml's string escaping.
    #[test]
    fn every_config_survives_a_round_trip(config in common::service_config()) {
        let text = config.to_toml_string().unwrap();
        let back = ServiceConfig::from_toml_str(&text);
        prop_assert_eq!(back.as_ref(), Ok(&config), "TOML was:\n{}", text);
    }
}
