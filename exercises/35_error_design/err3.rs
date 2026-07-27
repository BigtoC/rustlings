// Module 5 · Error Design — custom errors and `?`, part 3: `TryFrom` for
// validated construction.
//
// `From` is for conversions that CANNOT fail and lose nothing (and you get the
// matching `Into` for free). But many conversions can fail: not every `i32` is
// a valid percentage. `TryFrom` is the fallible sibling — it carries an
// associated `Error` type and returns `Result`, so you can validate at a type
// boundary and make an invalid value impossible to construct. Implement
// `TryFrom` and you get the matching `TryInto` for free, too.
//
// Note what the associated `Error` is here: a real error type, exactly as part 2
// defined one — `Debug` + `Display` + `std::error::Error`, carrying the rejected
// value. A bare `String` would also compile, but it throws away everything a
// caller might want: there is nothing to `match` on, no `source()` to chain, and to
// let `?` fold it into their own error enum they would need an
// `impl From<String>` — which collides with every other stringly error they handle.

use std::error::Error;
use std::fmt;

// A newtype that should only ever hold 0..=100.
#[derive(Debug)]
struct Percentage(u8);

// The rejection, as a real error type. It keeps the offending value so a caller
// can report it or wrap it in an error of their own.
#[derive(Debug)]
struct OutOfRange(i32);

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is not in 0..=100", self.0)
    }
}

// `Debug` + `Display` are the supertraits, and there is no lower-level cause to
// report here, so the default `source()` (which returns `None`) is correct.
impl Error for OutOfRange {}

impl TryFrom<i32> for Percentage {
    type Error = OutOfRange;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        // TODO: Accept only values in `0..=100` — build a `Percentage` when the
        // value fits, and hand back an `OutOfRange` carrying the rejected value
        // when it does not. An empty body has no `Result<Self, Self::Error>` to
        // return, so this will not compile yet.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_in_range() {
        assert_eq!(Percentage::try_from(50).unwrap().0, 50);
    }

    #[test]
    fn rejects_too_large() {
        assert!(Percentage::try_from(150).is_err());
    }

    #[test]
    fn rejects_negative() {
        assert!(Percentage::try_from(-1).is_err());
    }

    #[test]
    fn try_into_comes_for_free() {
        // Implementing `TryFrom` also gives us `TryInto`.
        let p: Percentage = 25i32.try_into().unwrap();
        assert_eq!(p.0, 25);
    }

    #[test]
    fn the_rejection_is_a_real_error() {
        let err = Percentage::try_from(150).unwrap_err();
        // `Display` carries the user-facing wording...
        assert_eq!(err.to_string(), "150 is not in 0..=100");
        // ...and because it implements `Error` it can be boxed and chained like
        // any other error in the ecosystem, which a `String` could never do.
        let boxed: Box<dyn Error> = Box::new(err);
        assert_eq!(boxed.to_string(), "150 is not in 0..=100");
        assert!(boxed.source().is_none());
    }
}
