// Module 5 · Checked math — part 1: a 256-bit `mul_div`, rounded down and up.
//
// Money in a ledger or a smart contract is an integer count of the smallest
// unit, often with 18 decimals (1.5 tokens is `1_500_000_000_000_000_000`),
// held in a `u128`. Prices, exchange rates and fees are fixed-point numbers
// with the same scale, so almost every calculation has the shape
// `amount * price / SCALE`, and the intermediate product is where it breaks.
// 500 million tokens (5e26 units) at a price of 1.5 is
// `5e26 * 1.5e18 / 1e18`. The answer, 7.5e26, fits easily, but the product
// `5e26 * 1.5e18 = 7.5e44` is far above `u128::MAX` (about 3.4e38).
//
// The quick fixes all fail:
//   - A plain `*` panics in debug builds ("attempt to multiply with
//     overflow") and wraps silently in release builds, unless the profile
//     sets `overflow-checks = true` (`31_debugging/debugging2`).
//   - `checked_mul` makes the overflow visible, but it refuses EVERY product
//     above `u128::MAX`, including the ones whose quotient fits. That is the
//     code below.
//   - Dividing first, `a / d * b`, never overflows early, but it throws the
//     remainder away: `1 / 3 * 3 == 0`.
//   - `f64` has a 53-bit mantissa. It cannot even hold 1e30 exactly.
//
// The real fix is the idea behind Uniswap's `FullMath.mulDiv` (which does it
// for 256-bit numbers with a 512-bit product): the product of two 128-bit
// numbers ALWAYS fits in 256 bits. Compute it exactly, as a high and a low
// `u128`, divide that 256-bit value by `d`, and fail only when the quotient
// itself does not fit in 128 bits, or when `d == 0`. Since Rust 1.91,
// `u128::carrying_mul(rhs, carry)` returns the full `self * rhs + carry` as a
// `(low, high)` pair. (`widening_mul`, the same thing without the carry, is
// still unstable.) std has no 256-bit type to divide with, so `div_wide`
// below does the 256-by-128-bit long division for you.
//
// Rounding is part of the contract. `mul_div_floor` rounds down, and
// `mul_div_ceil` rounds up whenever the division leaves a remainder; part 2
// shows why a vault needs both directions. The textbook ceiling
// `(x + d - 1) / d` has an overflow of its own: it breaks for any `x` near
// `u128::MAX`, even when the rounded-up answer fits.
//
// How interviewers probe this: "Compute `a * b / c` on u128 without
// overflowing. How big can the intermediate get? Why not divide first? How do
// you round up without overflowing? What does overflow do in a release
// build?"

/// Divides the 256-bit number `hi * 2^128 + lo` by `d`. Returns
/// `(quotient, remainder)`, or `None` when `d == 0` or the quotient does not
/// fit in a `u128`. Given: the division is not what this exercise drills.
fn div_wide(hi: u128, lo: u128, d: u128) -> Option<(u128, u128)> {
    // The quotient is below 2^128 exactly when `hi < d`.
    if d == 0 || hi >= d {
        return None;
    }
    if hi == 0 {
        return Some((lo / d, lo % d));
    }
    // Schoolbook long division in base 2: bring down one bit of `lo` at a
    // time. `rem < d` holds before every step, so `rem * 2 + bit` needs at
    // most 129 bits, and `carry` is the 129th.
    let (mut quot, mut rem) = (0u128, hi);
    for i in (0..128).rev() {
        let carry = rem >> 127;
        rem = (rem << 1) | ((lo >> i) & 1);
        quot <<= 1;
        if carry == 1 || rem >= d {
            // The true difference is below `d`, so it fits in 128 bits, and
            // the wrapping subtraction computes it even when `carry` is set.
            rem = rem.wrapping_sub(d);
            quot |= 1;
        }
    }
    Some((quot, rem))
}

/// `a * b / d`, rounded down. `None` when `d == 0` or the result does not fit
/// in a `u128`, and never just because `a * b` does not fit.
fn mul_div_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    // TODO: `checked_mul` gives up whenever `a * b` is above `u128::MAX`,
    // even when the quotient fits. `the_motivating_example_fits` fails:
    // `mul_div_floor(5e26, 1.5e18, 1e18)` returns `None` instead of
    // `Some(7.5e26)`. And a `d` of 0 panics with "attempt to divide by zero"
    // instead of returning `None`. Requirements: work with the exact 256-bit
    // product (no dividing first, no floats, no fallback that is exact only
    // some of the time), divide it with `div_wide`, and return `None` only
    // for `d == 0` or a quotient above `u128::MAX`. Until you do, the tests
    // will fail.
    Some(a.checked_mul(b)? / d)
}

/// `a * b / d`, rounded up: the smallest `q` with `q * d >= a * b`. `None`
/// when `d == 0` or the rounded-up result does not fit in a `u128`.
fn mul_div_ceil(a: u128, b: u128, d: u128) -> Option<u128> {
    // TODO: two bugs. The product still goes through `checked_mul`, and the
    // textbook ceiling `(x + d - 1) / d` overflows by itself:
    // `mul_div_ceil(u128::MAX, 1, 2)` panics with "attempt to add with
    // overflow", although the answer, 2^127, fits. Requirements: the same
    // exact product as `mul_div_floor`; add one to the quotient exactly when
    // the division leaves a remainder; return `None` for `d == 0` and for a
    // rounded-up result above `u128::MAX`, and never panic. Until you do, the
    // tests will fail.
    let product = a.checked_mul(b)?;
    Some((product + d - 1) / d)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    const E18: u128 = 1_000_000_000_000_000_000;
    const MAX: u128 = u128::MAX;

    // splitmix64: a tiny deterministic generator, so every run checks the
    // same cases.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        // A value with a random number of significant bits (up to 64), so
        // tiny and huge magnitudes both show up.
        fn any(&mut self) -> u128 {
            let shift = self.next() % 64;
            u128::from(self.next() >> shift)
        }
    }

    #[test]
    fn the_motivating_example_fits() {
        // 5e26 * 1.5e18 = 7.5e44 does not fit in a u128, but 7.5e26 does.
        let (a, b) = (5 * 10u128.pow(26), 15 * 10u128.pow(17));
        assert_eq!(a.checked_mul(b), None, "the product really overflows");
        let expected = Some(75 * 10u128.pow(25));
        assert_eq!(mul_div_floor(a, b, E18), expected);
        assert_eq!(mul_div_ceil(a, b, E18), expected);
    }

    #[test]
    fn results_that_fit_are_never_refused() {
        // (a, b, d, the exact quotient)
        let cases = [
            (MAX, MAX, MAX, MAX),
            (MAX, 2, 2, MAX),
            (MAX, 2, 3, MAX / 3 * 2),
            (MAX, 1, 1, MAX),
            (MAX, MAX - 1, MAX, MAX - 1),
            (1 << 127, 4, 8, 1 << 126),
            (E18 * E18, E18 * E18, E18 * E18, E18 * E18),
            (E18 * E18, 3 * E18, E18, 3 * E18 * E18),
        ];
        for (a, b, d, q) in cases {
            assert_eq!(mul_div_floor(a, b, d), Some(q), "floor({a} * {b} / {d})");
            assert_eq!(mul_div_floor(b, a, d), Some(q), "floor({b} * {a} / {d})");
            assert_eq!(mul_div_ceil(a, b, d), Some(q), "ceil({a} * {b} / {d})");
        }
    }

    #[test]
    fn exact_near_the_top_of_the_range() {
        // (MAX - k) * (MAX - j) = MAX * (MAX - k - j) + k * j, and k * j < MAX.
        for k in 0..40u128 {
            for j in 0..40u128 {
                let floor = MAX - k - j;
                let ceil = if k * j == 0 { floor } else { floor + 1 };
                let (a, b) = (MAX - k, MAX - j);
                assert_eq!(mul_div_floor(a, b, MAX), Some(floor), "k = {k}, j = {j}");
                assert_eq!(mul_div_ceil(a, b, MAX), Some(ceil), "k = {k}, j = {j}");
            }
        }
    }

    #[test]
    fn division_by_zero_is_none_not_a_panic() {
        for (a, b) in [(0, 0), (1, 1), (5, 0), (E18, E18), (MAX, MAX)] {
            assert_eq!(mul_div_floor(a, b, 0), None, "floor({a} * {b} / 0)");
            assert_eq!(mul_div_ceil(a, b, 0), None, "ceil({a} * {b} / 0)");
        }
    }

    #[test]
    fn a_quotient_above_max_is_none() {
        let cases = [
            (MAX, MAX, 1),
            (MAX, 2, 1),
            (1 << 127, 4, 2),
            (MAX, MAX, MAX - 1),
            (MAX, MAX, 1 << 127),
            (MAX, E18, E18 - 1),
        ];
        for (a, b, d) in cases {
            assert_eq!(mul_div_floor(a, b, d), None, "floor({a} * {b} / {d})");
            assert_eq!(mul_div_ceil(a, b, d), None, "ceil({a} * {b} / {d})");
        }
    }

    #[test]
    fn ceil_rounds_up_only_when_there_is_a_remainder() {
        // (a, b, d, floor, ceil)
        let cases = [
            (0, 5, 7, 0, 0),
            (5, 0, MAX, 0, 0),
            (1, 1, 3, 0, 1),
            (3, 1, 3, 1, 1),
            (7, 2, 7, 2, 2),
            (10, 10, 7, 14, 15),
            (1, 1, MAX, 0, 1),
            (MAX, 1, 2, MAX / 2, MAX / 2 + 1),
            (MAX, 1, MAX, 1, 1),
            (E18 + 1, E18, E18, E18 + 1, E18 + 1),
            (E18 + 1, E18 - 1, E18, E18 - 1, E18),
        ];
        for (a, b, d, floor, ceil) in cases {
            assert_eq!(
                mul_div_floor(a, b, d),
                Some(floor),
                "floor({a} * {b} / {d})"
            );
            assert_eq!(mul_div_ceil(a, b, d), Some(ceil), "ceil({a} * {b} / {d})");
        }
    }

    #[test]
    fn rounding_up_past_max_is_none() {
        // 7 * b = 2 * MAX + 1, so the exact quotient is MAX + 1/2: it rounds
        // down to MAX, and up to a value that does not fit.
        let b = MAX / 7 * 2 + 1;
        assert_eq!(mul_div_floor(7, b, 2), Some(MAX));
        assert_eq!(mul_div_ceil(7, b, 2), None);
        // Without a remainder there is nothing to round.
        assert_eq!(mul_div_ceil(MAX, 3, 3), Some(MAX));
    }

    #[test]
    fn matches_plain_u128_math_whenever_the_product_fits() {
        // 200_000 cases with every operand below 2^64. Then `a * b` fits in a
        // u128, so plain `*` and `/` are the oracle.
        let mut rng = Rng(0x5eed);
        for _ in 0..200_000 {
            let (a, b, d) = (rng.any(), rng.any(), rng.any().max(1));
            let product = a * b;
            assert_eq!(
                mul_div_floor(a, b, d),
                Some(product / d),
                "floor({a} * {b} / {d})"
            );
            assert_eq!(
                mul_div_ceil(a, b, d),
                Some(product.div_ceil(d)),
                "ceil({a} * {b} / {d})"
            );
        }
    }

    #[test]
    fn matches_exact_answers_for_products_far_above_max() {
        // Build a = p * d1 + r (with r < d1) and b = q * d2, and divide by
        // d = d1 * d2. Then, exactly,
        //     a * b / d = p * q + r * q / d1,
        // and every piece of the right-hand side fits in a u128, while a * b
        // is up to 2^254.
        let mut rng = Rng(0xc0ffee);
        for _ in 0..30_000 {
            let (p, q) = (rng.any(), rng.any());
            let d1 = (rng.any() >> 1).max(1);
            let d2 = (rng.any() >> 1).max(1);
            let r = rng.any() % d1;
            let (a, b, d) = (p * d1 + r, q * d2, d1 * d2);
            let floor = p * q + r * q / d1;
            let ceil = if (r * q).is_multiple_of(d1) {
                floor
            } else {
                floor + 1
            };
            assert_eq!(
                mul_div_floor(a, b, d),
                Some(floor),
                "floor({a} * {b} / {d})"
            );
            assert_eq!(mul_div_ceil(a, b, d), Some(ceil), "ceil({a} * {b} / {d})");
        }
    }
}
