// Module 5 · Error Design — custom errors and `?`, part 5: errors that cross
// threads (`Send + Sync`), and getting the concrete type back (downcasting).
//
// `Box<dyn Error>` is the quick "any error" type from
// `13_error_handling/errors5`. It works until an error has to leave the thread
// that made it. A trait object has only the auto traits (`Send` and `Sync`,
// from `30_send_sync`) that you WRITE into its type: `dyn Error` promises
// nothing about either, so `Box<dyn Error>` is neither, even when the value
// inside is a perfectly thread-safe `ParseIntError`. The compiler reasons
// about the type, not the value. So the moment a worker thread returns one
// through `thread::spawn`, whose closure result must be `Send + 'static`,
// rustc stops you with E0277 "`dyn std::error::Error` cannot be sent between
// threads safely". (The same wall appears with `tokio::spawn`, which also
// wants `Send + 'static`.)
//
// The fix belongs in the error TYPE: `Box<dyn Error + Send + Sync + 'static>`.
// Those are the bounds `anyhow::Error` puts on every error it wraps, what
// `io::Error::other` accepts, and tower's `BoxError` alias. Each bound has a
// job:
//
//   - `Send`: the box may MOVE to another thread (out of a `join`, through a
//     channel).
//   - `Sync`: a `&BoxError` may be SHARED between threads (several threads
//     logging the same error, an `Arc<BoxError>`). `Send` alone does not give
//     you that.
//   - `'static`: the error borrows nothing, so it can outlive the function
//     (and the thread) that made it. Downcasting needs it too: a `TypeId`
//     only exists for `'static` types. In a type alias a boxed trait object
//     already defaults to `+ 'static`; writing it out documents the intent.
//
// The extra bounds cost nothing at the use sites: `?` still boxes any
// `E: Error + Send + Sync + 'static` through std's
// `From<E> for Box<dyn Error + Send + Sync>`, and `"text".into()` still works.
// Those `From` impls exist for exactly two targets, `Box<dyn Error>` and
// `Box<dyn Error + Send + Sync>`, which is one more reason the bounds travel
// as a pair: a `Box<dyn Error + Send>` would not even get `?`.
//
// Boxing erases the static type, but not the information. Every `dyn Error`
// can report the `TypeId` of the concrete type behind it, and
// `downcast_ref::<T>()` asks "are you really a `T`?": `Some(&T)` if so, `None`
// if not. `is::<T>()` is the yes/no version, and `downcast::<T>()` on the box
// is the by-value one (it hands the box back on a miss). That is how a
// supervisor branches on the cause (retry on a timeout, reject bad input)
// without forcing every error into one big enum. Compare TYPES, never
// messages: a `to_string()` match breaks the day a message is reworded, and
// it can't tell two errors with the same text apart.
//
// How interviewers probe this: "Why won't this compile with `Box<dyn Error>`?",
// "Why `Send + Sync` and not just `Send`?", "Why the `'static`?", "How do you
// get a `ParseIntError` back out of a `Box<dyn Error>`?".

use std::error::Error;
use std::fmt;
use std::num::{IntErrorKind, ParseIntError};
use std::thread;

// TODO: `run_in_worker` below is rejected with E0277 "`dyn std::error::Error`
// cannot be sent between threads safely": a worker's return value must be
// `Send` to come back through `join`, and a bare `dyn Error` object promises
// no auto traits at all. One test also lets several threads read the same
// error, which hits the twin, E0277 "`dyn std::error::Error` cannot be shared
// between threads safely". Fix the error TYPE, not the call sites: this alias
// must name a boxed error that may move between threads, may be shared
// between threads, and borrows nothing. Don't turn the error into a `String`
// to get it across (the tests downcast it on the other side), don't remove
// the thread, and don't `unwrap` inside the worker. Until you fix the alias,
// this exercise will not compile.
type BoxError = Box<dyn Error>;

// This exercise's own domain error.
#[derive(Debug)]
struct ZeroNotAllowed;

impl fmt::Display for ZeroNotAllowed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("zero is not allowed")
    }
}

impl Error for ZeroNotAllowed {}

const LIMIT: u32 = 1000;

// Runs on a worker thread. Three ways to fail, three different concrete types
// behind the same `BoxError`.
fn parse_job(input: &str) -> Result<u32, BoxError> {
    // A `ParseIntError`, boxed by `?`.
    let parsed: Result<u32, ParseIntError> = input.trim().parse();
    let n = parsed?;
    if n == 0 {
        // Our own error type.
        return Err(Box::new(ZeroNotAllowed));
    }
    if n > LIMIT {
        // A plain message. std boxes it as a private string-error type.
        return Err(format!("{n} is above the limit of {LIMIT}").into());
    }
    Ok(n)
}

// Parse `input` on a worker thread and hand the result back to the caller.
fn run_in_worker(input: String) -> Result<u32, BoxError> {
    let worker = thread::spawn(move || parse_job(&input));
    // `join` is `Err` only if the worker PANICKED. That is a bug, not bad input.
    worker.join().expect("the worker thread panicked")
}

// What the supervisor decides about a failed job.
#[derive(Debug, PartialEq)]
enum Verdict {
    // The input was not a valid `u32`; keeps the reason.
    Malformed(IntErrorKind),
    // The input was `0`.
    Zero,
    // Any other error: keep its message for the log.
    Other(String),
}

fn verdict(err: &(dyn Error + 'static)) -> Verdict {
    // TODO: classify the error by its concrete TYPE. A `ParseIntError` becomes
    // `Malformed` with that error's `kind()`, a `ZeroNotAllowed` becomes
    // `Zero`, and anything else becomes `Other` with its `Display` text. The
    // box erased the type but did not destroy it: ask the trait object
    // whether it is a `T`. Don't compare messages (a test hands you a
    // plain-text error that reads exactly "zero is not allowed", and it must
    // NOT count as `Zero`). The empty body is E0308 "mismatched types"
    // (expected `Verdict`, found `()`). Until you classify by type, this
    // exercise will not compile.
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_job_returns_its_number() {
        assert_eq!(run_in_worker("42".to_string()).unwrap(), 42);
        assert_eq!(run_in_worker(" 1000 ".to_string()).unwrap(), 1000);
    }

    #[test]
    fn a_parse_failure_is_still_a_parse_int_error_after_the_join() {
        let err = run_in_worker("4x2".to_string()).unwrap_err();
        let parse = err
            .downcast_ref::<ParseIntError>()
            .expect("the cause should still be a ParseIntError");
        assert_eq!(parse.kind(), &IntErrorKind::InvalidDigit);
        assert_eq!(err.to_string(), "invalid digit found in string");
        assert_eq!(
            verdict(&*err),
            Verdict::Malformed(IntErrorKind::InvalidDigit)
        );
    }

    #[test]
    fn every_parse_failure_keeps_its_kind() {
        for (input, kind) in [
            ("", IntErrorKind::Empty),
            ("   ", IntErrorKind::Empty),
            ("-3", IntErrorKind::InvalidDigit),
            ("99999999999", IntErrorKind::PosOverflow),
        ] {
            let err = run_in_worker(input.to_string()).unwrap_err();
            assert_eq!(verdict(&*err), Verdict::Malformed(kind), "input {input:?}");
        }
    }

    #[test]
    fn zero_is_its_own_error_type() {
        let err = run_in_worker("0".to_string()).unwrap_err();
        assert_eq!(err.to_string(), "zero is not allowed");
        assert!(err.is::<ZeroNotAllowed>());
        assert!(err.downcast_ref::<ParseIntError>().is_none());
        assert_eq!(verdict(&*err), Verdict::Zero);
    }

    #[test]
    fn anything_else_is_reported_by_its_message() {
        let err = run_in_worker("5000".to_string()).unwrap_err();
        assert_eq!(
            verdict(&*err),
            Verdict::Other("5000 is above the limit of 1000".to_string())
        );
    }

    #[test]
    fn a_look_alike_message_is_not_the_same_error() {
        // Same text as the real errors, different types.
        let fake: BoxError = "zero is not allowed".into();
        assert_eq!(
            verdict(&*fake),
            Verdict::Other("zero is not allowed".to_string())
        );
        let fake: BoxError = "invalid digit found in string".into();
        assert_eq!(
            verdict(&*fake),
            Verdict::Other("invalid digit found in string".to_string())
        );
    }

    #[test]
    fn verdict_accepts_any_error_reference() {
        assert_eq!(verdict(&ZeroNotAllowed), Verdict::Zero);
        let parse = "".parse::<u8>().unwrap_err();
        assert_eq!(verdict(&parse), Verdict::Malformed(IntErrorKind::Empty));
    }

    #[test]
    fn the_error_itself_travels_back_from_a_thread() {
        // No `run_in_worker` here: this test spawns its own thread, so the
        // alias has to be `Send` whatever `run_in_worker` looks like.
        let handle = thread::spawn(|| parse_job("0"));
        let err = handle.join().unwrap().unwrap_err();
        assert!(err.is::<ZeroNotAllowed>());
    }

    #[test]
    fn one_error_can_be_read_by_several_threads() {
        let err = run_in_worker("x".to_string()).unwrap_err();
        let err = &err;
        // Scoped threads may borrow `err`, but only if `&BoxError` is `Send`,
        // which is exactly `BoxError: Sync`.
        let lines: Vec<String> = thread::scope(|s| {
            let readers: Vec<_> = (0..3)
                .map(|id| s.spawn(move || format!("reader {id}: {err}")))
                .collect();
            readers.into_iter().map(|r| r.join().unwrap()).collect()
        });
        assert_eq!(
            lines,
            [
                "reader 0: invalid digit found in string",
                "reader 1: invalid digit found in string",
                "reader 2: invalid digit found in string",
            ]
        );
    }
}
