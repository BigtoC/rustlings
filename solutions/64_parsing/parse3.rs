// Module 5 · Parsing — part 3: a boxed AST with `FromStr` and a fully parenthesized `Display` that round-trips (E0308).
//
// Part 2 computed the value while it parsed. Real tools, and the follow-ups
// of the take-home ("now add variables", "print it back", "simplify it"),
// first build a tree, the abstract syntax tree (AST), and then walk it as
// often as they need to. The AST keeps the structure and drops the syntax:
// `(((7)))` and `7` are the same `Num(7)`, because parentheses only decide
// the shape of a tree and leave no node behind.
//
// `Expr` is recursive, so its children sit behind a `Box`: without one the
// type would have infinite size (E0072, `26_smart_pointers_deep/smartptr1`).
// The parser has the same loop-and-level structure as part 2's. Where part 2
// folded each operand into a running value, this one folds it into a running
// TREE, and the tree built so far becomes the LEFT child of the new node. So
// `8 - 3 - 2` becomes `Binary(Sub, Binary(Sub, 8, 3), 2)`, a left-deep tree,
// which is what left associativity looks like as data. Moving `acc` into
// `Box::new(acc)` and assigning the new node back to `acc` in the same
// statement is fine: the old value is moved out before the new one moves in.
//
// `FromStr` is what makes `"8 - 3 - 2".parse::<Expr>()` work
// (`23_conversions/conversions3`). Look at its signature,
// `fn from_str(s: &str) -> Result<Self, Self::Err>`: no lifetime links `Self`
// to `s`, so a `FromStr` type can never borrow from its input.
// `impl<'a> FromStr for Expr<'a>` returning a `Var(&'a str)` fails with
// "lifetime may not live long enough", and changing the parameter to
// `s: &'a str` fails with E0308 "method not compatible with trait". So this AST
// owns its names, `Var(String)`: part 1's borrowed `Ident(&str)` is copied
// exactly once, when it becomes a node. A zero-copy AST (`Expr<'a>` with
// `Var(&'a str)`) needs an inherent
// `fn parse(src: &'a str) -> Result<Expr<'a>, ParseError>` instead, and serde's
// `Deserialize<'de>` lifetime exists for exactly that reason.
//
// Two phases, two error types. Parsing fails on syntax and evaluation fails on
// arithmetic, so `"1 / 0".parse::<Expr>()` succeeds and `eval` reports
// `DivByZero`. `ParseError` and `EvalError` keep the two apart, so neither enum
// has a variant that its caller can never see (`35_error_design`).
//
// `Display` prints the tree fully parenthesized, so its shape is visible:
// `((8 - 3) - 2)`. Printing needs no precedence rules then, and parsing the
// printed text gives back the same tree. A round trip like that is the cheapest
// strong test a parser can get, and the tests below check it on random trees.
//
// One limit remains. `nested` bounds parentheses and minus signs, but a flat
// `1 - 1 - ... - 1` is a left-deep tree with one level per operator, and
// `eval`, `Display`, `Debug` and the drop glue all recurse down it: at 100_000
// levels, dropping the tree overflows a 2 MiB test thread's stack. The module
// README lists the fixes; the tests stay far below that depth.
//
// How interviewers probe this: "Why `Box`? Why can't `FromStr` return an AST
// that borrows the input? How would you print with as few parentheses as
// possible? How do you test a parser? What happens when you drop a very deep
// tree?"

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::iter::Peekable;
use std::num::IntErrorKind;
use std::str::{CharIndices, FromStr};

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
// The AST.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Expr {
    /// A literal. The parser only produces non-negative ones: `-5` is
    /// `Neg(Num(5))`.
    Num(i64),
    /// A variable, looked up when the tree is evaluated.
    Var(String),
    /// Unary minus.
    Neg(Box<Expr>),
    /// `lhs op rhs`.
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParseError {
    /// The lexer rejected the input.
    Lex(LexError),
    /// The input ended where an operand or a `)` was expected.
    UnexpectedEnd,
    /// A token that cannot appear here, at byte offset `pos`.
    UnexpectedToken { pos: usize },
    /// A complete expression followed by more tokens, the first of them at
    /// byte offset `pos`.
    TrailingInput { pos: usize },
    /// Parentheses and unary minuses nested more than `MAX_DEPTH` levels deep.
    TooDeep,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Lex(e) => write!(f, "{e}"),
            ParseError::UnexpectedEnd => f.write_str("unexpected end of input"),
            ParseError::UnexpectedToken { pos } => write!(f, "unexpected token at byte {pos}"),
            ParseError::TrailingInput { pos } => write!(f, "unexpected input at byte {pos}"),
            ParseError::TooDeep => write!(f, "nested more than {MAX_DEPTH} levels deep"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::Lex(e) => Some(e),
            _ => None,
        }
    }
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError::Lex(e)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EvalError {
    /// A variable that `vars` does not define.
    UnknownVariable(String),
    /// A zero divisor.
    DivByZero,
    /// A result outside the `i64` range.
    Overflow,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::UnknownVariable(name) => write!(f, "unknown variable {name:?}"),
            EvalError::DivByZero => f.write_str("division by zero"),
            EvalError::Overflow => f.write_str("arithmetic overflow"),
        }
    }
}

impl Error for EvalError {}

impl Expr {
    /// The value of the tree, with part 2's checked arithmetic, operands
    /// left to right. (Given.)
    fn eval(&self, vars: &HashMap<&str, i64>) -> Result<i64, EvalError> {
        match self {
            Expr::Num(n) => Ok(*n),
            Expr::Var(name) => vars
                .get(name.as_str())
                .copied()
                .ok_or_else(|| EvalError::UnknownVariable(name.clone())),
            Expr::Neg(operand) => operand.eval(vars)?.checked_neg().ok_or(EvalError::Overflow),
            Expr::Binary(op, lhs, rhs) => {
                let (lhs, rhs) = (lhs.eval(vars)?, rhs.eval(vars)?);
                let value = match op {
                    BinOp::Add => lhs.checked_add(rhs),
                    BinOp::Sub => lhs.checked_sub(rhs),
                    BinOp::Mul => lhs.checked_mul(rhs),
                    BinOp::Div if rhs == 0 => return Err(EvalError::DivByZero),
                    BinOp::Div => lhs.checked_div(rhs),
                };
                value.ok_or(EvalError::Overflow)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The parser.

/// How many levels parentheses and unary minuses may nest.
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    tokens: Peekable<Lexer<'a>>,
    depth: usize,
}

impl<'a> Parser<'a> {
    /// The next token, without consuming it. A lex error is returned at once.
    fn peek(&mut self) -> Result<Option<Token<'a>>, ParseError> {
        match self.tokens.peek() {
            None => Ok(None),
            Some(Ok((_, token))) => Ok(Some(*token)),
            Some(Err(e)) => Err(ParseError::Lex(*e)),
        }
    }

    /// Consumes the next token, and returns it with its byte offset.
    fn bump(&mut self) -> Result<Option<(usize, Token<'a>)>, ParseError> {
        Ok(self.tokens.next().transpose()?)
    }

    /// Runs `parse` one nesting level deeper, or fails with `TooDeep` once
    /// `MAX_DEPTH` levels are in use. Every recursive call goes through here.
    fn nested(
        &mut self,
        parse: fn(&mut Self) -> Result<Expr, ParseError>,
    ) -> Result<Expr, ParseError> {
        if self.depth == MAX_DEPTH {
            return Err(ParseError::TooDeep);
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }

    /// Consumes the `)` that closes a parenthesized expression.
    fn close_paren(&mut self) -> Result<(), ParseError> {
        match self.bump()? {
            Some((_, Token::RParen)) => Ok(()),
            Some((pos, _)) => Err(ParseError::UnexpectedToken { pos }),
            None => Err(ParseError::UnexpectedEnd),
        }
    }

    // Part 2's grammar, building nodes instead of values. Each loop folds
    // the tree built so far into the LEFT child of the next node, so a chain
    // of equal-precedence operators becomes a left-deep tree.

    // expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Result<Expr, ParseError> {
        let mut acc = self.term()?;
        loop {
            let op = match self.peek()? {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => return Ok(acc),
            };
            self.bump()?;
            let rhs = self.term()?;
            acc = Expr::Binary(op, Box::new(acc), Box::new(rhs));
        }
    }

    // term := factor (('*' | '/') factor)*
    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut acc = self.factor()?;
        loop {
            let op = match self.peek()? {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                _ => return Ok(acc),
            };
            self.bump()?;
            let rhs = self.factor()?;
            acc = Expr::Binary(op, Box::new(acc), Box::new(rhs));
        }
    }

    // factor := '-' factor | NUMBER | IDENT | '(' expr ')'
    fn factor(&mut self) -> Result<Expr, ParseError> {
        match self.bump()? {
            Some((_, Token::Num(n))) => Ok(Expr::Num(n)),
            // The only copy of the source text: a `FromStr` result cannot
            // borrow from `s`, so the name becomes an owned `String` here.
            Some((_, Token::Ident(name))) => Ok(Expr::Var(name.to_owned())),
            // Through `nested`, so a run of minus signs cannot build a tree
            // (or a call stack) deeper than `MAX_DEPTH`.
            Some((_, Token::Minus)) => Ok(Expr::Neg(Box::new(self.nested(Self::factor)?))),
            Some((_, Token::LParen)) => {
                let inner = self.nested(Self::expr)?;
                self.close_paren()?;
                // No node for the parentheses: they only shaped the tree.
                Ok(inner)
            }
            Some((pos, _)) => Err(ParseError::UnexpectedToken { pos }),
            None => Err(ParseError::UnexpectedEnd),
        }
    }
}

impl FromStr for Expr {
    type Err = ParseError;

    /// Parses part 2's language plus variables: `i64` literals, identifiers,
    /// `+ - * /`, unary `-` and parentheses, with part 2's precedence and left
    /// associativity. It reports part 2's syntax errors, the same way: `Lex`,
    /// `UnexpectedEnd`, `UnexpectedToken { pos }`, `TrailingInput { pos }`, and
    /// `TooDeep` past `MAX_DEPTH` levels of parentheses and unary minuses. It
    /// evaluates nothing and never panics.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The same driver as part 2's `eval`: one expression, then the input
        // must be over.
        let mut parser = Parser {
            tokens: Lexer::new(s).peekable(),
            depth: 0,
        };
        let expr = parser.expr()?;
        match parser.bump()? {
            None => Ok(expr),
            Some((pos, _)) => Err(ParseError::TrailingInput { pos }),
        }
    }
}

impl fmt::Display for Expr {
    /// Fully parenthesized: every operator, unary or binary, is printed with
    /// its operands inside one pair of parentheses, and nothing else is.
    /// Binary operators have one space on each side, unary minus has none:
    /// `8 - 3 - 2` prints as `((8 - 3) - 2)`, `-x * 2` as `((-x) * 2)`, and
    /// `(((7)))` as `7`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `{lhs}` and `{rhs}` call this same method on the children, so the
        // recursion follows the tree.
        match self {
            Expr::Num(n) => write!(f, "{n}"),
            Expr::Var(name) => f.write_str(name),
            Expr::Neg(operand) => write!(f, "(-{operand})"),
            Expr::Binary(op, lhs, rhs) => write!(f, "({lhs} {op} {rhs})"),
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use BinOp::*;

    #[track_caller]
    fn parse(src: &str) -> Expr {
        src.parse()
            .unwrap_or_else(|e| panic!("{src:?} should parse, got {e:?}"))
    }

    fn num(n: i64) -> Expr {
        Expr::Num(n)
    }

    fn var(name: &str) -> Expr {
        Expr::Var(name.to_string())
    }

    fn neg(operand: Expr) -> Expr {
        Expr::Neg(Box::new(operand))
    }

    fn bin(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        Expr::Binary(op, Box::new(lhs), Box::new(rhs))
    }

    #[track_caller]
    fn parse_err(src: &str, expected: ParseError) {
        assert_eq!(src.parse::<Expr>(), Err(expected), "{src:?}");
    }

    #[test]
    fn numbers_and_variables_are_leaves() {
        assert_eq!(parse("42"), num(42));
        assert_eq!(parse("x"), var("x"));
        assert_eq!(parse("héllo"), var("héllo"));
        assert_eq!(parse("9223372036854775807"), num(i64::MAX));
        // Parentheses shape the tree but leave no node behind.
        assert_eq!(parse("(((7)))"), num(7));
    }

    #[test]
    fn equal_precedence_builds_a_left_deep_tree() {
        assert_eq!(
            parse("8 - 3 - 2"),
            bin(Sub, bin(Sub, num(8), num(3)), num(2))
        );
        assert_eq!(
            parse("8 / 4 / 2"),
            bin(Div, bin(Div, num(8), num(4)), num(2))
        );
        assert_eq!(
            parse("a - b + c - d"),
            bin(
                Sub,
                bin(Add, bin(Sub, var("a"), var("b")), var("c")),
                var("d")
            )
        );
        assert_eq!(
            parse("2 * 3 / 4"),
            bin(Div, bin(Mul, num(2), num(3)), num(4))
        );
    }

    #[test]
    fn precedence_and_parentheses_shape_the_tree() {
        assert_eq!(
            parse("2 + 3 * 4"),
            bin(Add, num(2), bin(Mul, num(3), num(4)))
        );
        assert_eq!(
            parse("2 * 3 + 4"),
            bin(Add, bin(Mul, num(2), num(3)), num(4))
        );
        assert_eq!(
            parse("(2 + 3) * 4"),
            bin(Mul, bin(Add, num(2), num(3)), num(4))
        );
        assert_eq!(
            parse("8 - (3 - 2)"),
            bin(Sub, num(8), bin(Sub, num(3), num(2)))
        );
    }

    #[test]
    fn unary_minus_is_a_node() {
        assert_eq!(parse("-x * 2"), bin(Mul, neg(var("x")), num(2)));
        assert_eq!(parse("2 * -3"), bin(Mul, num(2), neg(num(3))));
        assert_eq!(parse("--1"), neg(neg(num(1))));
        assert_eq!(parse("-(a + b)"), neg(bin(Add, var("a"), var("b"))));
        assert_eq!(parse("a - -b"), bin(Sub, var("a"), neg(var("b"))));
    }

    #[test]
    fn parsing_evaluates_nothing() {
        let no_vars = HashMap::new();
        let div = parse("1 / 0");
        assert_eq!(div, bin(Div, num(1), num(0)));
        assert_eq!(div.eval(&no_vars), Err(EvalError::DivByZero));

        let min_over_minus_one = parse("(-9223372036854775807 - 1) / -1");
        assert_eq!(min_over_minus_one.eval(&no_vars), Err(EvalError::Overflow));

        let unknown = parse("y + 1");
        assert_eq!(
            unknown.eval(&no_vars),
            Err(EvalError::UnknownVariable("y".to_string()))
        );
    }

    #[test]
    fn the_tree_evaluates_with_variables() {
        let vars = HashMap::from([("rate", 40), ("hours", 38), ("tax", 300), ("é", 2)]);
        assert_eq!(parse("rate * hours - tax").eval(&vars), Ok(1220));
        assert_eq!(parse("rate * (hours - tax)").eval(&vars), Ok(-10480));
        assert_eq!(parse("tax - hours - é").eval(&vars), Ok(260));
        assert_eq!(parse("-é * -é / é").eval(&vars), Ok(2));
    }

    #[test]
    fn syntax_errors_are_parse_errors() {
        parse_err("", ParseError::UnexpectedEnd);
        parse_err("1 +", ParseError::UnexpectedEnd);
        parse_err("(1", ParseError::UnexpectedEnd);
        parse_err("-", ParseError::UnexpectedEnd);
        parse_err(")", ParseError::UnexpectedToken { pos: 0 });
        parse_err("()", ParseError::UnexpectedToken { pos: 1 });
        parse_err("a * / b", ParseError::UnexpectedToken { pos: 4 });
        parse_err("(a b)", ParseError::UnexpectedToken { pos: 3 });
        parse_err("1 2", ParseError::TrailingInput { pos: 2 });
        parse_err("(x))", ParseError::TrailingInput { pos: 3 });
        // "héllo" is 6 bytes, so `2` sits at byte 7.
        parse_err("héllo 2", ParseError::TrailingInput { pos: 7 });
        parse_err(
            "1 $",
            ParseError::Lex(LexError::UnexpectedChar { ch: '$', pos: 2 }),
        );
        parse_err(
            "x - 99999999999999999999",
            ParseError::Lex(LexError::NumberTooLarge { pos: 4 }),
        );
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let depth = 100;
        let src = format!("{}x{}", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(parse(&src), var("x"));
        let depth = 100_000;
        let src = format!("{}x{}", "(".repeat(depth), ")".repeat(depth));
        parse_err(&src, ParseError::TooDeep);
        // Every minus sign is a node: 100_000 of them would be a tree too
        // deep to drop, print or evaluate without overflowing the stack.
        parse_err(&format!("{}x", "-".repeat(depth)), ParseError::TooDeep);
    }

    #[test]
    fn display_is_fully_parenthesized() {
        let cases = [
            ("42", "42"),
            ("x", "x"),
            ("(((7)))", "7"),
            ("8 - 3 - 2", "((8 - 3) - 2)"),
            ("8 - (3 - 2)", "(8 - (3 - 2))"),
            ("2+3*4", "(2 + (3 * 4))"),
            ("-x * 2", "((-x) * 2)"),
            ("--1", "(-(-1))"),
            ("-(a + b)", "(-(a + b))"),
            ("héllo/2", "(héllo / 2)"),
        ];
        for (src, printed) in cases {
            assert_eq!(parse(src).to_string(), printed, "{src:?}");
        }
    }

    #[test]
    fn display_round_trips() {
        let inputs = [
            "8 - 3 - 2",
            "8 - (3 - 2)",
            "2 + 3 * 4",
            "(2 + 3) * 4",
            "-x * 2",
            "2 * -3",
            "a / b / c",
            "rate * (hours - 1) / -2",
            "héllo - wörld * 2",
            "-(a + b) * -(c - d)",
        ];
        for src in inputs {
            let tree = parse(src);
            let printed = tree.to_string();
            let reparsed = parse(&printed);
            assert_eq!(reparsed, tree, "{src:?} printed as {printed:?}");
            // And printing is stable: the canonical form prints as itself.
            assert_eq!(reparsed.to_string(), printed, "{src:?}");
        }
    }

    // A tiny deterministic pseudo-random generator (a 64-bit LCG), so every
    // run checks exactly the same trees.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    fn random_tree(state: &mut u64, depth: u32) -> Expr {
        let roll = next(state) % 10;
        if depth == 0 || roll < 3 {
            if next(state).is_multiple_of(3) {
                let names = ["x", "y1", "_t", "é"];
                var(names[next(state) as usize % names.len()])
            } else {
                num([0, 1, 7, 42, i64::MAX][next(state) as usize % 5])
            }
        } else if roll < 4 {
            neg(random_tree(state, depth - 1))
        } else {
            let op = [Add, Sub, Mul, Div][next(state) as usize % 4];
            let lhs = random_tree(state, depth - 1);
            let rhs = random_tree(state, depth - 1);
            bin(op, lhs, rhs)
        }
    }

    fn precedence(tree: &Expr) -> u8 {
        match tree {
            Expr::Binary(Add | Sub, ..) => 1,
            Expr::Binary(..) => 2,
            Expr::Neg(_) => 3,
            Expr::Num(_) | Expr::Var(_) => 4,
        }
    }

    // Prints `tree` with as few parentheses as precedence and left
    // associativity allow, and no spaces: text the parser must rebuild
    // exactly.
    fn minimal(tree: &Expr) -> String {
        let wrap = |text: String, needed: bool| {
            if needed { format!("({text})") } else { text }
        };
        match tree {
            Expr::Num(n) => n.to_string(),
            Expr::Var(name) => name.clone(),
            Expr::Neg(operand) => format!("-{}", wrap(minimal(operand), precedence(operand) < 3)),
            Expr::Binary(op, lhs, rhs) => {
                let level = precedence(tree);
                let left = wrap(minimal(lhs), precedence(lhs) < level);
                let right = wrap(minimal(rhs), precedence(rhs) <= level);
                format!("{left}{op}{right}")
            }
        }
    }

    #[test]
    fn random_trees_survive_both_printers() {
        let mut state = 3;
        for _ in 0..2_000 {
            let tree = random_tree(&mut state, 5);
            let terse = minimal(&tree);
            assert_eq!(parse(&terse), tree, "parsing {terse:?}");
            let printed = tree.to_string();
            assert_eq!(parse(&printed), tree, "parsing {printed:?}");
        }
    }
}
