// Module 5 · Parsing — part 2: recursive descent with left associativity, unary minus and checked arithmetic.
//
// Recursive descent writes one function per precedence level, and each
// level reads its operands with the next tighter one:
//
//     expr   handles + and -, and reads its operands with term
//     term   handles * and /, and reads its operands with factor
//     factor reads a number or a parenthesized expr
//
// Precedence falls out of the call structure: in `2 + 3 * 4`, `expr` asks
// `term` for its right operand, and `term` swallows all of `3 * 4` first.
//
// Associativity is where the textbook grammar bites. Mathematically `8 - 3 - 2`
// is `(8 - 3) - 2`, a LEFT-recursive rule, `expr := expr '-' term`. Coded as
// written, `expr` calls itself before consuming a token and recurses until the
// stack runs out. The tempting "fix" is the given parser below:
// `expr := term (('+' | '-') expr)?`, which recurses on the RIGHT instead. It
// terminates and gets precedence right, but it reads `8 - 3 - 2` as
// `8 - (3 - 2)` = 7, and `8 / 4 / 2` as `8 / (4 / 2)` = 4. It also nests one
// call deeper for every operator, so a flat `1 + 1 + ... + 1` needs as much
// stack as it has terms. The standard shape is a LOOP: read one operand, then,
// while the next token is an operator of this level, read another operand and
// fold it into the running value. Precedence climbing and Pratt parsing are
// that same loop, with a table of binding powers instead of one function per
// level.
//
// Real nesting still recurses: parentheses, and here unary minus. Every
// level costs stack frames, and running out of stack is not a panic. The
// process aborts ("thread '...' has overflowed its stack"), and
// `catch_unwind` cannot stop it. So the given `nested` method caps the depth
// at `MAX_DEPTH` levels and returns `TooDeep` instead. serde_json uses the
// same limit, 128, by default.
//
// Evaluation must not panic either. `+`, `-` and `*` panic on overflow in debug
// builds and wrap in release (`31_debugging/debugging2`); `checked_*` returns
// `None` instead. Division fails in two ways, and both panic in EVERY build
// profile: a zero divisor ("attempt to divide by zero") and `i64::MIN / -1`
// ("attempt to divide with overflow", because 2^63 does not fit in an `i64`).
// `checked_div` returns `None` for both, so it cannot tell the caller which one
// happened: test the divisor for zero first. `checked_neg` catches the last
// asymmetry of two's complement, `-i64::MIN`. That asymmetry is also why this
// language has no literal for `i64::MIN`: 9223372036854775808 does not fit, so
// the lexer rejects it before any minus sign can apply (rustc special-cases a
// negated literal; this calculator does not), and the tests write
// `(-9223372036854775807 - 1)` instead.
//
// How interviewers probe this: "Why does your evaluator give 8-3-2 = 7? Why
// not just write the left-recursive rule? How do you add a new precedence
// level? What does `i64::MIN / -1` do? What input crashes your parser?"

use std::error::Error;
use std::fmt;
use std::iter::Peekable;
use std::num::IntErrorKind;
use std::str::CharIndices;

// ---------------------------------------------------------------------------
// Part 1's lexer, solved (given). Same rules, with the identifier helpers
// inlined.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'a> {
    Num(i64),
    Ident(&'a str),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LexError {
    UnexpectedChar { ch: char, pos: usize },
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

    fn eat_while(&mut self, pred: impl Fn(char) -> bool) -> usize {
        while self.chars.next_if(|&(_, c)| pred(c)).is_some() {}
        self.chars.peek().map_or(self.src.len(), |&(i, _)| i)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(usize, Token<'a>), LexError>;

    fn next(&mut self) -> Option<Self::Item> {
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
                return Some(match self.src[start..end].parse() {
                    Ok(n) => Ok((start, Token::Num(n))),
                    Err(e) => match e.kind() {
                        IntErrorKind::PosOverflow => Err(LexError::NumberTooLarge { pos: start }),
                        kind => unreachable!("ASCII digits failed with {kind:?}"),
                    },
                });
            }
            c if c.is_alphabetic() || c == '_' => {
                let end = self.eat_while(|c| c.is_alphanumeric() || c == '_');
                Token::Ident(&self.src[start..end])
            }
            ch => return Some(Err(LexError::UnexpectedChar { ch, pos: start })),
        };
        Some(Ok((start, token)))
    }
}

// ---------------------------------------------------------------------------
// The evaluator.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvalError {
    /// The lexer rejected the input.
    Lex(LexError),
    /// The input ended where an operand or a `)` was expected.
    UnexpectedEnd,
    /// A token that cannot appear here, at byte offset `pos`: an operator
    /// where an operand was expected, an identifier (this calculator has no
    /// variables), or anything but `)` after a parenthesized expression.
    UnexpectedToken { pos: usize },
    /// A complete expression followed by more tokens, the first of them at
    /// byte offset `pos`: `1 2`, `(1))`.
    TrailingInput { pos: usize },
    /// A zero divisor.
    DivByZero,
    /// A result outside the `i64` range.
    Overflow,
    /// Nesting deeper than `MAX_DEPTH` levels.
    TooDeep,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::Lex(e) => write!(f, "{e}"),
            EvalError::UnexpectedEnd => f.write_str("unexpected end of input"),
            EvalError::UnexpectedToken { pos } => write!(f, "unexpected token at byte {pos}"),
            EvalError::TrailingInput { pos } => write!(f, "unexpected input at byte {pos}"),
            EvalError::DivByZero => f.write_str("division by zero"),
            EvalError::Overflow => f.write_str("arithmetic overflow"),
            EvalError::TooDeep => write!(f, "nested more than {MAX_DEPTH} levels deep"),
        }
    }
}

impl Error for EvalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            EvalError::Lex(e) => Some(e),
            _ => None,
        }
    }
}

// `?` turns a `LexError` into an `EvalError` through this impl
// (`35_error_design/err1`).
impl From<LexError> for EvalError {
    fn from(e: LexError) -> Self {
        EvalError::Lex(e)
    }
}

/// How many levels parentheses and unary minuses may nest.
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    tokens: Peekable<Lexer<'a>>,
    depth: usize,
}

impl<'a> Parser<'a> {
    /// The next token, without consuming it. A lex error is returned at once.
    fn peek(&mut self) -> Result<Option<Token<'a>>, EvalError> {
        match self.tokens.peek() {
            None => Ok(None),
            Some(Ok((_, token))) => Ok(Some(*token)),
            Some(Err(e)) => Err(EvalError::Lex(*e)),
        }
    }

    /// Consumes the next token, and returns it with its byte offset.
    fn bump(&mut self) -> Result<Option<(usize, Token<'a>)>, EvalError> {
        // `Option<Result<T, E>>` -> `Result<Option<T>, E>`, then `?`.
        Ok(self.tokens.next().transpose()?)
    }

    /// Runs `parse` one nesting level deeper, or fails with `TooDeep` once
    /// `MAX_DEPTH` levels are in use. Every recursive call goes through here.
    fn nested(&mut self, parse: fn(&mut Self) -> Result<i64, EvalError>) -> Result<i64, EvalError> {
        if self.depth == MAX_DEPTH {
            return Err(EvalError::TooDeep);
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }

    // expr := term (('+' | '-') expr)?
    fn expr(&mut self) -> Result<i64, EvalError> {
        // TODO: `8 - 3 - 2` evaluates to 7 instead of 3, so
        // `subtraction_and_division_are_left_associative` fails. This rule
        // recurses on its RIGHT operand, which reads the input as
        // `8 - (3 - 2)`, and it nests one level deeper per operator, so
        // `a_long_flat_chain_does_not_nest` fails with `TooDeep`. `+` and `-`
        // also panic on overflow. Requirements: `+` and `-` are
        // left-associative; a flat chain of any length is read without
        // recursion; a result outside `i64`, even an intermediate one, is
        // `Err(Overflow)`, never a panic or a wrapped value. Until you read
        // the operators left to right, the tests will fail.
        let lhs = self.term()?;
        match self.peek()? {
            Some(Token::Plus) => {
                self.bump()?;
                Ok(lhs + self.nested(Self::expr)?)
            }
            Some(Token::Minus) => {
                self.bump()?;
                Ok(lhs - self.nested(Self::expr)?)
            }
            _ => Ok(lhs),
        }
    }

    // term := factor (('*' | '/') term)?
    fn term(&mut self) -> Result<i64, EvalError> {
        // TODO: the same right recursion makes `8 / 4 / 2` evaluate to 4
        // instead of 1. `*` panics on overflow, and `/` panics twice:
        // "attempt to divide by zero" (`division_by_zero_is_not_an_overflow`)
        // and "attempt to divide with overflow" for `i64::MIN / -1`
        // (`overflow_is_an_error_not_a_panic`), both in every build profile.
        // Requirements: left-associative, like `+` and `-`; a zero divisor is
        // `DivByZero` and `i64::MIN / -1` is `Overflow` (the tests tell the
        // two apart); `/` still rounds toward zero. Until you read `*` and `/`
        // left to right with checked arithmetic, the tests will fail.
        let lhs = self.factor()?;
        match self.peek()? {
            Some(Token::Star) => {
                self.bump()?;
                Ok(lhs * self.nested(Self::term)?)
            }
            Some(Token::Slash) => {
                self.bump()?;
                Ok(lhs / self.nested(Self::term)?)
            }
            _ => Ok(lhs),
        }
    }

    // factor := NUMBER | '(' expr ')'
    fn factor(&mut self) -> Result<i64, EvalError> {
        // TODO: `-5`, `2 * -3` and `-(2 + 3)` fail with `UnexpectedToken`
        // (`unary_minus` and more): this grammar has no unary minus.
        // Requirements: a `-` in front of an operand negates that operand
        // alone, so it binds tighter than `*` and `/` (`-3 - 2` is -5); it may
        // repeat (`--5` is 5); `-i64::MIN` is `Overflow`. A run of 100_000
        // minus signs must not overflow the stack: recurse only through
        // `nested`, or apply them in a loop. Until you parse unary minus, the
        // tests will fail.
        match self.bump()? {
            Some((_, Token::Num(n))) => Ok(n),
            Some((_, Token::LParen)) => {
                let value = self.nested(Self::expr)?;
                match self.bump()? {
                    Some((_, Token::RParen)) => Ok(value),
                    Some((pos, _)) => Err(EvalError::UnexpectedToken { pos }),
                    None => Err(EvalError::UnexpectedEnd),
                }
            }
            Some((pos, _)) => Err(EvalError::UnexpectedToken { pos }),
            None => Err(EvalError::UnexpectedEnd),
        }
    }
}

/// Evaluates an integer expression: `i64` literals, `+ - * /`, unary `-` and
/// parentheses.
///
/// - Unary `-` binds tightest, then `*` and `/`, then `+` and `-`.
/// - All four binary operators are left-associative: `8 - 3 - 2` is 3.
/// - `/` rounds toward zero, like Rust's: `-7 / 2` is -3.
/// - Every operation is checked when it is applied. A zero divisor is
///   `DivByZero`; any other result outside the `i64` range, even one that a
///   later operation would bring back, is `Overflow`.
/// - A flat chain like `1 + 2 + 3 + ...` may be any length. Nesting deeper
///   than `MAX_DEPTH` levels is `TooDeep`: parentheses always count, and so
///   do unary minuses when they are parsed by recursion.
/// - It never panics, whatever the input.
fn eval(src: &str) -> Result<i64, EvalError> {
    let mut parser = Parser {
        tokens: Lexer::new(src).peekable(),
        depth: 0,
    };
    let value = parser.expr()?;
    match parser.bump()? {
        None => Ok(value),
        Some((pos, _)) => Err(EvalError::TrailingInput { pos }),
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    // `i64::MIN` has no literal (9223372036854775808 does not fit in an
    // `i64`), so the tests spell it as an expression.
    const MIN: &str = "(-9223372036854775807 - 1)";

    #[track_caller]
    fn ok(src: &str, expected: i64) {
        assert_eq!(eval(src), Ok(expected), "eval({src:?})");
    }

    #[track_caller]
    fn err(src: &str, expected: EvalError) {
        assert_eq!(eval(src), Err(expected), "eval({src:?})");
    }

    #[test]
    fn numbers_and_parentheses() {
        ok("42", 42);
        ok(" 0 ", 0);
        ok("(42)", 42);
        ok("((7))", 7);
        ok("9223372036854775807", i64::MAX);
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        ok("2 + 3 * 4", 14);
        ok("2 * 3 + 4", 10);
        ok("(2 + 3) * 4", 20);
        ok("10 - 2 * 3", 4);
        ok("2 * (3 + 4) * 5", 70);
    }

    #[test]
    fn subtraction_and_division_are_left_associative() {
        ok("8 - 3 - 2", 3);
        ok("8 / 4 / 2", 1);
        ok("1 - 2 + 3", 2);
        ok("100 / 10 * 2", 20);
        ok("2 * 3 / 4", 1);
        ok("10 - 4 - 3 - 2 - 1", 0);
        ok("7 - 2 + 1 - 3", 3);
        ok("1 + 2 * 3 - 4 / 2 - 1", 4);
        ok("8-3-2", 3);
    }

    #[test]
    fn unary_minus() {
        ok("-5", -5);
        ok("- 5", -5);
        ok("--5", 5);
        ok("---1", -1);
        ok("-3 - 2", -5);
        ok("2 * -3", -6);
        ok("2 - -3", 5);
        ok("-(2 + 3)", -5);
        ok("-2 * -3", 6);
        ok("-(-(4))", 4);
    }

    #[test]
    fn unary_minus_binds_tighter_than_multiplication() {
        // (-2^62) * 2 is exactly i64::MIN, but -(2^62 * 2) overflows on the
        // way.
        ok("-4611686018427387904 * 2", i64::MIN);
        ok("-4611686018427387904 * 2 / 2", -4611686018427387904);
    }

    #[test]
    fn division_rounds_toward_zero() {
        ok("7 / 2", 3);
        ok("-7 / 2", -3);
        ok("7 / -2", -3);
        ok("-7 / -2", 3);
        ok("1 / 3", 0);
    }

    #[test]
    fn results_up_to_the_edges_of_i64() {
        ok("-9223372036854775807 - 1", i64::MIN);
        ok("9223372036854775807 - 1 + 1", i64::MAX);
        ok("3037000499 * 3037000499", 9_223_372_030_926_249_001);
        ok(&format!("{MIN} / 1"), i64::MIN);
        ok(&format!("{MIN} / -2"), 4_611_686_018_427_387_904);
        ok(&format!("{MIN} + 9223372036854775807"), -1);
    }

    #[test]
    fn overflow_is_an_error_not_a_panic() {
        err("9223372036854775807 + 1", EvalError::Overflow);
        err("-9223372036854775807 - 2", EvalError::Overflow);
        err("4611686018427387904 * 2", EvalError::Overflow);
        err("3037000500 * 3037000500", EvalError::Overflow);
        err(&format!("{MIN} * -1"), EvalError::Overflow);
        err(&format!("-{MIN}"), EvalError::Overflow);
        // 2^63 does not fit: the one quotient of two i64s that overflows.
        err(&format!("{MIN} / -1"), EvalError::Overflow);
    }

    #[test]
    fn every_intermediate_result_must_fit() {
        // The final values would fit, but a step on the way does not.
        err("9223372036854775807 + 1 - 1", EvalError::Overflow);
        err("4611686018427387904 * 2 / 2", EvalError::Overflow);
        err(&format!("{MIN} - 1 + 1"), EvalError::Overflow);
        // `-i64::MIN` overflows, even though the second minus would bring
        // the value back.
        err(&format!("--{MIN}"), EvalError::Overflow);
    }

    #[test]
    fn division_by_zero_is_not_an_overflow() {
        err("1 / 0", EvalError::DivByZero);
        err("0 / 0", EvalError::DivByZero);
        err("1 / (2 - 2)", EvalError::DivByZero);
        err(&format!("{MIN} / 0"), EvalError::DivByZero);
    }

    #[test]
    fn syntax_errors_say_where() {
        err("", EvalError::UnexpectedEnd);
        err("   ", EvalError::UnexpectedEnd);
        err("1 +", EvalError::UnexpectedEnd);
        err("(1 + 2", EvalError::UnexpectedEnd);
        err("-", EvalError::UnexpectedEnd);
        err("* 2", EvalError::UnexpectedToken { pos: 0 });
        err("+1", EvalError::UnexpectedToken { pos: 0 });
        err("1 + * 2", EvalError::UnexpectedToken { pos: 4 });
        err("()", EvalError::UnexpectedToken { pos: 1 });
        err("(1 2)", EvalError::UnexpectedToken { pos: 3 });
        err("x + 1", EvalError::UnexpectedToken { pos: 0 });
        err("1 2", EvalError::TrailingInput { pos: 2 });
        err("(1 + 2))", EvalError::TrailingInput { pos: 7 });
        err("8 - 3 )", EvalError::TrailingInput { pos: 6 });
        err("(é) + 1", EvalError::UnexpectedToken { pos: 1 });
        err("2 * 3 é", EvalError::TrailingInput { pos: 6 });
        // Positions are byte offsets: U+3000 IDEOGRAPHIC SPACE is 3 bytes.
        err("1 +\u{3000}* 2", EvalError::UnexpectedToken { pos: 6 });
        err("2\u{3000}3", EvalError::TrailingInput { pos: 4 });
    }

    #[test]
    fn lex_errors_pass_through() {
        let unexpected = |ch, pos| EvalError::Lex(LexError::UnexpectedChar { ch, pos });
        err("1 + $", unexpected('$', 4));
        err("€ * 2", unexpected('€', 0));
        err(
            "2 * 99999999999999999999",
            EvalError::Lex(LexError::NumberTooLarge { pos: 4 }),
        );
        // No literal for i64::MIN: the lexer rejects 2^63 before the minus
        // sign can apply.
        err(
            "-9223372036854775808",
            EvalError::Lex(LexError::NumberTooLarge { pos: 1 }),
        );
    }

    #[test]
    fn a_long_flat_chain_does_not_nest() {
        // Flat chains need no nesting at all, so their length is not limited
        // by `MAX_DEPTH`.
        let terms = 100_000;
        let sum = vec!["1"; terms].join(" + ");
        ok(&sum, 100_000);
        let difference = vec!["1"; terms].join("-");
        ok(&difference, -99_998);
        let product = format!("2{}", " * 1".repeat(terms));
        ok(&product, 2);
        let quotient = format!("1024{}{}", " / 2".repeat(10), " / 1".repeat(terms));
        ok(&quotient, 1);
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let depth = 100;
        ok(&format!("{}1{}", "(".repeat(depth), ")".repeat(depth)), 1);
        let depth = 100_000;
        let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        err(&src, EvalError::TooDeep);
        err(&"(".repeat(depth), EvalError::TooDeep);
        // A loop that applies the minus signs one by one may accept this, so
        // either answer is fine: what matters is that it returns.
        let minuses = format!("{}1", "-".repeat(depth));
        let result = eval(&minuses);
        assert!(
            result == Ok(1) || result == Err(EvalError::TooDeep),
            "100_000 minus signs, then 1: got {result:?}"
        );
    }

    // A tiny deterministic pseudo-random generator (a 64-bit LCG), so every
    // run checks exactly the same inputs.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    fn pick<'s>(state: &mut u64, from: &[&'s str]) -> &'s str {
        from[next(state) as usize % from.len()]
    }

    #[test]
    fn nothing_panics_on_random_input() {
        let pieces = [
            "1",
            "0",
            "7",
            "9223372036854775807",
            "4611686018427387904",
            "99999999999999999999",
            "+",
            "-",
            "*",
            "/",
            "(",
            ")",
            " ",
            "x",
            "$",
        ];
        let mut state = 7;
        for _ in 0..5_000 {
            let len = next(&mut state) % 12;
            let src: String = (0..len).map(|_| pick(&mut state, &pieces)).collect();
            let result = panic::catch_unwind(|| eval(&src));
            assert!(result.is_ok(), "eval({src:?}) panicked");
        }
    }

    // A random expression tree, its text, and its value.
    enum Tree {
        Num(i64),
        Neg(Box<Tree>),
        Bin(char, Box<Tree>, Box<Tree>),
    }

    fn random_tree(state: &mut u64, depth: u32) -> Tree {
        let roll = next(state) % 10;
        if depth == 0 || roll < 3 {
            let numbers = [
                0,
                1,
                2,
                3,
                5,
                7,
                10,
                12,
                100,
                3_037_000_500,
                4_611_686_018_427_387_904,
                i64::MAX,
            ];
            Tree::Num(numbers[next(state) as usize % numbers.len()])
        } else if roll < 4 {
            Tree::Neg(Box::new(random_tree(state, depth - 1)))
        } else {
            let op = ['+', '-', '*', '/'][next(state) as usize % 4];
            let lhs = random_tree(state, depth - 1);
            let rhs = random_tree(state, depth - 1);
            Tree::Bin(op, Box::new(lhs), Box::new(rhs))
        }
    }

    fn precedence(tree: &Tree) -> u8 {
        match tree {
            Tree::Bin('+' | '-', ..) => 1,
            Tree::Bin(..) => 2,
            Tree::Neg(_) => 3,
            Tree::Num(_) => 4,
        }
    }

    // Prints `tree` with as few parentheses as the rules allow (plus some
    // redundant ones), so the parser has to get precedence and
    // associativity right to rebuild it.
    fn render(tree: &Tree, state: &mut u64) -> String {
        let wrap = |text: String, needed: bool, state: &mut u64| {
            if needed || next(state).is_multiple_of(8) {
                format!("({text})")
            } else {
                text
            }
        };
        match tree {
            Tree::Num(n) => n.to_string(),
            Tree::Neg(operand) => {
                let inner = render(operand, state);
                format!("-{}", wrap(inner, precedence(operand) < 3, state))
            }
            Tree::Bin(op, lhs, rhs) => {
                let level = precedence(tree);
                let left = render(lhs, state);
                let left = wrap(left, precedence(lhs) < level, state);
                // Same level on the right needs parentheses: the operators
                // are left-associative.
                let right = render(rhs, state);
                let right = wrap(right, precedence(rhs) <= level, state);
                let space = pick(state, &["", " "]);
                format!("{left}{space}{op}{space}{right}")
            }
        }
    }

    // The value of `tree`, every operation checked, operands left to right.
    fn value(tree: &Tree) -> Result<i64, EvalError> {
        match tree {
            Tree::Num(n) => Ok(*n),
            Tree::Neg(operand) => value(operand)?.checked_neg().ok_or(EvalError::Overflow),
            Tree::Bin(op, lhs, rhs) => {
                let (lhs, rhs) = (value(lhs)?, value(rhs)?);
                let result = match op {
                    '+' => lhs.checked_add(rhs),
                    '-' => lhs.checked_sub(rhs),
                    '*' => lhs.checked_mul(rhs),
                    _ if rhs == 0 => return Err(EvalError::DivByZero),
                    _ => lhs.checked_div(rhs),
                };
                result.ok_or(EvalError::Overflow)
            }
        }
    }

    #[test]
    fn matches_a_tree_model_on_random_expressions() {
        let mut state = 2024;
        for _ in 0..3_000 {
            let tree = random_tree(&mut state, 5);
            let src = render(&tree, &mut state);
            let got = eval(&src);
            match value(&tree) {
                Ok(expected) => assert_eq!(got, Ok(expected), "eval({src:?})"),
                // When several steps fail, which error comes first depends on
                // the order of evaluation, so any arithmetic error will do.
                Err(_) => assert!(
                    matches!(got, Err(EvalError::Overflow | EvalError::DivByZero)),
                    "eval({src:?}): expected an arithmetic error, got {got:?}"
                ),
            }
        }
    }
}
