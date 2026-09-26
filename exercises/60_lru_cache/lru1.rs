// Module 2 · LRU cache — part 1: a warm-up with a tick-ordered `BTreeMap` as the recency list (E0308).
//
// "Implement an LRU cache" (LeetCode 146) is one of the most-asked live-coding
// problems in Rust interviews. A least-recently-used cache holds at most
// `capacity` entries. `get` returns a value and counts as a USE of its key;
// `put` inserts or updates a key, which is a use too. When a NEW key arrives
// at a full cache, the entry whose last use is the oldest is EVICTED to make
// room. So besides "key -> value" the cache needs a second index, "which key
// is the least recently used", and every hit must update it.
//
// The textbook answer is a hash map plus a doubly linked list, and part 2
// builds it. This warm-up is the version to write first in an interview,
// because it is short and has nothing that can dangle: stamp every use with a
// TICK from a counter, and keep a second, ordered map from tick to key.
//
//   map:   HashMap<K, (V, u64)>    key  -> (value, tick of its last use)
//   order: BTreeMap<u64, K>        tick -> key, oldest tick first
//
// Ticks only grow, so the least recently used key is `order`'s FIRST entry,
// and `BTreeMap::pop_first` (stable since Rust 1.66) removes it and hands it
// back in O(log n). Every operation is O(1) hash-map work plus O(log n) in
// the tree: no links, no `unsafe`, no `Rc<RefCell<..>>`. When the interviewer
// then asks "can you make it O(1)?", you go to part 2.
//
// The trap is keeping the two maps in step. Every use (a `get` hit, or a `put`
// that updates a key) must take the key's OLD tick out of `order` when it
// records the new one. Forget that, and `order` fills up with stale ticks: it
// grows on every hit, and `pop_first` hands you a stale tick whose key was
// used a moment ago, so the cache evicts a hot entry and keeps a cold one.
// The tests check that both maps hold the same keys after every operation.
//
// Why does `get` take `&mut self`? Because a hit is a WRITE: it changes the
// recency order. That is the follow-up question once the code works, and it
// has consequences. You cannot `get` through a `&LruCache`. A cache shared
// between threads goes behind a `Mutex`, and a `RwLock` buys nothing when
// every read is a write. Hiding the write behind `RefCell` would give you a
// `get(&self)`, but a cache that is not `Sync`. (The README has more.)
//
// Two smaller points. The key is stored in both maps, hence `K: Clone`. And a
// `u64` tick cannot run out in practice: at a billion uses per second it
// takes about 585 years.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

struct LruCache<K, V> {
    capacity: usize,
    // key -> (value, the tick of the key's last use)
    map: HashMap<K, (V, u64)>,
    // tick -> key. The first entry is the least recently used key and the
    // last one the most recently used.
    order: BTreeMap<u64, K>,
    // The tick that the next use gets.
    next_tick: u64,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "the capacity must be at least 1");
        LruCache {
            capacity,
            map: HashMap::new(),
            order: BTreeMap::new(),
            next_tick: 0,
        }
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    // Hands out a new tick, larger than every tick handed out before.
    fn tick(&mut self) -> u64 {
        let tick = self.next_tick;
        self.next_tick += 1;
        tick
    }

    // Returns the value stored under `key` and makes `key` the most recently
    // used. A miss returns `None` and leaves the entries and their order
    // alone.
    fn get(&mut self, key: &K) -> Option<&V> {
        // TODO: The empty body is rejected with E0308 "mismatched types":
        // expected `Option<&V>`, found `()`. On a hit, return the stored value
        // and record the use. Requirements:
        //   - a hit gives the key a NEW tick, and its OLD tick leaves `order`
        //     (the tests check that `map` and `order` agree after every step);
        //   - a miss returns `None`;
        //   - O(log n): one hash lookup and one tick moved in `order`; no
        //     scanning either map for the key or for its tick (a test counts
        //     the key comparisons);
        //   - the tests' values are not `Clone`, so return a reference into
        //     the cache. No `unsafe`, no `RefCell`, and don't change the
        //     struct, the signatures or the tests.
        // Until you return an `Option<&V>`, this exercise will not compile.
    }

    // Stores `value` under `key` and makes `key` the most recently used. If
    // `key` is already cached, its value is replaced and nothing is evicted.
    // A NEW key in a full cache first evicts the least recently used entry,
    // which is returned to the caller.
    fn put(&mut self, key: K, value: V) -> Option<(K, V)> {
        // TODO: E0308 again: expected `Option<(K, V)>`, found `()`.
        // Requirements:
        //   - an existing key gets the new value and a new tick (its old tick
        //     leaves `order`), and nothing is evicted, even when the cache is
        //     full;
        //   - a new key in a full cache first removes the entry with the
        //     SMALLEST tick from both maps and returns it as
        //     `Some((key, value))`, in O(log n): `order` already knows which
        //     tick is the smallest, so don't search `map` for it;
        //   - otherwise the new key is stored with a new tick and `put`
        //     returns `None`. The cache never holds more than `capacity`
        //     entries.
        // Until you return an `Option<(K, V)>`, this exercise will not
        // compile.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fmt::Debug;
    use std::hash::Hasher;

    // Deliberately not `Clone`: the cache has to hand out references to the
    // values it stores, and give an evicted value back by moving it.
    #[derive(Debug, PartialEq)]
    struct Token(String);

    fn token(name: &str) -> Token {
        Token(name.to_string())
    }

    // The cached keys from most to least recently used, after checking that
    // the two maps agree: every key in `map` has exactly one tick in `order`,
    // and it is the tick that `map` stores for it.
    fn recency<K: Hash + Eq + Clone + Debug, V>(cache: &LruCache<K, V>) -> Vec<K> {
        assert_eq!(
            cache.map.len(),
            cache.order.len(),
            "`map` and `order` hold different numbers of keys: a stale tick left in `order`?"
        );
        assert!(
            cache.map.len() <= cache.capacity,
            "the cache holds more than `capacity` entries"
        );
        for (tick, key) in &cache.order {
            match cache.map.get(key) {
                Some((_, last_use)) => {
                    assert_eq!(last_use, tick, "{key:?} is in `order` under a stale tick");
                }
                None => panic!("`order` holds {key:?}, which is not in `map`"),
            }
        }
        cache.order.values().rev().cloned().collect()
    }

    #[test]
    fn leetcode_146_example() {
        let mut cache = LruCache::new(2);
        assert_eq!(cache.put(1, 1), None);
        assert_eq!(cache.put(2, 2), None);
        assert_eq!(cache.get(&1), Some(&1));
        // The `get` made 1 the most recently used, so 3 pushes out 2.
        assert_eq!(cache.put(3, 3), Some((2, 2)));
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.put(4, 4), Some((1, 1)));
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&3), Some(&3));
        assert_eq!(cache.get(&4), Some(&4));
        assert_eq!(recency(&cache), [4, 3]);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn a_hit_refreshes_its_key_and_a_miss_changes_nothing() {
        let mut cache = LruCache::new(3);
        for name in ["a", "b", "c"] {
            assert!(cache.put(name, token(name)).is_none());
        }
        assert_eq!(recency(&cache), ["c", "b", "a"]);

        // "a" was the next to go. A hit makes it the most recently used.
        assert_eq!(cache.get(&"a"), Some(&token("a")));
        assert_eq!(recency(&cache), ["a", "c", "b"]);
        // Hitting the most recently used key again keeps it in front.
        assert_eq!(cache.get(&"a"), Some(&token("a")));
        assert_eq!(recency(&cache), ["a", "c", "b"]);

        // A miss neither inserts anything nor reorders the entries.
        assert_eq!(cache.get(&"zzz"), None);
        assert_eq!(recency(&cache), ["a", "c", "b"]);

        // So the next new key evicts "b", the least recently USED key, not
        // "a", the least recently INSERTED one.
        assert_eq!(cache.put("d", token("d")), Some(("b", token("b"))));
        assert_eq!(recency(&cache), ["d", "a", "c"]);
    }

    #[test]
    fn updating_a_key_replaces_its_value_promotes_it_and_never_evicts() {
        let mut cache = LruCache::new(3);
        assert!(cache.put("a", token("a1")).is_none());
        assert!(cache.put("b", token("b1")).is_none());
        assert!(cache.put("c", token("c1")).is_none());

        // The cache is full, but these keys are already in it: updates, not
        // inserts, so nothing may be evicted. First a key from the middle of
        // the order, then the least recently used one.
        assert_eq!(cache.put("b", token("b2")), None);
        assert_eq!(recency(&cache), ["b", "c", "a"]);
        assert_eq!(cache.put("a", token("a2")), None);
        assert_eq!(recency(&cache), ["a", "b", "c"]);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"b"), Some(&token("b2")));
        assert_eq!(cache.get(&"a"), Some(&token("a2")));

        // The updates counted as uses, so "c" goes first.
        assert_eq!(cache.put("d", token("d1")), Some(("c", token("c1"))));
        assert_eq!(cache.get(&"c"), None);
        assert_eq!(recency(&cache), ["d", "a", "b"]);
    }

    #[test]
    fn capacity_one_keeps_only_the_latest_key() {
        let mut cache = LruCache::new(1);
        assert_eq!(cache.put(7, token("seven")), None);
        // Updating the only key is not an eviction.
        assert_eq!(cache.put(7, token("SEVEN")), None);
        assert_eq!(cache.get(&7), Some(&token("SEVEN")));

        assert_eq!(cache.put(8, token("eight")), Some((7, token("SEVEN"))));
        assert_eq!(cache.get(&7), None);
        assert_eq!(cache.get(&8), Some(&token("eight")));
        assert_eq!(cache.put(9, token("nine")), Some((8, token("eight"))));
        assert_eq!(recency(&cache), [9]);
    }

    // A key that counts how often it is compared with another key. A hash
    // lookup compares the query with about one stored key; a scan compares
    // it with every entry it passes.
    #[derive(Clone)]
    struct Probe(u32);

    thread_local! {
        static COMPARISONS: Cell<usize> = const { Cell::new(0) };
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            COMPARISONS.set(COMPARISONS.get() + 1);
            self.0 == other.0
        }
    }

    impl Eq for Probe {}

    // `Hash` agrees with `Eq`: both look at the number only.
    impl Hash for Probe {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    #[test]
    fn hits_updates_and_evictions_are_hash_lookups_not_scans() {
        let mut cache = LruCache::new(256);
        for n in 0..256 {
            assert!(cache.put(Probe(n), token(&n.to_string())).is_none());
        }
        COMPARISONS.set(0);
        // The keys in a fixed shuffled order (97 is odd, so this visits each
        // of 0..256 once), so a scan from either end of either index would
        // have to pass about half of the entries every time.
        let shuffled: Vec<u32> = (0..256).map(|i| (i * 97 + 13) % 256).collect();
        for &n in &shuffled {
            let expected = token(&n.to_string());
            assert_eq!(cache.get(&Probe(n)), Some(&expected), "get({n})");
            assert!(cache.put(Probe(n), token("new")).is_none(), "put({n})");
        }
        // Only new keys now, so each put evicts the least recently used one,
        // in the order in which they were just used.
        for (i, &oldest) in shuffled.iter().enumerate() {
            let n = 256 + i as u32;
            let evicted = cache.put(Probe(n), token("fresh"));
            assert!(evicted.is_some_and(|(key, _)| key.0 == oldest), "put({n})");
        }
        // 768 operations. A hash lookup compares about once each (a little
        // more when two stored keys happen to share a hash tag), and even a
        // solution that looks a key up twice stays far below the limit. A
        // scan of 256 entries compares about 128 times per operation, or
        // more.
        let comparisons = COMPARISONS.get();
        assert!(
            comparisons <= 8 * 768,
            "{comparisons} key comparisons for 768 operations: does something scan the entries?"
        );
    }

    // The simplest possible LRU cache, to check the real one against: the
    // entries in a `Vec`, from least to most recently used. Every operation
    // is O(n), which is fine in a test.
    struct Model {
        capacity: usize,
        entries: Vec<(u32, u32)>,
    }

    impl Model {
        fn get(&mut self, key: u32) -> Option<u32> {
            let index = self.entries.iter().position(|&(k, _)| k == key)?;
            let entry = self.entries.remove(index);
            self.entries.push(entry);
            Some(entry.1)
        }

        fn put(&mut self, key: u32, value: u32) -> Option<(u32, u32)> {
            if let Some(index) = self.entries.iter().position(|&(k, _)| k == key) {
                self.entries.remove(index);
                self.entries.push((key, value));
                return None;
            }
            let evicted = if self.entries.len() == self.capacity {
                Some(self.entries.remove(0))
            } else {
                None
            };
            self.entries.push((key, value));
            evicted
        }

        // Most recently used first, like `recency`.
        fn keys(&self) -> Vec<u32> {
            self.entries.iter().rev().map(|&(k, _)| k).collect()
        }
    }

    // A tiny deterministic pseudo-random generator (Knuth's MMIX LCG), so
    // every run replays exactly the same operations.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, bound: u32) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) % u64::from(bound)) as u32
        }
    }

    #[test]
    fn a_thousand_random_operations_match_a_simple_model() {
        for (capacity, seed) in [(1, 1), (2, 7), (3, 42), (5, 2024)] {
            let mut rng = Lcg(seed);
            let mut cache = LruCache::new(capacity);
            let mut model = Model {
                capacity,
                entries: Vec::new(),
            };
            for step in 0..1_000 {
                // Only 8 distinct keys, so hits, misses, updates and
                // evictions all happen often.
                let key = rng.below(8);
                if rng.below(2) == 0 {
                    let expected = model.get(key);
                    assert_eq!(
                        cache.get(&key).copied(),
                        expected,
                        "capacity {capacity}, step {step}: get({key})"
                    );
                } else {
                    let expected = model.put(key, step);
                    assert_eq!(
                        cache.put(key, step),
                        expected,
                        "capacity {capacity}, step {step}: put({key}, {step})"
                    );
                }
                assert_eq!(
                    recency(&cache),
                    model.keys(),
                    "capacity {capacity}, step {step}: recency order"
                );
            }
        }
    }
}
