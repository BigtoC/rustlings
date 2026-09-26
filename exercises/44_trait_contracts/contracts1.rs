// Traits & Abstraction · Trait contracts — part 1: a `Hash` that agrees with a hand-written `PartialEq` (clippy::derived_hash_with_manual_eq).
//
// A `HashMap` finds a key in two steps. The key's HASH picks where to look,
// and `==` confirms the match among the few keys stored there. That only works
// if the two traits agree, and the `Hash` docs state the rule:
//
//     k1 == k2  implies  hash(k1) == hash(k2)
//
// The converse is NOT required: unequal keys may share a hash (a collision
// costs time, not correctness). A hash that ignores information `==` looks at
// is legal. A hash that looks at information `==` IGNORES is broken.
//
// `#[derive(Hash)]` hashes every field exactly as it is stored, and
// `#[derive(PartialEq)]` compares the same fields exactly, so the two derives
// agree with each other. Derive both, or write both by hand. Mixing them is the
// bug in this file: `PartialEq` was loosened to ignore ASCII case (HTTP header
// names are case-insensitive), but the derived `Hash` still hashes the name
// byte for byte. `"Content-Type"` and `"content-type"` are equal and hash
// differently, so a lookup with the "wrong" casing searches the wrong place and
// usually misses, and a `HashSet` usually keeps both spellings. "Usually",
// because with the default `RandomState` the hash keys change on every run: two
// different hashes can still land in the same place and then compare equal.
// A contract violation gives you a bug that comes and goes. That is why the
// tests below use `DefaultHasher::new()`, whose keys are fixed, and compare
// hashes directly.
//
// It is a LOGIC error, not undefined behavior. The `HashMap` docs say the
// behavior is unspecified but confined to the map that saw the bad key: it may
// panic, return wrong results, abort, leak or never terminate, but it will not
// corrupt memory. `Hash` and `Eq` are safe traits, so unsafe code must stay
// sound even when they lie. rustc cannot check the contract at all. Clippy has
// a deny-by-default lint for this exact pattern: `derived_hash_with_manual_eq`.
//
// Writing `Hash` by hand means feeding the hasher exactly the information `==`
// compares, normalized the same way `==` normalizes it. One more rule matters
// once a value is part of a bigger key. A `Hasher` may treat everything it is
// fed as one flat stream of bytes (`DefaultHasher` does), so without a marker
// where each field ends, the tuple `("ab", "c")` and the tuple `("a", "bc")`
// feed it the same stream, `abc`, and collide every time. The std `Hash` impls
// are PREFIX-FREE for that reason: a slice feeds its length first, and a `str`
// (through the default `Hasher::write_str`) feeds its bytes and then one
// `0xff` byte, a byte that never occurs in UTF-8. (A zero byte would not do:
// `'\0'` is a valid `char`.)
//
// Interviewers ask: "What must hold between `Eq` and `Hash`?", "What happens
// if it doesn't, is it UB?", "May two unequal keys have the same hash?" and
// "Why does `str`'s `Hash` write an extra `0xff` byte?".

use std::collections::HashMap;
use std::hash::Hash;

// An HTTP header name. Header names are case-insensitive: `Content-Type`,
// `content-type` and `CONTENT-TYPE` are the same header. Only ASCII letters
// fold; other bytes compare exactly. A `Header` stores the name exactly as it
// was given, because a proxy forwards headers with their original spelling.
//
// TODO: The test `equal_headers_hash_equally` fails: the headers
// `Header::new("Content-Type")` and `Header::new("content-type")` are equal,
// yet they hash differently. The set and map tests (usually) fail with it: a
// lookup with a different casing misses, and a `HashSet` keeps several
// casings of one name. Clippy rejects the starter too, with its
// deny-by-default lint `derived_hash_with_manual_eq` ("you are deriving
// `Hash` but have implemented `PartialEq` explicitly"). Make `Header`'s hash
// agree with its `PartialEq`. Requirements:
//   - equal headers must hash equally, with ANY `Hasher`;
//   - unequal headers must feed the hasher PREFIX-FREE bytes: different, and
//     neither sequence the start of the other. The tests check real header
//     names, two tuples and every name of up to three characters over a
//     small alphabet (NUL included), so a hash that reads only part of the
//     name (its length, some of its bytes, their sum) fails, and so does
//     one that folds more than `==` does;
//   - keep `PartialEq` as it is, and keep storing the name as given (no
//     lowercasing in `new`: the tests check the spelling);
//   - hashing runs on every lookup, so it should not allocate.
// Until you make the hash agree with `==`, the tests will fail.
#[derive(Debug, Hash, Eq)]
struct Header(String);

impl Header {
    fn new(name: &str) -> Self {
        Header(name.to_string())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Header {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

// A tiny header map: names are case-insensitive, values are stored as given.
struct Headers {
    map: HashMap<Header, String>,
}

impl Headers {
    fn new() -> Self {
        Headers {
            map: HashMap::new(),
        }
    }

    // Sets `name` to `value`. If the name is already present in some other
    // casing, the value is replaced but the key keeps its FIRST spelling:
    // `HashMap::insert` does not replace a key that compares equal.
    fn set(&mut self, name: &str, value: &str) {
        self.map.insert(Header::new(name), value.to_string());
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.map.get(&Header::new(name)).map(String::as_str)
    }

    // How the stored key for `name` is spelled.
    fn spelling(&self, name: &str) -> Option<&str> {
        self.map
            .get_key_value(&Header::new(name))
            .map(|(key, _)| key.as_str())
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::hash::{DefaultHasher, Hasher};

    // `DefaultHasher::new()` always starts from the same keys, so within one
    // build of the program equal inputs always give equal hashes.
    fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    // A `Hasher` that does not hash at all: it records the bytes it is fed.
    // Every `write_*` method funnels into `write` by default, so this shows
    // exactly what a `Hash` impl tells a hasher.
    #[derive(Default)]
    struct Recorder(Vec<u8>);

    impl Hasher for Recorder {
        fn finish(&self) -> u64 {
            0
        }

        fn write(&mut self, bytes: &[u8]) {
            self.0.extend_from_slice(bytes);
        }
    }

    fn fed_bytes<T: Hash + ?Sized>(value: &T) -> Vec<u8> {
        let mut recorder = Recorder::default();
        value.hash(&mut recorder);
        recorder.0
    }

    fn h(name: &str) -> Header {
        Header::new(name)
    }

    #[test]
    fn equal_headers_hash_equally() {
        let pairs = [
            ("Content-Type", "content-type"),
            ("CONTENT-TYPE", "Content-type"),
            ("x-request-id", "X-Request-ID"),
            ("Grüße-X", "grüße-x"),
            ("", ""),
        ];
        for (a, b) in pairs {
            // `PartialEq` is given: these really are equal...
            assert_eq!(h(a), h(b), "{a:?} and {b:?} must stay equal");
            // ...so the contract says they must hash equally.
            assert_eq!(hash_of(&h(a)), hash_of(&h(b)), "hash({a:?}) != hash({b:?})");
            assert_eq!(fed_bytes(&h(a)), fed_bytes(&h(b)), "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn different_names_feed_different_bytes() {
        // Same lengths, same first letters, one-letter differences, empty.
        // Plus two pairs that only a fold looser than `==` would merge: `^`
        // and `~` differ in the same bit as `A` and `a` but are not letters,
        // and `==` folds ASCII letters only, so `Ü` and `ü` stay different.
        let names = [
            "accept",
            "cookie",
            "origin",
            "age",
            "accept-encoding",
            "x-a",
            "x-b",
            "x-^",
            "x-~",
            "Ü-id",
            "ü-id",
            "a",
            "",
        ];
        for (i, a) in names.iter().enumerate() {
            for b in &names[i + 1..] {
                assert_ne!(h(a), h(b));
                let (fa, fb) = (fed_bytes(&h(a)), fed_bytes(&h(b)));
                assert_ne!(
                    fa, fb,
                    "{a:?} and {b:?} are different headers but look the same to a hasher"
                );
                // Prefix-free, in the words of the `Hash` docs: neither
                // sequence may be the start of the other.
                assert!(
                    !fa.starts_with(&fb) && !fb.starts_with(&fa),
                    "the bytes for {a:?} and {b:?} are not prefix-free"
                );
            }
        }
    }

    #[test]
    fn a_header_stays_prefix_free_inside_a_tuple() {
        let splits = [
            ((h("ab"), h("c")), (h("a"), h("bc"))),
            ((h(""), h("x")), (h("x"), h(""))),
        ];
        for (left, right) in &splits {
            assert_ne!(left, right);
            assert_ne!(
                fed_bytes(left),
                fed_bytes(right),
                "{left:?} and {right:?} feed the same bytes"
            );
            assert_ne!(hash_of(left), hash_of(right));
        }
    }

    #[test]
    fn every_short_name_feeds_its_own_bytes() {
        // Every name of up to three characters over a tiny alphabet: 85
        // different headers. A hash that reads only part of a name (its
        // length, its first or last byte, the sum of its bytes) gives two of
        // them the same bytes. The alphabet includes `'\0'`, a valid `char`,
        // so a zero byte cannot mark where a name ends.
        let alphabet = ["a", "B", "-", "\0"];
        let mut names = vec![String::new()];
        for len in 1..=3 {
            let longer: Vec<String> = names
                .iter()
                .filter(|name| name.len() == len - 1)
                .flat_map(|name| alphabet.map(|c| format!("{name}{c}")))
                .collect();
            names.extend(longer);
        }
        assert_eq!(names.len(), 85);
        let fed: Vec<Vec<u8>> = names.iter().map(|name| fed_bytes(&h(name))).collect();
        for (i, a) in fed.iter().enumerate() {
            for (j, b) in fed.iter().enumerate().skip(i + 1) {
                assert!(
                    !a.starts_with(b) && !b.starts_with(a),
                    "the bytes for {:?} and {:?} are not prefix-free",
                    names[i],
                    names[j]
                );
            }
        }
    }

    #[test]
    fn a_hash_set_keeps_one_entry_per_name() {
        let set: HashSet<Header> = ["Content-Type", "content-type", "CONTENT-TYPE", "Accept"]
            .into_iter()
            .map(Header::new)
            .collect();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&h("cOnTeNt-TyPe")));
        assert!(set.contains(&h("accept")));
        assert!(!set.contains(&h("content-length")));
    }

    #[test]
    fn lookups_ignore_case() {
        let mut headers = Headers::new();
        headers.set("Content-Type", "text/html");
        headers.set("X-Request-ID", "42");
        assert_eq!(headers.get("content-type"), Some("text/html"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("text/html"));
        assert_eq!(headers.get("x-request-id"), Some("42"));
        assert_eq!(headers.get("content-length"), None);
    }

    #[test]
    fn setting_again_replaces_the_value_but_keeps_the_first_spelling() {
        let mut headers = Headers::new();
        headers.set("Content-Type", "text/plain");
        headers.set("content-type", "application/json");
        headers.set("CONTENT-TYPE", "text/html");
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("Content-Type"), Some("text/html"));
        assert_eq!(headers.spelling("content-TYPE"), Some("Content-Type"));
    }

    #[test]
    fn a_header_keeps_its_spelling() {
        assert_eq!(h("X-Request-ID").as_str(), "X-Request-ID");
        assert_eq!(h("content-type").as_str(), "content-type");
    }
}
