// Module 1 · Borrow-checker errors — part 3: conditionally returning a borrow (E0499).
//
// The shape: look something up through a `&mut`, return the reference if you
// found it, otherwise insert a new element and return a reference to that.
// You will find this function in caches, interners and word counters. It is
// correct code: no execution ever holds two live `&mut` to the same data. Yet
// both functions below are rejected with E0499 "cannot borrow ... as mutable
// more than once at a time". It is a known false positive. RFC 2094, the RFC
// that introduced non-lexical lifetimes (NLL), uses this very `HashMap`
// function (in generic form) as its "problem case #3". The RFC's design aimed
// to accept it, with outlives constraints that hold only at particular
// program points. The NLL that shipped kept a simpler, location-insensitive
// version of those constraints, and that version rejects it.
//
// Why NLL says no. `map.get_mut(&key)` creates a mutable borrow of `*map`.
// Like every borrow it gets a region: the set of program points where rustc
// must treat it as live. On the `Some` arm that reference is RETURNED, so the
// region must outlive the lifetime in the signature, and that lifetime belongs
// to the CALLER: seen from inside the function, it covers every point of the
// body. NLL's outlives constraints are location-insensitive: a requirement
// that arises on one path holds at every point, and nothing asks which path
// actually returns. So the requirement from the `Some` path spreads to the
// `None` path too. The first borrow still counts as live at `map.insert(..)`,
// and that second `&mut` is E0499.
//
// The return is really the cause. If the `Some` arm only used the reference
// locally (`Some(v) => v.push('x')`), NLL would end the borrow at that last use
// (as in `24_ownership_model/ownership3`), and the `None` arm would compile
// fine. You only hit this error when the borrow MIGHT leave the function.
//
// Polonius, the next-generation borrow checker, reasons about each program
// point separately. At `map.insert(..)` on the `None` path, can the first
// borrow still reach a use? No: it only flows into the return value on the
// `Some` path. So Polonius accepts both functions in this file unchanged. But
// Polonius is not on stable Rust; it is only available on nightly, behind a
// `-Z` flag. On stable you restructure the code so that no borrow has to cross
// the conditional return:
//
//   - `HashMap` has an API built for exactly this: the ENTRY API. It takes one
//     mutable borrow, looks the key up ONCE and gives you a handle for either
//     outcome (occupied or vacant). There is no second borrow at all. (RFC
//     2094 notes that this borrow-checker limitation was one of the reasons
//     the entry API was created.)
//   - `Vec` has no entry API. Search for the INDEX first. A `usize` is not a
//     borrow. Push if you need to, and only then borrow the element mutably:
//     one fresh borrow, on the single path that returns.
//
// Interviewers like this question because the wrong answers are revealing:
// "clone it", "`unsafe` pointer cast to launder the lifetime", "wrap everything
// in `Rc<RefCell<..>>`". A strong answer names the false positive and explains
// why NLL's location-insensitive constraints reject it. It mentions Polonius,
// then fixes the design with the entry API or an index. It also notes that the
// entry fix is FASTER: one hash lookup instead of the three the starter would
// do on a miss.

use std::collections::HashMap;

// Part A: a `HashMap` "get or insert default".
//
// Returns a mutable reference to the value stored under `key`, inserting an
// empty `String` first if the key is missing. The caller writes through the
// returned reference, so it must point INTO the map.
fn get_or_default(map: &mut HashMap<u32, String>, key: u32) -> &mut String {
    // TODO: rustc rejects this body twice with E0499 "cannot borrow `*map` as
    // mutable more than once at a time" (at the `insert` and at the second
    // `get_mut`), plus the note "returning this value requires that `*map` is
    // borrowed for `'1`". `'1` is the elided lifetime of `map`, and it lasts
    // for the caller's whole borrow. Restructure the body so no borrow of `*map`
    // has to survive the conditional return. Requirements:
    //   - keep the signature and the behavior: a miss inserts `String::new()`,
    //     a hit leaves the stored value untouched, and the returned reference
    //     points into the map;
    //   - hash and look up `key` only ONCE, on a hit and on a miss;
    //   - no copying the stored value (`.clone()`, `.cloned()`, `to_owned()`:
    //     the tests compare heap pointers), no `unsafe`, no `RefCell`, and
    //     don't change the tests.
    // Until you restructure it so no borrow of `*map` spans the conditional
    // return, this exercise will not compile.
    match map.get_mut(&key) {
        Some(v) => v,
        None => {
            map.insert(key, String::new());
            map.get_mut(&key).unwrap()
        }
    }
}

// Part B: the same shape on a `Vec`, where there is no entry API.
//
// `Item` deliberately does NOT derive `Clone`. Copying data does not get you
// out of a borrow-checker error, and a copy could not be a reference into
// `items` anyway.
struct Item {
    name: String,
    count: u32,
}

// Returns the item called `name`, first appending `Item { name, count: 0 }` if
// there is none. Existing items keep their positions; new items go at the end.
fn find_or_push<'a>(items: &'a mut Vec<Item>, name: &str) -> &'a mut Item {
    // TODO: The same E0499, now on `*items`: "cannot borrow `*items` as mutable
    // more than once at a time" (at the `push` and at `last_mut`), plus the
    // note "returning this value requires that `*items` is borrowed for `'a`".
    // The lifetime is written out here, which makes the problem easier to see.
    // Because the `&mut Item` from `iter_mut()` MIGHT be returned, the borrow
    // of `*items` behind it must last for all of `'a`, which means it is still
    // live on the fall-through path. `Vec` has no entry API, so change what the
    // search gives you: nothing borrowed from `items` may be carried past the
    // `if`. Requirements:
    //   - keep the signature and the behavior described above;
    //   - `Item` must not gain `Clone`; no copying, moving, swapping or
    //     re-inserting existing items (the tests check order and heap
    //     pointers);
    //   - no `unsafe`, no `RefCell`, don't change the tests.
    // Until you restructure it so no borrow of `*items` spans the conditional
    // return, this exercise will not compile.
    if let Some(item) = items.iter_mut().find(|i| i.name == name) {
        return item;
    }
    items.push(Item {
        name: name.to_string(),
        count: 0,
    });
    items.last_mut().unwrap()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Part A: `get_or_default` ----

    #[test]
    fn missing_key_inserts_empty_string_and_writes_through() {
        let mut map = HashMap::new();
        let value = get_or_default(&mut map, 7);
        // A miss starts from an empty `String`...
        assert!(value.is_empty());
        // ...and the reference points INTO the map. Writing through it must
        // change the stored value (a leaked or copied `String` would not).
        value.push_str("seven");
        assert_eq!(map.len(), 1);
        assert_eq!(map[&7], "seven");
    }

    #[test]
    fn existing_key_is_returned_not_overwritten() {
        let mut map = HashMap::from([(1, String::from("one")), (2, String::from("two"))]);
        let stored = map[&1].as_ptr();
        let value = get_or_default(&mut map, 1);
        // A hit must hand back what is already stored, not a fresh empty
        // `String` (an unconditional `insert` would wipe it)...
        assert_eq!(*value, "one");
        // ...and it must be that very `String`, not a copy: its heap buffer is
        // the one that was already in the map. A `.cloned()` copy put back with
        // `insert` would pass every other check in this test.
        assert_eq!(value.as_ptr(), stored);
        value.push('!');
        assert_eq!(map[&1], "one!");
        // Other entries are untouched and nothing new was inserted.
        assert_eq!(map[&2], "two");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn repeated_calls_accumulate_per_key() {
        let mut map = HashMap::new();
        // Interleave two keys: every call after the first one for a key is a hit
        // and must keep what earlier calls wrote.
        for (key, piece) in [(1, "a"), (2, "x"), (1, "b"), (1, "c"), (2, "y")] {
            get_or_default(&mut map, key).push_str(piece);
        }
        assert_eq!(map[&1], "abc");
        assert_eq!(map[&2], "xy");
        assert_eq!(map.len(), 2);
    }

    // ---- Part B: `find_or_push` ----

    fn item(name: &str, count: u32) -> Item {
        Item {
            name: name.to_string(),
            count,
        }
    }

    // `Item` derives nothing, so the tests compare a plain summary of the vector
    // (name and count, in order).
    fn summary(items: &[Item]) -> Vec<(&str, u32)> {
        items.iter().map(|i| (i.name.as_str(), i.count)).collect()
    }

    #[test]
    fn existing_item_is_returned_and_updates_persist() {
        let mut items = vec![item("apple", 3), item("pear", 1), item("plum", 0)];
        let pear_name = items[1].name.as_ptr();
        let found = find_or_push(&mut items, "pear");
        assert_eq!(found.name, "pear");
        assert_eq!(found.count, 1);
        // It is the element that was already there, not a rebuilt copy: its
        // `name` still uses the same heap buffer.
        assert_eq!(found.name.as_ptr(), pear_name);
        found.count += 10;
        // The write landed in the vector, nothing was appended, and nothing
        // moved: "pear" is still in the middle.
        assert_eq!(summary(&items), [("apple", 3), ("pear", 11), ("plum", 0)]);
    }

    #[test]
    fn missing_item_is_appended_with_zero_count() {
        let mut items = vec![item("apple", 3)];
        let added = find_or_push(&mut items, "kiwi");
        assert_eq!(added.name, "kiwi");
        assert_eq!(added.count, 0);
        // The returned reference must be the element in the vector, not a
        // separate `Item` that merely looks the same.
        added.count = 5;
        assert_eq!(summary(&items), [("apple", 3), ("kiwi", 5)]);
    }

    #[test]
    fn works_on_an_empty_vec() {
        let mut items = Vec::new();
        find_or_push(&mut items, "first").count += 1;
        assert_eq!(summary(&items), [("first", 1)]);
    }

    #[test]
    fn repeated_calls_never_duplicate_and_keep_insertion_order() {
        // A word counter: hits on the first, middle and last element, and
        // misses in between.
        let mut items = Vec::new();
        for word in ["b", "a", "b", "c", "a", "b"] {
            find_or_push(&mut items, word).count += 1;
        }
        // Exactly one entry per distinct word, in first-seen order.
        assert_eq!(summary(&items), [("b", 3), ("a", 2), ("c", 1)]);
    }
}
