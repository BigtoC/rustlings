// Traits & Abstraction · Testing seams — part 3: a TTL cache with lazy expiry on an injected clock (E0308).
//
// Another classic live-coding round: a cache whose entries expire. Every
// entry gets a deadline, `expires_at = now + ttl`, when it is inserted, and
// from that instant on it must never be returned again. With the `Clock`
// seam from part 2, "wait thirty seconds" becomes one call to
// `clock.advance(..)`, so the tests can hit the deadline to the nanosecond.
// They check the three instants that matter: just before, exactly at, and
// after. The boundary is part of the contract: here an entry is expired from
// `now >= expires_at` on, so a TTL of zero is expired at once.
//
// When do expired entries actually leave the map? There are two strategies,
// and real caches (Redis, for example) use both:
//
//   - LAZY expiry: `get` checks the deadline and, if it has passed, removes
//     the entry on the spot and answers "not found". Each lookup stays O(1)
//     and needs no background work, but an entry nobody asks for again
//     stays in memory until something else removes it. It also means `get`
//     changes the map, so it takes `&mut self`.
//   - EAGER expiry: something walks the entries and drops the dead ones.
//     Here that is `purge_expired`, one O(n) pass with `HashMap::retain`,
//     meant to run now and then. When a full pass is too slow, keep a min-heap
//     of deadlines (`BinaryHeap<Reverse<..>>`) or a timer wheel, and pop only
//     what is due.
//
// Expect one fight with the borrow checker in `get`. The natural version,
// "look the key up; if the entry is still fresh, return a reference to it;
// otherwise remove it", is NLL problem case #3 again: the shared borrow that
// might be returned counts as live on the path that removes, and rustc
// rejects the `remove` with E0502 "cannot borrow `self.entries` as mutable
// because it is also borrowed as immutable". `37_borrowck_errors/borrowck3`
// explains why it happens and how to restructure around it.
//
// Deadlines can overflow, too: `Duration + Duration` panics on overflow, and
// a caller may well pass `Duration::MAX` to mean "keep forever".
//
// How interviewers probe this: "Lazy or eager expiry, and what does each
// cost?", "What does `get` return exactly at the deadline?", "Does an
// overwrite refresh the TTL?", "How would you test it without sleeping?",
// and "How do you make it thread-safe?" (wrap it in a `Mutex`: even `get`
// writes, so an `RwLock` read lock is not enough).

use std::borrow::Borrow;
use std::cell::Cell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::time::{Duration, Instant};

// The seam, as in part 2.
trait Clock {
    // Time since some fixed, arbitrary origin. Only differences matter.
    fn now(&self) -> Duration;
}

// The production clock.
#[derive(Clone, Copy)]
struct MonotonicClock {
    origin: Instant,
}

impl MonotonicClock {
    fn new() -> Self {
        MonotonicClock {
            origin: Instant::now(),
        }
    }
}

impl Clock for MonotonicClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

// A single-threaded fake is enough here: the map is not shared between
// threads.
#[derive(Clone)]
struct FakeClock {
    now: Rc<Cell<Duration>>,
}

impl FakeClock {
    fn new() -> Self {
        FakeClock {
            now: Rc::new(Cell::new(Duration::ZERO)),
        }
    }

    fn advance(&self, by: Duration) {
        self.now.set(self.now.get() + by);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.now.get()
    }
}

struct Entry<V> {
    value: V,
    // Expired from this instant on.
    expires_at: Duration,
}

struct TtlMap<K, V, C: Clock> {
    clock: C,
    entries: HashMap<K, Entry<V>>,
}

impl<K: Hash + Eq, V, C: Clock> TtlMap<K, V, C> {
    fn new(clock: C) -> Self {
        TtlMap {
            clock,
            entries: HashMap::new(),
        }
    }

    // How many entries are STORED, including expired ones that nothing has
    // removed yet.
    fn len(&self) -> usize {
        self.entries.len()
    }

    // Stores `value` under `key` until `ttl` from now, replacing any entry
    // with the same key (the new entry gets its own, fresh deadline).
    // Returns the value it replaced, but only if that one was still live.
    fn insert(&mut self, key: K, value: V, ttl: Duration) -> Option<V> {
        // TODO: the empty body is E0308 "mismatched types" (expected
        // `Option<V>`, found `()`). Requirements:
        //   - the deadline is `now + ttl`; a TTL too large to add (up to
        //     `Duration::MAX`) means "never expires", not a panic;
        //   - an existing entry is replaced, and its value is returned only if
        //     it had not expired yet: an expired value is never handed out;
        //   - no `Clone` bounds, no `unsafe`; don't change the tests.
        // Until `insert` stores the entry and returns the right old value,
        // this exercise will not compile.
    }

    // Returns the value stored under `key` if it has not expired. An expired
    // entry is removed on the spot (lazy expiry) and gives `None`.
    fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        // TODO: the empty body is E0308 "mismatched types" (expected
        // `Option<&V>`, found `()`). Requirements:
        //   - an entry is live while `now < expires_at`, and at `expires_at`
        //     it is already expired;
        //   - a live entry is returned by reference, straight out of the map;
        //   - an expired entry is removed from the map, so `len` drops, and
        //     the answer is `None`;
        //   - only `key`'s entry is looked at: a lookup stays O(1), and other
        //     dead entries stay stored until `purge_expired` runs;
        //   - no copying values, no `unsafe`, no `RefCell`; don't change the
        //     tests.
        // If rustc rejects your first version with E0502, see the header.
        // Until `get` returns only live values and removes dead ones, this
        // exercise will not compile.
    }

    // Removes every expired entry and returns how many it removed.
    fn purge_expired(&mut self) -> usize {
        // TODO: the empty body is E0308 "mismatched types" (expected
        // `usize`, found `()`). Requirements: one pass over the map that
        // keeps exactly the live entries (same rule as `get`) and reports how
        // many it dropped; no `unsafe`, don't change the tests. Until
        // `purge_expired` removes the dead entries and counts them, this
        // exercise will not compile.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    // Deliberately not `Clone`: the map must hand out references to the
    // values it stores.
    #[derive(Debug, PartialEq, Eq)]
    struct Session {
        user: String,
    }

    fn session(user: &str) -> Session {
        Session {
            user: user.to_string(),
        }
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn cache() -> (FakeClock, TtlMap<String, Session, FakeClock>) {
        let clock = FakeClock::new();
        let map = TtlMap::new(clock.clone());
        (clock, map)
    }

    #[test]
    fn a_value_is_returned_until_just_before_its_deadline() {
        let (clock, mut map) = cache();
        let alice = session("alice");
        let heap = alice.user.as_ptr();
        assert_eq!(map.insert("alice".to_string(), alice, secs(30)), None);
        let got = map.get("alice").expect("fresh entry");
        // The very value that was inserted, by reference.
        assert_eq!(got.user.as_ptr(), heap);
        clock.advance(secs(30) - Duration::from_nanos(1));
        assert_eq!(map.get("alice"), Some(&session("alice")));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn at_the_deadline_the_entry_is_already_expired() {
        let (clock, mut map) = cache();
        map.insert("alice".to_string(), session("alice"), secs(30));
        clock.advance(secs(30));
        assert_eq!(map.get("alice"), None);
    }

    #[test]
    fn get_removes_an_expired_entry_lazily() {
        let (clock, mut map) = cache();
        map.insert("alice".to_string(), session("alice"), secs(5));
        map.insert("bob".to_string(), session("bob"), secs(50));
        clock.advance(secs(10));
        // Nothing has looked at "alice" since it expired: it is still stored.
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("alice"), None);
        assert_eq!(map.len(), 1, "the expired entry must be removed by `get`");
        assert_eq!(map.get("alice"), None);
        assert_eq!(map.get("bob"), Some(&session("bob")));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn get_touches_only_the_key_it_looks_up() {
        // Lazy expiry is per key: a lookup is O(1) and must not sweep the
        // whole map. Dead entries that nobody asks for wait for a purge.
        let (clock, mut map) = cache();
        map.insert("alice".to_string(), session("alice"), secs(5));
        map.insert("bob".to_string(), session("bob"), secs(5));
        map.insert("carol".to_string(), session("carol"), secs(50));
        clock.advance(secs(10));
        assert_eq!(map.get("carol"), Some(&session("carol")));
        assert_eq!(map.get("nobody"), None);
        assert_eq!(map.len(), 3, "a hit or a miss must not sweep the map");
        assert_eq!(map.get("alice"), None);
        assert_eq!(map.len(), 2, "only the dead entry asked for goes");
        assert_eq!(map.purge_expired(), 1);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn a_missing_key_is_none_and_changes_nothing() {
        let (_clock, mut map) = cache();
        assert_eq!(map.get("nobody"), None);
        map.insert("alice".to_string(), session("alice"), secs(5));
        assert_eq!(map.get("nobody"), None);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn a_zero_ttl_is_expired_immediately() {
        let (_clock, mut map) = cache();
        assert_eq!(
            map.insert("alice".to_string(), session("alice"), Duration::ZERO),
            None
        );
        assert_eq!(map.get("alice"), None);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn an_overwrite_refreshes_the_ttl_and_returns_the_old_value() {
        let (clock, mut map) = cache();
        let first = session("first");
        let first_heap = first.user.as_ptr();
        map.insert("k".to_string(), first, secs(10));
        clock.advance(secs(8));
        let old = map.insert("k".to_string(), session("second"), secs(10));
        // The live value that was replaced comes back by value.
        let old = old.expect("the replaced value was still live");
        assert_eq!(old.user, "first");
        assert_eq!(old.user.as_ptr(), first_heap);
        assert_eq!(map.len(), 1);
        // The first deadline (10 s) no longer applies; the new one is 18 s.
        clock.advance(secs(7));
        assert_eq!(map.get("k"), Some(&session("second")));
        clock.advance(secs(3));
        assert_eq!(map.get("k"), None);
    }

    #[test]
    fn overwriting_an_expired_entry_returns_nothing() {
        let (clock, mut map) = cache();
        map.insert("k".to_string(), session("stale"), secs(5));
        clock.advance(secs(5));
        // "stale" expired at 5 s, so it must not come back out of `insert`.
        assert_eq!(map.insert("k".to_string(), session("fresh"), secs(5)), None);
        assert_eq!(map.get("k"), Some(&session("fresh")));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn purge_removes_exactly_the_expired_entries_and_counts_them() {
        let (clock, mut map) = cache();
        map.insert("a".to_string(), session("a"), secs(5));
        map.insert("b".to_string(), session("b"), secs(10));
        map.insert("c".to_string(), session("c"), secs(20));
        map.insert("d".to_string(), session("d"), secs(30));
        clock.advance(secs(10));
        // "a" is long gone and "b" expires right now; "c" and "d" are live.
        assert_eq!(map.purge_expired(), 2);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("c"), Some(&session("c")));
        assert_eq!(map.get("d"), Some(&session("d")));
        // Nothing left to purge until time moves on.
        assert_eq!(map.purge_expired(), 0);
        clock.advance(secs(10));
        assert_eq!(map.purge_expired(), 1);
        assert_eq!(map.get("d"), Some(&session("d")));
    }

    #[test]
    fn purging_an_empty_map_removes_nothing() {
        let (_clock, mut map) = cache();
        assert_eq!(map.purge_expired(), 0);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn a_huge_ttl_never_expires_and_does_not_panic() {
        let (clock, mut map) = cache();
        clock.advance(secs(5));
        // `5 s + Duration::MAX` does not fit in a `Duration`.
        map.insert("forever".to_string(), session("forever"), Duration::MAX);
        clock.advance(secs(100 * 365 * 24 * 3600));
        assert_eq!(map.get("forever"), Some(&session("forever")));
        assert_eq!(map.purge_expired(), 0);
    }

    #[test]
    fn keys_are_looked_up_by_borrowed_form() {
        // `K = String`, but `get` takes a `&str` (or a `&String`), like
        // `HashMap::get` does.
        let (_clock, mut map) = cache();
        map.insert("alice".to_string(), session("alice"), SECOND);
        let owned = "alice".to_string();
        assert!(map.get(&owned).is_some());
        assert!(map.get("alice").is_some());
    }

    #[test]
    fn works_with_other_key_types_and_the_real_clock() {
        // A monotonic clock never goes backwards, so both checks are
        // deterministic: a TTL of zero is always expired, an hour is not.
        let mut map = TtlMap::new(MonotonicClock::new());
        assert_eq!(map.insert(7_u32, "seven", Duration::from_secs(3600)), None);
        assert_eq!(map.insert(8_u32, "eight", Duration::ZERO), None);
        assert_eq!(map.get(&7), Some(&"seven"));
        assert_eq!(map.get(&8), None);
        assert_eq!(
            map.insert(7, "SEVEN", Duration::from_secs(3600)),
            Some("seven")
        );
    }
}
