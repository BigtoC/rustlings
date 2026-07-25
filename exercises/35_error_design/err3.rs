// Module 5 · Error Design — custom errors and `?`, part 3: `TryFrom` for
// validated construction.
//
// `From` is for conversions that CANNOT fail and lose nothing (and you get the
// matching `Into` for free). But many conversions can fail: not every `i32` is
// a valid percentage. `TryFrom` is the fallible sibling — it carries an
// associated `Error` type and returns `Result`, so you can validate at a type
// boundary and make an invalid value impossible to construct. Implement
// `TryFrom` and you get the matching `TryInto` for free, too.

// A newtype that should only ever hold 0..=100.
struct Percentage(u8);

impl TryFrom<i32> for Percentage {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        // TODO: Accept only values in `0..=100`. When `value` is in range,
        // return `Ok(Percentage(value as u8))`; otherwise return
        //   Err(format!("{value} is not in 0..=100"))
        // An empty body has no return value of type `Result<Self, Self::Error>`,
        // so until you return one from every path this will not compile.
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
}
