// Module 5 · Debugging — part 1: the `{:?}` formatter and `#[derive(Debug)]`.
//
// `println!("{}", x)` uses the `Display` trait: a clean, human-facing string
// that you write by hand. `println!("{:?}", x)` uses the `Debug` trait: a
// programmer-facing dump meant for logging and, well, debugging.
//
// Almost every type can get a Debug impl for free by adding `#[derive(Debug)]`.
// Once a type is `Debug`, you get three superpowers at once:
//   - `format!("{:?}", x)`   -> compact one-line dump
//   - `format!("{:#?}", x)`  -> "pretty", indented multi-line dump
//   - `dbg!(x)`              -> prints file:line plus the `{:#?}` dump to stderr
//                               and hands the value back (great for probing).
// (`dbg!` is a debugging aid, not something you leave in finished code.)

#[derive(Debug)]
struct Config {
    retries: u32,
    verbose: bool,
}

fn main() {
    let config = Config {
        retries: 3,
        verbose: true,
    };
    // Once `Config: Debug`, both of these formatters start working.
    println!("compact: {config:?}");
    println!("pretty:\n{config:#?}");

    // Read the fields directly too, so the values are genuinely used.
    if config.verbose {
        println!("retrying up to {} times", config.retries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_matches() {
        let config = Config {
            retries: 3,
            verbose: true,
        };
        // The derived `Debug` prints the type name, then each field as
        // `name: value`, comma-separated inside braces.
        assert_eq!(
            format!("{config:?}"),
            "Config { retries: 3, verbose: true }"
        );
    }

    #[test]
    fn pretty_debug_is_multiline() {
        let config = Config {
            retries: 3,
            verbose: true,
        };
        // The `{:#?}` alternate form spreads the fields across lines.
        let pretty = format!("{config:#?}");
        assert!(pretty.contains("retries: 3"));
        assert!(pretty.contains('\n'));
    }
}
