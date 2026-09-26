// Module 1 · Drop and RAII — part 1: predict the drop order: locals, fields, parameters (quiz).
//
// Rust has no garbage collector and no `finally`. It has one rule instead:
// when the OWNER of a value goes away, the value is dropped, right there, at a
// point you can read off the source code. Its `drop` runs first (if its type
// implements `Drop`), and then its fields are dropped, one by one. That is
// RAII ("resource acquisition is initialization"): a `MutexGuard` unlocks, a
// `File` closes and a `Vec` frees its buffer because their owner went away,
// not because somebody remembered to call `close()`. Parts 3 to 6 of this
// module build and debug such types. This part and the next one ask WHEN the
// drops happen, because "in what order are these dropped?" is a favorite
// warm-up question, and because the answer decides whether a guard still
// holds its lock on the next line.
//
// Every scenario below is a function that creates `Noisy` values. A `Noisy`
// writes its name into a log when it is dropped, and `mark("...")` writes a
// milestone into the same log, so the log is the order of events. For each
// scenario, write down the log you expect as the `const` slice under it. Each
// test runs one scenario and compares the log with your constant. On purpose,
// the failure message does NOT print the real log: work it out from the code,
// don't read it off a failed test. (Nothing stops you from printing the log,
// as with any quiz that is checked at run time. It just teaches you nothing.)
//
// Things to keep apart while you predict:
//   - a VALUE and a BINDING: a value is dropped once, by whoever owns it at
//     that moment, and moving it moves the drop along with it;
//   - a binding and a PATTERN: `_x` is a binding with an unusual name, `_` is
//     a pattern that binds nothing at all;
//   - the order things were CREATED in and the order they were DECLARED in.
//
// None of these five answers depends on the edition. Part 2 has one that does.
//
// How interviewers probe it: "What is the difference between
// `let _ = mutex.lock().unwrap();` and `let _guard = mutex.lock().unwrap();`?",
// "In what order are a struct's fields dropped? A function's locals? Its
// parameters?", "What does `drop(x)` actually do?" (look at its body in the
// std docs: it is empty).

use std::cell::RefCell;

thread_local! {
    // The log of this thread. Every test runs on its own thread, so each test
    // sees only its own entries.
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

// Writes a milestone into the log.
fn mark(event: &'static str) {
    LOG.with_borrow_mut(|log| log.push(event));
}

// Writes its name into the log when it is dropped.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        mark(self.0);
    }
}

// ---- The quiz ---------------------------------------------------------------
//
// TODO: Every `S_*` constant below is still `&[]`, but every scenario logs
// something, so each test fails with "S_LOCALS: wrong prediction" (and so on
// for the others). Replace each `&[]` with the complete log you predict for the
// scenario above it: every `Noisy` name and every `mark` entry, in the order
// they happen, written like `&["end of body", "x"]`. Predict first and run
// second: a guess you had to flip teaches you nothing. Don't change the
// scenarios or the tests. Until every prediction is right, the tests will fail.

// 1. Locals, and the elements of a `Vec`.
fn locals() {
    let _first = Noisy("first");
    let _list: Vec<Noisy> = ["list[0]", "list[1]"].into_iter().map(Noisy).collect();
    let _last = Noisy("last");
    mark("end of body");
}

// TODO: your prediction for `locals()`.
const S_LOCALS: &[&str] = &[];

// 2. `let _` versus `let _name`.
fn let_underscore() {
    let _guard = Noisy("guard");
    let _ = Noisy("temporary");
    let named = Noisy("named");
    let _ = named;
    mark("end of body");
}

// TODO: your prediction for `let_underscore()`.
const S_LET_UNDERSCORE: &[&str] = &[];

// 3. A struct with a `Drop` impl of its own, built with its fields in a
// different order than they are declared.
struct Request {
    header: Noisy,
    body: Noisy,
}

impl Drop for Request {
    fn drop(&mut self) {
        mark("request");
    }
}

fn fields() {
    let _request = Request {
        body: Noisy("body"),
        header: Noisy("header"),
    };
    mark("end of body");
}

// TODO: your prediction for `fields()`.
const S_FIELDS: &[&str] = &[];

// 4. Function parameters.
fn handle(_first: Noisy, _second: Noisy) {
    let _local = Noisy("local");
    mark("end of body");
}

fn params() {
    handle(Noisy("first"), Noisy("second"));
    mark("returned");
}

// TODO: your prediction for `params()`.
const S_PARAMS: &[&str] = &[];

// 5. Moves, `drop()` and assignment.
fn moves() {
    let a = Noisy("a");
    let b = Noisy("b");
    let mut _slot = Noisy("old");
    drop(a);
    _slot = Noisy("new");
    let _moved = b;
    mark("end of body");
}

// TODO: your prediction for `moves()`.
const S_MOVES: &[&str] = &[];

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Runs one scenario on an empty log and returns what it logged.
    fn record(scenario: fn()) -> Vec<&'static str> {
        LOG.with_borrow_mut(Vec::clear);
        scenario();
        LOG.take()
    }

    #[test]
    fn how_the_log_works() {
        // Not part of the quiz: one `Noisy`, dropped where its scope ends.
        let log = record(|| {
            mark("created");
            {
                let _only = Noisy("only");
                mark("inner scope ends");
            }
            mark("after the inner scope");
        });
        assert_eq!(
            log,
            [
                "created",
                "inner scope ends",
                "only",
                "after the inner scope"
            ]
        );
    }

    #[test]
    fn quiz_locals_and_vec_elements() {
        assert!(
            record(locals) == S_LOCALS,
            "S_LOCALS: wrong prediction (in which order do locals go, and in \
             which order does a `Vec` drop its elements?)"
        );
    }

    #[test]
    fn quiz_let_underscore() {
        assert!(
            record(let_underscore) == S_LET_UNDERSCORE,
            "S_LET_UNDERSCORE: wrong prediction (which of these `let`s bind \
             anything, and does `let _ = named;` move `named`?)"
        );
    }

    #[test]
    fn quiz_struct_fields() {
        assert!(
            record(fields) == S_FIELDS,
            "S_FIELDS: wrong prediction (does `Request::drop` run before or \
             after its fields are dropped, and does creation order matter?)"
        );
    }

    #[test]
    fn quiz_function_parameters() {
        assert!(
            record(params) == S_PARAMS,
            "S_PARAMS: wrong prediction (who owns an argument once it has \
             been passed, and when does that owner go away?)"
        );
    }

    #[test]
    fn quiz_moves_drop_and_assignment() {
        assert!(
            record(moves) == S_MOVES,
            "S_MOVES: wrong prediction (where does each value's CURRENT owner \
             go away, and what happens to the old value on assignment?)"
        );
    }
}
