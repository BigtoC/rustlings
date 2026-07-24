// Module 5 · Debugging — part 3: implementing `Display` by hand.
//
// `Debug` (`{:?}`) is auto-derivable and meant for programmers. `Display`
// (`{}`) is the polished, human-facing text and it is NEVER derived — you write
// it yourself by implementing `std::fmt::Display`.
//
// The signature always looks like this:
//
//     impl fmt::Display for MyType {
//         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//             write!(f, "...", ...)   // note: NO trailing semicolon -> this is
//         }                           // the returned `fmt::Result`
//     }
//
// `write!` targets the formatter `f` (like `format!` builds a `String`) and
// returns a `fmt::Result` that you must propagate. Here `Color` also derives
// `Debug`, so you can compare the two: `{:?}` gives the struct dump while `{}`
// gives your custom hex string.

use std::fmt;

#[derive(Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Write the color as a CSS-style hex string like `#ff00aa`: a `#`
        // followed by each channel as EXACTLY two lowercase hex digits.
        // Hint: the `{:02x}` format specifier zero-pads to width 2 in lowercase
        // hex, e.g. `write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)`.
        // Return that `write!(...)` (no trailing semicolon) so the function
        // yields a `fmt::Result`. Until then this will not compile, because the
        // body currently produces `()` instead of `fmt::Result`.
    }
}

fn main() {
    let color = Color {
        r: 255,
        g: 0,
        b: 170,
    };
    println!("display: {color}");
    println!("debug:   {color:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_lowercase_hex() {
        let color = Color {
            r: 255,
            g: 0,
            b: 170,
        };
        assert_eq!(format!("{color}"), "#ff00aa");
    }

    #[test]
    fn display_zero_pads_each_channel() {
        let color = Color { r: 1, g: 2, b: 3 };
        assert_eq!(format!("{color}"), "#010203");
    }
}
