//! Integration tests for `deep_dive::api_surface` (the `api_surface` lab).
//!
//! Cargo compiles this file as a crate of its own that links `deep_dive`, so
//! it sees exactly what a dependent crate sees: the `pub` API, with
//! `#[non_exhaustive]` and the sealed traits in force. The `props` module at
//! the bottom holds the property tests; it is compiled only under
//! `--cfg proptest`:
//!
//! ```text
//! cargo test --manifest-path deep-dive/Cargo.toml --test api_surface
//! RUSTFLAGS="--cfg proptest" cargo test --manifest-path deep-dive/Cargo.toml --test api_surface
//! ```

mod common;

use common::{DropLog, Tracked, strings};
use deep_dive::api_surface::{
    Csv, DecodeError, HasUrl, Method, NoUrl, Opts, Recorder, Request, RequestBuilder, Status,
    ToFields, Tsv, decode, encode, encode_record, insert_sorted,
};
use std::mem::size_of;

#[test]
fn a_status_match_outside_the_crate_needs_a_wildcard_arm() {
    fn describe(status: Status) -> &'static str {
        match status {
            Status::Success => "ok",
            Status::Redirect => "moved",
            Status::ClientError => "the client's fault",
            Status::ServerError => "the server's fault",
            // Required here (E0004 without it), although every variant is
            // listed: a later release may add one.
            _ => "unknown",
        }
    }
    let described: Vec<_> = [204, 302, 404, 500]
        .into_iter()
        .filter_map(Status::from_code)
        .map(describe)
        .collect();
    assert_eq!(
        described,
        ["ok", "moved", "the client's fault", "the server's fault"]
    );
    assert_eq!(Status::from_code(100), None);
}

#[test]
fn a_decode_error_match_needs_a_wildcard_arm_too() {
    let offset = |error: &DecodeError| match error {
        DecodeError::DanglingEscape { at } | DecodeError::UnknownEscape { at, .. } => Some(*at),
        _ => None,
    };
    let error = decode::<Csv>(r"ok,bad\x").unwrap_err();
    assert_eq!(offset(&error), Some(6));
}

#[test]
fn opts_outside_the_crate_start_from_default() {
    // `Opts { timeout_ms: 500, ..Opts::default() }` would be E0639 here.
    let mut opts = Opts::default();
    opts.timeout_ms = 500;
    let request = RequestBuilder::default()
        .url("https://example.com/")
        .opts(opts.clone())
        .send();
    assert_eq!(request.opts, opts);
    assert_eq!(request.opts.retries, 3);
}

#[test]
fn a_request_is_plain_data_that_users_build_and_destructure() {
    // Legal because `Request` is exhaustive with all fields `pub`, which is
    // also why adding a field to it would be semver-major.
    let by_hand = Request {
        method: Method::Post,
        url: "https://example.com/upload".to_string(),
        headers: vec![("content-type".to_string(), "text/csv".to_string())],
        opts: Opts::default(),
    };
    let built = RequestBuilder::default()
        .method(Method::Post)
        .header("content-type", "text/csv")
        .url("https://example.com/upload")
        .send();
    assert_eq!(built, by_hand);
    let Request { method, url, .. } = built;
    assert_eq!(
        (method, url.as_str()),
        (Method::Post, "https://example.com/upload")
    );
}

#[test]
fn the_typestate_builder_keeps_headers_across_the_transition() {
    let no_url: RequestBuilder<NoUrl> = RequestBuilder::default().header("accept", "text/csv");
    let has_url: RequestBuilder<HasUrl> = no_url.url("https://example.com/export");
    let request = has_url.header("accept", "text/tab-separated-values").send();
    assert_eq!(request.method, Method::Get);
    let names: Vec<&str> = request.headers.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["accept", "accept"]);
}

#[test]
fn the_builder_state_costs_no_memory() {
    assert_eq!(size_of::<NoUrl>(), 0);
    assert_eq!(size_of::<HasUrl>(), 0);
    assert_eq!(
        size_of::<RequestBuilder<NoUrl>>(),
        size_of::<RequestBuilder<HasUrl>>()
    );
}

#[test]
fn the_url_is_moved_into_the_request_not_copied() {
    let url = String::from("https://example.com/big-export");
    let heap = url.as_ptr();
    let request = RequestBuilder::default().url(url).send();
    // A moved `String` keeps its heap buffer; a copy could not share it.
    assert_eq!(request.url.as_ptr(), heap);
}

// A user type: `ToFields` is not sealed, so this impl is allowed. It is also
// the downstream code that a new required method on `ToFields` would break.
struct Contact {
    name: String,
    city: String,
}

impl ToFields for Contact {
    fn to_fields(&self) -> Vec<String> {
        vec![self.name.clone(), self.city.clone()]
    }
}

#[test]
fn users_implement_the_unsealed_to_fields_trait() {
    let contact = Contact {
        name: "Lovelace, Ada".to_string(),
        city: r"London\UK".to_string(),
    };
    let line = encode_record::<Csv>(&contact);
    assert_eq!(line, r"Lovelace\, Ada,London\\UK");
    assert_eq!(decode::<Csv>(&line).unwrap(), contact.to_fields());
}

#[test]
fn hand_picked_records_round_trip_in_both_formats() {
    let records = [
        strings(&[""]),
        strings(&["", ""]),
        strings(&[r"\", ","]),
        strings(&["tab\there", "comma,here", r"slash\here"]),
        strings(&[r"\\,", "\t\t"]),
    ];
    for fields in records {
        let csv = encode::<Csv, _>(&fields);
        assert_eq!(decode::<Csv>(&csv).unwrap(), fields, "csv line {csv:?}");
        let tsv = encode::<Tsv, _>(&fields);
        assert_eq!(decode::<Tsv>(&tsv).unwrap(), fields, "tsv line {tsv:?}");
    }
}

#[test]
fn insert_sorted_drops_every_element_exactly_once() {
    let log = DropLog::default();
    {
        let mut v = Vec::new();
        for (id, key) in (0..).zip([5, 1, 3, 1, 9]) {
            insert_sorted(&mut v, Tracked::new(key, id, &log));
        }
        let keys: Vec<u32> = v.iter().map(|t| t.key).collect();
        assert_eq!(keys, [1, 1, 3, 5, 9]);
        // Equal keys keep their insertion order: id 1 came before id 3.
        let ids: Vec<u32> = v.iter().map(|t| t.id).collect();
        assert_eq!(ids, [1, 3, 2, 0, 4]);
        assert!(log.borrow().is_empty(), "insert_sorted dropped a value");
    }
    let mut dropped = log.borrow().clone();
    dropped.sort_unstable();
    assert_eq!(dropped, [0, 1, 2, 3, 4]);
}

// The seam of `50_testing_seams`: the code under test depends on a trait, and
// the test hands it a recording double. `Notifier` is local to this crate, so
// implementing it for the foreign `Recorder` is allowed by the orphan rule.
trait Notifier {
    fn notify(&self, message: &str);
}

impl Notifier for Recorder<String> {
    fn notify(&self, message: &str) {
        self.record(message.to_string());
    }
}

fn close_account(user: &str, notifier: &impl Notifier) {
    notifier.notify(&format!("goodbye, {user}"));
}

#[test]
fn a_recorder_is_a_test_double_behind_a_trait_seam() {
    let recorder = Recorder::new();
    close_account("ada", &recorder);
    close_account("bob", &recorder);
    assert_eq!(
        *recorder.calls(),
        strings(&["goodbye, ada", "goodbye, bob"])
    );
}

/// The property tests: the invariants from the `api_surface` module docs,
/// checked on 256 generated inputs each, plus the planted-bug
/// demonstrations. Compiled only under `--cfg proptest`.
#[cfg(proptest)]
mod props {
    use super::common::strategies::{config, demo_config, line, record, sorted_and_value};
    use deep_dive::api_surface::{Csv, Tsv, decode, encode, insert_sorted, planted};
    use proptest::prelude::*;
    use proptest::test_runner::{TestError, TestRunner};

    proptest! {
        #![proptest_config(config())]

        #[test]
        fn csv_round_trips(fields in record()) {
            prop_assert_eq!(decode::<Csv>(&encode::<Csv, _>(&fields)), Ok(fields));
        }

        #[test]
        fn tsv_round_trips(fields in record()) {
            prop_assert_eq!(decode::<Tsv>(&encode::<Tsv, _>(&fields)), Ok(fields));
        }

        // `any::<String>()` covers the rest of Unicode. proptest's character
        // strategy leans toward a list of "special" characters (the
        // backslash among them), so it finds escaping bugs too, only less
        // reliably than the targeted `record()` strategy.
        #[test]
        fn arbitrary_strings_round_trip(
            fields in prop::collection::vec(any::<String>(), 1..8),
        ) {
            prop_assert_eq!(decode::<Csv>(&encode::<Csv, _>(&fields)), Ok(fields));
        }

        #[test]
        fn every_line_that_decodes_is_canonical(line in line()) {
            if let Ok(fields) = decode::<Csv>(&line) {
                prop_assert_eq!(encode::<Csv, _>(&fields), line);
            }
        }

        #[test]
        fn insert_sorted_keeps_the_vector_sorted((mut v, x) in sorted_and_value()) {
            let mut expected = v.clone();
            expected.push(x);
            expected.sort_unstable();
            insert_sorted(&mut v, x);
            prop_assert_eq!(v, expected);
        }
    }

    // The planted-bug demonstrations. `TestRunner::run` is what `proptest!`
    // expands to: it generates cases until one fails, then shrinks that case
    // and returns the minimal failing input, which these tests assert. Both
    // failure conditions are simple enough that shrinking reaches the
    // smallest possible counterexample from any random start.

    #[test]
    fn shrinking_reduces_the_escaping_bug_to_a_lone_backslash() {
        let mut runner = TestRunner::new(demo_config());
        let result = runner.run(&record(), |fields| {
            let line = planted::encode_without_escaping_the_escape::<Csv, _>(&fields);
            prop_assert_eq!(decode::<Csv>(&line), Ok(fields));
            Ok(())
        });
        match result {
            Err(TestError::Fail(_, minimal)) => assert_eq!(minimal, [r"\"]),
            other => panic!("expected the planted bug to be found, got {other:?}"),
        }
    }

    #[test]
    fn shrinking_reduces_the_insert_bug_to_one_element() {
        let mut runner = TestRunner::new(demo_config());
        let result = runner.run(&sorted_and_value(), |(mut v, x)| {
            planted::insert_sorted_largest_at_front(&mut v, x);
            prop_assert!(v.is_sorted(), "{v:?} is not sorted");
            Ok(())
        });
        match result {
            Err(TestError::Fail(_, minimal)) => assert_eq!(minimal, (vec![0], 1)),
            other => panic!("expected the planted bug to be found, got {other:?}"),
        }
    }

    // The same two properties as failing `proptest!` tests, to read
    // proptest's own report ("Test failed: ...", then "minimal failing
    // input: ..."). Run them by hand:
    //   RUSTFLAGS="--cfg proptest" cargo test \
    //       --manifest-path deep-dive/Cargo.toml --test api_surface \
    //       -- --ignored watch_
    proptest! {
        #![proptest_config(demo_config())]

        #[test]
        #[ignore = "planted bug: fails on purpose, run by hand with --ignored"]
        fn watch_shrinking_on_the_escaping_bug(fields in record()) {
            let line = planted::encode_without_escaping_the_escape::<Csv, _>(&fields);
            prop_assert_eq!(decode::<Csv>(&line), Ok(fields));
        }

        #[test]
        #[ignore = "planted bug: fails on purpose, run by hand with --ignored"]
        fn watch_shrinking_on_the_insert_bug((mut v, x) in sorted_and_value()) {
            planted::insert_sorted_largest_at_front(&mut v, x);
            prop_assert!(v.is_sorted(), "{:?} is not sorted", v);
        }
    }
}
