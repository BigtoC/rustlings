// Traits & Abstraction · Trait contracts — part 3: `f64` is not `Ord`: `total_cmp`, a totally ordered key, a min-heap (E0277, E0599).
//
// `f64` implements `PartialEq` and `PartialOrd`, and deliberately NOT `Eq`,
// `Ord` or `Hash`. IEEE 754 arithmetic breaks the contracts from parts 1 and 2:
//
//   - `NaN == NaN` is false, but `Eq` promises that every value equals itself;
//   - `NaN < x`, `NaN > x` and `NaN == x` are all false, so `partial_cmp`
//     returns `None`: there is no answer, and `Ord` must always answer;
//   - `-0.0 == 0.0` is true, although the two are different values
//     (`1.0 / -0.0` is negative infinity). A `Hash` that agreed with that `==`
//     could not simply hash the bits.
//
// So every API that needs a total order refuses floats at COMPILE time:
// `sort`, `BinaryHeap`, `BTreeMap` keys, `Iterator::max`, `binary_search`. The
// error is E0277 "the trait bound `f64: Ord` is not satisfied". The tempting
// ways around it are both wrong. `a.partial_cmp(b).unwrap()` panics on the
// first NaN. `.unwrap_or(Ordering::Equal)` makes NaN "equal" to everything,
// which is not transitive (1 = NaN and NaN = 2, but 1 < 2): `sort` may then
// return an unspecified order, and since Rust 1.81 it may even panic.
//
// `f64::total_cmp` (stable since 1.62) is the real answer. It implements the
// `totalOrder` predicate of IEEE 754-2008:
//
//     -NaN < -inf < -1.0 < -0.0 < 0.0 < 1.0 < inf < NaN
//
// It compares the BIT PATTERNS as signed integers (after flipping all bits
// but the sign of negative numbers), so two values are equal under it exactly
// when their bits are equal: NaN equals itself, and -0.0 comes before 0.0.
// Note the NEGATIVE NaN at the front. A NaN's sign bit is not something to
// rely on: the bit pattern of `f64::NAN` is not guaranteed, and on x86
// `0.0 / 0.0` computed at run time gives a negative NaN, which `total_cmp`
// sorts FIRST. The tests below build their NaNs from explicit bits.
//
// To put floats into ordered or hashed collections, wrap them. The orphan rule
// forbids `impl Ord for f64` in your crate (E0117, see `42_coherence`), so the
// wrapper is a newtype whose `Eq`, `Ord` and `Hash` all follow the same total
// order and therefore agree with each other. (In production code the
// `ordered-float` crate's `OrderedFloat` and `NotNan` do this job, with
// slightly different rules for NaN and zeros.) One more classic finishes the
// module. `BinaryHeap` is a MAX-heap, and Dijkstra's algorithm needs the
// CHEAPEST state first. The standard way to flip a heap is `std::cmp::Reverse`,
// whose `Ord` is the reverse of its content's.
//
// Interviewers ask: "Why doesn't `Vec<f64>::sort()` compile?", "How do you
// sort floats that may contain NaN?", "Can an `f64` be a `HashMap` key?" and
// "How do you put f64-keyed items into a `BinaryHeap` as a min-heap?".

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::hash::{Hash, Hasher};

// Part A: sorts `values` smallest first under IEEE 754's total order (the
// order shown above).
fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    // `sort_by` takes the comparison as a function, and `f64::total_cmp` is
    // one with exactly the right signature, `fn(&f64, &f64) -> Ordering`. It
    // is a genuine total order (it compares bit patterns), so the sort is
    // well defined: NaNs are kept and placed by their sign, and -0.0 comes
    // before 0.0. The sort is still in place, in `values`' own buffer.
    values.sort_by(f64::total_cmp);
    values
}

// Part B: a float that IS totally ordered, so it can be a key in a heap, a
// `BTreeSet` or a `HashSet`.
//
// Only `Debug`, `Clone` and `Copy` are derived. Every comparison trait is
// written by hand below and follows `f64::total_cmp`, so they all describe
// ONE order and cannot disagree. `total_cmp` returns `Equal` exactly when the
// bit patterns are equal, so `==` compares bits, `Eq`'s reflexivity holds
// (a NaN equals itself), and hashing the bits agrees with `==`.
#[derive(Debug, Clone, Copy)]
struct TotalF64(f64);

impl PartialEq for TotalF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0).is_eq()
    }
}

// `==` is now reflexive, symmetric and transitive: an equivalence.
impl Eq for TotalF64 {}

impl PartialOrd for TotalF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TotalF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for TotalF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Equal under `total_cmp` means identical bits, so equal keys feed
        // identical bytes. (f64's own `==` could not hash like this: it calls
        // -0.0 and 0.0 equal although their bits differ.)
        self.0.to_bits().hash(state);
    }
}

// Part C: the `k` smallest values of `values`, smallest first, in the same
// total order as `sorted`. The heap keeps at most `k` values. It is a
// max-heap, so its root is the LARGEST value kept so far, which is the one to
// evict when a new value arrives: O(n log k) time and O(k) extra memory,
// instead of sorting all n values.
fn k_smallest(values: &[f64], k: usize) -> Vec<f64> {
    // The same bounded max-heap, now of `TotalF64`s: they are `Ord`, and in
    // the same order `sorted` uses, so the root is always the largest value
    // kept so far (a positive NaN is larger than everything). A new value
    // goes in and the largest of the k + 1 comes out. `into_sorted_vec`
    // returns the survivors in ascending order, and unwrapping each one gives
    // back the original `f64`, bits and all.
    let mut heap = BinaryHeap::new();
    for &value in values {
        heap.push(TotalF64(value));
        if heap.len() > k {
            heap.pop();
        }
    }
    heap.into_sorted_vec()
        .into_iter()
        .map(|TotalF64(value)| value)
        .collect()
}

// Part D: Dijkstra's shortest path with `f64` weights.
//
// A search state: the cheapest known cost of reaching `node`.
//
// With the cost stored as a `TotalF64`, every field is `Eq + Ord`, so all
// four traits can be derived and agree with each other. A derived order is
// lexicographic in declaration order: `cost` decides, and `node` breaks ties,
// so two different states are never `Equal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct State {
    cost: TotalF64,
    node: usize,
}

// The cheapest total weight of a path from `start` to `goal`, or `None` if
// `goal` cannot be reached. `graph[u]` lists the edges leaving `u` as
// `(v, weight)` pairs. Weights are finite and not negative.
fn shortest_path(graph: &[Vec<(usize, f64)>], start: usize, goal: usize) -> Option<f64> {
    // `Reverse` flips the order of whatever it wraps, so the max-heap of
    // `Reverse<State>` pops the state with the SMALLEST cost first. That is
    // the order Dijkstra's algorithm needs: when `goal` is popped, no cheaper
    // path to it can still be waiting in the heap, so the early return is
    // correct. (Negating the costs would flip the order too, but it hides the
    // intent, and it does not work for unsigned integer costs.)
    let mut dist = vec![f64::INFINITY; graph.len()];
    let mut heap = BinaryHeap::new();
    dist[start] = 0.0;
    heap.push(Reverse(State {
        cost: TotalF64(0.0),
        node: start,
    }));
    while let Some(Reverse(State {
        cost: TotalF64(cost),
        node,
    })) = heap.pop()
    {
        if node == goal {
            return Some(cost);
        }
        // A stale entry: a cheaper path to `node` was found after this state
        // was pushed, and that path has been (or will be) expanded instead.
        if cost > dist[node] {
            continue;
        }
        for &(next, weight) in &graph[node] {
            let next_cost = cost + weight;
            if next_cost < dist[next] {
                dist[next] = next_cost;
                heap.push(Reverse(State {
                    cost: TotalF64(next_cost),
                    node: next,
                }));
            }
        }
    }
    None
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    use std::collections::{BTreeSet, HashSet};
    use std::hash::{DefaultHasher, Hash, Hasher};

    // Quiet NaNs with an explicit sign. `f64::NAN`'s bits are not guaranteed.
    const NAN: f64 = f64::from_bits(0x7ff8_0000_0000_0000);
    const NEG_NAN: f64 = f64::from_bits(0xfff8_0000_0000_0000);
    const INF: f64 = f64::INFINITY;
    const NEG_INF: f64 = f64::NEG_INFINITY;

    // Floats compared by bit pattern: tells -0.0 from 0.0, and NaN == NaN.
    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    // ---- Part A: `sorted` ----

    #[test]
    fn sorted_follows_the_total_order() {
        let input = vec![1.0, NAN, -0.0, 0.0, NEG_INF, -1.0, INF];
        let expected = [NEG_INF, -1.0, -0.0, 0.0, 1.0, INF, NAN];
        assert_eq!(bits(&sorted(input)), bits(&expected));
    }

    #[test]
    fn sorted_puts_negative_zero_before_zero() {
        assert_eq!(bits(&sorted(vec![0.0, -0.0])), bits(&[-0.0, 0.0]));
        assert_eq!(
            bits(&sorted(vec![0.0, -0.0, 0.0, -0.0])),
            bits(&[-0.0, -0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn sorted_puts_a_negative_nan_first() {
        let input = vec![1.0, NAN, NEG_NAN, -1.0, NEG_INF];
        let expected = [NEG_NAN, NEG_INF, -1.0, 1.0, NAN];
        assert_eq!(bits(&sorted(input)), bits(&expected));
    }

    #[test]
    fn sorted_handles_trivial_inputs() {
        assert!(sorted(Vec::new()).is_empty());
        assert_eq!(bits(&sorted(vec![NAN])), bits(&[NAN]));
        assert_eq!(sorted(vec![3.5, -2.25, 3.5]), [-2.25, 3.5, 3.5]);
    }

    #[test]
    fn sorted_sorts_in_place() {
        let values = vec![3.0, 1.0, 2.0];
        let buffer = values.as_ptr();
        let result = sorted(values);
        assert_eq!(result, [1.0, 2.0, 3.0]);
        assert_eq!(
            result.as_ptr(),
            buffer,
            "the result must reuse the input Vec"
        );
    }

    // ---- Part B: `TotalF64` ----

    #[test]
    fn total_f64_equality_is_bitwise() {
        assert_eq!(TotalF64(NAN), TotalF64(NAN));
        assert_ne!(TotalF64(-0.0), TotalF64(0.0));
        assert_ne!(TotalF64(NAN), TotalF64(NEG_NAN));
        assert_eq!(TotalF64(1.5), TotalF64(1.5));
        assert_ne!(TotalF64(1.5), TotalF64(2.5));
    }

    #[test]
    fn total_f64_order_is_total() {
        assert!(TotalF64(-0.0) < TotalF64(0.0));
        assert!(TotalF64(NAN) > TotalF64(INF));
        assert!(TotalF64(NEG_NAN) < TotalF64(NEG_INF));
        assert_eq!(
            TotalF64(NAN).partial_cmp(&TotalF64(1.0)),
            Some(Ordering::Greater)
        );
        assert_eq!(TotalF64(NAN).cmp(&TotalF64(NAN)), Ordering::Equal);
        let max = [2.0, NAN, INF, -1.0].map(TotalF64).into_iter().max();
        assert_eq!(max.map(|t| t.0.to_bits()), Some(NAN.to_bits()));
    }

    #[test]
    fn total_f64_traits_agree_with_each_other() {
        let values = [NEG_NAN, NEG_INF, -1.0, -0.0, 0.0, 1e-300, 1.0, INF, NAN];
        for &a in &values {
            for &b in &values {
                let (x, y) = (TotalF64(a), TotalF64(b));
                let ord = x.cmp(&y);
                assert_eq!(ord, a.total_cmp(&b), "{a:?} vs {b:?}");
                assert_eq!(ord == Ordering::Equal, x == y, "{a:?} vs {b:?}");
                assert_eq!(x.partial_cmp(&y), Some(ord), "{a:?} vs {b:?}");
                assert_eq!(x < y, ord == Ordering::Less, "{a:?} < {b:?}");
                if x == y {
                    assert_eq!(hash_of(&x), hash_of(&y), "{a:?} vs {b:?}");
                }
            }
        }
    }

    #[test]
    fn total_f64_hash_keeps_different_keys_apart() {
        // Unequal keys MAY share a hash, but a hash that throws information
        // away (a constant, or one that merges -0.0 and 0.0) turns a
        // `HashSet` into a list. Nine different keys, nine different hashes.
        let values = [NEG_NAN, NEG_INF, -1.0, -0.0, 0.0, 1e-300, 1.0, INF, NAN];
        let hashes: HashSet<u64> = values
            .iter()
            .map(|&value| hash_of(&TotalF64(value)))
            .collect();
        assert_eq!(hashes.len(), values.len());
    }

    #[test]
    fn total_f64_is_a_hash_key() {
        let set: HashSet<TotalF64> = [0.0, -0.0, NAN, NAN, 1.0, 1.0, INF]
            .into_iter()
            .map(TotalF64)
            .collect();
        assert_eq!(set.len(), 5);
        assert!(set.contains(&TotalF64(NAN)));
        assert!(set.contains(&TotalF64(-0.0)));
        assert!(!set.contains(&TotalF64(NEG_NAN)));
    }

    #[test]
    fn total_f64_is_a_btree_key() {
        let set: BTreeSet<TotalF64> = [3.0, NAN, -0.0, 0.0, NAN, -7.5]
            .into_iter()
            .map(TotalF64)
            .collect();
        let in_order: Vec<f64> = set.iter().map(|t| t.0).collect();
        assert_eq!(bits(&in_order), bits(&[-7.5, -0.0, 0.0, 3.0, NAN]));
    }

    // ---- Part C: `k_smallest` ----

    #[test]
    fn k_smallest_returns_the_smallest_first() {
        assert_eq!(k_smallest(&[5.0, 1.0, 4.0, 2.0], 2), [1.0, 2.0]);
        assert_eq!(k_smallest(&[2.0, 2.0, 1.0], 2), [1.0, 2.0]);
        assert_eq!(k_smallest(&[-3.5, 8.0, 0.25], 1), [-3.5]);
    }

    #[test]
    fn k_smallest_handles_edge_cases() {
        assert!(k_smallest(&[5.0, 1.0], 0).is_empty());
        assert!(k_smallest(&[], 3).is_empty());
        assert_eq!(k_smallest(&[3.0, 1.0], 5), [1.0, 3.0]);
        assert_eq!(k_smallest(&[3.0, 1.0], usize::MAX), [1.0, 3.0]);
    }

    #[test]
    fn k_smallest_ranks_nan_and_zeros_like_sorted() {
        assert_eq!(k_smallest(&[NAN, 3.0, -1.0, NAN], 2), [-1.0, 3.0]);
        assert_eq!(
            bits(&k_smallest(&[NAN, 3.0, -1.0], 3)),
            bits(&[-1.0, 3.0, NAN])
        );
        assert_eq!(bits(&k_smallest(&[0.0, -0.0], 1)), bits(&[-0.0]));
        assert_eq!(bits(&k_smallest(&[1.0, NEG_NAN], 1)), bits(&[NEG_NAN]));
    }

    #[test]
    fn k_smallest_matches_sorting_everything() {
        // A deterministic pseudo-random input with duplicates and both signs.
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        let values: Vec<f64> = (0..500)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x % 2001) as f64 / 8.0 - 125.0
            })
            .collect();
        let all = sorted(values.clone());
        for k in [1, 7, 100, 499, 500] {
            assert_eq!(bits(&k_smallest(&values, k)), bits(&all[..k]), "k = {k}");
        }
    }

    // ---- Part D: `shortest_path` ----

    // An undirected graph with `n` nodes.
    fn graph(n: usize, edges: &[(usize, usize, f64)]) -> Vec<Vec<(usize, f64)>> {
        let mut adj = vec![Vec::new(); n];
        for &(u, v, w) in edges {
            adj[u].push((v, w));
            adj[v].push((u, w));
        }
        adj
    }

    // Bellman-Ford: slow, but needs no heap at all.
    fn reference(graph: &[Vec<(usize, f64)>], start: usize, goal: usize) -> Option<f64> {
        let mut dist = vec![INF; graph.len()];
        dist[start] = 0.0;
        for _ in 0..graph.len() {
            for u in 0..graph.len() {
                for &(v, w) in &graph[u] {
                    if dist[u] + w < dist[v] {
                        dist[v] = dist[u] + w;
                    }
                }
            }
        }
        (dist[goal] < INF).then_some(dist[goal])
    }

    #[test]
    fn the_direct_edge_is_not_the_cheapest_path() {
        // 0 -> 3 directly costs 10; the detour 0 -> 1 -> 2 -> 3 costs 3.
        let g = graph(4, &[(0, 3, 10.0), (0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)]);
        assert_eq!(shortest_path(&g, 0, 3), Some(3.0));
        assert_eq!(shortest_path(&g, 3, 0), Some(3.0));
    }

    #[test]
    fn a_cheaper_path_found_later_wins() {
        // 1 is first reached for 4.0, then for 0.5 + 0.25 = 0.75 via 2.
        let g = graph(4, &[(0, 1, 4.0), (0, 2, 0.5), (2, 1, 0.25), (1, 3, 1.0)]);
        assert_eq!(shortest_path(&g, 0, 1), Some(0.75));
        assert_eq!(shortest_path(&g, 0, 3), Some(1.75));
    }

    #[test]
    fn trivial_and_unreachable_goals() {
        let g = graph(4, &[(0, 1, 2.5)]);
        assert_eq!(shortest_path(&g, 0, 0), Some(0.0));
        assert_eq!(shortest_path(&g, 0, 1), Some(2.5));
        assert_eq!(shortest_path(&g, 0, 3), None);
        assert_eq!(shortest_path(&g, 2, 3), None);
        let zero = graph(3, &[(0, 1, 0.0), (1, 2, 0.0)]);
        assert_eq!(shortest_path(&zero, 0, 2), Some(0.0));
    }

    #[test]
    fn matches_bellman_ford_on_a_grid() {
        // A 6 x 6 grid with uneven weights, so many paths compete.
        let side = 6;
        let mut edges = Vec::new();
        for r in 0..side {
            for c in 0..side {
                let u = r * side + c;
                let w = ((r * 7 + c * 3) % 5) as f64 + 0.5;
                if c + 1 < side {
                    edges.push((u, u + 1, w));
                }
                if r + 1 < side {
                    edges.push((u, u + side, 5.5 - w));
                }
            }
        }
        let g = graph(side * side, &edges);
        for goal in 0..side * side {
            assert_eq!(
                shortest_path(&g, 0, goal),
                reference(&g, 0, goal),
                "goal {goal}"
            );
        }
    }
}
