// Module 5 · Slices and strings — part 3: merging intervals with a sort, `last_mut` and a let chain (E0308).
//
// "Merge overlapping intervals" (LeetCode 56) is the standard sort-then-sweep
// question. Sort the intervals by their start. Then walk them once, keeping
// the merged list built so far: an interval that starts at or before the end
// of the LAST merged interval overlaps it (sorting guarantees it cannot reach
// back to any earlier one), so it extends that last interval; anything else
// starts a new one. O(n log n) for the sort plus O(n) for the sweep.
//
// Two details decide whether the answer is right, and interviewers ask about
// both:
//
//   - Closed or half-open? These intervals are CLOSED, `(start, end)` means
//     start..=end, so `(1, 3)` and `(3, 5)` touch at 3 and merge. Compare with
//     `<=`, not `<`. (Even with integer endpoints, `(1, 2)` and `(3, 4)` do
//     NOT merge: they stand for ranges of real numbers, and 2.5 is in
//     neither.)
//   - Extend with the max. A later interval can sit entirely inside the last
//     one, like `(2, 3)` after `(1, 10)`: the new end is `max(last.end, end)`,
//     not `end`.
//
// The Rust part is how you extend "the last merged interval" in place.
// `out.last_mut()` returns `Option<&mut (i32, i32)>`: `None` for an empty
// list, and otherwise a `&mut` you can write through. The natural shape,
// "if there is a last interval AND it overlaps, extend it, otherwise push",
// is exactly an edition 2024 let chain:
//
//     if let Some(last) = out.last_mut() && cond(last) { .. } else { .. }
//
// Let chains are stable since Rust 1.88, in edition 2024 only. Before that
// you wrote a `match` with a guard (`Some(last) if cond(last) => ..`) or two
// nested `if`s with the push duplicated. Pushing in the `else` branch
// compiles: `last` is not used there, so its borrow of `out` is already over
// (non-lexical lifetimes). `37_borrowck_errors/borrowck3` shows when such a
// borrow does NOT end in time: when one path returns it.
//
// Which sort? `sort_unstable_by_key(|&(start, _)| start)` sorts in place and
// never allocates. The stable `sort_by_key` allocates a scratch buffer for
// anything but short slices, and stability buys nothing here: intervals with
// equal starts all merge into one anyway.
//
// How interviewers probe this: "Why is sorting by start enough?", "Do (1, 3)
// and (3, 5) merge?", "What about an interval inside another one?", and
// "Can you do it without a second `Vec`?" (see the hint).

// Merges overlapping CLOSED intervals `(start, end)`, each with
// `start <= end`, and returns the result sorted by start. Intervals that
// share an endpoint touch, and touching intervals merge: `(1, 3)` and
// `(3, 5)` become `(1, 5)`. The input may be in any order.
fn merge_intervals(intervals: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    // TODO: rustc rejects this with E0308 "mismatched types" (expected
    // `Vec<(i32, i32)>`, found `()`): the body is empty. rustc's "consider
    // returning the local binding `intervals`" compiles but fails the tests.
    // Implement the sort-then-sweep merge described above. Requirements:
    // O(n log n) time; closed intervals, so touching ones merge; an interval
    // inside another one must not shrink it; the output is sorted by start
    // and no two output intervals overlap or touch; an empty input gives an
    // empty output. Extend the last merged interval in place (`last_mut()`)
    // rather than popping and re-pushing it. No `unsafe`. Until you return
    // the merged `Vec`, this exercise will not compile.
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // A slow but obviously correct model: keep merging any two intervals that
    // overlap or touch until none do, then sort.
    fn model(mut v: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
        let mut changed = true;
        while changed {
            changed = false;
            'search: for a in 0..v.len() {
                for b in a + 1..v.len() {
                    let ((s1, e1), (s2, e2)) = (v[a], v[b]);
                    if s1 <= e2 && s2 <= e1 {
                        v[a] = (s1.min(s2), e1.max(e2));
                        v.swap_remove(b);
                        changed = true;
                        break 'search;
                    }
                }
            }
        }
        v.sort_unstable();
        v
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
    fn merges_the_classic_example() {
        assert_eq!(
            merge_intervals(vec![(1, 3), (2, 6), (8, 10), (15, 18)]),
            vec![(1, 6), (8, 10), (15, 18)]
        );
    }

    #[test]
    fn unsorted_input_is_sorted_first() {
        assert_eq!(
            merge_intervals(vec![(15, 18), (2, 6), (8, 10), (1, 3)]),
            vec![(1, 6), (8, 10), (15, 18)]
        );
        // Sorted by END, (2, 3) would come first and (1, 10) would then be
        // merged into it as (2, 10).
        assert_eq!(merge_intervals(vec![(1, 10), (2, 3)]), vec![(1, 10)]);
    }

    #[test]
    fn touching_intervals_merge() {
        assert_eq!(merge_intervals(vec![(1, 4), (4, 5)]), vec![(1, 5)]);
        assert_eq!(merge_intervals(vec![(3, 5), (1, 3)]), vec![(1, 5)]);
        assert_eq!(
            merge_intervals(vec![(0, 0), (0, 1), (1, 1), (1, 2)]),
            vec![(0, 2)]
        );
    }

    #[test]
    fn neighbors_with_a_gap_stay_apart() {
        assert_eq!(merge_intervals(vec![(3, 4), (1, 2)]), vec![(1, 2), (3, 4)]);
        assert_eq!(
            merge_intervals(vec![(5, 5), (1, 1), (3, 3)]),
            vec![(1, 1), (3, 3), (5, 5)]
        );
    }

    #[test]
    fn a_contained_interval_does_not_shrink_the_outer_one() {
        assert_eq!(merge_intervals(vec![(2, 3), (1, 10)]), vec![(1, 10)]);
        assert_eq!(
            merge_intervals(vec![(1, 10), (2, 3), (4, 5), (9, 12)]),
            vec![(1, 12)]
        );
    }

    #[test]
    fn equal_starts_merge_whatever_their_order() {
        // An unstable sort may put these three in any order: the merge must
        // not depend on it.
        assert_eq!(merge_intervals(vec![(1, 2), (1, 5), (1, 3)]), vec![(1, 5)]);
        assert_eq!(merge_intervals(vec![(1, 5), (1, 2), (1, 3)]), vec![(1, 5)]);
    }

    #[test]
    fn empty_and_single_inputs() {
        assert_eq!(merge_intervals(Vec::new()), Vec::new());
        assert_eq!(merge_intervals(vec![(7, 9)]), vec![(7, 9)]);
    }

    #[test]
    fn negative_and_extreme_endpoints() {
        let (min, max) = (i32::MIN, i32::MAX);
        assert_eq!(
            merge_intervals(vec![(max, max), (-5, -1), (min, -10), (-5, -5), (0, 0)]),
            vec![(min, -10), (-5, -1), (0, 0), (max, max)]
        );
        assert_eq!(merge_intervals(vec![(0, max), (min, 0)]), vec![(min, max)]);
    }

    #[test]
    fn a_long_chain_collapses_into_one_interval() {
        // (99, 100), (98, 99), ..., (0, 1): every neighbor touches.
        let chain: Vec<(i32, i32)> = (0..100).rev().map(|i| (i, i + 1)).collect();
        assert_eq!(merge_intervals(chain), vec![(0, 100)]);
    }

    #[test]
    fn matches_a_brute_force_model() {
        let mut state = 56;
        for _ in 0..2_000 {
            let len = next(&mut state) % 9;
            let intervals: Vec<(i32, i32)> = (0..len)
                .map(|_| {
                    let start = (next(&mut state) % 21) as i32 - 10;
                    let width = (next(&mut state) % 5) as i32;
                    (start, start + width)
                })
                .collect();
            let expected = model(intervals.clone());
            assert_eq!(
                merge_intervals(intervals.clone()),
                expected,
                "input {intervals:?}"
            );
        }
    }
}
