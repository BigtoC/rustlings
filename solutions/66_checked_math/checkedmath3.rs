// Module 5 · Checked math — part 3: a fixed-point `Decimal` with 18 decimals.
//
// Why not `f64` for money? Because 0.1 has no finite binary expansion. It is
// stored as the nearest double, and the error shows up in the first sum:
// `0.1 + 0.2` prints as `0.30000000000000004` and `1.1 * 3.0` as
// `3.3000000000000003`. Money needs exact DECIMAL fractions and an explicit
// rounding rule, and fixed point gives you both: store an integer count of
// the smallest unit, with a fixed scale. `Decimal(n)` below means `n / 10^18`,
// the 18 decimals of Ether's wei and of CosmWasm's `Decimal`, in a `u128`.
//
// Addition and subtraction are then plain integer operations (`checked_add`
// is given). Multiplication is not: `(a / 10^18) * (b / 10^18)` is
// `(a * b / 10^18) / 10^18`, so the raw product must be divided by the scale
// once. That raw product carries 36 decimal places and leaves the `u128`
// range for everyday values. That is part 1's problem, and part 1's
// `mul_div_floor` (given below) is its answer. Division is the mirror image:
// `a * 10^18 / b`.
//
// Parsing and printing are where most hand-written decimal types go wrong.
// Never go through `f64`: `"0.1".parse::<f64>()` is already inexact. Validate
// the text yourself, because the integer parsers are more lenient than a
// money format should be: `"+1".parse::<u128>()` is `Ok(1)`. Scale the
// fraction by its length (`"0.5"` is 5 followed by 17 zeros, not 5), and
// report an overflow as `Overflow`, not as bad syntax. When printing, keep the
// fraction's leading zeros (0.05 is not 0.5) and drop its trailing ones, so
// that every value has exactly one canonical string: `x.to_string()` parses
// back to `x`, and a canonical `s` prints back as `s`.
//
// `checked_mul` and `checked_div` round down (toward zero). Part 2 showed
// when a caller needs the other direction; a production type offers both.
//
// How interviewers probe this: "Why not floats for money? How do you
// multiply two fixed-point numbers? What are the largest value and the
// smallest step? How do you parse "0.1" exactly? What is `1 / 3 * 3`?"

use std::error::Error;
use std::fmt;
use std::iter;
use std::str::FromStr;

/// The number of digits after the decimal point.
const DECIMALS: usize = 18;
/// The raw value of 1: 10^18.
const SCALE: u128 = 1_000_000_000_000_000_000;

/// Divides the 256-bit number `hi * 2^128 + lo` by `d`. Returns
/// `(quotient, remainder)`, or `None` when `d == 0` or the quotient does not
/// fit in a `u128`. (From part 1, given.)
fn div_wide(hi: u128, lo: u128, d: u128) -> Option<(u128, u128)> {
    if d == 0 || hi >= d {
        return None;
    }
    if hi == 0 {
        return Some((lo / d, lo % d));
    }
    let (mut quot, mut rem) = (0u128, hi);
    for i in (0..128).rev() {
        let carry = rem >> 127;
        rem = (rem << 1) | ((lo >> i) & 1);
        quot <<= 1;
        if carry == 1 || rem >= d {
            rem = rem.wrapping_sub(d);
            quot |= 1;
        }
    }
    Some((quot, rem))
}

/// `a * b / d` through the exact 256-bit product, rounded down. `None` when
/// `d == 0` or the result does not fit in a `u128`. (Part 1's solution,
/// given.)
fn mul_div_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    let (lo, hi) = a.carrying_mul(b, 0);
    div_wide(hi, lo, d).map(|(quot, _)| quot)
}

/// A non-negative fixed-point number with 18 decimal places: `Decimal(n)` is
/// `n / 10^18`. The smallest step is `Decimal(1)`, 0.000000000000000001, and
/// the largest value is 340282366920938463463.374607431768211455.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Decimal(u128);

#[derive(Debug, PartialEq, Eq)]
enum DecimalError {
    /// The input string is empty.
    Empty,
    /// The input is not `digits` or `digits.digits`.
    Invalid,
    /// More than 18 digits after the decimal point.
    TooManyDecimals,
    /// The value is above `Decimal::MAX`.
    Overflow,
    /// The divisor is zero.
    DivisionByZero,
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DecimalError::Empty => "empty decimal",
            DecimalError::Invalid => "invalid decimal",
            DecimalError::TooManyDecimals => "more than 18 decimal places",
            DecimalError::Overflow => "decimal overflow",
            DecimalError::DivisionByZero => "division by zero",
        })
    }
}

impl Error for DecimalError {}

impl Decimal {
    const ZERO: Decimal = Decimal(0);
    const ONE: Decimal = Decimal(SCALE);
    const MAX: Decimal = Decimal(u128::MAX);

    /// `self + rhs`, exactly: two values with the same scale add as integers.
    fn checked_add(self, rhs: Decimal) -> Result<Decimal, DecimalError> {
        self.0
            .checked_add(rhs.0)
            .map(Decimal)
            .ok_or(DecimalError::Overflow)
    }

    /// `self * rhs`, rounded down to 18 decimals. `Overflow` when the result
    /// is above `Decimal::MAX`.
    fn checked_mul(self, rhs: Decimal) -> Result<Decimal, DecimalError> {
        // (a / S) * (b / S) = (a * b / S) / S, so the raw result is
        // `a * b / S`. `mul_div_floor` keeps the full 256-bit product, so
        // only a result above `u128::MAX` is an overflow.
        mul_div_floor(self.0, rhs.0, SCALE)
            .map(Decimal)
            .ok_or(DecimalError::Overflow)
    }

    /// `self / rhs`, rounded down to 18 decimals. `DivisionByZero` when `rhs`
    /// is zero, `Overflow` when the result is above `Decimal::MAX`.
    fn checked_div(self, rhs: Decimal) -> Result<Decimal, DecimalError> {
        // (a / S) / (b / S) = (a * S / b) / S, so the raw result is
        // `a * S / b`. `mul_div_floor` would report a zero divisor as `None`
        // too, so check for it first to return the more precise error.
        if rhs == Decimal::ZERO {
            return Err(DecimalError::DivisionByZero);
        }
        mul_div_floor(self.0, SCALE, rhs.0)
            .map(Decimal)
            .ok_or(DecimalError::Overflow)
    }
}

impl FromStr for Decimal {
    type Err = DecimalError;

    /// Parses `digits` or `digits.digits`: ASCII digits only, and at least
    /// one digit on each side of the point. Leading zeros are fine ("007.50"
    /// is 7.5).
    ///
    /// - `""` is `Empty`.
    /// - Anything else that does not match, such as a sign, an exponent,
    ///   whitespace, `_`, `,`, `"1."` or `".5"`, is `Invalid`.
    /// - More than 18 digits after the point (even zeros) is
    ///   `TooManyDecimals`.
    /// - A value above `Decimal::MAX` is `Overflow`, however many digits it
    ///   has.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(DecimalError::Empty);
        }
        let (int, frac) = match s.split_once('.') {
            Some((int, frac)) => (int, Some(frac)),
            None => (s, None),
        };
        // Validate the text before any arithmetic: every byte an ASCII digit,
        // and neither side of the point empty. This is what rejects signs,
        // exponents, whitespace and non-ASCII digits.
        let is_number = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if !is_number(int) || frac.is_some_and(|frac| !is_number(frac)) {
            return Err(DecimalError::Invalid);
        }
        let frac = frac.unwrap_or("");
        if frac.len() > DECIMALS {
            return Err(DecimalError::TooManyDecimals);
        }
        // Base-10 digits into an integer, with every step checked. Leading
        // zeros cost nothing, so only a value that is really too big fails.
        fn value(mut digits: impl Iterator<Item = u8>) -> Option<u128> {
            digits.try_fold(0u128, |acc, b| {
                acc.checked_mul(10)?.checked_add(u128::from(b - b'0'))
            })
        }
        let int = value(int.bytes())
            .and_then(|int| int.checked_mul(SCALE))
            .ok_or(DecimalError::Overflow)?;
        // Pad the fraction with zeros to exactly 18 digits: "5" means
        // 500_000_000_000_000_000 units. Eighteen digits always fit in a
        // `u128`.
        let padded = frac.bytes().chain(iter::repeat(b'0')).take(DECIMALS);
        let frac = value(padded).expect("18 digits always fit in a u128");
        int.checked_add(frac)
            .map(Decimal)
            .ok_or(DecimalError::Overflow)
    }
}

impl fmt::Display for Decimal {
    /// The integer part, then, only if the fraction is not zero, a `.` and
    /// the fraction's digits without trailing zeros: "0", "10", "7.5",
    /// "0.05", "0.000000000000000001".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (int, frac) = (self.0 / SCALE, self.0 % SCALE);
        if frac == 0 {
            return write!(f, "{int}");
        }
        // Drop the trailing zeros arithmetically, and let the zero-padded
        // width put the leading zeros back: 0.05 is the fraction 5 printed
        // two digits wide.
        let (mut frac, mut width) = (frac, DECIMALS);
        while frac.is_multiple_of(10) {
            frac /= 10;
            width -= 1;
        }
        write!(f, "{int}.{frac:0width$}")
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        s.parse()
            .unwrap_or_else(|e| panic!("{s:?} should parse, got {e:?}"))
    }

    // splitmix64: a tiny deterministic generator.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        // A raw value with a random number of significant bits (up to 128).
        fn raw(&mut self) -> u128 {
            let value = (u128::from(self.next()) << 64) | u128::from(self.next());
            value >> (self.next() % 128)
        }
    }

    #[test]
    fn parses_integers_and_fractions() {
        let cases = [
            ("0", 0),
            ("1", SCALE),
            ("10", 10 * SCALE),
            ("1.5", 15 * SCALE / 10),
            ("0.05", 5 * SCALE / 100),
            ("007.50", 75 * SCALE / 10),
            ("0.000000000000000001", 1),
            ("0.123456789012345678", 123_456_789_012_345_678),
            ("340282366920938463463.374607431768211455", u128::MAX),
        ];
        for (s, raw) in cases {
            assert_eq!(s.parse::<Decimal>(), Ok(Decimal(raw)), "{s:?}");
        }
    }

    #[test]
    fn leading_zeros_are_not_an_overflow() {
        let s = format!("{}1.5", "0".repeat(60));
        assert_eq!(s.parse::<Decimal>(), Ok(Decimal(15 * SCALE / 10)));
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!("".parse::<Decimal>(), Err(DecimalError::Empty));
        let invalid = [
            "-1", "+1", "1e3", "1E3", " 1", "1 ", "1.", ".5", ".", "1.2.3", "1,5", "1_000", "abc",
            "0x10", "1.+5", "1.-5", "NaN", "inf", "\u{661}", "\u{bd}",
        ];
        for s in invalid {
            assert_eq!(s.parse::<Decimal>(), Err(DecimalError::Invalid), "{s:?}");
        }
    }

    #[test]
    fn rejects_more_than_18_decimals() {
        for s in [
            "0.0000000000000000001",
            "1.0000000000000000000",
            "0.1234567890123456789",
        ] {
            assert_eq!(
                s.parse::<Decimal>(),
                Err(DecimalError::TooManyDecimals),
                "{s:?}"
            );
        }
    }

    #[test]
    fn rejects_values_above_max() {
        let too_big = [
            "340282366920938463463.374607431768211456".to_string(),
            "340282366920938463463.4".to_string(),
            "340282366920938463464".to_string(),
            format!("1{}", "0".repeat(40)),
            "9".repeat(100),
            format!("{}.5", "9".repeat(100)),
        ];
        for s in too_big {
            assert_eq!(s.parse::<Decimal>(), Err(DecimalError::Overflow), "{s:?}");
        }
    }

    #[test]
    fn prints_the_canonical_form() {
        let cases = [
            (0, "0"),
            (SCALE, "1"),
            (10 * SCALE, "10"),
            (100 * SCALE, "100"),
            (15 * SCALE / 10, "1.5"),
            (5 * SCALE / 100, "0.05"),
            (SCALE / 10, "0.1"),
            (1, "0.000000000000000001"),
            (SCALE + 1, "1.000000000000000001"),
            (1005 * SCALE / 100, "10.05"),
            (u128::MAX, "340282366920938463463.374607431768211455"),
        ];
        for (raw, s) in cases {
            assert_eq!(Decimal(raw).to_string(), s, "Decimal({raw})");
        }
    }

    #[test]
    fn canonical_strings_round_trip() {
        for s in [
            "0",
            "3",
            "40",
            "0.5",
            "0.05",
            "12.3456",
            "0.000000000000000001",
            "99.999999999999999999",
            "340282366920938463463.374607431768211455",
        ] {
            assert_eq!(dec(s).to_string(), s);
        }
        assert_eq!(
            dec("007.500").to_string(),
            "7.5",
            "not canonical, so it changes"
        );
    }

    #[test]
    fn every_value_round_trips_through_its_string() {
        let mut rng = Rng(0xdec);
        for _ in 0..20_000 {
            let mut raw = rng.raw();
            // Every other value is cut to fewer decimals, so that trailing
            // zeros show up.
            if rng.next().is_multiple_of(2) {
                let unit = 10u128.pow(u32::try_from(rng.next() % 30).unwrap());
                raw = raw / unit * unit;
            }
            let s = Decimal(raw).to_string();
            assert_eq!(
                s.parse::<Decimal>(),
                Ok(Decimal(raw)),
                "Decimal({raw}) printed as {s:?}"
            );
        }
    }

    #[test]
    fn one_point_one_times_three_is_exactly_three_point_three() {
        assert_ne!((1.1_f64 * 3.0).to_string(), "3.3", "f64 can't do this");
        let product = dec("1.1").checked_mul(dec("3")).unwrap();
        assert_eq!(product, dec("3.3"));
        assert_eq!(product.to_string(), "3.3");
    }

    #[test]
    fn addition_is_exact() {
        assert_eq!(dec("0.1").checked_add(dec("0.2")), Ok(dec("0.3")));
        assert_eq!(
            Decimal::MAX.checked_add(Decimal(1)),
            Err(DecimalError::Overflow)
        );
    }

    #[test]
    fn mul_rounds_down_to_18_decimals() {
        // (a, b, a * b rounded down)
        let cases = [
            ("2.5", "2.5", "6.25"),
            ("1.5", "1", "1.5"),
            ("123.456", "0", "0"),
            ("0.333333333333333333", "3", "0.999999999999999999"),
            ("0.000000001", "0.000000001", "0.000000000000000001"),
            ("0.000000001", "0.0000000009", "0"),
            ("0.000000000000000001", "0.5", "0"),
        ];
        for (a, b, product) in cases {
            assert_eq!(dec(a).checked_mul(dec(b)), Ok(dec(product)), "{a} * {b}");
            assert_eq!(dec(b).checked_mul(dec(a)), Ok(dec(product)), "{b} * {a}");
        }
    }

    #[test]
    fn mul_does_not_overflow_on_the_way() {
        // Every raw product here is far above u128::MAX, but the results fit.
        assert_eq!(Decimal::MAX.checked_mul(Decimal::ONE), Ok(Decimal::MAX));
        assert_eq!(
            dec("300000000000000000000").checked_mul(dec("0.5")),
            Ok(dec("150000000000000000000"))
        );
        assert_eq!(
            dec("10000000000").checked_mul(dec("10000000000")),
            Ok(dec("100000000000000000000"))
        );
        // Fractions on both sides, and the big factor second.
        assert_eq!(
            dec("0.5").checked_mul(dec("300000000000000000000")),
            Ok(dec("150000000000000000000"))
        );
        assert_eq!(
            dec("12345678901.5").checked_mul(dec("12345678901.5")),
            Ok(dec("152415787538942246702.25"))
        );
        // (2^64 - 1) * 2^64 raw units: just below 2^128.
        assert_eq!(
            dec("18446744073.709551615").checked_mul(dec("18446744073.709551616")),
            Ok(Decimal(u128::MAX - u128::from(u64::MAX)))
        );
    }

    #[test]
    fn mul_overflow_is_an_error() {
        let overflowing = [
            (Decimal::MAX, dec("2")),
            (Decimal::MAX, dec("1.000000000000000001")),
            (dec("20000000000"), dec("20000000000")),
            // Exactly 2^128 raw units: one step above `Decimal::MAX`.
            (dec("18446744073.709551616"), dec("18446744073.709551616")),
        ];
        for (a, b) in overflowing {
            assert_eq!(
                a.checked_mul(b),
                Err(DecimalError::Overflow),
                "{a:?} * {b:?}"
            );
        }
    }

    #[test]
    fn div_rounds_down_to_18_decimals() {
        // (a, b, a / b rounded down)
        let cases = [
            ("10", "4", "2.5"),
            ("7.5", "2.5", "3"),
            ("0", "7", "0"),
            ("1", "3", "0.333333333333333333"),
            ("2", "3", "0.666666666666666666"),
            ("1", "0.000000000000000001", "1000000000000000000"),
            ("0.000000000000000001", "2", "0"),
        ];
        for (a, b, quotient) in cases {
            assert_eq!(dec(a).checked_div(dec(b)), Ok(dec(quotient)), "{a} / {b}");
        }
    }

    #[test]
    fn div_does_not_overflow_on_the_way() {
        assert_eq!(Decimal::MAX.checked_div(Decimal::ONE), Ok(Decimal::MAX));
        assert_eq!(
            dec("300000000000000000000").checked_div(dec("2")),
            Ok(dec("150000000000000000000"))
        );
    }

    #[test]
    fn div_errors_say_what_went_wrong() {
        assert_eq!(
            Decimal::ONE.checked_div(Decimal::ZERO),
            Err(DecimalError::DivisionByZero)
        );
        assert_eq!(
            Decimal::ZERO.checked_div(Decimal::ZERO),
            Err(DecimalError::DivisionByZero)
        );
        assert_eq!(
            Decimal::MAX.checked_div(dec("0.5")),
            Err(DecimalError::Overflow)
        );
        assert_eq!(
            dec("1000").checked_div(Decimal(1)),
            Err(DecimalError::Overflow)
        );
    }

    #[test]
    fn a_third_times_three_is_not_one() {
        let third = Decimal::ONE.checked_div(dec("3")).unwrap();
        assert_eq!(third.to_string(), "0.333333333333333333");
        let back = third.checked_mul(dec("3")).unwrap();
        assert_eq!(back.to_string(), "0.999999999999999999");
    }
}
