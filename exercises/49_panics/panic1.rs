// Traits & Abstraction · Panics — part 1: `catch_unwind`, `AssertUnwindSafe`, and reading the payload (E0277).
//
// A panic means "this is a bug": an index out of bounds, an `unwrap` on
// `None`, a broken invariant. Expected failures are `Result`s
// (`35_error_design`). By default a panic UNWINDS: it walks back up the stack
// of the thread that panicked, running every destructor on the way
// (`39_drop_raii`), until something catches it (below) or it leaves the
// thread's closure. Then that thread is over, and whoever joins it gets
// `Err(payload)` from `join()`. When the main thread panics, the process
// exits with code 101.
//
// Some code must outlive a panic in the code it calls: a test harness (libtest
// runs every test inside `catch_unwind(AssertUnwindSafe(..))`, which is how
// one failing test does not stop the others), a thread pool whose workers
// must survive a bad job, a server that turns a panicking request handler
// into a "500 Internal Server Error" instead of dying (tower-http's
// `CatchPanic` layer does this), or a function called from C, where
// unwinding must not escape. For those boundaries,
// `std::panic::catch_unwind(f)` runs `f` and turns an unwinding panic into
// `Err(payload)`. It is not a `try`/`catch` for everyday errors: it catches
// only UNWINDING panics (with `panic = "abort"` there is nothing to catch,
// see the README), and whether panics unwind at all is the final binary's
// choice, not the library's.
//
// `catch_unwind` requires `F: UnwindSafe`. That auto trait is missing for
// `&mut T`, and for `&T` when `T` has interior mutability (`Cell`, `RefCell`:
// anything built on `UnsafeCell` that does not opt back in). The reason is not
// memory safety: safe code cannot cause undefined behavior by panicking, and
// the escape hatch, `AssertUnwindSafe`, is a SAFE wrapper. The reason is
// logic. After a caught panic, the caller can still see, through those
// references, whatever the closure was halfway through changing, so an
// invariant may be broken. `UnwindSafe` is a lint written in the type system:
// "did you think about that?". `Mutex` and `RwLock` do implement it, because
// they report a panic to the next user: poisoning, part 2.
//
// `run_isolated` below takes ANY closure, so it cannot know whether its
// closure is unwind safe, and adding the bound only pushes the question onto
// every caller: the tests pass closures that capture a `&mut Vec` and a
// `&Cell`. (rustc's help suggests exactly that bound; here it is the wrong
// fix.) A general-purpose boundary asserts unwind safety on its caller's
// behalf and documents what that means: nothing is rolled back, and the
// caller sees the half-done work.
//
// The payload is a `Box<dyn Any + Send>`: `panic!` accepts only messages, but
// `std::panic::panic_any` can throw any `Send + 'static` value. A message
// without formatting arguments arrives as a `&'static str`, a formatted one as
// a `String`, and which one you get is an implementation detail: on this
// toolchain `panic!("job {} failed", 7)` is a `&'static str` (the compiler
// inlines the literal 7), while `.expect("msg")` gives a `String`. Handle
// both. And look inside the right thing: a `Box<dyn Any + Send>` is itself
// `'static` and `Send`, so it is ALSO an `Any`, and asking the box whether it
// is a `String` quietly says no. That is the trap of `32_dispatch/dispatch5`.
//
// How interviewers probe it: "What does `catch_unwind` catch, and what can't
// it catch?", "What do `UnwindSafe` and `AssertUnwindSafe` actually
// guarantee?", "Why not use `catch_unwind` for error handling?", and "How do
// you get the message out of a panic?"
//
// Note: test binaries unwind even though this course's profiles set
// `panic = "abort"` (Cargo ignores that setting for test targets), so every
// `catch_unwind` in this module lives in a `#[test]`. In `main`, a panic
// aborts the process.

use std::any::Any;

/// What `panic_message` returns for a payload that is not a string.
const NON_STRING_PAYLOAD: &str = "panicked with a non-string payload";

/// The message carried by a panic payload, or `NON_STRING_PAYLOAD`. It takes
/// the payload the way std hands it to a panic hook
/// (`PanicHookInfo::payload()` returns a `&(dyn Any + Send)`).
fn panic_message(payload: &(dyn Any + Send)) -> String {
    // TODO: `panic_message_reads_a_static_str` and
    // `panic_message_reads_a_string` fail: this returns the fallback for every
    // payload. A `dyn Any` can only be looked into by asking whether it is one
    // particular concrete type. Return the message when the payload is either
    // of the two string types a panic message can arrive as (see above: you
    // cannot predict which one), and `NON_STRING_PAYLOAD` for anything else.
    // No `unsafe`, no `Debug` formatting of the payload, and don't change the
    // tests. Until you read both string types out of the payload, the tests
    // will fail.
    let _ = payload;
    String::from(NON_STRING_PAYLOAD)
}

/// Runs `f`. Returns what it returned, or, if it panicked, the panic message.
/// Nothing that `f` changed before it panicked is rolled back.
fn run_isolated<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    // TODO: every test whose closure panics fails, with the closure's own
    // message ("boom", "job 7 failed", ...): the panic goes straight through
    // `run_isolated` and fails the test that called it. Stop the unwinding
    // here and return `Err` with the message `panic_message` reads from the
    // payload itself (not from the box it arrives in). Handing `f` to std's
    // catching function as it is gives E0277 "the type `impl FnOnce() -> R`
    // may not be safely transferred across an unwind boundary". Don't follow
    // rustc's help and add that bound to `f`: the tests call `run_isolated`
    // with closures that capture a `&mut Vec` and a `&Cell` (E0277 again, at
    // every such call). Keep the signature (no new bounds on `f` or `R`, so no
    // `UnwindSafe`, `Send` or `'static`), no threads, no panic hook, no
    // `unsafe`, and don't change the tests. Until `run_isolated` catches the
    // panic, the tests will fail.
    Ok(f())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::panic::panic_any;
    use std::rc::Rc;
    use std::thread;

    // A job that fails for one input, with a formatted message.
    fn double(n: u32) -> u32 {
        if n == 7 {
            panic!("job {n} failed");
        }
        n * 2
    }

    #[test]
    fn panic_message_reads_a_static_str() {
        let payload: &'static str = "boom";
        assert_eq!(panic_message(&payload), "boom");
    }

    #[test]
    fn panic_message_reads_a_string() {
        assert_eq!(panic_message(&String::from("job 7 failed")), "job 7 failed");
        assert_eq!(panic_message(&String::new()), "");
    }

    #[test]
    fn panic_message_falls_back_for_other_payloads() {
        assert_eq!(panic_message(&42u8), NON_STRING_PAYLOAD);
        assert_eq!(panic_message(&()), NON_STRING_PAYLOAD);
    }

    #[test]
    fn returns_the_value_when_nothing_panics() {
        assert_eq!(run_isolated(|| 6 * 7), Ok(42));
        // Any `R` will do, including one that owns heap data.
        let words = run_isolated(|| vec![String::from("a"), String::from("b")]);
        assert_eq!(words, Ok(vec![String::from("a"), String::from("b")]));
    }

    #[test]
    fn a_literal_panic_becomes_its_message() {
        let result = run_isolated(|| -> u32 { panic!("boom") });
        assert_eq!(result, Err(String::from("boom")));
    }

    #[test]
    fn a_formatted_panic_becomes_its_message() {
        assert_eq!(
            run_isolated(|| double(7)),
            Err(String::from("job 7 failed"))
        );
        assert_eq!(run_isolated(|| double(4)), Ok(8));
    }

    // A settings lookup that knows a single key.
    fn setting(key: &str) -> Option<u32> {
        (key == "port").then_some(8080)
    }

    #[test]
    fn panics_from_std_carry_their_messages_too() {
        assert_eq!(
            run_isolated(|| setting("timeout").expect("timeout must be set")),
            Err(String::from("timeout must be set"))
        );
        assert_eq!(
            run_isolated(|| setting("timeout").unwrap()),
            Err(String::from("called `Option::unwrap()` on a `None` value"))
        );
        assert_eq!(run_isolated(|| setting("port").unwrap()), Ok(8080));
    }

    #[test]
    fn a_non_string_payload_gets_the_fallback() {
        let result = run_isolated(|| -> u32 { panic_any(42u8) });
        assert_eq!(result, Err(String::from(NON_STRING_PAYLOAD)));
        // A `String` thrown with `panic_any` is still a message.
        let result = run_isolated(|| -> u32 { panic_any(String::from("custom")) });
        assert_eq!(result, Err(String::from("custom")));
    }

    #[test]
    fn the_closure_may_borrow_mutably_and_nothing_is_rolled_back() {
        // Capturing `&mut log` makes this closure `!UnwindSafe`.
        let mut log = Vec::new();
        let result = run_isolated(|| {
            log.push("started");
            panic!("disk full");
        });
        assert_eq!(result, Err(String::from("disk full")));
        // `catch_unwind` stops the unwinding; it does not undo anything. This
        // half-done work is exactly what `UnwindSafe` asks you to think about.
        assert_eq!(log, ["started"]);
    }

    #[test]
    fn one_panicking_job_does_not_stop_the_others() {
        // `&Cell` is neither `UnwindSafe` nor `Send`.
        let finished = Cell::new(0);
        let results: Vec<Result<u32, String>> = [1, 7, 3]
            .into_iter()
            .map(|n| {
                run_isolated(|| {
                    let doubled = double(n);
                    finished.set(finished.get() + 1);
                    doubled
                })
            })
            .collect();
        assert_eq!(
            results,
            [Ok(2), Err(String::from("job 7 failed")), Ok(6)],
            "the job after the panicking one must still run"
        );
        assert_eq!(finished.get(), 2);
    }

    #[test]
    fn the_result_does_not_have_to_be_send() {
        let shared = run_isolated(|| Rc::new(5)).unwrap();
        assert_eq!(*shared, 5);
        assert_eq!(Rc::strong_count(&shared), 1);
    }

    #[test]
    fn a_join_error_carries_the_same_kind_of_payload() {
        // `JoinHandle::join` hands back the payload of the thread's panic.
        let worker = 3;
        let payload = thread::spawn(move || -> u32 { panic!("worker {worker} died") })
            .join()
            .unwrap_err();
        assert_eq!(panic_message(&*payload), "worker 3 died");
    }
}
