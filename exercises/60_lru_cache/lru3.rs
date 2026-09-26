// Module 2 · LRU cache — part 3: `get` by a borrowed key, and a `peek` that takes `&self` (E0308, E0599).
//
// The cache from part 2 is fast, but its API makes callers pay. On an
// `LruCache<String, V>`, `get(&K)` wants a `&String`, so a caller holding a
// `&str` (a header name, a token from a parser, a literal) has to allocate a
// `String` just to ask, and then throw it away. `HashMap::get` has no such
// problem, because its key parameter is generic over a BORROWED form of the
// key. That signature, why `Borrow` (and not `AsRef`) makes it sound, and why
// `get(&owned_key)` keeps working are the lesson of `45_sized_deref/borrow1`.
// Here you apply it to a type of your own: the lookup inside hands the
// borrowed key straight to the `HashMap`, which accepts it for the same
// reason. The tests also use `Arc<str>` keys, the thread-safe sibling of
// `borrow1`'s `Rc<str>`: `Arc<T>: Borrow<T>`, so a `&str` finds them too.
//
// The second change is a read that does NOT count as a use. Monitoring, "is
// it cached?" checks, debugging and tests all want to look at an entry
// without changing which entry is evicted next. Such a `peek` writes nothing,
// so it can take `&self`, and that matters more than it looks. It works
// through a `&LruCache` and through a `RwLock` read guard, where `get` does
// not compile: `get` only reads a value, but the recency list changes on
// every hit, so it needs `&mut self` and therefore the write lock. Many
// readers can `peek` at once; every `get` takes the lock exclusively.
//
// How interviewers probe it: "Why can a `HashMap<String, V>` be queried with
// a `&str`? Why `Borrow` and not `AsRef`? What is the `?Sized` for? Which of
// your methods need `&mut self`, and what does that mean behind a lock?"

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

// Part 2's cache, finished.
struct LruCache<K, V> {
    capacity: usize,
    // key -> the index of its entry in `nodes`
    map: HashMap<K, usize>,
    // Every entry, linked into one list through `prev` and `next`.
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

    // Takes entry `i` out of the list. It stays where it is in `nodes`.
    fn unlink(&mut self, i: usize) {
        let (prev, next) = (self.nodes[i].prev, self.nodes[i].next);
        match prev {
            Some(p) => self.nodes[p].next = next,
            None => self.head = next,
        }
        match next {
            Some(n) => self.nodes[n].prev = prev,
            None => self.tail = prev,
        }
        self.nodes[i].prev = None;
        self.nodes[i].next = None;
    }

    // Links entry `i`, which is not in the list, in as the new head.
    fn push_front(&mut self, i: usize) {
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
    //
    // TODO: The tests call `get("alpha")` on an `LruCache<String, _>`, and
    // rustc rejects it with E0308 "mismatched types": expected `&String`,
    // found `&str`. The tests also look up `Vec<u8>` keys with a `&[u8]` and
    // `Arc<str>` keys with a `&str`. Make `get` accept every borrowed form of
    // `K` that `HashMap::get` itself accepts. Requirements:
    //   - `get(&owned_key)` must keep working, and so must plain `u32` keys
    //     looked up with a `&u32`;
    //   - the argument is looked up as it is: no building a `K` from it
    //     (`to_string`, `to_owned`, `K::from`, `into`), and no scanning the
    //     entries; one hash lookup, as before;
    //   - the body can stay exactly as it is. Keep the behavior of part 2,
    //     and don't change `put` or the tests.
    // Until `get` takes a borrowed form of the key, this exercise will not
    // compile.
    fn get(&mut self, key: &K) -> Option<&V> {
        let &i = self.map.get(key)?;
        self.unlink(i);
        self.push_front(i);
        Some(&self.nodes[i].value)
    }

    // TODO: The tests also call `peek`, which does not exist yet: E0599 "no
    // method named `peek` found for struct `LruCache<K, V>` in the current
    // scope". Add it here. Requirements:
    //   - it returns the value stored under a key as an `Option<&V>`, and it
    //     accepts the same key forms as `get`;
    //   - one hash lookup, like `get`: no scanning `nodes` or the list (a
    //     test counts the key comparisons);
    //   - it does NOT count as a use: the recency order stays exactly as it
    //     was, so a peeked entry can still be the next one evicted;
    //   - it works through shared access: one test calls it on a
    //     `&LruCache`, another through a `RwLock` read guard;
    //   - no `RefCell` or `Cell`, no copying the value (the tests' values are
    //     not `Clone`), no `unsafe`, and don't change the tests.
    // Until you add `peek`, this exercise will not compile.

    // Stores `value` under `key` and makes `key` the most recently used. An
    // update replaces the value and never evicts. A NEW key in a full cache
    // first evicts the least recently used entry, which is returned to the
    // caller.
    fn put(&mut self, key: K, value: V) -> Option<(K, V)> {
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
            let i = self.nodes.len();
            self.nodes.push(entry);
            self.map.insert(key, i);
            self.push_front(i);
            return None;
        }
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
    use std::hash::Hasher;
    use std::sync::{Arc, RwLock};

    // Deliberately not `Clone`: `get` and `peek` have to hand out references
    // to the stored values.
    #[derive(Debug, PartialEq)]
    struct Token(String);

    fn token(name: &str) -> Token {
        Token(name.to_string())
    }

    // The cached keys from most to least recently used.
    fn recency<K: Clone, V>(cache: &LruCache<K, V>) -> Vec<K> {
        let mut keys = Vec::new();
        let mut cur = cache.head;
        while let Some(i) = cur {
            assert!(keys.len() < cache.nodes.len(), "the links run in a cycle");
            keys.push(cache.nodes[i].key.clone());
            cur = cache.nodes[i].next;
        }
        keys
    }

    fn cache_of(keys: &[&str]) -> LruCache<String, Token> {
        let mut cache = LruCache::new(keys.len());
        for key in keys {
            assert!(cache.put(key.to_string(), token(key)).is_none());
        }
        cache
    }

    #[test]
    fn string_keys_can_be_looked_up_with_a_str() {
        let mut cache = cache_of(&["alpha", "beta"]);
        // A `&str`, so no `String` has to be built just to ask.
        assert_eq!(cache.get("alpha"), Some(&token("alpha")));
        assert_eq!(cache.get("gamma"), None);
        // A `&String` works too: every type borrows as itself.
        let owned = String::from("beta");
        assert_eq!(cache.get(&owned), Some(&token("beta")));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn a_get_by_str_still_counts_as_a_use() {
        let mut cache = cache_of(&["a", "b"]);
        // "a" was the least recently used; the hit saves it.
        assert_eq!(cache.get("a"), Some(&token("a")));
        let evicted = cache.put(String::from("c"), token("c"));
        assert_eq!(evicted, Some((String::from("b"), token("b"))));
        assert_eq!(recency(&cache), ["c", "a"]);
    }

    #[test]
    fn other_owned_keys_accept_their_borrowed_forms() {
        // `Vec<u8>` keys, looked up with a byte slice.
        let mut bytes = LruCache::new(2);
        assert!(bytes.put(b"GET".to_vec(), token("get")).is_none());
        assert_eq!(bytes.get(&b"GET"[..]), Some(&token("get")));
        assert_eq!(bytes.peek(&b"PUT"[..]), None);

        // `Arc<str>` keys, looked up with a `&str`. (`put` clones every key
        // once, and cloning an `Arc<str>` only bumps a counter.)
        let mut shared: LruCache<Arc<str>, Token> = LruCache::new(2);
        assert!(shared.put(Arc::from("tenant-1"), token("one")).is_none());
        assert_eq!(shared.get("tenant-1"), Some(&token("one")));
        assert_eq!(shared.peek("tenant-1"), Some(&token("one")));
        assert_eq!(shared.get("tenant-2"), None);

        // Plain `u32` keys, looked up with a `&u32` as before.
        let mut numbers = LruCache::new(2);
        assert!(numbers.put(7_u32, token("seven")).is_none());
        assert_eq!(numbers.get(&7), Some(&token("seven")));
        assert_eq!(numbers.peek(&7), Some(&token("seven")));
        assert_eq!(numbers.peek(&8), None);
    }

    #[test]
    fn peek_does_not_save_the_lru_entry_but_get_does() {
        // "old" is the least recently used. Peeking at it does not count as
        // a use, so it is still the one evicted next.
        let mut cache = cache_of(&["old", "new"]);
        assert_eq!(cache.peek("old"), Some(&token("old")));
        let evicted = cache.put(String::from("next"), token("next"));
        assert_eq!(evicted, Some((String::from("old"), token("old"))));
        assert_eq!(cache.peek("old"), None);

        // The same with `get`: now "old" survives and "new" goes instead.
        let mut cache = cache_of(&["old", "new"]);
        assert_eq!(cache.get("old"), Some(&token("old")));
        let evicted = cache.put(String::from("next"), token("next"));
        assert_eq!(evicted, Some((String::from("new"), token("new"))));
        assert_eq!(cache.peek("old"), Some(&token("old")));
    }

    #[test]
    fn peek_leaves_the_recency_order_untouched() {
        let mut cache = cache_of(&["a", "b", "c"]);
        assert_eq!(cache.get("b"), Some(&token("b")));
        let before = recency(&cache);
        assert_eq!(before, ["b", "c", "a"]);
        // Head, middle, tail and a miss.
        for key in ["b", "c", "a", "zzz"] {
            let expected = (key != "zzz").then(|| token(key));
            assert_eq!(cache.peek(key), expected.as_ref(), "peek({key:?})");
            assert_eq!(recency(&cache), before, "after peek({key:?})");
        }
    }

    #[test]
    fn peek_works_through_shared_references_and_read_locks() {
        // Takes the cache by shared reference, as a metrics or debug
        // endpoint would.
        fn describe(cache: &LruCache<String, Token>, key: &str) -> String {
            match cache.peek(key) {
                Some(Token(name)) => format!("{key} = {name}"),
                None => format!("{key} is not cached"),
            }
        }

        let lock = RwLock::new(cache_of(&["a", "b"]));
        {
            // A read guard only gives `&LruCache`: `peek` is all it allows.
            let cache = lock.read().unwrap();
            assert_eq!(describe(&cache, "a"), "a = a");
            assert_eq!(describe(&cache, "zzz"), "zzz is not cached");
            assert_eq!(cache.peek("b"), Some(&token("b")));
        }
        // `get` reorders the list, so it needs the write lock.
        assert_eq!(lock.write().unwrap().get("a"), Some(&token("a")));
        let cache = lock.into_inner().unwrap();
        assert_eq!(recency(&cache), ["a", "b"]);
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
    fn get_and_peek_are_one_hash_lookup_not_a_scan() {
        let mut cache = LruCache::new(256);
        for n in 0..256 {
            assert!(cache.put(Probe(n), token(&n.to_string())).is_none());
        }
        COMPARISONS.set(0);
        for n in 0..256 {
            let expected = token(&n.to_string());
            assert_eq!(cache.peek(&Probe(n)), Some(&expected), "peek({n})");
            assert_eq!(cache.get(&Probe(n)), Some(&expected), "get({n})");
        }
        // 512 hits. A hash lookup compares about once per hit (a little more
        // when two stored keys happen to share a hash tag). A scan of 256
        // entries compares about 128 times per hit, or more.
        let comparisons = COMPARISONS.get();
        assert!(
            comparisons <= 4 * 512,
            "{comparisons} key comparisons for 512 hits: do `get` or `peek` scan the entries?"
        );
    }

    // The simplest possible LRU cache, to check the real one against: the
    // entries in a `Vec`, from least to most recently used.
    struct Model {
        capacity: usize,
        entries: Vec<(u32, u32)>,
    }

    impl Model {
        fn peek(&self, key: u32) -> Option<u32> {
            self.entries
                .iter()
                .find(|&&(k, _)| k == key)
                .map(|&(_, v)| v)
        }

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
    fn random_gets_peeks_and_puts_match_a_simple_model() {
        for (capacity, seed) in [(1, 3), (3, 11), (4, 99)] {
            let mut rng = Lcg(seed);
            let mut cache = LruCache::new(capacity);
            let mut model = Model {
                capacity,
                entries: Vec::new(),
            };
            for step in 0..1_000 {
                let key = rng.below(6);
                match rng.below(3) {
                    0 => assert_eq!(
                        cache.get(&key).copied(),
                        model.get(key),
                        "capacity {capacity}, step {step}: get({key})"
                    ),
                    1 => assert_eq!(
                        cache.peek(&key).copied(),
                        model.peek(key),
                        "capacity {capacity}, step {step}: peek({key})"
                    ),
                    _ => assert_eq!(
                        cache.put(key, step),
                        model.put(key, step),
                        "capacity {capacity}, step {step}: put({key}, {step})"
                    ),
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
