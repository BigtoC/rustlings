// Module 5 · Slices and strings — part 1: two pointers without `len() - 1` underflow or `i32` overflow (E0308).
//
// Two pointers is the first pattern of most live-coding rounds. On a SORTED
// slice, put one index at each end. If the pair's sum is too small, only a
// bigger left element can help, so move `i` right; if it is too big, move `j`
// left. Every step rules out a whole row of pairs, so the scan is O(n) instead
// of the O(n^2) double loop. Three-sum is the same idea one level up: sort,
// fix the smallest element of the triple, and two-pointer the rest, O(n^2).
//
// The algorithm is the same in every language. The bugs below are not: they
// are the two places where Rust's integer rules turn the textbook solution
// into a panic.
//
//   1. Indices are `usize`, and a `usize` cannot be negative. On an empty
//      slice `nums.len() - 1` is `0 - 1`. With overflow checks on (debug
//      builds and tests) that PANICS with "attempt to subtract with overflow".
//      In a release build the checks are off, `j` wraps to `usize::MAX`, and
//      the first `nums[..]` panics with "index out of bounds" instead. A
//      Java `int` index would just be `-1`, and C++'s unsigned `size_t` wraps
//      silently; Rust makes you decide what an empty input means before you
//      compute `len() - 1`, or not compute it at all.
//   2. `i32 + i32` can overflow even when the ANSWER fits in an `i32`. On the
//      way to a small target the pointers visit pairs like `1 + i32::MAX`.
//      With overflow checks on that panics with "attempt to add with
//      overflow". In release it wraps to a large NEGATIVE number, the pointer
//      moves the wrong way, and the function quietly returns a wrong answer.
//      Widened to `i64` (`i64::from` is lossless), every sum of two or three
//      `i32`s is exact.
//
// `checked_add` and `saturating_add` look like fixes and are not. Giving up
// at the first overflow (a `?` on `checked_add`) misses pairs further in, and
// a sum saturated to `i32::MAX` compares EQUAL to a target of `i32::MAX`.
// `wrapping_add` is the release-build bug on purpose. The tests have a case
// for each. (The overflow rules themselves are in `31_debugging/debugging2`;
// here they hide inside an algorithm.)
//
// How interviewers probe this: "What does your code do on an empty input?",
// "Can that sum overflow, and what happens in release?", and for three-sum,
// "How do you avoid duplicate triples without a `HashSet`?".

use std::cmp::Ordering;

// Returns the indices `(i, j)`, with `i < j`, of two elements of `nums` that
// add up to `target`, or `None` if no two elements do. `nums` is sorted in
// non-decreasing order. When several pairs work, any one of them will do.
fn two_sum_sorted(nums: &[i32], target: i32) -> Option<(usize, usize)> {
    // `checked_sub` asks "is there a last index?" instead of assuming one, and
    // `?` returns `None` for an empty slice before any index exists. A
    // one-element slice gets `j == 0`, so the loop never runs.
    let mut i = 0;
    let mut j = nums.len().checked_sub(1)?;
    // The sum of two `i32`s always fits in an `i64`, and `i64::from` is
    // lossless, so every comparison is exact: nothing wraps or saturates, and
    // the pointers always move the right way.
    let target = i64::from(target);
    while i < j {
        let sum = i64::from(nums[i]) + i64::from(nums[j]);
        match sum.cmp(&target) {
            Ordering::Equal => return Some((i, j)),
            Ordering::Less => i += 1,
            Ordering::Greater => j -= 1,
        }
    }
    None
}

// Returns every distinct triple of values from `nums` (taken from three
// different positions) that adds up to zero. Each triple is sorted ascending,
// the list is sorted ascending (lexicographically), and no triple appears
// twice. `nums` may be in any order and may contain duplicates.
fn three_sum(nums: &[i32]) -> Vec<[i32; 3]> {
    // Sorting needs a mutable, owned buffer, so sort a copy of the borrowed
    // slice. `sort_unstable` sorts in place without allocating.
    let mut v = nums.to_vec();
    v.sort_unstable();
    let mut out = Vec::new();
    for (a, &first) in v.iter().enumerate() {
        // Skipping a repeated first value (and, below, a repeated second
        // value) is what dedupes: the same triple would need the same first
        // and second values. It also keeps the output sorted, because `a` and,
        // for a fixed `a`, `lo` only ever grow.
        if a > 0 && first == v[a - 1] {
            continue;
        }
        // `a < v.len()` here, so `v.len() - 1` cannot underflow.
        let (mut lo, mut hi) = (a + 1, v.len() - 1);
        while lo < hi {
            // Three `i32`s always add up exactly in an `i64`.
            let sum = i64::from(first) + i64::from(v[lo]) + i64::from(v[hi]);
            match sum.cmp(&0) {
                Ordering::Less => lo += 1,
                Ordering::Greater => hi -= 1,
                Ordering::Equal => {
                    out.push([first, v[lo], v[hi]]);
                    lo += 1;
                    while lo < hi && v[lo] == v[lo - 1] {
                        lo += 1;
                    }
                }
            }
        }
    }
    out
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // Checks that `got` is a valid answer: two different in-bounds positions
    // whose values add up to `target`, compared without overflow.
    fn assert_valid_pair(nums: &[i32], target: i32, got: Option<(usize, usize)>) {
        let Some((i, j)) = got else {
            panic!("expected a pair adding up to {target} in {nums:?}, got None");
        };
        assert!(i < j && j < nums.len(), "bad pair ({i}, {j}) for {nums:?}");
        assert_eq!(i64::from(nums[i]) + i64::from(nums[j]), i64::from(target));
    }

    // A tiny deterministic pseudo-random generator (a 64-bit LCG), so every
    // run checks exactly the same inputs.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    // ---- two_sum_sorted ----

    #[test]
    fn finds_the_pair_in_the_classic_examples() {
        assert_eq!(two_sum_sorted(&[2, 7, 11, 15], 9), Some((0, 1)));
        assert_eq!(two_sum_sorted(&[2, 3, 4], 6), Some((0, 2)));
        assert_eq!(two_sum_sorted(&[-1, 0], -1), Some((0, 1)));
        assert_eq!(two_sum_sorted(&[-8, -3, 0, 4, 9, 20], 13), Some((3, 4)));
    }

    #[test]
    fn returns_none_when_no_two_elements_add_up() {
        assert_eq!(two_sum_sorted(&[1, 2, 3], 7), None);
        // One element may not be used twice: `1 + 1` is not a pair here.
        assert_eq!(two_sum_sorted(&[1, 2, 3], 2), None);
        assert_eq!(two_sum_sorted(&[-5, -2, 10], 0), None);
    }

    #[test]
    fn any_valid_pair_is_accepted_when_there_are_several() {
        for (nums, target) in [(&[2, 2, 2, 2][..], 4), (&[1, 2, 3, 4, 5, 6], 7)] {
            assert_valid_pair(nums, target, two_sum_sorted(nums, target));
        }
        // The same value at two positions is a real pair.
        assert_valid_pair(&[3, 3], 6, two_sum_sorted(&[3, 3], 6));
    }

    #[test]
    fn empty_and_one_element_inputs_have_no_pair() {
        assert_eq!(two_sum_sorted(&[], 0), None);
        assert_eq!(two_sum_sorted(&[5], 10), None);
        assert_eq!(two_sum_sorted(&[5], 5), None);
    }

    #[test]
    fn sums_near_i32_max_are_exact() {
        let max = i32::MAX;
        assert_eq!(two_sum_sorted(&[max - 1, max], 5), None);
        // The answer is small, but the scan starts at `1 + i32::MAX`: giving
        // up at the first overflow would miss it.
        assert_eq!(two_sum_sorted(&[1, 4, max - 1, max], 5), Some((0, 1)));
        // `1 + i32::MAX` saturates to `i32::MAX`, but it is not equal to it.
        assert_eq!(two_sum_sorted(&[1, max], max), None);
        assert_eq!(two_sum_sorted(&[0, 1, max - 1], max), Some((1, 2)));
    }

    #[test]
    fn sums_near_i32_min_are_exact() {
        let min = i32::MIN;
        assert_eq!(two_sum_sorted(&[min, min + 1], 5), None);
        // `i32::MIN + -1` saturates to `i32::MIN`, but it is not equal to it.
        assert_eq!(two_sum_sorted(&[min, -1], min), None);
        assert_eq!(two_sum_sorted(&[min, -4, -1, i32::MAX], -5), Some((1, 2)));
        // The two extremes add up to -1, which fits.
        assert_eq!(two_sum_sorted(&[min, i32::MAX], -1), Some((0, 1)));
    }

    #[test]
    fn two_sum_matches_a_brute_force_model() {
        // Values and targets are drawn from the edges of `i32` as well as the
        // middle, so most pairs overflow an `i32` sum.
        let (min, max) = (i32::MIN, i32::MAX);
        let pool = [min, min + 1, -3, -1, 0, 1, 2, 3, max - 1, max];
        let targets = [min, -4, -1, 0, 1, 3, max - 2, max];
        let mut state = 7;
        for _ in 0..2_000 {
            let len = (next(&mut state) % 7) as usize;
            let mut nums: Vec<i32> = (0..len)
                .map(|_| pool[(next(&mut state) as usize) % pool.len()])
                .collect();
            nums.sort_unstable();
            let target = targets[(next(&mut state) as usize) % targets.len()];
            let exists = nums.iter().enumerate().any(|(i, &a)| {
                nums[i + 1..]
                    .iter()
                    .any(|&b| i64::from(a) + i64::from(b) == i64::from(target))
            });
            let got = two_sum_sorted(&nums, target);
            if exists {
                assert_valid_pair(&nums, target, got);
            } else {
                assert_eq!(got, None, "{nums:?} has no pair adding up to {target}");
            }
        }
    }

    // ---- three_sum ----

    #[test]
    fn three_sum_classic_example() {
        assert_eq!(
            three_sum(&[-1, 0, 1, 2, -1, -4]),
            vec![[-1, -1, 2], [-1, 0, 1]]
        );
    }

    #[test]
    fn three_sum_reports_each_triple_once() {
        assert_eq!(three_sum(&[0, 0, 0]), vec![[0, 0, 0]]);
        assert_eq!(three_sum(&[0, 0, 0, 0, 0, 0]), vec![[0, 0, 0]]);
        assert_eq!(
            three_sum(&[2, -2, 0, 2, -2, 0, 0]),
            vec![[-2, 0, 2], [0, 0, 0]]
        );
    }

    #[test]
    fn three_sum_small_inputs_have_no_triple() {
        let none: Vec<[i32; 3]> = Vec::new();
        assert_eq!(three_sum(&[]), none);
        assert_eq!(three_sum(&[0]), none);
        assert_eq!(three_sum(&[1, -1]), none);
        assert_eq!(three_sum(&[0, 1, 1]), none);
    }

    #[test]
    fn three_sum_is_exact_at_the_extremes() {
        let (min, max) = (i32::MIN, i32::MAX);
        // `i32::MIN + 1 + i32::MAX` is 0, but most other sums here overflow.
        assert_eq!(
            three_sum(&[max, min, 1, 0, -1, max]),
            vec![[min, 1, max], [-1, 0, 1]]
        );
        let none: Vec<[i32; 3]> = Vec::new();
        assert_eq!(three_sum(&[min, min, max, max]), none);
        // These add up to -2^32 and 2^32. Both wrap around to exactly 0 in an
        // `i32`, but neither is 0.
        assert_eq!(three_sum(&[min, min, 0]), none);
        assert_eq!(three_sum(&[max, 2, max]), none);
    }

    #[test]
    fn three_sum_matches_a_brute_force_model() {
        let mut state = 42;
        for _ in 0..1_000 {
            let len = (next(&mut state) % 11) as usize;
            let nums: Vec<i32> = (0..len)
                .map(|_| (next(&mut state) % 9) as i32 - 4)
                .collect();
            // Every combination of three positions; the set sorts and dedupes.
            let mut expected = BTreeSet::new();
            for (a, &x) in nums.iter().enumerate() {
                for (b, &y) in nums.iter().enumerate().skip(a + 1) {
                    for &z in &nums[b + 1..] {
                        if x + y + z == 0 {
                            let mut triple = [x, y, z];
                            triple.sort_unstable();
                            expected.insert(triple);
                        }
                    }
                }
            }
            let expected: Vec<[i32; 3]> = expected.into_iter().collect();
            assert_eq!(three_sum(&nums), expected, "input {nums:?}");
        }
    }
}
