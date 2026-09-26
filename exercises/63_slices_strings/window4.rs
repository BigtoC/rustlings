// Module 5 · Slices and strings — part 4: binary search with `partition_point`, `midpoint` and half-open ranges (E0308).
//
// Slices have `binary_search`, which returns `Ok(i)` for AN index holding the
// value and `Err(i)` for the index where it would be inserted. With duplicates
// it may return any of them (on Rust 1.96, `[2; 8].binary_search(&2)` is
// `Ok(7)`, the LAST one), so it cannot answer "where do the 2s start?" or "how
// many 2s are there?". Those questions need a boundary, not a match:
//
//   - `slice.partition_point(pred)` takes a predicate that is true for a
//     prefix of the slice and false after it, and returns the index of the
//     first element where it is false. That is C++'s `lower_bound` /
//     `upper_bound` and Python's `bisect_left` / `bisect_right`:
//     `partition_point(|&e| e < x)` is the first index with `e >= x`, and
//     `partition_point(|&e| e <= x)` is the first index with `e > x`.
//   - Half-open ranges, `lo..hi` meaning lo <= x < hi, keep the arithmetic
//     honest. An empty range is `lo == hi`, the size is `hi - lo`, two
//     neighboring ranges share an endpoint, and no `- 1` ever appears. The
//     one subtraction left, "end boundary minus start boundary", is a `usize`
//     subtraction, so a REVERSED range (`lo > hi`) can panic with "attempt to
//     subtract with overflow" unless you design it away.
//
// "Binary search on the answer" is the interview favorite built on top: when
// the answer is a number and "is `x` enough?" is monotone (false for small
// `x`, true from some point on), search for the first `x` that is enough.
// `first_true` is that search over a half-open `u64` range. Its midpoint is
// written `(lo + hi) / 2`, the bug Joshua Bloch described in 2006 after it
// had hidden in Java's `Arrays.binarySearch` for about nine years: once
// `lo + hi` passes the type's maximum, it overflows. Rust's debug builds
// panic with "attempt to add with overflow". In release the sum wraps, `mid`
// lands BELOW `lo`, and the search can move backwards and never finish.
// `lo + (hi - lo) / 2` cannot overflow (with `lo <= hi`), and
// `lo.midpoint(hi)` (stable for unsigned integers since Rust 1.85) says what
// it means. The ranges here are unsigned on purpose: signed `midpoint` (1.87)
// rounds toward ZERO, so `(-3i32).midpoint(-2)` is -2, which is `hi`, a probe
// outside the half-open range.
//
// `min_ship_capacity` (LeetCode 1011) is the classic "binary search on the
// answer": the smallest ship capacity that moves every package, in order,
// within `days` days. A bigger ship never needs more days, so "does capacity
// `c` fit in `days`?" is monotone in `c`, and the answer lies between the
// heaviest package (below that, a package never fits) and the total weight
// (everything in one day always fits). Weights are `u32`; add them up as
// `u64`, or two heavy packages already overflow the sum.
//
// How interviewers probe this: "Implement `lower_bound`", "Why is
// `(lo + hi) / 2` a bug?", "What does `binary_search` return for duplicates?",
// and "Why is the capacity check monotone?".

// Counts the elements `x` of `sorted` with `lo <= x < hi` (half-open).
// `sorted` is in non-decreasing order. An empty or reversed range
// (`lo >= hi`) counts 0.
fn count_in_range(sorted: &[i32], lo: i32, hi: i32) -> usize {
    // TODO: rustc rejects this with E0308 "mismatched types" (expected
    // `usize`, found `()`): the body is empty. Implement it with two
    // `partition_point` calls, one for each boundary. Requirements: O(log n),
    // however many duplicates there are (so no walking from a
    // `binary_search` hit); half-open, so elements equal to `hi` do not
    // count; a reversed range gives 0 instead of a "subtract with overflow"
    // panic. No `unsafe`. Until you return the count, this exercise will not
    // compile.
}

// Returns the smallest `x` in the half-open range `lo..hi` for which
// `pred(x)` is true, or `hi` if there is none. `pred` must be monotone on the
// range: false, ..., false, then true from some point on (possibly never, or
// from the start).
fn first_true(mut lo: u64, mut hi: u64, mut pred: impl FnMut(u64) -> bool) -> u64 {
    // TODO: Once the other two functions compile, the `first_true_*near_max`
    // tests panic with "attempt to add with overflow" on the `mid` line: in
    // `(lo + hi) / 2`, the sum passes `u64::MAX` once both ends are large,
    // for example as soon as the search has moved `lo` past `u64::MAX / 2`.
    // Compute the midpoint without overflow. Requirements: every probe stays
    // inside `lo..hi` (the tests check it, so a midpoint built on
    // `wrapping_add` or `saturating_add` fails); at most 64 probes for any
    // range; no `unsafe`. Widening to `u128` works but is not needed. Until
    // you rule out the overflow, the tests will fail.
    while lo < hi {
        let mid = (lo + hi) / 2;
        if pred(mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

// Returns the smallest ship capacity that ships every package within `days`
// days. The packages go in the given order; each day the ship is loaded with
// the next packages as long as their total stays within its capacity.
// `days` is at least 1. No packages need a capacity of 0.
fn min_ship_capacity(weights: &[u32], days: u32) -> u64 {
    // TODO: rustc rejects this with E0308 "mismatched types" (expected `u64`,
    // found `()`): the body is empty. Binary search on the answer with
    // `first_true`. Requirements: O(n log(total weight)); the capacity check
    // loads packages greedily, in order; the search range is right for any
    // input (a single package heavier than the rest, `days` larger than the
    // number of packages, packages near `u32::MAX`). No `unsafe`. Until you
    // return the capacity, this exercise will not compile.
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny deterministic pseudo-random generator (a 64-bit LCG), so every
    // run checks exactly the same inputs.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    // Runs `first_true(lo, hi, pred)` and checks every probe: it must lie
    // inside `lo..hi`, and a binary search over fewer than 2^64 values needs
    // at most 64 of them. (The limit also turns a search that would loop
    // forever into a quick failure.)
    fn checked(lo: u64, hi: u64, pred: impl Fn(u64) -> bool) -> u64 {
        let mut probes = 0;
        first_true(lo, hi, |x| {
            probes += 1;
            assert!(probes <= 64, "more than 64 probes: not a binary search");
            assert!(lo <= x && x < hi, "probed {x}, outside {lo}..{hi}");
            pred(x)
        })
    }

    // `checked` with the predicate `x >= threshold`.
    fn search(lo: u64, hi: u64, threshold: u64) -> u64 {
        checked(lo, hi, |x| x >= threshold)
    }

    // `min_ship_capacity` searches with `first_true`. The ship tests call this
    // first, so a broken `first_true` fails them quickly instead of looping
    // forever inside `min_ship_capacity`.
    fn assert_first_true_works() {
        for hi in 0..8 {
            for threshold in 0..9 {
                let got = search(0, hi, threshold);
                assert_eq!(got, threshold.min(hi), "first_true is broken");
            }
        }
    }

    // How many days capacity `cap` needs, by brute force.
    fn days_needed(weights: &[u32], cap: u64) -> u64 {
        let (mut days, mut load) = (1, 0);
        for &w in weights {
            if load + u64::from(w) > cap {
                days += 1;
                load = 0;
            }
            load += u64::from(w);
        }
        days
    }

    // ---- count_in_range ----

    #[test]
    fn counts_duplicates_in_a_half_open_range() {
        let v = [1, 2, 2, 2, 3, 5];
        assert_eq!(count_in_range(&v, 2, 3), 3);
        assert_eq!(count_in_range(&v, 2, 4), 4);
        assert_eq!(count_in_range(&v, 0, 100), 6);
        assert_eq!(count_in_range(&v, 5, 6), 1);
        // Half-open: `hi` itself is not counted.
        assert_eq!(count_in_range(&v, 1, 2), 1);
        // `[2; 8].binary_search(&2)` returns `Ok(7)` on Rust 1.96: a match,
        // not the first 2. Counting from it would give 1.
        assert_eq!(count_in_range(&[2; 8], 2, 3), 8);
    }

    #[test]
    fn values_that_are_not_in_the_slice() {
        let v = [1, 2, 2, 2, 3, 5];
        assert_eq!(count_in_range(&v, 4, 5), 0);
        assert_eq!(count_in_range(&v, 6, 10), 0);
        assert_eq!(count_in_range(&v, -5, 1), 0);
        assert_eq!(count_in_range(&v, -5, 2), 1);
        assert_eq!(count_in_range(&[], 0, 10), 0);
    }

    #[test]
    fn empty_and_reversed_ranges_count_zero() {
        let v = [1, 2, 2, 2, 3, 5];
        assert_eq!(count_in_range(&v, 2, 2), 0);
        assert_eq!(count_in_range(&v, 5, 2), 0);
        assert_eq!(count_in_range(&v, i32::MAX, i32::MIN), 0);
    }

    #[test]
    fn count_at_the_extremes_of_i32() {
        let (min, max) = (i32::MIN, i32::MAX);
        let v = [min, min, 0, max, max];
        // A half-open range with an `i32` end can never include `i32::MAX`.
        assert_eq!(count_in_range(&v, min, max), 3);
        assert_eq!(count_in_range(&v, min, min + 1), 2);
        assert_eq!(count_in_range(&v, max - 1, max), 0);
    }

    #[test]
    fn count_matches_a_linear_scan() {
        let mut state = 1011;
        for _ in 0..2_000 {
            let len = next(&mut state) % 12;
            let mut v: Vec<i32> = (0..len)
                .map(|_| (next(&mut state) % 7) as i32 - 3)
                .collect();
            v.sort_unstable();
            let lo = (next(&mut state) % 11) as i32 - 5;
            let hi = (next(&mut state) % 11) as i32 - 5;
            let expected = v.iter().filter(|&&x| lo <= x && x < hi).count();
            assert_eq!(count_in_range(&v, lo, hi), expected, "{v:?} in {lo}..{hi}");
        }
    }

    // ---- first_true ----

    #[test]
    fn first_true_in_a_small_range() {
        assert_eq!(checked(0, 100, |x| x * x >= 50), 8);
        assert_eq!(checked(10, 20, |_| false), 20);
        assert_eq!(checked(10, 20, |_| true), 10);
        // An empty range has nothing to probe.
        assert_eq!(checked(5, 5, |_| panic!("probed an empty range")), 5);
    }

    #[test]
    fn first_true_every_threshold_in_small_ranges() {
        for lo in 0..20 {
            for hi in lo..20 {
                for threshold in 0..25 {
                    let expected = threshold.clamp(lo, hi);
                    assert_eq!(search(lo, hi, threshold), expected);
                }
            }
        }
    }

    #[test]
    fn first_true_near_max() {
        let max = u64::MAX;
        assert_eq!(search(0, max, max - 3), max - 3);
        assert_eq!(search(0, max, max - 1), max - 1);
        assert_eq!(search(max - 10, max, max - 1), max - 1);
        assert_eq!(search(0, max, 1 << 63), 1 << 63);
    }

    #[test]
    fn first_true_none_near_max() {
        let max = u64::MAX;
        // Never true: the answer is `hi`, and `hi` itself is never probed.
        assert_eq!(search(0, max, max), max);
        assert_eq!(search(max - 1, max, max), max);
        // Always true: the answer is `lo`.
        assert_eq!(search(0, max, 0), 0);
    }

    // ---- min_ship_capacity ----

    #[test]
    fn ship_capacity_classic_examples() {
        assert_first_true_works();
        assert_eq!(min_ship_capacity(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 5), 15);
        assert_eq!(min_ship_capacity(&[3, 2, 2, 4, 1, 4], 3), 6);
        assert_eq!(min_ship_capacity(&[1, 2, 3, 1, 1], 4), 3);
    }

    #[test]
    fn ship_capacity_edge_cases() {
        assert_first_true_works();
        // One day: everything goes at once.
        assert_eq!(min_ship_capacity(&[1, 2, 3, 4], 1), 10);
        // More days than packages: the heaviest package decides.
        assert_eq!(min_ship_capacity(&[1, 2, 3, 4], 10), 4);
        assert_eq!(min_ship_capacity(&[10], 3), 10);
        assert_eq!(min_ship_capacity(&[1, 10, 1], 3), 10);
        assert_eq!(min_ship_capacity(&[], 1), 0);
    }

    #[test]
    fn ship_capacity_with_heavy_packages() {
        assert_first_true_works();
        // The loads and the total weight pass `u32::MAX` here.
        let heavy = u64::from(u32::MAX);
        assert_eq!(min_ship_capacity(&[u32::MAX; 3], 3), heavy);
        assert_eq!(min_ship_capacity(&[u32::MAX, 1, u32::MAX], 3), heavy);
        assert_eq!(min_ship_capacity(&[u32::MAX, 1, u32::MAX], 2), heavy + 1);
        assert_eq!(min_ship_capacity(&[u32::MAX, u32::MAX, 5], 2), heavy + 5);
    }

    #[test]
    fn ship_capacity_matches_a_brute_force_model() {
        assert_first_true_works();
        let mut state = 1;
        for _ in 0..500 {
            let len = next(&mut state) % 9;
            let weights: Vec<u32> = (0..len)
                .map(|_| (next(&mut state) % 10) as u32 + 1)
                .collect();
            let days = (next(&mut state) % 5) as u32 + 1;
            // Try every capacity from 0 upward until one fits.
            let expected = (0..)
                .find(|&cap| {
                    weights.iter().all(|&w| u64::from(w) <= cap)
                        && days_needed(&weights, cap) <= u64::from(days)
                })
                .unwrap();
            assert_eq!(
                min_ship_capacity(&weights, days),
                expected,
                "{weights:?} in {days} days"
            );
        }
    }
}
