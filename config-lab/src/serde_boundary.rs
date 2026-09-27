//! Part 1 · `serde_boundary`: validate once, where the data comes in.
//!
//! A config file is untrusted input. The goal is that no code past
//! `ServiceConfig::from_toml_str` ever has to ask "is this email valid?" or
//! "did the file say what the operator meant?". Five serde attributes do that:
//!
//! | Attribute | On | What it guarantees |
//! | --- | --- | --- |
//! | `try_from = "String"` | [`Email`] | Deserializing goes through `TryFrom<String>`, the one checked constructor |
//! | `into = "String"` | [`Email`] | Serializing writes the plain string (it clones the `Email` first) |
//! | `deny_unknown_fields` | both structs | A misspelled key is an error instead of being ignored |
//! | `rename_all = "kebab-case"` | both structs | The file says `admin-email`, the code says `admin_email` |
//! | `default` | a field, or a whole struct | A missing key takes a value from `Default`, never a guess |
//!
//! Invariants:
//!
//! - An [`Email`] always holds exactly one `@`, a valid local part and a
//!   lowercase domain. Its field is private, so `TryFrom<String>` (and
//!   `FromStr`, which calls it) is the only way to build one, and with
//!   `try_from` serde has to use it too.
//! - Every key in the file is one the code knows. A typo is rejected, with
//!   the list of keys that would have been accepted.
//! - A missing optional key means the documented default: an empty
//!   `alert-emails` list, and [`ServerConfig::default`] for `[server]` and for
//!   each key missing inside it.
//!
//! Every serde attribute sits in a `cfg_attr` whose condition requires
//! `feature = "serde"`, so the types, the validation and the defaults all
//! exist without serde (see `features_and_cfg`). Only the TOML functions and
//! `UncheckedEmail` (a `#[cfg(feature = "serde")]` item) disappear.
//!
//! Broken on purpose, kept for comparison:
//!
//! - `UncheckedEmail`: the same newtype with a plain
//!   `#[derive(Deserialize)]`. The derived code sits inside this crate, so
//!   the private field does not stop it, and it accepts any string.
//! - The `lenient` feature drops `deny_unknown_fields`. That makes it
//!   non-additive: see "Why features must be additive" in the README.

use crate::features_and_cfg::truncate_on_char_boundary;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// Longest address accepted, in bytes (the usual SMTP path limit).
const MAX_LEN: usize = 254;
/// Longest local part (before the `@`), in bytes.
const MAX_LOCAL_LEN: usize = 64;
/// Longest domain label (between dots), in bytes.
const MAX_LABEL_LEN: usize = 63;
/// How much of a rejected address an error message repeats back.
const SHOWN_LEN: usize = 64;

/// An email address that has been checked.
///
/// The rules are deliberately simple (this lab is about where validation
/// happens, not about RFC 5322): one `@`; a local part of 1 to 64 ASCII
/// letters, digits and `. _ % + -`, with no dot at either end and no `..`;
/// a domain of two or more dot-separated labels of letters, digits and `-`
/// (1 to 63 bytes each, no `-` at either end); at most 254 bytes in total.
/// Domains are case-insensitive, so the domain is stored in lowercase; the
/// local part is kept as written.
///
/// The only ways in are [`TryFrom<String>`] and [`FromStr`]:
///
/// ```
/// use config_lab::Email;
///
/// let email = Email::try_from("Ops@Example.COM".to_string()).unwrap();
/// assert_eq!(email.as_str(), "Ops@example.com");
/// assert!(Email::try_from("not an email".to_string()).is_err());
/// ```
///
/// The field is private, so code outside this crate cannot skip the check:
/// E0423 "cannot initialize a tuple struct which contains private fields"
/// (through the path `config_lab::serde_boundary::Email` it is E0603 "tuple
/// struct constructor `Email` is private"). Stable rustdoc does not check the
/// error code of a `compile_fail` doctest, so the doctest above is this one's
/// positive control: the same import and the same string, built through
/// `try_from` instead of the private constructor.
///
/// ```compile_fail,E0423
/// use config_lab::Email;
///
/// let email = Email("not an email".to_string());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct Email(String);

impl Email {
    /// The whole address, with the domain in lowercase.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part before the `@`, as written.
    pub fn local_part(&self) -> &str {
        self.split().0
    }

    /// The part after the `@`, in lowercase.
    pub fn domain(&self) -> &str {
        self.split().1
    }

    fn split(&self) -> (&str, &str) {
        self.0
            .split_once('@')
            .expect("an Email always contains one '@'")
    }
}

/// The checked constructor, and the one `#[serde(try_from = "String")]` calls.
/// It takes the `String` by value, so a valid address is stored without a
/// copy, and a rejected one travels back inside the error.
impl TryFrom<String> for Email {
    type Error = EmailError;

    fn try_from(mut s: String) -> Result<Self, Self::Error> {
        match check(&s) {
            Ok(at) => {
                s[at + 1..].make_ascii_lowercase();
                Ok(Email(s))
            }
            Err(kind) => Err(EmailError { input: s, kind }),
        }
    }
}

impl FromStr for Email {
    type Err = EmailError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Email::try_from(s.to_owned())
    }
}

/// What `#[serde(into = "String")]` calls when serializing, after cloning.
impl From<Email> for String {
    fn from(email: Email) -> String {
        email.0
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Returns the byte index of the `@`, or why `s` is not an address.
fn check(s: &str) -> Result<usize, EmailErrorKind> {
    if s.len() > MAX_LEN {
        return Err(EmailErrorKind::TooLong);
    }
    let Some((local, domain)) = s.split_once('@') else {
        return Err(EmailErrorKind::MissingAt);
    };
    if domain.contains('@') {
        return Err(EmailErrorKind::MultipleAt);
    }
    if !is_valid_local_part(local) {
        return Err(EmailErrorKind::LocalPart);
    }
    if !is_valid_domain(domain) {
        return Err(EmailErrorKind::Domain);
    }
    Ok(local.len())
}

fn is_valid_local_part(local: &str) -> bool {
    (1..=MAX_LOCAL_LEN).contains(&local.len())
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._%+-".contains(&b))
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
}

fn is_valid_domain(domain: &str) -> bool {
    let is_valid_label = |label: &str| {
        (1..=MAX_LABEL_LEN).contains(&label.len())
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    };
    domain.contains('.') && domain.split('.').all(is_valid_label)
}

/// Why an address was rejected, plus the rejected string itself.
///
/// A real error type (compare `35_error_design/err3`): `Display` is the
/// message serde puts into the TOML error, and [`EmailError::into_input`]
/// hands the caller's `String` back, like `FromUtf8Error::into_bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailError {
    input: String,
    kind: EmailErrorKind,
}

impl EmailError {
    /// Which rule the address broke.
    pub fn kind(&self) -> EmailErrorKind {
        self.kind
    }

    /// The rejected string, unchanged.
    pub fn into_input(self) -> String {
        self.input
    }
}

/// The rule an address broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmailErrorKind {
    /// More than 254 bytes.
    TooLong,
    /// No `@` at all.
    MissingAt,
    /// A second `@`.
    MultipleAt,
    /// The part before the `@` is empty or too long, or has a bad character
    /// or a misplaced dot.
    LocalPart,
    /// The part after the `@` is not two or more valid labels.
    Domain,
}

impl fmt::Display for EmailErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EmailErrorKind::TooLong => "longer than 254 bytes",
            EmailErrorKind::MissingAt => "missing '@'",
            EmailErrorKind::MultipleAt => "more than one '@'",
            EmailErrorKind::LocalPart => "bad local part (before the '@')",
            EmailErrorKind::Domain => "bad domain (after the '@')",
        })
    }
}

impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Repeat at most 64 bytes of the input. Slicing at a fixed byte index
        // would panic inside a multi-byte character.
        let shown = truncate_on_char_boundary(&self.input, SHOWN_LEN);
        let ellipsis = if shown.len() < self.input.len() {
            "..."
        } else {
            ""
        };
        write!(f, "invalid email {shown:?}{ellipsis}: {}", self.kind)
    }
}

// No `source()`: the rejection has no lower-level cause.
impl Error for EmailError {}

/// How much the service logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// The `[server]` table. Every key is optional.
///
/// The container-level `#[serde(default)]` fills each missing key from
/// [`ServerConfig::default`], which is written by hand: `#[derive(Default)]`
/// would give port 0 and no connections (compare `47_type_level/builder1`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case", default))]
// The `lenient` anti-pattern: `not(feature = ..)` is the tell-tale sign of a
// feature that takes something away.
#[cfg_attr(
    all(feature = "serde", not(feature = "lenient")),
    serde(deny_unknown_fields)
)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    pub log_level: LogLevel,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1024,
            log_level: LogLevel::Info,
        }
    }
}

/// A whole service config file.
///
/// ```toml
/// name = "billing"
/// admin-email = "ops@example.com"
/// alert-emails = ["oncall@example.com"]   # optional, default []
///
/// [server]                                # optional, and so is every key
/// port = 9000
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[cfg_attr(
    all(feature = "serde", not(feature = "lenient")),
    serde(deny_unknown_fields)
)]
pub struct ServiceConfig {
    /// Required.
    pub name: String,
    /// Required, and valid because [`Email`] is.
    pub admin_email: Email,
    /// Optional: a missing key means an empty list (`Vec::default()`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub alert_emails: Vec<Email>,
    /// Optional: a missing table means [`ServerConfig::default`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub server: ServerConfig,
}

#[cfg(feature = "serde")]
impl ServiceConfig {
    /// Parses and validates a TOML config file.
    ///
    /// A published library would wrap `toml::de::Error` in an error type of
    /// its own, so that a new major version of toml would not be a breaking
    /// change for its callers. Here the toml type is kept for its `message()`
    /// and `span()`, which the tests use.
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Writes the config back out as TOML.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }
}

/// BROKEN ON PURPOSE: [`Email`] with a plain `#[derive(Deserialize)]`.
///
/// The field is just as private, yet any string deserializes, because the
/// derived impl is generated inside this crate and may build `Self(..)`
/// directly. A derive is not a constructor you wrote, so it does not run
/// your checks: route it through one with `#[serde(try_from = "String")]`,
/// or write `Deserialize` by hand (deserialize a `String`, then call
/// `Email::try_from` and map the error with `serde::de::Error::custom`, which
/// is what the attribute generates). The integration test
/// `derived_deserialize_skips_validation` shows it accepting "not an email".
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct UncheckedEmail(String);

#[cfg(feature = "serde")]
impl UncheckedEmail {
    /// Whatever the file said.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email(s: &str) -> Result<Email, EmailErrorKind> {
        s.parse::<Email>().map_err(|e| e.kind())
    }

    #[test]
    fn accepts_ordinary_addresses() {
        for s in [
            "ops@example.com",
            "first.last+tag@mail.example.co.uk",
            "a_b%c-d@x-y.io",
            "0@0.aa",
        ] {
            assert_eq!(email(s).map(|e| e.to_string()), Ok(s.to_string()), "{s}");
        }
    }

    #[test]
    fn rejects_each_rule_with_its_kind() {
        use EmailErrorKind::*;
        let long_local = format!("{}@example.com", "a".repeat(65));
        let long_label = format!("ops@{}.com", "a".repeat(64));
        let too_long = format!("ops@{}.com", ["a"; 125].join("."));
        assert!(too_long.len() > 254);
        let cases: [(&str, EmailErrorKind); 16] = [
            ("", MissingAt),
            ("ops.example.com", MissingAt),
            ("ops@@example.com", MultipleAt),
            ("a@b@example.com", MultipleAt),
            ("@example.com", LocalPart),
            (".ops@example.com", LocalPart),
            ("ops.@example.com", LocalPart),
            ("o..ps@example.com", LocalPart),
            ("o ps@example.com", LocalPart),
            ("ops\u{e9}@example.com", LocalPart),
            (&long_local, LocalPart),
            ("ops@", Domain),
            ("ops@localhost", Domain),
            ("ops@-example.com", Domain),
            ("ops@example..com", Domain),
            (&long_label, Domain),
        ];
        for (input, kind) in cases {
            assert_eq!(email(input), Err(kind), "{input:?}");
        }
        assert_eq!(email(&too_long), Err(TooLong));
    }

    #[test]
    fn domain_is_lowercased_local_part_is_kept() {
        let e = email("Ops.Team@Mail.Example.COM").unwrap();
        assert_eq!(e.local_part(), "Ops.Team");
        assert_eq!(e.domain(), "mail.example.com");
        assert_eq!(e.as_str(), "Ops.Team@mail.example.com");
        // Normalizing twice changes nothing.
        assert_eq!(email(e.as_str()), Ok(e.clone()));
    }

    #[test]
    fn a_valid_string_is_moved_in_not_copied() {
        let s = String::from("ops@example.com");
        let ptr = s.as_ptr();
        let e = Email::try_from(s).unwrap();
        assert_eq!(e.as_str().as_ptr(), ptr);
        // And back out again, through the impl `into = "String"` uses.
        let back = String::from(e);
        assert_eq!(back.as_ptr(), ptr);
    }

    #[test]
    fn a_rejected_string_comes_back_in_the_error() {
        let err = Email::try_from(String::from("ops.example.com")).unwrap_err();
        assert_eq!(err.kind(), EmailErrorKind::MissingAt);
        assert_eq!(err.into_input(), "ops.example.com");
    }

    #[test]
    fn the_message_names_the_input_and_the_rule() {
        let err = "ops.example.com".parse::<Email>().unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"invalid email "ops.example.com": missing '@'"#
        );
    }

    #[test]
    fn the_message_repeats_at_most_64_bytes_and_never_splits_a_char() {
        // 30 three-byte characters (90 bytes): the cut at byte 64 falls inside
        // one, so the message shows 21 of them (63 bytes), where `&input[..64]`
        // would panic.
        let input = "\u{20ac}".repeat(30);
        let err = input.parse::<Email>().unwrap_err();
        let expected = format!("invalid email {:?}...: missing '@'", "\u{20ac}".repeat(21));
        assert_eq!(err.to_string(), expected);
    }

    #[test]
    fn server_defaults_are_the_documented_ones() {
        let d = ServerConfig::default();
        assert_eq!(
            (d.host.as_str(), d.port, d.max_connections, d.log_level),
            ("127.0.0.1", 8080, 1024, LogLevel::Info)
        );
    }

    #[test]
    fn everything_but_toml_works_without_serde() {
        // Nothing here needs the `serde` feature: this test also runs under
        // `cargo test --no-default-features`.
        let config = ServiceConfig {
            name: "billing".to_string(),
            admin_email: "ops@example.com".parse().unwrap(),
            alert_emails: vec![],
            server: ServerConfig {
                port: 9000,
                ..ServerConfig::default()
            },
        };
        assert_eq!(config.admin_email.domain(), "example.com");
        assert_eq!(config.server.host, "127.0.0.1");
    }
}
