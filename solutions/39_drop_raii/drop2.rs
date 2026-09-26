// Module 1 · Drop and RAII — part 2: predict when temporaries are dropped (quiz).
//
// A TEMPORARY is a value that no binding owns: the `Noisy` in
// `Noisy("t").len()`, or the `MutexGuard` in `m.lock().unwrap().len()`. Part 1
// was mostly about values with an owner. A temporary has none, so the language
// decides when it goes away: at the end of its TEMPORARY SCOPE. For most
// temporaries that is the end of the enclosing statement, but not for all of
// them. The scenarios below probe the cases that matter in practice:
//   - a `let` statement whose initializer creates temporaries;
//   - a `let` that binds a REFERENCE to a temporary (`let r = &make();`);
//   - the tail expression of a block (the last expression, without `;`);
//   - the condition of an `if`.
//
// Why interviewers care: a guard that is a temporary unlocks (or releases its
// `RefCell` borrow) when its temporary scope ends, not after its last use. The
// same rules therefore decide whether `if m.lock().unwrap().is_empty() { .. }`
// may lock `m` again inside the block. `31_debugging/debugging6..8` (later in
// the course) turns this into real bugs, with the `match`, `while let` and
// `if let` scrutinees that this quiz leaves out.
//
// As in part 1, write the log you expect as the `const` under each scenario;
// the test does not print the real log. The Reference's "Destructors" chapter
// has the rules (see this module's README), but try without it first.
//
// One of these four answers is different in edition 2021. Once you pass, find
// out which one without changing the course: from the rustlings directory,
// compile this file on its own with
//   rustc --edition 2021 --test exercises/39_drop_raii/drop2.rs -o target/drop2
// then run `target/drop2` and see which test fails. (Don't switch the edition
// in the course's `Cargo.toml`: other exercises rely on edition-2024 features.)
//
// How interviewers probe it: "When is a temporary dropped?", "What is
// temporary lifetime extension?", "Why does `let r = &String::from("hi");`
// compile, while `let s = String::from("hi").as_str();` is E0716 as soon as
// `s` is used?" (`37_borrowck_errors/borrowck2`), and "What did Rust 2024
// change about temporaries?"

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

impl Noisy {
    // Returns a plain number, so the result borrows nothing from `self`.
    fn len(&self) -> usize {
        self.0.len()
    }
}

// ---- The quiz ---------------------------------------------------------------
//
// The rule behind every answer: a temporary is dropped at the end of the
// smallest enclosing TEMPORARY SCOPE. A statement is one, and so are an `if`
// condition and (since edition 2024) a block's tail expression. Temporaries
// that end at the same point are dropped in reverse order of creation. The
// one exception is lifetime extension: `let x = &temp;` gives the temporary
// the lifetime of the enclosing block, as if it were a hidden local.

// 1. Two temporaries in one `let` statement.
fn statement() {
    let _total = Noisy("left").len() + Noisy("right").len();
    mark("next statement");
}

// Both temporaries live until the end of the `let` statement (the `;`), not
// just until `.len()` returns, and they go in reverse order of creation.
// `let _n = guard_temp().len();` releases a guard at the `;` for this reason.
const S_STATEMENT: &[&str] = &["right", "left", "next statement"];

// 2. A reference to a temporary, bound with `let`.
fn borrowed() {
    let _borrowed = &Noisy("borrowed");
    let _local = Noisy("local");
    mark("end of body");
}

// Temporary lifetime extension: the operand of `&` in a `let` initializer
// lives as long as the block, exactly like a hidden local declared at that
// statement. `_local` is declared after it, so `_local` goes first. (A method
// call is not extended: `let s = make().as_str();` is E0716 once `s` is used.)
const S_BORROWED: &[&str] = &["end of body", "local", "borrowed"];

// 3. A temporary in the tail expression of a block.
fn block_tail() {
    let _len = {
        let _local = Noisy("local");
        Noisy("tail").len()
    };
    mark("after the block");
}

// Since edition 2024, a block's tail expression is a temporary scope of its
// own, so its temporaries go BEFORE the block's locals. In edition 2021 they
// lived until the end of the enclosing statement, after the locals, which is
// why `fn f() -> usize { let c = RefCell::new(..); c.borrow().len() }` used
// to be E0597: the `Ref` outlived `c`.
const S_TAIL: &[&str] = &["tail", "local", "after the block"];

// 4. A temporary in an `if` condition.
fn if_condition() {
    if Noisy("condition").len() > 3 {
        mark("then block");
    }
    mark("after the if");
}

// An `if` condition is a temporary scope of its own (so is a `while`
// condition): its temporaries are dropped once the condition has been
// evaluated, BEFORE the then-block runs. A `match` or `if let` scrutinee is
// NOT one of these, which is the bug in `31_debugging/debugging6..8`.
const S_IF_CONDITION: &[&str] = &["condition", "then block", "after the if"];

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
    fn quiz_temporaries_in_a_let_statement() {
        assert!(
            record(statement) == S_STATEMENT,
            "S_STATEMENT: wrong prediction (where does the temporary scope of \
             a `let` initializer end, and in which order do its temporaries go?)"
        );
    }

    #[test]
    fn quiz_a_borrowed_temporary() {
        assert!(
            record(borrowed) == S_BORROWED,
            "S_BORROWED: wrong prediction (what does `let x = &temp;` do to the \
             temporary's lifetime?)"
        );
    }

    #[test]
    fn quiz_a_block_tail_temporary() {
        assert!(
            record(block_tail) == S_TAIL,
            "S_TAIL: wrong prediction (in edition 2024, which goes first: the \
             tail expression's temporary or the block's locals?)"
        );
    }

    #[test]
    fn quiz_an_if_condition_temporary() {
        assert!(
            record(if_condition) == S_IF_CONDITION,
            "S_IF_CONDITION: wrong prediction (is the condition's temporary \
             still alive inside the then-block?)"
        );
    }
}
