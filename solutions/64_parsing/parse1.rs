// Module 5 · Parsing — part 1: a lexer whose tokens borrow the source, cut at byte offsets (E0308).
//
// "Write a calculator" is a classic take-home and live-coding problem, and
// every solution starts the same way: a LEXER (tokenizer) turns the text into
// tokens, so that the parser never sees whitespace or single characters.
// "12 + rate" becomes `Num(12)`, `Plus`, `Ident("rate")`.
//
// In Rust the interesting question is what a token HOLDS. An identifier
// could be a `String`, but then lexing allocates once per name for text that
// already sits in the input. Here `Ident(&'a str)` is a slice of the source
// itself: no allocation, no copy. The `'a` belongs to the source string, not
// to the lexer. `Lexer<'a>` holds a `src: &'a str`, and `&self.src[a..b]` is
// a `&'a str`: the borrow goes through that `&'a str` field, not through
// `&mut self` (the `impl<'a>` lesson of `25_lifetimes_deep/lifetimes6`).
// So the tokens outlive the lexer, and that is exactly why the lexer can be a
// std `Iterator`. `Iterator::Item` cannot borrow from the iterator itself
// (that needs a lending iterator, `43_assoc_types/gat1`), but it can borrow
// from something the iterator borrows.
//
// Slicing a `&str` takes BYTE offsets, and both ends must be char boundaries
// (`63_slices_strings/window2`). `src.char_indices()` yields
// `(byte_offset, char)` pairs, and wrapping it in `Peekable` adds one character
// of lookahead: `peek()` looks at the next pair without consuming it, and
// `next_if(pred)` consumes it only when `pred` says yes. When an identifier or
// a number stops, the offset of the peeked character is where it ends, or
// `src.len()` if the input ran out. Positions in tokens and errors are byte
// offsets too. That is what a caller needs to slice the input or underline the
// error, and in "é + $" the `$` is at byte 5, not at character 4.
//
// Numbers: `str::parse::<i64>` does the arithmetic. Its error type
// `ParseIntError` has `kind()`, and on a run of ASCII digits the only
// possible kind is `IntErrorKind::PosOverflow`, which becomes this lexer's
// `NumberTooLarge`. A calculator's input is user input, so `.unwrap()` is a
// crash waiting to happen (`67_code_review/review1`), and parsing as `u64`
// and casting with `as i64` turns 9223372036854775808 into `i64::MIN`
// without a word.
//
// Which characters are letters and digits? `char::is_alphabetic` knows every
// script, so "héllo" and "日本" are single identifiers. `char::is_numeric` is
// just as generous: '١' (ARABIC-INDIC DIGIT ONE) and '½' are "numeric", and
// `parse::<i64>` rejects both. Numbers are ASCII digits only.
//
// How interviewers probe this: "Why `&'a str` and not `String`? Can a token
// outlive the lexer? Why can't a std `Iterator` hand out slices of its own
// buffer? Which position do you report for an error after an 'é'? What
// happens on 99999999999999999999?"

use std::error::Error;
use std::fmt;
use std::iter::Peekable;
use std::num::IntErrorKind;
use std::str::CharIndices;

/// One token of a calculator's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'a> {
    /// An integer literal: ASCII digits only, at most `i64::MAX`. There are
    /// no negative literals: `-5` is `Minus`, then `Num(5)`.
    Num(i64),
    /// A name. It is a slice of the source, so it copies nothing.
    Ident(&'a str),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `(`
    LParen,
    /// `)`
    RParen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LexError {
    /// A character that cannot start a token, at byte offset `pos`.
    UnexpectedChar { ch: char, pos: usize },
    /// An integer literal above `i64::MAX`, starting at byte offset `pos`.
    NumberTooLarge { pos: usize },
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedChar { ch, pos } => {
                write!(f, "unexpected character {ch:?} at byte {pos}")
            }
            LexError::NumberTooLarge { pos } => {
                write!(f, "number at byte {pos} does not fit in an i64")
            }
        }
    }
}

impl Error for LexError {}

/// Can `c` start an identifier? A letter of any script, or `_`.
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Can `c` continue an identifier? A letter or a digit of any script, or `_`.
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Splits `src` into tokens, one per call to `next`. The lexing rules:
///
/// - Whitespace (`char::is_whitespace`, so Unicode spaces too) separates
///   tokens and is skipped.
/// - `+ - * / ( )` are one token each.
/// - A number is the longest run of ASCII digits `0`-`9` (leading zeros are
///   fine). If its value is above `i64::MAX`, the item is
///   `Err(NumberTooLarge { pos })`, with `pos` the offset of its first digit.
/// - An identifier is a character that satisfies `is_ident_start`, then the
///   longest run of characters that satisfy `is_ident_continue`. So "x2" is
///   one identifier, and "2x" is `Num(2)` followed by `Ident("x")`.
/// - Any other character is `Err(UnexpectedChar { ch, pos })`.
///
/// Every `Ok` item is `(pos, token)`, where `pos` is the byte offset in `src`
/// of the token's first character. After an error the lexer carries on right
/// after the offending character or literal, so the caller can collect every
/// error in one pass or stop at the first one.
struct Lexer<'a> {
    src: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            src,
            chars: src.char_indices().peekable(),
        }
    }

    /// Consumes characters while `pred` holds, and returns the byte offset
    /// just past the last one it consumed. `next_if` only consumes a
    /// character that matches, so the first one that doesn't stays in the
    /// iterator for the next token, and its offset (or `src.len()` at the
    /// end of the input) is exactly where this token ends.
    fn eat_while(&mut self, pred: impl Fn(char) -> bool) -> usize {
        while self.chars.next_if(|&(_, c)| pred(c)).is_some() {}
        self.chars.peek().map_or(self.src.len(), |&(i, _)| i)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(usize, Token<'a>), LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        // `find` consumes the whitespace AND the first other character, and
        // its `?` ends the iteration when only whitespace was left. Every
        // offset comes from `char_indices`, so every slice below starts and
        // ends on a char boundary.
        let (start, c) = self.chars.find(|&(_, c)| !c.is_whitespace())?;
        let token = match c {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '0'..='9' => {
                let end = self.eat_while(|c| c.is_ascii_digit());
                // A non-empty run of ASCII digits can only fail to parse by
                // being too large: `PosOverflow` is the one reachable kind.
                return Some(match self.src[start..end].parse() {
                    Ok(n) => Ok((start, Token::Num(n))),
                    Err(e) => match e.kind() {
                        IntErrorKind::PosOverflow => Err(LexError::NumberTooLarge { pos: start }),
                        kind => unreachable!("ASCII digits failed with {kind:?}"),
                    },
                });
            }
            c if is_ident_start(c) => {
                let end = self.eat_while(is_ident_continue);
                // `self.src` is a `&'a str`, so this slice is a `&'a str`
                // too: it borrows the source, not the lexer.
                Token::Ident(&self.src[start..end])
            }
            // The bad character is already consumed, so the next call
            // carries on after it.
            ch => return Some(Err(LexError::UnexpectedChar { ch, pos: start })),
        };
        Some(Ok((start, token)))
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use Token::*;

    type Item<'a> = Result<(usize, Token<'a>), LexError>;

    // Every item consumes at least one byte of input, so a lexer that yields
    // more than `src.len()` items is not making progress (and would loop
    // forever). Collect at most that many, and fail instead of hanging.
    #[track_caller]
    fn lex(src: &str) -> Vec<Item<'_>> {
        let mut lexer = Lexer::new(src);
        let mut items = Vec::new();
        for _ in 0..=src.len() {
            match lexer.next() {
                Some(item) => items.push(item),
                None => return items,
            }
        }
        panic!(
            "more than {} items for {src:?}: the lexer is not consuming its input",
            src.len()
        );
    }

    // Only the tokens, for inputs that must lex without errors.
    #[track_caller]
    fn tokens(src: &str) -> Vec<Token<'_>> {
        lex(src)
            .into_iter()
            .map(|item| match item {
                Ok((_, token)) => token,
                Err(e) => panic!("{src:?} should lex, got {e:?}"),
            })
            .collect()
    }

    fn unexpected(ch: char, pos: usize) -> LexError {
        LexError::UnexpectedChar { ch, pos }
    }

    #[test]
    fn lexes_an_expression_with_byte_positions() {
        assert_eq!(
            lex("12 + x*(3 - y1)/_tmp"),
            vec![
                Ok((0, Num(12))),
                Ok((3, Plus)),
                Ok((5, Ident("x"))),
                Ok((6, Star)),
                Ok((7, LParen)),
                Ok((8, Num(3))),
                Ok((10, Minus)),
                Ok((12, Ident("y1"))),
                Ok((14, RParen)),
                Ok((15, Slash)),
                Ok((16, Ident("_tmp"))),
            ]
        );
    }

    #[test]
    fn tokens_need_no_whitespace_between_them() {
        assert_eq!(tokens("1+2"), [Num(1), Plus, Num(2)]);
        assert_eq!(
            tokens("(a)-b"),
            [LParen, Ident("a"), RParen, Minus, Ident("b")]
        );
        assert_eq!(tokens("x2"), [Ident("x2")]);
        assert_eq!(tokens("2x"), [Num(2), Ident("x")]);
        assert_eq!(tokens("a_b c__"), [Ident("a_b"), Ident("c__")]);
    }

    #[test]
    fn minus_is_always_its_own_token() {
        assert_eq!(lex("-5"), vec![Ok((0, Minus)), Ok((1, Num(5)))]);
        assert_eq!(tokens("3-2"), [Num(3), Minus, Num(2)]);
        assert_eq!(tokens("--7"), [Minus, Minus, Num(7)]);
    }

    #[test]
    fn empty_and_blank_inputs_have_no_tokens() {
        assert_eq!(lex(""), vec![]);
        assert_eq!(lex(" \t\r\n "), vec![]);
        // U+3000 IDEOGRAPHIC SPACE (3 bytes) and U+00A0 NO-BREAK SPACE (2
        // bytes) are whitespace too.
        assert_eq!(lex("\u{3000}1\u{a0}"), vec![Ok((3, Num(1)))]);
    }

    #[test]
    fn a_unicode_identifier_is_one_token() {
        assert_eq!(lex("héllo"), vec![Ok((0, Ident("héllo")))]);
        // "héllo" is 6 bytes, so `+` sits at byte 7 and "wörld" at byte 9.
        assert_eq!(
            lex("héllo + wörld"),
            vec![
                Ok((0, Ident("héllo"))),
                Ok((7, Plus)),
                Ok((9, Ident("wörld")))
            ]
        );
        assert_eq!(
            lex("日本 語*π2"),
            vec![
                Ok((0, Ident("日本"))),
                Ok((7, Ident("語"))),
                Ok((10, Star)),
                Ok((11, Ident("π2"))),
            ]
        );
        // A digit of another script may continue a name, but not start a
        // number.
        assert_eq!(lex("x١"), vec![Ok((0, Ident("x١")))]);
    }

    #[test]
    fn identifiers_are_slices_of_the_source() {
        let src = String::from("alpha + héllo*beta_2");
        let mut found = Vec::new();
        for item in lex(&src) {
            if let Ok((pos, Ident(name))) = item {
                // The same bytes at the same address: a borrow of `src`,
                // not a copy of it.
                let at_pos = src.get(pos..pos + name.len());
                assert!(
                    at_pos.is_some_and(|s| std::ptr::eq(name, s)),
                    "Ident({name:?}) is not a slice of the source at byte {pos}"
                );
                found.push(name);
            }
        }
        assert_eq!(found, ["alpha", "héllo", "beta_2"]);
    }

    #[test]
    fn tokens_outlive_the_lexer() {
        let src = String::from("width * height");
        let collected: Vec<Item<'_>> = {
            let lexer = Lexer::new(&src);
            // At most one item per byte, as in `lex`, so a lexer that stops
            // consuming fails here instead of collecting forever.
            lexer.take(src.len() + 1).collect()
        };
        // The lexer is gone, but the tokens borrow `src`, which is still
        // alive.
        assert_eq!(
            collected,
            vec![
                Ok((0, Ident("width"))),
                Ok((6, Star)),
                Ok((8, Ident("height")))
            ]
        );
    }

    #[test]
    fn unexpected_characters_report_their_byte_offset() {
        assert_eq!(lex("$"), vec![Err(unexpected('$', 0))]);
        assert_eq!(lex("€"), vec![Err(unexpected('€', 0))]);
        // 'é' is 2 bytes: counted in chars the `$` is at 4, but its byte
        // offset is 5.
        assert_eq!(
            lex("é + $"),
            vec![Ok((0, Ident("é"))), Ok((3, Plus)), Err(unexpected('$', 5))]
        );
        assert_eq!(
            lex("1 % 2"),
            vec![Ok((0, Num(1))), Err(unexpected('%', 2)), Ok((4, Num(2)))]
        );
    }

    #[test]
    fn lexing_resumes_after_an_error() {
        // '€' is 3 bytes (6..9), so the last number starts at byte 10.
        assert_eq!(
            lex("1 $ 2 € 3"),
            vec![
                Ok((0, Num(1))),
                Err(unexpected('$', 2)),
                Ok((4, Num(2))),
                Err(unexpected('€', 6)),
                Ok((10, Num(3))),
            ]
        );
        assert_eq!(
            lex("$$"),
            vec![Err(unexpected('$', 0)), Err(unexpected('$', 1))]
        );
    }

    #[test]
    fn digits_are_ascii_only() {
        // ARABIC-INDIC DIGITs, FULLWIDTH DIGIT ONE and VULGAR FRACTION ONE
        // HALF are all `char::is_numeric`, and `parse::<i64>` rejects every
        // one of them.
        assert_eq!(
            lex("١٢"),
            vec![Err(unexpected('١', 0)), Err(unexpected('٢', 2))]
        );
        assert_eq!(lex("１"), vec![Err(unexpected('１', 0))]);
        assert_eq!(lex("1½"), vec![Ok((0, Num(1))), Err(unexpected('½', 1))]);
    }

    #[test]
    fn numbers_up_to_i64_max() {
        assert_eq!(tokens("0"), [Num(0)]);
        assert_eq!(tokens("007"), [Num(7)]);
        assert_eq!(tokens("9223372036854775807"), [Num(i64::MAX)]);
        // Leading zeros are not an overflow, however many there are.
        let src = format!("{}42", "0".repeat(100));
        assert_eq!(tokens(&src), [Num(42)]);
    }

    #[test]
    fn a_number_above_i64_max_is_an_error() {
        let too_large = |pos| Err(LexError::NumberTooLarge { pos });
        // i64::MAX + 1: parsing as u64 and casting would give i64::MIN.
        assert_eq!(lex("9223372036854775808"), vec![too_large(0)]);
        assert_eq!(
            lex("x + 18446744073709551616"),
            vec![Ok((0, Ident("x"))), Ok((2, Plus)), too_large(4)]
        );
        // The whole literal is skipped, and lexing carries on after it.
        assert_eq!(
            lex("99999999999999999999 1"),
            vec![too_large(0), Ok((21, Num(1)))]
        );
        assert_eq!(lex(&"9".repeat(1000)), vec![too_large(0)]);
    }

    #[test]
    fn the_lexer_stays_finished() {
        let mut lexer = Lexer::new("7 ");
        assert_eq!(lexer.next(), Some(Ok((0, Num(7)))));
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.next(), None);
    }

    // A slow but obviously correct model: classify each character by index
    // into a collected `Vec`, with no lookahead tricks.
    fn model(src: &str) -> Vec<Item<'_>> {
        let chars: Vec<(usize, char)> = src.char_indices().collect();
        let end_of = |k: usize| chars.get(k).map_or(src.len(), |&(i, _)| i);
        let mut items = Vec::new();
        let mut k = 0;
        while k < chars.len() {
            let (pos, c) = chars[k];
            k += 1;
            let single = match c {
                '+' => Some(Plus),
                '-' => Some(Minus),
                '*' => Some(Star),
                '/' => Some(Slash),
                '(' => Some(LParen),
                ')' => Some(RParen),
                _ => None,
            };
            if c.is_whitespace() {
                continue;
            } else if let Some(token) = single {
                items.push(Ok((pos, token)));
            } else if c.is_ascii_digit() {
                let mut value: Option<i64> = Some(i64::from(c as u8 - b'0'));
                while k < chars.len() && chars[k].1.is_ascii_digit() {
                    let digit = i64::from(chars[k].1 as u8 - b'0');
                    value = value
                        .and_then(|v| v.checked_mul(10))
                        .and_then(|v| v.checked_add(digit));
                    k += 1;
                }
                items.push(match value {
                    Some(n) => Ok((pos, Num(n))),
                    None => Err(LexError::NumberTooLarge { pos }),
                });
            } else if c.is_alphabetic() || c == '_' {
                while k < chars.len() && (chars[k].1.is_alphanumeric() || chars[k].1 == '_') {
                    k += 1;
                }
                items.push(Ok((pos, Ident(&src[pos..end_of(k)]))));
            } else {
                items.push(Err(unexpected(c, pos)));
            }
        }
        items
    }

    // A tiny deterministic pseudo-random generator (a 64-bit LCG), so every
    // run checks exactly the same inputs.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    #[test]
    fn matches_a_simple_model() {
        let alphabet = [
            "0",
            "7",
            "9",
            "x",
            "é",
            "日",
            "_",
            "١",
            " ",
            "\u{3000}",
            "+",
            "-",
            "*",
            "/",
            "(",
            ")",
            "$",
            "€",
            "999999999",
            "99999999999",
        ];
        let mut state = 64;
        for _ in 0..3_000 {
            let len = next(&mut state) % 16;
            let src: String = (0..len)
                .map(|_| alphabet[(next(&mut state) as usize) % alphabet.len()])
                .collect();
            assert_eq!(lex(&src), model(&src), "input {src:?}");
        }
    }
}
