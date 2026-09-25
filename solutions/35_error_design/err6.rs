// Module 5 · Error Design — custom errors and `?`, part 6: why an
// anyhow-style `Report` must NOT implement `Error`.
//
// Parts 4 and 5 built an application error: an opaque report that owns any
// `Error + Send + Sync + 'static`. The last piece of anyhow's ergonomics is
// that `?` accepts ANY error without a `From` impl per error type. That takes
// ONE blanket impl,
//
//     impl<E: Error + Send + Sync + 'static> From<E> for Report
//
// and it collides with an impl every type already has. `core` contains the
// reflexive `impl<T> From<T> for T` (it is why `?` works when the error types
// already match). If `Report` itself implements `Error`, then `E = Report`
// is covered by BOTH impls: two different `From<Report> for Report`, which is
// E0119 "conflicting implementations of trait `From<Report>` for type
// `Report`". Coherence allows at most one impl of a trait for any one type,
// and stable Rust has no specialization to pick the "more specific" one.
//
// So a report type must choose: implement `Error`, or let `?` convert from
// any error. anyhow and eyre choose `?`: `anyhow::Error` does NOT implement
// `std::error::Error`. They make up for it with a few small impls, and you
// write the two that matter here:
//
//   - Inspect: `AsRef<dyn Error + Send + Sync>` (anyhow also has `Deref`), so
//     callers can still `downcast_ref` the cause and walk its `source()`s.
//   - Hand off: `From<Report> for Box<dyn Error + Send + Sync + 'static>`, so
//     `?` can pass a report on to code that expects a boxed std error. That
//     impl is legal, and does not overlap std's
//     `From<E: Error + Send + Sync> for Box<dyn Error + Send + Sync>`,
//     precisely BECAUSE `Report` is not an `Error` (part 4 relied on the same
//     fact for its two `Context` impls).
//
// std made the same trade-off: `Box<dyn Error>` does not implement `Error`
// either. std's impl is `impl<E: Error> Error for Box<E>`, for SIZED `E` only,
// because with `E: ?Sized` the impl `From<E: Error> for Box<dyn Error>` would
// overlap with `From<T> for T` in exactly this way.
//
// That settles the design question interviewers love, "thiserror or anyhow?".
// LIBRARIES return typed, matchable errors (parts 1-3), because their callers
// have to branch on the cause; often a `#[non_exhaustive]` enum, so adding a
// variant later is not a breaking change (std does this with `io::ErrorKind`
// and with the `IntErrorKind` you matched in part 5). APPLICATIONS return an
// opaque report (parts 4-6), because at the top you mostly add context, log
// and exit, and when you do need to branch, `downcast_ref` gets you there.
//
// How interviewers probe this: "Why doesn't `anyhow::Error` implement
// `std::error::Error`?", "How can `?` turn any error into an
// `anyhow::Error`?", "How do you return an `anyhow::Error` from a function
// whose signature says `Box<dyn Error + Send + Sync>`?".

use std::error::Error;
use std::fmt;

type BoxError = Box<dyn Error + Send + Sync + 'static>;

// An opaque application error: any std error, boxed. The derived `Debug`
// prints the inner error's `Debug` inside `Report(..)`, and `Display` below
// delegates to the inner error.
#[derive(Debug)]
struct Report(BoxError);

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// No `impl Error for Report`: that is what leaves room for the blanket `From`
// below. The two impls here give back what callers lost. `AsRef` lends out the
// inner error (so `downcast_ref` and `source()` still work), and
// `From<Report> for BoxError` hands over the box the report already owns.
// That `From` is legal only because `Report` is not an `Error`, so std's
// blanket `From<E: Error>` for boxes does not cover it.
impl AsRef<dyn Error + Send + Sync> for Report {
    fn as_ref(&self) -> &(dyn Error + Send + Sync + 'static) {
        &*self.0
    }
}

impl From<Report> for BoxError {
    fn from(report: Report) -> Self {
        report.0
    }
}

// The whole point of the type: `?` turns ANY std error into a `Report`.
impl<E> From<E> for Report
where
    E: Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Report(Box::new(error))
    }
}

// A local error type, so `?` below meets three different error types.
#[derive(Debug)]
struct MissingStar;

impl fmt::Display for MissingStar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected `count*factor`")
    }
}

impl Error for MissingStar {}

// "3*1.5" -> 4.5
fn scaled(text: &str) -> Result<f64, Report> {
    let (count, factor) = text.split_once('*').ok_or(MissingStar)?;
    let count: u32 = count.trim().parse()?; // ParseIntError
    let factor: f64 = factor.trim().parse()?; // ParseFloatError
    Ok(f64::from(count) * factor)
}

// "2" -> 3
fn run(text: &str) -> Result<i64, Report> {
    let n: i64 = text.trim().parse()?;
    Ok(n + 1)
}

// The same job for a caller that only knows std's boxed errors.
fn run_boxed(text: &str) -> Result<i64, BoxError> {
    let n = run(text)?;
    Ok(n)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::{ParseFloatError, ParseIntError};
    use std::ptr;

    // Borrow the error inside a report. Spelling out the target type keeps
    // this unambiguous even if `Report` has more than one `AsRef` impl.
    fn inner(report: &Report) -> &(dyn Error + Send + Sync + 'static) {
        report.as_ref()
    }

    // A test-only error with a cause of its own.
    #[derive(Debug)]
    struct Stage {
        source: ParseIntError,
    }

    impl fmt::Display for Stage {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("stage 2 failed")
        }
    }

    impl Error for Stage {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.source)
        }
    }

    // Converts ANY qualifying error: only the blanket `From` can do this.
    fn wrap<E: Error + Send + Sync + 'static>(error: E) -> Report {
        Report::from(error)
    }

    #[test]
    fn run_adds_one() {
        assert_eq!(run("2").unwrap(), 3);
        assert_eq!(run(" -8 ").unwrap(), -7);
    }

    #[test]
    fn question_mark_accepts_three_error_types() {
        assert_eq!(scaled("3*1.5").unwrap(), 4.5);

        let err = scaled("4").unwrap_err();
        assert!(inner(&err).is::<MissingStar>());
        assert_eq!(err.to_string(), "expected `count*factor`");

        let err = scaled("x*2").unwrap_err();
        assert!(inner(&err).downcast_ref::<ParseIntError>().is_some());
        assert!(inner(&err).downcast_ref::<ParseFloatError>().is_none());

        let err = scaled("2*y").unwrap_err();
        assert!(inner(&err).downcast_ref::<ParseFloatError>().is_some());
        assert_eq!(err.to_string(), "invalid float literal");
    }

    #[test]
    fn display_and_debug_come_from_the_inner_error() {
        let err = run("1.5").unwrap_err();
        assert_eq!(err.to_string(), "invalid digit found in string");
        assert!(format!("{err:?}").contains("InvalidDigit"));
    }

    #[test]
    fn any_error_type_converts_into_a_report() {
        let report = wrap(MissingStar);
        assert!(inner(&report).is::<MissingStar>());
        let report = wrap(std::io::Error::other("disk on fire"));
        assert!(inner(&report).is::<std::io::Error>());
        assert_eq!(report.to_string(), "disk on fire");
    }

    #[test]
    fn the_cause_chain_is_reachable_through_as_ref() {
        let report = wrap(Stage {
            source: "z".parse::<u8>().unwrap_err(),
        });
        let err = inner(&report);
        assert!(err.is::<Stage>());
        let cause = err.source().expect("Stage has a cause");
        assert!(cause.is::<ParseIntError>());
    }

    #[test]
    fn a_report_hands_off_to_std_boxed_error_code() {
        assert_eq!(run_boxed("41").unwrap(), 42);
        let err = run_boxed("seven").unwrap_err();
        // Still the original error, not a string or a report inside a box.
        assert!(err.downcast_ref::<ParseIntError>().is_some());
        assert_eq!(err.to_string(), "invalid digit found in string");
    }

    #[test]
    fn handing_off_moves_the_existing_box() {
        let report = run("seven").unwrap_err();
        let before: *const (dyn Error + Send + Sync) = inner(&report);
        let boxed: BoxError = report.into();
        // Same heap address: the report gave up the box it already owned
        // instead of wrapping itself in a second one.
        assert!(ptr::addr_eq(before, &*boxed));
    }
}
