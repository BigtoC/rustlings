// Module 2 · LRU cache — part 2: O(1) with a doubly linked list of indices in a `Vec` (E0308).
//
// Part 1 was O(log n). The textbook LRU cache is O(1): a hash map from each
// key to a list node, plus a doubly linked list of the nodes in recency order.
// A hit unlinks its node and links it back in at the front; an eviction takes
// the node at the back. Each is a handful of link writes, because every node
// knows both of its neighbors. This is the version interviewers want, and the
// one where Rust pushes back.
//
// "Why can't the nodes just hold `prev` and `next` references?" A node in the
// middle is pointed at by TWO neighbors. Two `&mut` to one node are not
// allowed, and with shared `&` links nothing could ever relink a node (short
// of putting a `Cell` in every field). On top of that, the nodes need an owner
// that outlives every link: the cache would have to borrow from itself, a
// self-referential struct that cannot even be moved. So the real choices are:
//
//   - `Rc<RefCell<Node>>` links. They compile, but every node is a separate
//     allocation with two counters and a borrow flag, and every access is a
//     run-time borrow check that panics on a mistake
//     (`26_smart_pointers_deep/smartptr3`). Strong links in both directions
//     make each pair of neighbors a cycle, which leaks unless `Drop` unlinks
//     every node (or `prev` is a `Weak`, as in `smartptr2`). And the cache is
//     neither `Send` nor `Sync`.
//   - Raw pointers (`*mut Node` or `NonNull<Node>`) and `unsafe`. This is
//     what the `lru` crate and std's `LinkedList` do (see
//     `deep-dive/src/unsafe_list.rs`): fast, but every relink is a chance for
//     undefined behavior.
//   - INDICES into one `Vec`, the arena of `59_arena`. A link is a `usize`
//     that borrows nothing, all the nodes share one allocation, and the cache
//     is `Send` and `Sync` whenever `K` and `V` are. A wrong index is a logic
//     bug or a panic, never undefined behavior.
//
// Here every entry lives in `nodes: Vec<Entry<K, V>>`, and `map` takes a key
// to its entry's index. `head` is the most recently used entry and `tail`
// the least recently used one. `prev` points toward the head, `next` toward
// the tail, and `None` ends the list in both directions. Two helpers do all
// the link surgery: `unlink(i)` takes entry `i` out of the list, and
// `push_front(i)` links it back in as the head. `get` and `put` are then a
// few hash-map operations and helper calls. Borrowing is where you have to be
// careful: `&mut self.nodes[i]` borrows the whole `Vec` (the checker does not
// look at `i`, see `37_borrowck_errors/borrowck1`), so you cannot hold it
// while you write to a neighbor.
//
// Eviction never shrinks `nodes`: in a full cache, the new key moves INTO THE
// SLOT of the entry it evicts. `mem::replace` puts the new entry in and hands
// back the old one by value, so its key and value can go to the caller.
// (Moving the entry out of `self.nodes[i]` directly is E0507 "cannot move out
// of index of `Vec<Entry<K, V>>`", the lesson of
// `24_ownership_model/ownership4`.) No other entry moves, so no other link
// needs fixing, and `nodes.len()` stays at `capacity` from then on.
// `Vec::remove` would shift every later index and `swap_remove` would move
// the last entry, and either breaks the links. The free list and generations
// of `59_arena/arena2` are not needed here: a slot is only ever freed to be
// reused at once, and no index leaves the cache.
//
// How interviewers probe it: "Walk me through `get` in O(1). Why a DOUBLY
// linked list? Why does a node store its key? (So an eviction can remove the
// key from the map.) Why does `get` take `&mut self`, and how would you share
// this cache between threads?"

use std::collections::HashMap;
use std::hash::Hash;
use std::mem;

// One cached entry, and its place in the recency list.
struct Entry<K, V> {
    key: K,
    value: V,
    // The neighbor toward the head (used more recently).
    prev: Option<usize>,
    // The neighbor toward the tail (used less recently).
    next: Option<usize>,
}

struct LruCache<K, V> {
    capacity: usize,
    // key -> the index of its entry in `nodes`
    map: HashMap<K, usize>,
    // Every entry, linked into one list through `prev` and `next`. It grows
    // until the cache is full and then keeps its length.
    nodes: Vec<Entry<K, V>>,
    // The most recently used entry.
    head: Option<usize>,
    // The least recently used entry: the next one to be evicted.
    tail: Option<usize>,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "the capacity must be at least 1");
        LruCache {
            capacity,
            map: HashMap::new(),
            nodes: Vec::new(),
            head: None,
            tail: None,
        }
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    // Takes entry `i` out of the list: its neighbors (or `head` / `tail`)
    // now point past it. The entry itself stays where it is in `nodes`.
    fn unlink(&mut self, i: usize) {
        // Copy the two links out first. They are `Copy` indices, so no
        // borrow of `nodes` is held while the neighbors are written (holding
        // `&mut self.nodes[i]` across those writes would be E0499: the
        // checker sees every `self.nodes[_]` as the same place). A missing
        // neighbor means `i` was at that end of the list, so the end marker
        // moves instead.
        let (prev, next) = (self.nodes[i].prev, self.nodes[i].next);
        match prev {
            Some(p) => self.nodes[p].next = next,
            None => self.head = next,
        }
        match next {
            Some(n) => self.nodes[n].prev = prev,
            None => self.tail = prev,
        }
        // Not strictly needed (`push_front` overwrites both), but a detached
        // entry with no stale links is easier to debug.
        self.nodes[i].prev = None;
        self.nodes[i].next = None;
    }

    // Links entry `i`, which is not in the list, in as the new head.
    fn push_front(&mut self, i: usize) {
        // Three link writes and one end marker, whatever the list's length.
        // The old head (if any) gets a `prev`; an empty list gets a tail.
        self.nodes[i].prev = None;
        self.nodes[i].next = self.head;
        match self.head {
            Some(h) => self.nodes[h].prev = Some(i),
            None => self.tail = Some(i),
        }
        self.head = Some(i);
    }

    // Returns the value stored under `key` and makes `key` the most recently
    // used. A miss returns `None` and changes nothing.
    fn get(&mut self, key: &K) -> Option<&V> {
        // The map lookup yields a `usize`, which borrows nothing, so the two
        // `&mut self` helper calls are fine. Relinking the head is harmless:
        // it unlinks and becomes the head again.
        let &i = self.map.get(key)?;
        self.unlink(i);
        self.push_front(i);
        Some(&self.nodes[i].value)
    }

    // The same contract as part 1: stores `value` under `key` and makes `key`
    // the most recently used. An update replaces the value and never evicts.
    // A NEW key in a full cache first evicts the least recently used entry,
    // which is returned to the caller.
    fn put(&mut self, key: K, value: V) -> Option<(K, V)> {
        // An update: the entry stays in its slot and only moves to the
        // front. Returning before the capacity check keeps it from evicting.
        if let Some(&i) = self.map.get(&key) {
            self.nodes[i].value = value;
            self.unlink(i);
            self.push_front(i);
            return None;
        }
        let entry = Entry {
            key: key.clone(),
            value,
            prev: None,
            next: None,
        };
        if self.nodes.len() < self.capacity {
            // Not full yet: the new entry gets the next slot.
            let i = self.nodes.len();
            self.nodes.push(entry);
            self.map.insert(key, i);
            self.push_front(i);
            return None;
        }
        // Full: recycle the tail's slot. `mem::replace` swaps the new entry
        // in and returns the old one BY VALUE, so its key and value can be
        // moved out to the caller without E0507. No other slot changes, so
        // every other index in `map` and in the links stays valid.
        let t = self.tail.expect("a full cache has a tail");
        self.unlink(t);
        let old = mem::replace(&mut self.nodes[t], entry);
        self.map.remove(&old.key);
        self.map.insert(key, t);
        self.push_front(t);
        Some((old.key, old.value))
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

    // The indices of the linked entries from head to tail, after checking
    // that the list is well formed: the walk ends (no cycle), `head` has no
    // `prev`, `tail` has no `next`, and walking backward from `tail` visits
    // exactly the same entries in reverse.
    fn walk<K, V>(cache: &LruCache<K, V>) -> Vec<usize> {
        let limit = cache.nodes.len();
        let mut forward = Vec::new();
        let mut cur = cache.head;
        while let Some(i) = cur {
            assert!(forward.len() < limit, "the `next` links run in a cycle");
            forward.push(i);
            cur = cache.nodes[i].next;
        }
        let mut backward = Vec::new();
        let mut cur = cache.tail;
        while let Some(i) = cur {
            assert!(backward.len() < limit, "the `prev` links run in a cycle");
            backward.push(i);
            cur = cache.nodes[i].prev;
        }
        backward.reverse();
        assert_eq!(
            forward, backward,
            "walking forward from `head` and backward from `tail` disagree"
        );
        if let Some(head) = cache.head {
            assert_eq!(cache.nodes[head].prev, None, "`head` has a `prev`");
        }
        if let Some(tail) = cache.tail {
            assert_eq!(cache.nodes[tail].next, None, "`tail` has a `next`");
        }
        forward
    }

    // The cached keys from most to least recently used, after checking the
    // whole structure: the list is well formed and holds every entry in
    // `nodes` exactly once, and `map` takes each key to its own entry.
    fn recency<K: Hash + Eq + Clone + Debug, V>(cache: &LruCache<K, V>) -> Vec<K> {
        let order = walk(cache);
        assert_eq!(
            order.len(),
            cache.nodes.len(),
            "the list does not hold every entry in `nodes`"
        );
        assert_eq!(
            cache.map.len(),
            cache.nodes.len(),
            "`map` and `nodes` hold different numbers of entries"
        );
        assert!(
            cache.nodes.len() <= cache.capacity,
            "the cache holds more than `capacity` entries"
        );
        for &i in &order {
            let key = &cache.nodes[i].key;
            assert_eq!(cache.map.get(key), Some(&i), "`map` loses track of {key:?}");
        }
        order.iter().map(|&i| cache.nodes[i].key.clone()).collect()
    }

    // A cache with `keys` in `nodes` in that order, linked from the first
    // (the head) to the last (the tail), built without `put`, so that the
    // helper tests do not depend on it.
    fn linked(keys: &[u32]) -> LruCache<u32, Token> {
        let n = keys.len();
        let nodes = keys
            .iter()
            .enumerate()
            .map(|(i, &key)| Entry {
                key,
                value: token(&key.to_string()),
                prev: i.checked_sub(1),
                next: (i + 1 < n).then_some(i + 1),
            })
            .collect();
        LruCache {
            capacity: n.max(1),
            map: keys.iter().enumerate().map(|(i, &key)| (key, i)).collect(),
            nodes,
            head: (n > 0).then_some(0),
            tail: n.checked_sub(1),
        }
    }

    #[test]
    fn unlink_and_push_front_relink_the_middle_the_head_and_the_tail() {
        let mut cache = linked(&[10, 11, 12, 13]);
        assert_eq!(walk(&cache), [0, 1, 2, 3]);

        // From the middle: both neighbors are updated.
        cache.unlink(1);
        assert_eq!(walk(&cache), [0, 2, 3]);
        cache.push_front(1);
        assert_eq!(walk(&cache), [1, 0, 2, 3]);

        // From the tail: `tail` moves back one entry.
        cache.unlink(3);
        assert_eq!(walk(&cache), [1, 0, 2]);
        assert_eq!(cache.tail, Some(2));
        cache.push_front(3);
        assert_eq!(walk(&cache), [3, 1, 0, 2]);

        // From the head: `head` moves forward one entry.
        cache.unlink(3);
        assert_eq!(walk(&cache), [1, 0, 2]);
        assert_eq!(cache.head, Some(1));
        cache.push_front(3);
        assert_eq!(walk(&cache), [3, 1, 0, 2]);

        // Nothing moved inside `nodes`: only links changed.
        let keys: Vec<u32> = cache.nodes.iter().map(|e| e.key).collect();
        assert_eq!(keys, [10, 11, 12, 13]);
        assert_eq!(recency(&cache), [13, 11, 10, 12]);
    }

    #[test]
    fn unlink_and_push_front_handle_a_single_entry() {
        let mut cache = linked(&[5]);
        cache.unlink(0);
        assert_eq!(cache.head, None);
        assert_eq!(cache.tail, None);
        assert_eq!(walk(&cache), []);
        cache.push_front(0);
        assert_eq!(cache.head, Some(0));
        assert_eq!(cache.tail, Some(0));
        assert_eq!(recency(&cache), [5]);
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
    fn a_hit_moves_its_entry_to_the_front_and_a_miss_changes_nothing() {
        let mut cache = LruCache::new(3);
        for name in ["a", "b", "c"] {
            assert!(cache.put(name, token(name)).is_none());
        }
        assert_eq!(recency(&cache), ["c", "b", "a"]);

        // The tail, the middle and the head.
        assert_eq!(cache.get(&"a"), Some(&token("a")));
        assert_eq!(recency(&cache), ["a", "c", "b"]);
        assert_eq!(cache.get(&"c"), Some(&token("c")));
        assert_eq!(recency(&cache), ["c", "a", "b"]);
        assert_eq!(cache.get(&"c"), Some(&token("c")));
        assert_eq!(recency(&cache), ["c", "a", "b"]);

        assert_eq!(cache.get(&"zzz"), None);
        assert_eq!(recency(&cache), ["c", "a", "b"]);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn updating_a_key_replaces_its_value_promotes_it_and_never_evicts() {
        let mut cache = LruCache::new(3);
        assert!(cache.put("a", token("a1")).is_none());
        assert!(cache.put("b", token("b1")).is_none());
        assert!(cache.put("c", token("c1")).is_none());
        let slots = [cache.map[&"a"], cache.map[&"b"], cache.map[&"c"]];

        // The cache is full, but these keys are already in it: updates, not
        // inserts, so nothing may be evicted. First a key from the middle of
        // the list, then the tail.
        assert_eq!(cache.put("b", token("b2")), None);
        assert_eq!(recency(&cache), ["b", "c", "a"]);
        assert_eq!(cache.put("a", token("a2")), None);
        assert_eq!(recency(&cache), ["a", "b", "c"]);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"b"), Some(&token("b2")));
        assert_eq!(cache.get(&"a"), Some(&token("a2")));
        // An update changes the value in place: every entry keeps its slot.
        assert_eq!([cache.map[&"a"], cache.map[&"b"], cache.map[&"c"]], slots);

        // The updates counted as uses, so "c" goes first.
        assert_eq!(cache.put("d", token("d1")), Some(("c", token("c1"))));
        assert_eq!(cache.get(&"c"), None);
        assert_eq!(recency(&cache), ["d", "a", "b"]);
    }

    #[test]
    fn capacity_one_keeps_only_the_latest_key() {
        let mut cache = LruCache::new(1);
        assert_eq!(cache.put(7, token("seven")), None);
        assert_eq!(cache.put(7, token("SEVEN")), None);
        assert_eq!(cache.get(&7), Some(&token("SEVEN")));

        assert_eq!(cache.put(8, token("eight")), Some((7, token("SEVEN"))));
        assert_eq!(cache.get(&7), None);
        assert_eq!(cache.get(&8), Some(&token("eight")));
        assert_eq!(cache.put(9, token("nine")), Some((8, token("eight"))));
        assert_eq!(recency(&cache), [9]);
        assert_eq!(cache.nodes.len(), 1);
    }

    #[test]
    fn a_new_key_takes_over_the_evicted_entrys_slot() {
        let mut cache = LruCache::new(3);
        for name in ["a", "b", "c"] {
            assert!(cache.put(name, token(name)).is_none());
        }
        // Until the cache is full, each new entry goes at the end of `nodes`.
        assert_eq!(
            [cache.map[&"a"], cache.map[&"b"], cache.map[&"c"]],
            [0, 1, 2]
        );
        assert_eq!(cache.get(&"a"), Some(&token("a")));

        // "b" (slot 1) is the least recently used. "d" must land in slot 1,
        // and nothing else may move: not "a" in slot 0, nor "c" in slot 2.
        assert_eq!(cache.put("d", token("d")), Some(("b", token("b"))));
        assert_eq!(cache.nodes.len(), 3);
        assert_eq!(cache.map[&"d"], 1);
        assert_eq!(cache.map[&"a"], 0);
        assert_eq!(cache.map[&"c"], 2);
        assert_eq!(recency(&cache), ["d", "a", "c"]);

        // Next is "c", in the LAST slot.
        assert_eq!(cache.put("e", token("e")), Some(("c", token("c"))));
        assert_eq!(cache.map[&"e"], 2);
        assert_eq!(recency(&cache), ["e", "d", "a"]);

        // Then "a", in the FIRST slot.
        assert_eq!(cache.put("f", token("f")), Some(("a", token("a"))));
        assert_eq!(cache.map[&"f"], 0);
        assert_eq!(recency(&cache), ["f", "e", "d"]);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"a"), None);
    }

    #[test]
    fn ten_thousand_puts_into_capacity_three_keep_three_slots() {
        let mut cache = LruCache::new(3);
        for i in 0..10_000u32 {
            let evicted = cache.put(i, token(&i.to_string()));
            // Every key is new, so from the fourth put on, each one evicts
            // the key from three puts earlier.
            let expected = i.checked_sub(3).map(|old| (old, token(&old.to_string())));
            assert_eq!(evicted, expected, "put({i})");
            assert!(cache.nodes.len() <= 3, "put({i}) grew `nodes` past 3");
        }
        assert_eq!(cache.nodes.len(), 3);
        assert_eq!(cache.map.len(), 3);
        assert_eq!(recency(&cache), [9_999, 9_998, 9_997]);
    }

    #[test]
    fn an_index_linked_cache_is_send_and_sync() {
        // Links are plain `usize`s, so nothing stops the cache from moving
        // to (or being shared with) another thread. An `Rc<RefCell<..>>`
        // list could do neither.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LruCache<String, Token>>();
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
