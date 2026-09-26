// Traits & Abstraction · Builders and typestate — part 1: `Default` + struct update, then a validating builder (E0308, E0599, clippy::return_self_not_must_use).
//
// "Design a config type for this service" is one of the most common API
// prompts in a Rust interview, and there are two idiomatic answers.
//
// The first is `Default` plus struct update syntax. Give every field a
// sensible value in `impl Default`, and callers name only what they change:
//
//     let config = ServerConfig { port: 9090, ..ServerConfig::default() };
//
// `..base` moves (or copies) every field you did not list out of `base`. It
// needs no code beyond the `impl Default` and is the first thing to reach for
// (its one run-time cost: `base` is built in full, so a field you override is
// built and then dropped). But every field must be visible to the caller
// (E0451 otherwise; from another crate, all of them must be `pub`), nothing
// can be validated (who stops `workers: 0`?), and nothing can be converted on
// the way in. The defaults here are not the field types' own defaults
// (`u16::default()` is 0), so `#[derive(Default)]` would be wrong: this
// `impl Default` is written by hand.
//
// The second answer is a builder: a separate type with one method per setting
// and a `build()` that checks everything and returns
// `Result<ServerConfig, BuildError>`. That is where required fields and
// invariants live. Here the host is required. `Default` binds to loopback,
// which is right on a developer's machine, but a deployed service that
// silently falls back to loopback is unreachable, so the builder refuses to
// guess. Every OPTIONAL setting falls back to `ServerConfig::default()`, so
// the defaults are written down exactly once.
//
// Three details separate a good builder from one that merely works:
//
//   - Setters take `impl Into<String>`, not `String` or `&str`. A `String`
//     parameter makes every caller with a literal write `.to_string()`. A
//     `&str` parameter makes a caller who already OWNS a `String` pay for a
//     copy. With `impl Into<String>` an owned `String` moves in for free (the
//     identity impl `From<T> for T`) and a `&str` is copied once, which is
//     the copy the setter needed anyway. The price: one monomorphized copy of
//     the setter per argument type, and a generic signature in the docs.
//   - A collection setter takes `IntoIterator`, not `Vec<String>`, with
//     `I::Item: Into<String>` converting each item. Then an array of
//     literals, a `Vec<String>`, a `map` adapter and even an `Option` all
//     work. Calling it twice ADDS to the list. A setter that silently
//     replaced the earlier items would surprise every caller.
//   - The builder is `#[must_use]`. Every setter consumes the builder and
//     returns the updated one, so a builder that ends up in a statement of
//     its own is a bug: `ServerConfig::builder().host(h).port(9000);` builds
//     a builder and drops it at the `;`, and nothing is ever configured.
//     (After `builder.port(9000);` the old binding is moved-from, so using it
//     again is at least E0382 "use of moved value".) With `#[must_use]` on
//     the TYPE, every such statement is an `unused_must_use` warning,
//     whichever method produced the value. (std's `thread::Builder` is
//     `#[must_use = "must eventually spawn the thread"]` for the same
//     reason.) This file turns on clippy's
//     `return_self_not_must_use` lint as an error for the builder's `impl`,
//     and rustlings runs clippy after the tests, so the attribute is graded
//     too. That lint only checks public API, which is why the items are `pub`.
//
// How interviewers probe this: "Builder, or `Default` plus struct update?",
// "Why `impl Into<String>`, and what does it cost?", "Consuming (`self`) or
// `&mut self` setters?", "What does `builder.port(9000);` on its own do?".

use std::error::Error;
use std::fmt;
use std::time::Duration;

// Plain data with public fields, so struct update syntax works on it.
#[derive(Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub timeout: Duration,
    pub tags: Vec<String>,
}

impl ServerConfig {
    // The validated way in.
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}

// Written by hand: `#[derive(Default)]` would use each field type's own
// default (an empty host, port 0, 0 workers, a zero timeout).
impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: 4,
            timeout: Duration::from_secs(30),
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    // No host was set, or it was blank.
    MissingHost,
    // `workers(0)`: a pool with no workers would never serve a request.
    ZeroWorkers,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::MissingHost => f.write_str("no host was set"),
            BuildError::ZeroWorkers => f.write_str("workers must be at least 1"),
        }
    }
}

// A real error type, so callers can use `?` with it (`35_error_design`).
impl Error for BuildError {}

// `None` means "not set": `build()` fills it in from `ServerConfig::default()`.
// `#[must_use]` on the TYPE covers every expression that produces a builder,
// so a builder chain that ends in `;` is an `unused_must_use` warning, and
// clippy's `return_self_not_must_use` is satisfied for every setter at once.
// The message is shown in that warning.
#[must_use = "a builder does nothing until you call `build()`"]
#[derive(Debug, Default)]
pub struct ServerConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    workers: Option<usize>,
    timeout: Option<Duration>,
    tags: Vec<String>,
}

// Clippy's `return_self_not_must_use` is in the pedantic group, so it is off
// unless you ask for it. This exercise asks for it, as an error.
#[deny(clippy::return_self_not_must_use)]
impl ServerConfigBuilder {
    // `impl Into<String>` accepts everything `String` has a `From` impl for.
    // An owned `String` goes through the identity `From<T> for T`, so it is
    // moved, not copied (so is the `String` inside a `Cow::Owned`); a `&str`,
    // a `&String` or a `Cow::Borrowed` is copied exactly once.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = Some(workers);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    // `IntoIterator` accepts collections, borrowed collections, adapters and
    // `Option`s alike, and `I::Item: Into<String>` converts each item the way
    // `host` does. `extend` appends, so a second call adds to the list.
    pub fn tags<I>(mut self, tags: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn build(self) -> Result<ServerConfig, BuildError> {
        // `filter` turns a blank host into `None` and `ok_or` turns `None` into
        // the error. The `String` itself is moved through, never copied.
        let host = self
            .host
            .filter(|host| !host.trim().is_empty())
            .ok_or(BuildError::MissingHost)?;
        // The defaults live in one place: whatever the caller did not set
        // comes from `ServerConfig::default()`.
        let defaults = ServerConfig::default();
        let workers = self.workers.unwrap_or(defaults.workers);
        if workers == 0 {
            return Err(BuildError::ZeroWorkers);
        }
        Ok(ServerConfig {
            host,
            port: self.port.unwrap_or(defaults.port),
            workers,
            timeout: self.timeout.unwrap_or(defaults.timeout),
            tags: self.tags,
        })
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    // ---- `Default` and struct update ----

    #[test]
    fn default_is_the_local_development_config() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.workers, 4);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.tags.is_empty());
    }

    #[test]
    fn struct_update_changes_only_the_fields_you_name() {
        let config = ServerConfig {
            port: 9090,
            tags: strings(&["blue"]),
            ..ServerConfig::default()
        };
        assert_eq!(
            config,
            ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 9090,
                workers: 4,
                timeout: Duration::from_secs(30),
                tags: strings(&["blue"]),
            }
        );
    }

    // ---- The builder ----

    #[test]
    fn unset_settings_come_from_default() {
        let config = ServerConfig::builder().host("api.internal").build();
        assert_eq!(
            config,
            Ok(ServerConfig {
                host: "api.internal".to_string(),
                ..ServerConfig::default()
            })
        );
    }

    #[test]
    fn every_setter_is_applied() {
        let config = ServerConfig::builder()
            .host("api.internal")
            .port(443)
            .workers(16)
            .timeout(Duration::from_millis(1500))
            .tags(["prod", "eu-west"])
            .build();
        assert_eq!(
            config,
            Ok(ServerConfig {
                host: "api.internal".to_string(),
                port: 443,
                workers: 16,
                timeout: Duration::from_millis(1500),
                tags: strings(&["prod", "eu-west"]),
            })
        );
    }

    #[test]
    fn setters_come_in_any_order_and_the_last_call_wins() {
        let config = ServerConfig::builder()
            .port(1)
            .host("first.internal")
            .workers(2)
            .port(2)
            .host(String::from("second.internal"))
            .build()
            .unwrap();
        assert_eq!(config.host, "second.internal");
        assert_eq!(config.port, 2);
        assert_eq!(config.workers, 2);
    }

    #[test]
    fn host_accepts_borrowed_and_owned_strings() {
        let name = String::from("by-ref.internal");
        let hosts = [
            ServerConfig::builder().host("literal.internal").build(),
            ServerConfig::builder()
                .host(String::from("owned.internal"))
                .build(),
            ServerConfig::builder().host(&name).build(),
            ServerConfig::builder()
                .host(Cow::Borrowed("cow.internal"))
                .build(),
            ServerConfig::builder()
                .host(format!("shard-{}.internal", 7))
                .build(),
        ]
        .map(|config| config.unwrap().host);
        assert_eq!(
            hosts,
            [
                "literal.internal",
                "owned.internal",
                "by-ref.internal",
                "cow.internal",
                "shard-7.internal",
            ]
        );
        // `&name` was only borrowed, so it is still ours.
        assert_eq!(name, "by-ref.internal");
    }

    #[test]
    fn an_owned_host_is_moved_in_not_copied() {
        let host = String::from("api.internal");
        let heap = host.as_ptr();
        let config = ServerConfig::builder().host(host).build().unwrap();
        // Moving a `String` moves only its (pointer, capacity, length) header.
        // A copy (`to_string`, `to_owned`, `String::from(s.as_ref())`, ...) is
        // made while the original is still alive, so it has to live in a new
        // allocation at a different address.
        assert_eq!(config.host.as_ptr(), heap);
    }

    #[test]
    fn tags_accept_anything_iterable() {
        let shared = strings(&["eu-west"]);
        let config = ServerConfig::builder()
            .host("api.internal")
            // An array of `&str`.
            .tags(["prod", "canary"])
            // A `Vec<String>`.
            .tags(strings(&["blue"]))
            // Borrowed `String`s: each `&String` is copied into a new `String`.
            .tags(&shared)
            // An iterator adapter.
            .tags((1..=2).map(|n| format!("shard-{n}")))
            // An `Option` is an iterator of zero or one items.
            .tags(Some("pinned"))
            .build()
            .unwrap();
        assert_eq!(
            config.tags,
            [
                "prod", "canary", "blue", "eu-west", "shard-1", "shard-2", "pinned",
            ]
        );
        assert_eq!(shared, ["eu-west"]);
    }

    #[test]
    fn tags_add_to_the_list_instead_of_replacing_it() {
        let config = ServerConfig::builder()
            .host("api.internal")
            .tags(["a"])
            .tags(Vec::<String>::new())
            .tags(["b", "c"])
            .build()
            .unwrap();
        assert_eq!(config.tags, ["a", "b", "c"]);
    }

    #[test]
    fn owned_tags_are_moved_in_not_copied() {
        let tags = strings(&["prod", "eu-west"]);
        let heaps: Vec<*const u8> = tags.iter().map(|tag| tag.as_ptr()).collect();
        let config = ServerConfig::builder()
            .host("api.internal")
            .tags(tags)
            .build()
            .unwrap();
        let after: Vec<*const u8> = config.tags.iter().map(|tag| tag.as_ptr()).collect();
        assert_eq!(after, heaps);
    }

    #[test]
    fn a_missing_host_is_an_error_not_a_fallback() {
        // `Default`'s loopback address is for local runs. The builder must not
        // quietly use it.
        assert_eq!(
            ServerConfig::builder().build(),
            Err(BuildError::MissingHost)
        );
        assert_eq!(
            ServerConfig::builder().port(443).workers(8).build(),
            Err(BuildError::MissingHost)
        );
    }

    #[test]
    fn a_blank_host_counts_as_missing() {
        for blank in ["", " ", "\t\n"] {
            assert_eq!(
                ServerConfig::builder().host(blank).build(),
                Err(BuildError::MissingHost),
                "host {blank:?}"
            );
        }
    }

    #[test]
    fn zero_workers_is_an_error() {
        assert_eq!(
            ServerConfig::builder()
                .host("api.internal")
                .workers(0)
                .build(),
            Err(BuildError::ZeroWorkers)
        );
        // One worker is the smallest valid pool.
        let config = ServerConfig::builder()
            .host("api.internal")
            .workers(1)
            .build()
            .unwrap();
        assert_eq!(config.workers, 1);
    }

    #[test]
    fn the_host_is_checked_before_the_workers() {
        assert_eq!(
            ServerConfig::builder().workers(0).build(),
            Err(BuildError::MissingHost)
        );
    }

    #[test]
    fn conditional_settings_rebind_the_builder() {
        // A consuming builder inside an `if`: each setter hands the builder
        // back, so you store it again.
        for canary in [false, true] {
            let mut builder = ServerConfig::builder().host("api.internal");
            if canary {
                builder = builder.workers(1).tags(["canary"]);
            }
            let config = builder.build().unwrap();
            assert_eq!(config.workers, if canary { 1 } else { 4 });
            assert_eq!(config.tags.len(), usize::from(canary));
        }
    }

    #[test]
    fn build_errors_work_with_the_question_mark() -> Result<(), Box<dyn Error>> {
        let config = ServerConfig::builder().host("api.internal").build()?;
        assert_eq!(config.port, 8080);
        let err: Box<dyn Error> = ServerConfig::builder().build().unwrap_err().into();
        assert_eq!(err.to_string(), "no host was set");
        Ok(())
    }
}
