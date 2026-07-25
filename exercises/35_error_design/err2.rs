// Module 5 · Error Design — custom errors and `?`, part 2: `Display`, `Error`,
// and the `source()` cause chain.
//
// A type is "a real error" when it implements `std::error::Error`, whose
// supertraits are `Debug` (for programmers) and `Display` (for end users), plus
// an optional `source()` method that returns the lower-level error that caused
// this one. Returning `Some(&cause)` from `source()` builds a chain that generic
// reporters walk to print "... caused by ..." lines, and that `Box<dyn Error>`
// relies on. `Display` can never be derived — you always write `fmt` by hand.

use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

// An error that remembers WHICH line failed and keeps the underlying parse
// error as its cause.
#[derive(Debug)]
struct ParseRecordError {
    line: usize,
    source: ParseIntError,
}

impl fmt::Display for ParseRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Write the human-facing message into the formatter and return the
        // resulting `fmt::Result`:
        //   write!(f, "failed to parse record on line {}", self.line)
        // An empty body returns `()`, not `fmt::Result`, so this will not
        // compile until you write to `f` here.
    }
}

impl Error for ParseRecordError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // TODO: Expose the underlying cause so reporters can walk the chain:
        //   Some(&self.source)
        // An empty body returns `()`, not `Option<...>`; return the cause here.
    }
}

fn parse_record(line: usize, text: &str) -> Result<i32, ParseRecordError> {
    text.parse::<i32>()
        .map_err(|e| ParseRecordError { line, source: e })
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_line() {
        let err = parse_record(7, "xyz").unwrap_err();
        assert_eq!(err.to_string(), "failed to parse record on line 7");
    }

    #[test]
    fn source_exposes_the_cause() {
        let err = parse_record(7, "xyz").unwrap_err();
        assert!(err.source().is_some());
    }
}
