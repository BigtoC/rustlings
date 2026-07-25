// Module 5 · Error Design — custom errors and `?`, part 1: `From` powers `?`.
//
// The `?` operator is the workhorse of Rust error handling, but it does more
// than "return early on `Err`". When you write `expr?` and `expr` is
// `Err(e)`, it desugars to `return Err(From::from(e))`. That `From::from` is
// the hinge: it converts the low-level error into whatever error type your
// function returns. So one `impl From<LowLevel> for MyError` lets `?` bridge a
// foreign error into your domain error everywhere, replacing piles of
// `.map_err(|e| MyError::...(e))` calls with a bare `?`.

use std::num::ParseIntError;

// Our domain error: either the number parsed but is out of range, or the string
// failed to parse as a number at all (wrapping the std error as the cause).
#[derive(Debug, PartialEq)]
enum ConfigError {
    OutOfRange(u32),
    Parse(ParseIntError),
}

impl From<ParseIntError> for ConfigError {
    // TODO: Return the incoming `ParseIntError` wrapped as `ConfigError::Parse`.
    //   fn from(e: ParseIntError) -> Self {
    //       ConfigError::Parse(e)
    //   }
    // An empty body returns `()` instead of `Self`, so the trait won't be
    // satisfied. Until you return a `ConfigError` here, this exercise will not
    // compile.
    fn from(e: ParseIntError) -> Self {}
}

fn parse_port(s: &str) -> Result<u16, ConfigError> {
    // `s.parse()` yields `Result<u32, ParseIntError>`; `?` converts the error
    // to `ConfigError` via the `From` impl above.
    let n: u32 = s.parse()?;
    if n > 65535 {
        return Err(ConfigError::OutOfRange(n));
    }
    Ok(n as u16)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_port() {
        assert_eq!(parse_port("8080"), Ok(8080));
    }

    #[test]
    fn non_numeric_becomes_parse_error() {
        assert!(matches!(parse_port("nope"), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn too_large_becomes_out_of_range() {
        assert!(matches!(
            parse_port("99999"),
            Err(ConfigError::OutOfRange(99999))
        ));
    }
}
