// Module 2 · Data structures — part 3: a hash map with separate chaining.
//
// A hash map turns a key into an array index by HASHING it. Two different keys
// can hash to the same bucket — a "collision" — so each bucket is itself a
// small list of entries ("separate chaining"). Lookup hashes the key to find
// the bucket, then walks that short list to find the matching key.
//
// We store `(String, i32)` pairs in `Vec<Vec<(String, i32)>>`. The bucket index
// comes from `std::collections::hash_map::DefaultHasher`: feed the key in with
// `Hash::hash`, read the digest with `Hasher::finish`, and reduce it modulo the
// number of buckets. `bucket_index` below is already written for you.
//
// (Everything here is safe std. The `deep-dive/` crate holds the raw-pointer
// data structures; this module is about *design*, not pointers.)

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BUCKET_COUNT: usize = 4;

struct HashTable {
    buckets: Vec<Vec<(String, i32)>>,
}

impl HashTable {
    fn new() -> Self {
        HashTable {
            buckets: vec![Vec::new(); BUCKET_COUNT],
        }
    }

    // Which bucket does `key` belong to? (Provided — study it, then use it.)
    fn bucket_index(&self, key: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % self.buckets.len() as u64) as usize
    }

    // Insert a key/value pair. If the key already exists, OVERWRITE its value
    // instead of adding a duplicate entry.
    fn insert(&mut self, key: String, value: i32) {
        // TODO: Find the bucket with `self.bucket_index(&key)`, then borrow it
        // mutably (`&mut self.buckets[idx]`). Walk its entries: if one already
        // has this key, set its value and return. Otherwise `push` the new
        // `(key, value)` pair onto the bucket.
    }

    // Look a key up. Returns `None` if it is not present.
    fn get(&self, key: &str) -> Option<i32> {
        // TODO: Find the bucket with `self.bucket_index(key)`, then search its
        // entries for one whose key matches `key`. Return `Some(value)` if
        // found (values are `i32`, so copy it out), else `None`.
        //
        // Until you return an `Option<i32>` here, this exercise will not
        // compile (the body's type `()` does not match the return type).
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_is_none() {
        let table = HashTable::new();
        assert_eq!(table.get("nope"), None);
    }

    #[test]
    fn insert_then_get() {
        let mut table = HashTable::new();
        table.insert("one".to_string(), 1);
        table.insert("two".to_string(), 2);
        assert_eq!(table.get("one"), Some(1));
        assert_eq!(table.get("two"), Some(2));
        assert_eq!(table.get("three"), None);
    }

    #[test]
    fn insert_overwrites_existing_key() {
        let mut table = HashTable::new();
        table.insert("key".to_string(), 10);
        table.insert("key".to_string(), 20);
        assert_eq!(table.get("key"), Some(20));
    }

    #[test]
    fn many_keys_survive_collisions() {
        // With only 4 buckets, inserting more than 4 distinct keys guarantees
        // that some share a bucket. Every key must still be retrievable.
        let mut table = HashTable::new();
        let keys = ["a", "b", "c", "d", "e", "f", "g", "h"];
        for (i, k) in keys.iter().enumerate() {
            table.insert(k.to_string(), i as i32);
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(table.get(k), Some(i as i32));
        }
    }
}
