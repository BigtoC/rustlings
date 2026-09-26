// Module 1 · Interior mutability — part 1: `Cell`, `get` vs `replace` / `take` (E0594, E0599).
//
// `&T` means SHARED, not "immutable". Aliasing XOR mutability is the default
// rule, and interior mutability is the opt-out: a few std types allow
// mutation through a shared reference because they keep the rule some other
// way. All of them are built on `UnsafeCell<T>`, the one primitive through
// which mutating behind a `&` is not undefined behavior. You met `RefCell` in
// `26_smart_pointers_deep/smartptr3`. This module is about the rest of the
// family, and about picking the cheapest one that fits.
//
// Why mutate through `&self` at all? Because some state changes while the
// object stays "logically unchanged": a hit counter, a memo, a "last used"
// slot. A lookup should not demand exclusive access, and behind a shared
// `Rc` it cannot get it: an `Rc<T>` only derefs to `&T` (`Rc::get_mut`
// returns `None` while another handle exists).
//
// `Cell<T>` is the cheapest member of the family. It has no borrow flag and
// no guard, and it has the same layout as `T` (`Cell<u32>` is 4 bytes, where
// `RefCell<u32>` is 16 on a 64-bit target). It stays sound with one rule:
// through `&self` it NEVER hands out a reference to its contents. You can
// only move whole values in and out:
//
//   - `get()` returns a COPY, so it exists only for `T: Copy`. For a `String`
//     a bitwise copy would mean two owners of one heap buffer.
//   - `set(v)` stores `v` and drops the old value; `update(f)` (stable since
//     Rust 1.88) is `set(f(get()))`, again for `Copy` types.
//   - `replace(v)` stores `v` and RETURNS the old value, and `take()` returns
//     it and leaves `T::default()` behind. They are the `mem::replace` and
//     `mem::take` from `24_ownership_model/ownership4..5`, through a `&`.
//
// Since nobody can hold a reference INTO the cell, a `set` can never leave a
// reference dangling, and there is nothing to check at run time.
//
// `Cell<T>` is `Send` when `T` is, but it is never `Sync`: two threads
// holding `&Cell<u32>` could `set` it at the same moment, which is a data
// race. So a `Cell` is for state that one thread owns. Its thread-safe
// cousins are the atomics (`36_atomics`) and `Mutex` (`30_send_sync`).
//
// Reading the errors: rustc runs ALL type checking before borrow checking,
// and a function body that fails type checking is not borrow-checked at all.
// So the E0599s from Part B are listed before the E0594 from Part A.
//
// How interviewers probe this: "Why does `Cell::get` need `Copy`?", "How do
// you change a `String` that lives in a `Cell`?", "`Cell` or `RefCell`, and
// what does each cost?", "Why is `Cell` `Send` but not `Sync`?", "Is a
// `&self` method that mutates a code smell?" (Not for caches, counters and
// memos: that is exactly what interior mutability is for.)

use std::cell::Cell;
use std::collections::HashMap;

// A small price cache that a whole program shares, for example through an
// `Rc<Cache>`. Every method takes `&self`.
struct Cache {
    entries: HashMap<String, u32>,
    // A `Copy` counter that one thread owns: `Cell` is enough.
    hits: Cell<u32>,
    // The most recent key looked up: "" until the first lookup.
    last: Cell<String>,
}

impl Cache {
    fn new(entries: &[(&str, u32)]) -> Self {
        Cache {
            entries: entries
                .iter()
                .map(|&(key, price)| (key.to_string(), price))
                .collect(),
            hits: Cell::new(0),
            last: Cell::new(String::new()),
        }
    }

    // ---- Part A — a `Copy` counter ------------------------------------------

    // Looks `key` up. A lookup that finds a price counts as a hit. Every
    // lookup, hit or miss, becomes the most recent key.
    fn get(&self, key: &str) -> Option<u32> {
        self.remember(key.to_string());
        let price = self.entries.get(key).copied();
        if price.is_some() {
            // `Cell::update` is a `get` followed by a `set`: the new value
            // goes in, the old one is simply overwritten, and no reference to
            // the inside ever exists. That is why it works through `&self`
            // with no borrow flag, and why it needs `u32: Copy`.
            self.hits.update(|hits| hits + 1);
        }
        price
    }

    // How many lookups found a price so far.
    fn hits(&self) -> u32 {
        self.hits.get()
    }

    // ---- Part B — a `String` in a `Cell` ------------------------------------

    // Makes `key` the most recent key and returns the key it replaces ("" if
    // there was none).
    fn remember(&self, key: String) -> String {
        // `replace` is `mem::replace` through a shared reference: it moves
        // `key` in and the old `String` out, header and all, so the heap
        // buffer is never copied, and it never needs a reference to the
        // inside.
        self.last.replace(key)
    }

    // Hands the most recent key to the caller and forgets it: until the next
    // lookup, the most recent key is "" again.
    fn take_last(&self) -> String {
        // `take` is `mem::take` through a shared reference: it moves the key
        // out and leaves `String::default()`, an empty `String` that owns no
        // allocation, behind.
        self.last.take()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn menu() -> Cache {
        Cache::new(&[("espresso", 250), ("latte", 380), ("mocha", 420)])
    }

    // ---- Part A ----

    #[test]
    fn three_lookups_through_shared_references_are_three_hits() {
        // Not `mut`, and two shared references are alive at once: only
        // `&self` methods can be called here.
        let cache = menu();
        let a = &cache;
        let b = &cache;
        assert_eq!(a.get("latte"), Some(380));
        assert_eq!(b.get("espresso"), Some(250));
        assert_eq!(a.get("mocha"), Some(420));
        assert_eq!(b.hits(), 3);
    }

    #[test]
    fn misses_are_not_hits() {
        let cache = menu();
        assert_eq!(cache.get("tea"), None);
        assert_eq!(cache.get(""), None);
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.get("latte"), Some(380));
        assert_eq!(cache.get("latte"), Some(380));
        assert_eq!(cache.hits(), 2);
    }

    #[test]
    fn each_cache_counts_its_own_hits() {
        let cafe = menu();
        let kiosk = menu();
        assert_eq!(cafe.get("latte"), Some(380));
        assert_eq!(cafe.get("mocha"), Some(420));
        assert_eq!(kiosk.get("espresso"), Some(250));
        assert_eq!(cafe.hits(), 2, "`kiosk`'s lookup was counted for `cafe`");
        assert_eq!(kiosk.hits(), 1, "`cafe`'s lookups were counted for `kiosk`");
        assert_eq!(cafe.take_last(), "mocha");
        assert_eq!(kiosk.take_last(), "espresso");
    }

    #[test]
    fn every_rc_handle_bumps_the_same_counter() {
        // `Rc<Cache>` hands out only `&Cache`, so a `&mut self` method could
        // not be called through either handle.
        let cache = Rc::new(menu());
        let other = Rc::clone(&cache);
        assert_eq!(cache.get("latte"), Some(380));
        assert_eq!(other.get("mocha"), Some(420));
        assert_eq!(other.get("decaf"), None);
        assert_eq!(cache.hits(), 2);
        assert_eq!(other.hits(), 2);
    }

    // ---- Part B ----

    #[test]
    fn remember_hands_back_the_previous_key() {
        let cache = menu();
        assert_eq!(
            cache.remember("first".to_string()),
            "",
            "nothing was remembered yet"
        );
        let second = String::from("second");
        let heap = second.as_ptr();
        assert_eq!(cache.remember(second), "first");
        let back = cache.remember("third".to_string());
        assert_eq!(back, "second");
        // A `String` moved in and out keeps its heap buffer. A copy (`clone`,
        // `to_string`, ...) would be a new allocation at a different address.
        assert_eq!(back.as_ptr(), heap, "the key was copied, not moved");
    }

    #[test]
    fn every_lookup_becomes_the_most_recent_key() {
        let cache = menu();
        assert_eq!(cache.get("latte"), Some(380));
        assert_eq!(cache.get("tea"), None);
        assert_eq!(cache.take_last(), "tea", "a miss is a lookup too");
        assert_eq!(cache.hits(), 1);
    }

    #[test]
    fn take_last_hands_the_key_over_and_forgets_it() {
        let cache = menu();
        let key = String::from("mocha");
        let heap = key.as_ptr();
        assert_eq!(cache.remember(key), "");
        let taken = cache.take_last();
        assert_eq!(taken, "mocha");
        assert_eq!(taken.as_ptr(), heap, "the key was copied, not moved");
        assert_eq!(cache.take_last(), "", "`take_last` must forget the key");
        assert_eq!(cache.remember("latte".to_string()), "");
    }

    #[test]
    fn remembering_keys_does_not_count_hits() {
        let cache = menu();
        assert_eq!(cache.remember("espresso".to_string()), "");
        assert_eq!(cache.take_last(), "espresso");
        assert_eq!(cache.hits(), 0);
    }

    // ---- Both parts ----

    #[test]
    fn the_cells_are_no_bigger_than_their_values() {
        // `Cell<T>` has the same size as `T`: it keeps no borrow flag and no
        // lock. A `RefCell` would add a borrow counter to each field.
        let cache = menu();
        assert_eq!(
            size_of_val(&cache.hits),
            size_of::<u32>(),
            "the hit counter carries extra bookkeeping (a borrow flag or a lock?)"
        );
        assert_eq!(
            size_of_val(&cache.last),
            size_of::<String>(),
            "the key slot carries extra bookkeeping (a borrow flag or a lock?)"
        );
    }
}
