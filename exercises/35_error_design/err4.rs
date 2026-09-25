// Module 5 · Error Design — custom errors and `?`, part 4: context chains that
// keep the cause.
//
// Parts 1-3 designed LIBRARY errors: small, matchable types that say exactly
// what went wrong. An APPLICATION has a different problem. By the time a
// `ParseIntError` reaches `main`, "invalid digit found in string" is useless on
// its own: which file, which field, which step of startup? The fix is not to
// replace the low-level error with a better string — `format!("bad port: {e}")`
// throws the cause away, so nobody can inspect it later — but to WRAP it. Each
// layer adds one line of context and keeps the error below it as its
// `source()`. A reporter then walks the chain from part 2 and prints
//
//     loading config
//     caused by: parsing port
//     caused by: invalid digit found in string
//
// That is what `anyhow::Context` and `eyre::WrapErr` give you, and it is built
// from two ideas you already know:
//
//   1. An extension trait. `Context<T>` is a LOCAL trait, so this crate may
//      implement it for the foreign type `Result<T, E>`; `.context("...")` then
//      becomes a method on every `Result` whose error type qualifies.
//   2. A blanket impl. ONE `impl<T, E> Context<T> for Result<T, E>` with
//      `E: Error + Send + Sync + 'static` covers every error in the ecosystem
//      (`ParseIntError`, `io::Error`, your own enums). The bounds are exactly
//      what the report needs to own the cause as a `BoxError`: `Error` so it
//      can be displayed and walked, `Send + Sync` so the report can cross
//      threads (part 5), and `'static` so the box borrows nothing (which is
//      also what makes it downcastable, part 5).
//
// The twist: adding context to a result that ALREADY holds a `Report` needs a
// second impl, for `Result<T, Report>`. The two impls could not coexist if
// `Report` implemented `Error`: the blanket impl would cover `E = Report` too,
// and rustc would reject the pair with E0119 "conflicting implementations of
// trait `Context<_>` for type `Result<_, Report>`". They are allowed because
// `Report` is a local type with no `Error` impl, and this crate is the only
// place such an impl could ever be written. It is the same reason
// `anyhow::Error` does not implement `Error`; part 6 shows the other half.
//
// std has no stable way to walk a chain for you (`Error::sources()` is still
// behind the unstable `error_iter` feature), so `chain()` below is yours.
//
// How interviewers probe this: "How do you add context without losing the
// underlying error?", "Why is `map_err(|e| format!(...))` a bad fix?", "How
// would you print the whole chain?", "Why the `Send + Sync + 'static` bounds?".

use std::error::Error;
use std::fmt;

// The boxed cause a report owns. In a type alias `Box<dyn Error + Send + Sync>`
// already means `+ 'static`; spelling it out documents the intent.
type BoxError = Box<dyn Error + Send + Sync + 'static>;

// An application-level error: one line of context plus the error it wraps.
// Deliberately NOT `Clone` (a boxed `dyn Error` can't be cloned) and
// deliberately NOT `Error` (see above).
#[derive(Debug)]
struct Report {
    msg: String,
    source: Option<BoxError>,
}

impl Report {
    // A report with no cause at all, like `anyhow!("...")`.
    fn new(msg: &str) -> Self {
        Report {
            msg: msg.to_string(),
            source: None,
        }
    }
}

impl fmt::Display for Report {
    // Only the top line. The rest of the story lives in the chain.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

// Given: a `Report` can't sit in a `source` slot by itself, because it is not
// an `Error`. When a report gets MORE context, the old report goes into this
// adapter, which IS an `Error`: its `Display` is the old message and its
// `source()` is the old cause. anyhow does the same thing internally.
#[derive(Debug)]
struct Layer(Report);

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.msg)
    }
}

impl Error for Layer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // The stored cause is a `dyn Error + Send + Sync`, but `source()` must
        // return a plain `dyn Error`. Dropping auto traits is a coercion, and
        // coercions don't happen INSIDE an `Option`, so `map` does it one
        // reference at a time.
        self.0
            .source
            .as_deref()
            .map(|e| e as &(dyn Error + 'static))
    }
}

// Given: hand a report, chain and all, to code that only knows std's boxed
// errors (a `main` returning `Result<(), BoxError>`, a library callback). This
// impl is legal only because `Report` is NOT an `Error`: if it were, std's own
// `impl<E: Error + Send + Sync> From<E> for Box<dyn Error + Send + Sync>`
// would already cover `Report`, and rustc would reject this one with E0119.
impl From<Report> for BoxError {
    fn from(report: Report) -> Self {
        Box::new(Layer(report))
    }
}

// Add one line of context to a failed result, keeping the error as the cause.
trait Context<T> {
    fn context(self, msg: &str) -> Result<T, Report>;
}

// TODO: every `.context(...)` call in this file is rejected with E0599 "no
// method named `context` found for enum `Result<T, E>` in the current scope":
// the trait exists, but nothing implements it. Write two impls.
//   (a) A blanket impl for `Result<T, E>` covering every `E` that is
//       `Error + Send + Sync + 'static`. On `Err(e)`, the new report's `msg` is
//       the context line and its source is `e` ITSELF, boxed.
//   (b) An impl for `Result<T, Report>`, for adding context to a result that
//       already holds a report. The old report becomes the new one's source
//       (it isn't an `Error`, so it goes in through the given `Layer`; the
//       given `From<Report> for BoxError` does exactly that).
// `Ok` values pass through untouched in both. Keep the cause as a VALUE: no
// `to_string()` / `format!` of the inner error (the tests downcast the root
// cause back to its concrete type), and don't merge two messages into one
// string. Don't reach for `impl Error for Report` either: it collides with
// the given `From` impl and with a pair of impls in the tests (E0119), and it
// is the very thing this design avoids.
// Until you implement `Context` for both kinds of `Result`, this exercise will
// not compile.

impl Report {
    // Every message in the chain, outermost first.
    fn chain(&self) -> Vec<String> {
        // TODO: return this report's own `msg`, then the `Display` text of each
        // error reached by following `source()` until it returns `None`. The
        // empty body is E0308 "mismatched types" (expected `Vec<String>`, found
        // `()`). One trap on the way: the stored cause is a
        // `&(dyn Error + Send + Sync)`, but `source()` returns
        // `&(dyn Error + 'static)`, and rustc won't convert one into the other
        // inside an `Option` (E0308 again); `Layer::source` above shows the way
        // through. Until you build and return the list of messages, this
        // exercise will not compile.
    }
}

#[derive(Debug, PartialEq)]
struct Config {
    port: u16,
}

fn parse_port(text: &str) -> Result<u16, Report> {
    // `?` here is `Report` into `Report`: core's reflexive `From<T> for T`.
    let port = text.trim().parse::<u16>().context("parsing port")?;
    if port == 0 {
        return Err(Report::new("port 0 is reserved"));
    }
    Ok(port)
}

fn load(text: &str) -> Result<Config, Report> {
    let port = parse_port(text).context("loading config")?;
    Ok(Config { port })
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::{IntErrorKind, ParseIntError};

    // Follow `source()` down to the last error of a chain.
    fn deepest<'a>(mut err: &'a (dyn Error + 'static)) -> &'a (dyn Error + 'static) {
        while let Some(next) = err.source() {
            err = next;
        }
        err
    }

    fn root_cause(report: &Report) -> &(dyn Error + 'static) {
        deepest(
            report
                .source
                .as_deref()
                .expect("the report should have a cause"),
        )
    }

    // Stands in for a `main` or a callback whose signature only knows std's
    // boxed errors: `?` goes through the given `From<Report> for BoxError`.
    fn start(text: &str) -> Result<Config, BoxError> {
        let config = load(text)?;
        Ok(config)
    }

    // A test-only error with a cause of its own, like part 2's
    // `ParseRecordError`. Not `Clone`: it can only be moved into a report.
    #[derive(Debug)]
    struct BadLine {
        line: usize,
        source: ParseIntError,
    }

    impl fmt::Display for BadLine {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "line {} is malformed", self.line)
        }
    }

    impl Error for BadLine {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.source)
        }
    }

    #[test]
    fn good_input_loads() {
        assert_eq!(load("8080").unwrap(), Config { port: 8080 });
        assert_eq!(load(" 443 ").unwrap(), Config { port: 443 });
    }

    #[test]
    fn context_lines_stack_outermost_first() {
        let report = load("80x").unwrap_err();
        assert_eq!(
            report.chain(),
            [
                "loading config",
                "parsing port",
                "invalid digit found in string"
            ]
        );
    }

    #[test]
    fn empty_and_too_large_inputs_keep_their_own_causes() {
        assert_eq!(
            load("").unwrap_err().chain(),
            [
                "loading config",
                "parsing port",
                "cannot parse integer from empty string"
            ]
        );
        assert_eq!(
            load("70000").unwrap_err().chain(),
            [
                "loading config",
                "parsing port",
                "number too large to fit in target type"
            ]
        );
    }

    #[test]
    fn display_is_only_the_top_line() {
        assert_eq!(load("80x").unwrap_err().to_string(), "loading config");
    }

    #[test]
    fn the_root_cause_is_still_the_original_parse_error() {
        let report = load("99999").unwrap_err();
        // A stringified cause would still PRINT the same, but it would no
        // longer BE a `ParseIntError`.
        let parse = root_cause(&report)
            .downcast_ref::<ParseIntError>()
            .expect("the root cause should still be a ParseIntError");
        assert_eq!(parse.kind(), &IntErrorKind::PosOverflow);
    }

    #[test]
    fn a_report_without_a_cause_can_still_get_context() {
        let report = load("0").unwrap_err();
        assert_eq!(report.chain(), ["loading config", "port 0 is reserved"]);
    }

    #[test]
    fn a_single_report_is_a_one_line_chain() {
        assert_eq!(Report::new("boom").chain(), ["boom"]);
    }

    #[test]
    fn chain_walks_into_the_errors_own_sources() {
        let bad = BadLine {
            line: 3,
            source: "x".parse::<u8>().unwrap_err(),
        };
        let result: Result<(), BadLine> = Err(bad);
        let report = result
            .context("reading table")
            .context("starting up")
            .unwrap_err();
        assert_eq!(
            report.chain(),
            [
                "starting up",
                "reading table",
                "line 3 is malformed",
                "invalid digit found in string"
            ]
        );
        // Two layers of context later, the error in the middle is still the
        // very `BadLine` value that went in.
        let layer = report.source.as_deref().expect("a cause");
        let bad = layer
            .source()
            .and_then(|e| e.downcast_ref::<BadLine>())
            .expect("the second link should still be a BadLine");
        assert_eq!(bad.line, 3);
    }

    #[test]
    fn a_report_handed_to_std_code_keeps_its_chain() {
        assert_eq!(start("22").unwrap(), Config { port: 22 });
        let err = start("80x").unwrap_err();
        assert_eq!(err.to_string(), "loading config");
        let next = err
            .source()
            .expect("the boxed report should keep its cause");
        assert_eq!(next.to_string(), "parsing port");
        assert!(deepest(&*err).is::<ParseIntError>());
    }

    // The `Context` design in miniature: one blanket impl for every std error
    // and one for `Report`. Like the two `Context` impls, this pair compiles
    // only while `Report` is NOT an `Error` (with `impl Error for Report` it is
    // E0119).
    trait Label {
        fn label(&self) -> &'static str;
    }

    impl<E: Error> Label for E {
        fn label(&self) -> &'static str {
            "std error"
        }
    }

    impl Label for Report {
        fn label(&self) -> &'static str {
            "report"
        }
    }

    #[test]
    fn a_report_is_not_itself_a_std_error() {
        assert_eq!(Report::new("boom").label(), "report");
        assert_eq!(load("80x").unwrap_err().label(), "report");
        assert_eq!("x".parse::<u8>().unwrap_err().label(), "std error");
    }

    #[test]
    fn ok_values_pass_through_untouched() {
        let fine: Result<Vec<u8>, ParseIntError> = Ok(vec![1, 2, 3]);
        assert_eq!(fine.context("unused").unwrap(), [1, 2, 3]);
        let fine: Result<&str, Report> = Ok("still here");
        assert_eq!(fine.context("unused").unwrap(), "still here");
    }
}
