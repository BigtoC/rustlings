// Traits & Abstraction · Deref, Borrow and Cow — part 3: `K: Borrow<Q>` lookups, and an `Rc<str>` interner (E0308, E0277).
//
// Why can you call `map.get("alice")` on a `HashMap<String, u32>`? The map
// stores `String` keys, and a `&str` is not a `&String`. If `get` took
// `key: &K`, every lookup would first have to allocate a `String` copy of the
// key, only to throw it away. So `get` is generic over the type you look up
// WITH:
//
//     fn get<Q>(&self, key: &Q) -> Option<&V>
//     where
//         K: Borrow<Q>,
//         Q: Hash + Eq + ?Sized,
//
// `K: Borrow<Q>` says "a stored key can be viewed as a `&Q`", so the map can
// test `stored.borrow() == key`. `Q: Hash + Eq` lets it hash and compare the
// query. `?Sized` lets `Q` be `str`, `[u8]` or `Path`, types without a size
// of their own (they only ever live behind a pointer; more on that in
// `sized1..3`, at the end of this module). std provides
// `String: Borrow<str>`, `Vec<T>: Borrow<[T]>`, `PathBuf: Borrow<Path>`,
// `Box<T>`, `Rc<T>` and `Arc<T>: Borrow<T>`, and the reflexive `T: Borrow<T>`
// for every type, which is why `map.get(&owned_key)` still works.
//
// What makes this an interview question is the CONTRACT. The map hashed each
// stored key as a `K`, and now hashes the query as a `Q`, so the two must
// agree: `Borrow<Q>` promises that `Hash`, `Eq` and `Ord` give the same
// answers for the borrowed form as for the owned one. That is the difference
// from `AsRef` (see `23_conversions/conversions5`), which promises only a
// cheap reference conversion. `String` is `AsRef<[u8]>` but NOT
// `Borrow<[u8]>`, because a `str` hashes differently from a `[u8]` holding
// the same bytes (a test below shows it). And a case-insensitive key type
// must not implement `Borrow<str>`: "Alice" and "alice" would be equal keys
// with different `str` hashes (compare `44_trait_contracts/contracts1`).
// Your own key types can join in when they keep the contract: a newtype
// `UserId(String)` with a DERIVED `Hash` and `Eq` hashes and compares exactly
// like its `str`, so `impl Borrow<str> for UserId` is sound.
//
// The same bound decides what a `HashSet` can be searched with, and `Borrow`
// does not chain. `Rc<String>` is `Borrow<String>` (through the impl for any
// `Rc<T>`) but not `Borrow<str>`, so a `HashSet<Rc<String>>` cannot be
// searched with a `&str`: E0277 "the trait bound `Rc<String>: Borrow<str>` is
// not satisfied". `Rc<str>` is `Borrow<str>`, and it is leaner too: one
// allocation holding both reference counts and the bytes, where `Rc<String>`
// needs two allocations and an extra pointer hop.
//
// How interviewers probe it: "Why does `HashMap::get` take a `&Q` where
// `K: Borrow<Q>`?", "What is the difference between `AsRef<str>` and
// `Borrow<str>`?", "When must a type NOT implement `Borrow`?", and "Why
// `Rc<str>` rather than `Rc<String>`?"

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::Rc;

// ---- Part A — lookups through `Borrow` ------------------------------------

// Looks `key` up in `map`.
// The same bounds as `HashMap::get`. `K: Borrow<Q>` lets the map view each
// stored key as a `&Q`, and the `Borrow` contract guarantees that `Q` hashes
// and compares like `K`, so hashing the query finds the right bucket. The
// reflexive `K: Borrow<K>` keeps `&K` working, and `?Sized` admits `str`,
// `[u8]` and `Path`. The result borrows only `map` (`'m`), never the key.
fn lookup<'m, K, V, Q>(map: &'m HashMap<K, V>, key: &Q) -> Option<&'m V>
where
    K: Hash + Eq + Borrow<Q>,
    Q: Hash + Eq + ?Sized,
{
    map.get(key)
}

// Given code that already relies on `lookup` taking the borrowed form.
fn score_of(scores: &HashMap<String, u32>, name: &str) -> u32 {
    lookup(scores, name).copied().unwrap_or(0)
}

// A user id. `Hash`, `PartialEq` and `Eq` are derived, so they hash and
// compare the inner `String` exactly as `str` does.
#[derive(Debug, PartialEq, Eq, Hash)]
struct UserId(String);

impl UserId {
    fn new(id: &str) -> Self {
        UserId(id.to_string())
    }
}

// With a second `Borrow` impl, rustc takes `Q` from the argument (`str`)
// instead of inferring `Q = UserId`. The impl keeps the contract: the derived
// `Hash` and `Eq` of a one-field newtype hash and compare exactly like the
// inner `String`, which hashes and compares like its `str`.
impl Borrow<str> for UserId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// ---- Part B — an interner --------------------------------------------------

// Hands out one shared handle per distinct string, so equal strings are
// stored once and can be compared by pointer.
// `Rc<str>` is `Borrow<str>` (through the impl for any `Rc<T>`, with
// `T = str`), so the set can be searched with the `&str` itself. `Rc<String>`
// was only `Borrow<String>`, and `Borrow` does not chain. `Rc::from(s)` copies
// the text once, into the same allocation as the reference counts, and every
// later `intern` of an equal string clones that one `Rc`.
#[derive(Default)]
struct Interner {
    set: HashSet<Rc<str>>,
}

impl Interner {
    fn new() -> Self {
        Self::default()
    }

    fn intern(&mut self, s: &str) -> Rc<str> {
        if let Some(existing) = self.set.get(s) {
            return Rc::clone(existing);
        }
        let fresh: Rc<str> = Rc::from(s);
        self.set.insert(Rc::clone(&fresh));
        fresh
    }

    fn len(&self) -> usize {
        self.set.len()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hasher};
    use std::path::{Path, PathBuf};

    fn scores() -> HashMap<String, u32> {
        HashMap::from([("alice".to_string(), 10), ("bob".to_string(), 7)])
    }

    // ----- Part A: `lookup` -----

    #[test]
    fn string_keys_are_found_by_str() {
        let map = scores();
        assert_eq!(lookup(&map, "alice"), Some(&10));
        assert_eq!(lookup(&map, "bob"), Some(&7));
        assert_eq!(lookup(&map, "carol"), None);
        assert_eq!(lookup(&map, ""), None);
    }

    #[test]
    fn score_of_defaults_to_zero() {
        let map = scores();
        assert_eq!(score_of(&map, "alice"), 10);
        assert_eq!(score_of(&map, "nobody"), 0);
    }

    #[test]
    fn an_owned_key_still_works() {
        // Here `Q` is `String` itself: every type is `Borrow` of itself.
        let map = scores();
        let key = String::from("bob");
        assert_eq!(lookup(&map, &key), Some(&7));
    }

    #[test]
    fn byte_keys_are_found_by_slice() {
        let map: HashMap<Vec<u8>, &str> =
            HashMap::from([(b"GET".to_vec(), "read"), (vec![0xff, 0x00], "binary")]);
        assert_eq!(lookup(&map, b"GET".as_slice()), Some(&"read"));
        assert_eq!(lookup(&map, &[0xff, 0x00][..]), Some(&"binary"));
        assert_eq!(lookup(&map, b"PUT".as_slice()), None);
    }

    #[test]
    fn path_keys_are_found_by_path() {
        let map: HashMap<PathBuf, u64> = HashMap::from([
            (PathBuf::from("/etc/hosts"), 512),
            (PathBuf::from("notes.txt"), 64),
        ]);
        assert_eq!(lookup(&map, Path::new("/etc/hosts")), Some(&512));
        assert_eq!(lookup(&map, Path::new("notes.txt")), Some(&64));
        assert_eq!(lookup(&map, Path::new("/etc/passwd")), None);
    }

    #[test]
    fn user_ids_are_found_by_str() {
        let map = HashMap::from([(UserId::new("alice"), 3), (UserId::new("bob"), 5)]);
        assert_eq!(lookup(&map, "alice"), Some(&3));
        assert_eq!(lookup(&map, &UserId::new("bob")), Some(&5));
        // Keys compare exactly, as `str` does: case matters.
        assert_eq!(lookup(&map, "Alice"), None);
    }

    #[test]
    fn std_map_methods_accept_a_str_for_a_user_id_too() {
        // `contains_key`, `Index` and `remove` use the same `K: Borrow<Q>`
        // bound as `get`.
        let mut map = HashMap::from([(UserId::new("alice"), 3), (UserId::new("bob"), 5)]);
        assert!(map.contains_key("alice"));
        assert_eq!(map["bob"], 5);
        assert_eq!(map.remove("alice"), Some(3));
        assert!(!map.contains_key("alice"));
    }

    #[test]
    fn the_result_borrows_the_map_not_the_key() {
        let map = scores();
        let found = {
            let key = String::from("alice");
            lookup(&map, key.as_str())
        };
        // `key` is gone here; the result only borrows `map`.
        assert_eq!(found, Some(&10));
    }

    // A key type that counts how often it is hashed, so the next test can
    // tell a hash lookup (one hash of the query) from a scan (none).
    // Thread-local, because the tests run on several threads at once.
    thread_local! {
        static HASHES: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Counted(u32);

    impl Hash for Counted {
        fn hash<H: Hasher>(&self, state: &mut H) {
            HASHES.with(|n| n.set(n.get() + 1));
            self.0.hash(state);
        }
    }

    #[test]
    fn lookup_is_a_hash_lookup_not_a_scan() {
        let map: HashMap<Counted, u32> = (0..100).map(|i| (Counted(i), i * 2)).collect();
        HASHES.with(|n| n.set(0));
        assert_eq!(lookup(&map, &Counted(42)), Some(&84));
        assert_eq!(
            HASHES.with(Cell::get),
            1,
            "`lookup` should be one hash lookup: hash the query once and let \
             the map find the bucket (0 hashes means it compared the query \
             against every key, 2 means it looked the key up twice)"
        );
    }

    #[test]
    fn borrowed_and_owned_forms_must_hash_alike() {
        // The `Borrow` contract, checked with a fixed (unkeyed) hasher.
        let hasher = BuildHasherDefault::<DefaultHasher>::default();
        assert_eq!(
            hasher.hash_one("alice"),
            hasher.hash_one(String::from("alice"))
        );
        assert_eq!(
            hasher.hash_one("alice"),
            hasher.hash_one(UserId::new("alice"))
        );
        // Same bytes, different hash: a `str` hashes its bytes and then a
        // `0xff` end marker, a `[u8]` its length and then its bytes. That is
        // why `String` is `AsRef<[u8]>` but not `Borrow<[u8]>`.
        assert_ne!(
            hasher.hash_one("alice"),
            hasher.hash_one(b"alice".as_slice())
        );
    }

    // ----- Part B: the interner -----

    #[test]
    fn interning_twice_returns_the_same_rc() {
        let mut interner = Interner::new();
        let first: Rc<str> = interner.intern("hello");
        let second: Rc<str> = interner.intern("hello");
        assert!(Rc::ptr_eq(&first, &second));
        // `first`, `second` and the handle kept in the set.
        assert_eq!(Rc::strong_count(&first), 3);
        assert_eq!(&*first, "hello");
        assert_eq!(interner.len(), 1);
        // The set itself can be searched with the text, no handle needed.
        assert!(interner.set.contains("hello"));
        assert!(!interner.set.contains("hell"));
    }

    #[test]
    fn distinct_strings_get_distinct_handles() {
        let mut interner = Interner::new();
        let x: Rc<str> = interner.intern("x");
        let y: Rc<str> = interner.intern("y");
        assert!(!Rc::ptr_eq(&x, &y));
        assert_eq!(interner.len(), 2);
        // Where the text comes from does not matter, only its value.
        let owned = String::from("x");
        let x_again: Rc<str> = interner.intern(&owned);
        assert!(Rc::ptr_eq(&x, &x_again));
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn the_empty_string_is_a_string_too() {
        let mut interner = Interner::new();
        let a: Rc<str> = interner.intern("");
        let b: Rc<str> = interner.intern("");
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(&*a, "");
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn handles_outlive_the_text_they_were_made_from() {
        let mut interner = Interner::new();
        let handle: Rc<str> = {
            let temporary = String::from("user-42");
            interner.intern(&temporary)
        };
        assert_eq!(&*handle, "user-42");
        let again: Rc<str> = interner.intern("user-42");
        assert!(Rc::ptr_eq(&handle, &again));
    }
}
