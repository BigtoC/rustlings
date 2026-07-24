// Module 1 · Lifetimes (deep) — part 6: lifetimes in an `impl` block.
//
// When you write `impl<'a> Parser<'a>`, the `'a` names the lifetime of the data
// the struct borrows. Inside methods, lifetime ELISION kicks in: a method that
// returns a reference and takes `&self` will, by default, tie that reference to
// `&self` — i.e. the result may not outlive the borrow of `self`.
//
// Sometimes that default is too restrictive. `remainder` hands back a slice of
// the ORIGINAL text, which lives for `'a`. That slice should stay valid even
// after the `Parser` itself is gone, so we must annotate the return type as
// `&'a str` EXPLICITLY, overriding the elided (and shorter) `&self` lifetime.

struct Parser<'a> {
    text: &'a str,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Parser { text }
    }

    fn remainder(&self) -> &'a str {
        self.text
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remainder_outlives_the_parser() {
        let text = String::from("hello world");
        let rest;
        {
            let parser = Parser::new(&text);
            rest = parser.remainder();
            // `parser` is dropped at the end of this block.
        }
        // `rest` still points into `text`, which is alive, so this is fine —
        // but only once `remainder` returns `&'a str` rather than `&self`.
        assert_eq!(rest, "hello world");
    }
}
