// Module 1 · Drop and RAII — part 5: the drop check: adding `impl Drop` can break code (E0597).
//
// Locals are dropped in reverse order of declaration (part 1). Usually the
// borrow checker does not mind a reference that outlives what it points at,
// as long as nobody USES the reference afterwards: a `Vec<&u8>` may hold
// references to a local that is dropped before the `Vec` is, because dropping
// a `&u8` does nothing. `count_short` below does exactly that, and it
// compiles.
//
// A `Drop` impl changes the picture. `Inspector::drop` reads `self.0`, so
// dropping an `Inspector` IS a use of its borrow, and the borrow must still be
// valid when the inspector is dropped. The borrow checker's DROP CHECK
// (dropck) enforces that. It never looks inside a `drop` body. For every type
// with a `Drop` impl it assumes the worst, that `drop` may use every reference
// the value holds, so whatever the value borrows must STRICTLY outlive it:
// dropped later, not at the same moment. Delete `impl Drop for Inspector` and
// `run_shift` compiles as it is. That is the surprise interviewers ask about:
// adding an `impl Drop` can be a breaking change.
//
// rustc's error here even mentions "the `Drop` code for type `Vec`", which
// looks odd, because `count_short` shows that a `Vec` of references is fine.
// It is fine because std's `Vec` promises, with the attribute `#[may_dangle]`
// on its (`unsafe`) `Drop` impl, that its `drop` never reads the elements; it
// only drops them. For `&u8` elements that is a no-op, so dangling references
// are harmless. But dropping an `Inspector` element runs `Inspector::drop`,
// which reads through the reference, so a `Vec<Inspector<'a>>` still needs
// `'a` to be alive when the `Vec` is dropped. Only std (and nightly code) can
// make that promise: `#[may_dangle]` is the unstable `dropck_eyepatch` feature
// and needs `unsafe`, so neither is available in this course (nor needed).
//
// How interviewers probe it: "Why can adding `impl Drop` make code stop
// compiling?", "What is the drop check?", "Why does `Vec<&T>` not have that
// problem?" (`#[may_dangle]`), and "How do you fix it?" (make the borrowed
// value strictly outlive the borrower).

use std::cell::RefCell;

thread_local! {
    // The log of this thread. Every test runs on its own thread, so each test
    // sees only its own entries.
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn record(entry: String) {
    LOG.with_borrow_mut(|log| log.push(entry));
}

// The day counts of a shift. Its `Drop` only writes to the log, so that the
// tests can see WHEN the roster is freed.
struct Roster {
    days_left: Vec<u8>,
}

impl Drop for Roster {
    fn drop(&mut self) {
        record(format!("roster of {} freed", self.days_left.len()));
    }
}

impl Roster {
    // Parses a comma-separated list of day counts, skipping anything that is
    // not a number from 0 to 255.
    fn parse(text: &str) -> Self {
        let days_left = text
            .split(',')
            .filter_map(|days| days.trim().parse().ok())
            .collect();
        Roster { days_left }
    }

    fn days_left(&self) -> &[u8] {
        &self.days_left
    }
}

// The Rustonomicon's inspector: it borrows a day count and reads it when it is
// dropped.
struct Inspector<'a>(&'a u8);

impl Drop for Inspector<'_> {
    fn drop(&mut self) {
        record(format!("I was only {} days from retirement!", self.0));
    }
}

// Puts one inspector on duty for every entry of the roster. They all retire
// (are dropped) when the shift ends. Returns how many were on duty.
fn run_shift(text: &str) -> usize {
    // Declaring `roster` FIRST makes it outlive `on_duty`: locals are dropped
    // in reverse order, so the inspectors (and the `Vec` that owns them) are
    // dropped before the roster they borrow from, which is exactly what the
    // drop check demands. A `drop(on_duty)` at the end would NOT be enough:
    // if a `push` panicked, unwinding would still drop `roster` first, while
    // `on_duty` holds inspectors that read it, and the checker rejects that
    // path too.
    let roster = Roster::parse(text);
    let mut on_duty = Vec::new();
    for days in roster.days_left() {
        on_duty.push(Inspector(days));
    }
    on_duty.len()
}

// For contrast (already fine, nothing to fix): the SAME declaration order,
// but the `Vec` holds plain `&u8`s. Dropping a `&u8` reads nothing, and `Vec`
// promises not to read its elements, so the dangling references are never
// used. Returns how many entries are at most `max`.
fn count_short(text: &str, max: u8) -> usize {
    let mut short: Vec<&u8> = Vec::new();
    let roster = Roster::parse(text);
    short.extend(roster.days_left().iter().filter(|&&days| days <= max));
    short.len()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Empties this thread's log and returns what was in it.
    fn take_log() -> Vec<String> {
        LOG.take()
    }

    #[test]
    fn an_inspector_reads_its_borrow_when_dropped() {
        take_log();
        let days = 7;
        {
            let _inspector = Inspector(&days);
            assert!(LOG.with_borrow(Vec::is_empty));
        }
        assert_eq!(take_log(), ["I was only 7 days from retirement!"]);
    }

    #[test]
    fn every_inspector_retires_before_the_roster_is_freed() {
        take_log();
        assert_eq!(run_shift("30, 12"), 2);
        assert_eq!(
            take_log(),
            [
                // A `Vec` drops its elements front to back (`drop1`).
                "I was only 30 days from retirement!",
                "I was only 12 days from retirement!",
                // Only then does the data they borrowed go away.
                "roster of 2 freed",
            ]
        );
    }

    #[test]
    fn a_single_entry_shift() {
        take_log();
        assert_eq!(run_shift("5"), 1);
        assert_eq!(
            take_log(),
            ["I was only 5 days from retirement!", "roster of 1 freed"]
        );
    }

    #[test]
    fn an_empty_shift_still_frees_its_roster() {
        take_log();
        assert_eq!(run_shift(""), 0);
        assert_eq!(take_log(), ["roster of 0 freed"]);
    }

    #[test]
    fn plain_references_may_outlive_their_target() {
        take_log();
        assert_eq!(count_short("3, 40, 1, 12", 5), 2);
        // Nothing reads the `&u8`s after the roster is gone.
        assert_eq!(take_log(), ["roster of 4 freed"]);
    }
}
