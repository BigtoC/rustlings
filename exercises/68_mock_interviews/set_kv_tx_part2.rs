// Module 5 · Mock interviews — part 4: the follow-up round, a `count(value)` that survives rollbacks, in 15 minutes (E0599).
//
// The last round of the course: the second half of `set_kv_tx`. Real
// interviews rarely end when the first problem works. The interviewer
// extends it and watches how your design takes the change. A design that
// made part 1 easy can make part 2 hard, and reworking it under the clock is
// part of what gets graded. Run it like the other rounds: timer first, plan
// and costs out loud, `h` for the rubric only afterwards.
//
// The new method looks trivial, and a version that walks every key passes
// every correctness test below: it computes the answer from what `get`
// sees, so transactions cannot confuse it. That is exactly why the
// interviewer rules it out (and one test below has a time budget that such
// a scan cannot meet). The real question is how to keep a count up to date
// through writes that a rollback can take back. Which operations change
// what `get` sees, and which don't? What does a rollback have to undo, and
// where does it find the values it has to count again? And, in Rust, how do
// you update the counts while you hold a `&str` borrowed from the same
// store?

// TODO: Part 2 (15 minutes). The interviewer adds one method to your `Kv` from
// `set_kv_tx`:
//     fn count(&self, value: &str) -> usize
// It returns how many keys have the value `value` right now, exactly as
// `get` sees them: a write inside an open transaction counts at once, a
// rollback takes it back, and a deleted key does not count.
// Example: set a = x; set b = x; count x is 2; begin; set a = y; count x is
// 1 and count y is 1; delete b; count x is 0; rollback; count x is 2 and
// count y is 0.
// Constraints: no operation may look at every key. The target for `count`
// is O(1) per call (O(depth), with `depth` open transactions, is fine), and
// a `rollback` still costs time in proportion to what its transaction
// wrote, not to the size of the store. Part 1's other rules still hold, and
// its tests come first below.
// Start from your part-1 code: copy the fields and method bodies of your
// `Kv` from `set_kv_tx` over the stubs below. Only the signatures are
// repeated here, so that reading ahead does not give part 1 away. Then add
// `count`. Until you do, every call in the tests is E0599 "`Kv` is not an
// iterator": the only `count` that rustc knows is `Iterator::count`, so it
// suggests implementing `Iterator`. Don't: `count` belongs to `Kv` itself.
// Until you add `count`, this exercise will not compile.

// Everything that `commit` and `rollback` can fail with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxError {
    // `commit` or `rollback` while no transaction is open.
    NoTransaction,
}

// Your part-1 fields (and helper types) go here.
struct Kv {}

impl Kv {
    fn new() -> Self {
        todo!()
    }

    fn set(&mut self, key: &str, value: &str) {
        todo!()
    }

    fn get(&self, key: &str) -> Option<&str> {
        todo!()
    }

    fn delete(&mut self, key: &str) -> bool {
        todo!()
    }

    fn begin(&mut self) {
        todo!()
    }

    fn commit(&mut self) -> Result<(), TxError> {
        todo!()
    }

    fn rollback(&mut self) -> Result<(), TxError> {
        todo!()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ops::Range;
    use std::time::{Duration, Instant};

    // ---- Part 1, repeated: your part-1 code must still pass these ----

    #[test]
    fn statement_example() {
        let mut kv = Kv::new();
        kv.set("a", "10");
        kv.begin();
        kv.set("a", "20");
        kv.begin();
        assert!(kv.delete("a"));
        assert_eq!(kv.get("a"), None);
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), Some("20"));
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.get("a"), Some("20"));
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));
        assert_eq!(kv.get("a"), Some("20"));
    }

    #[test]
    fn a_nested_commit_merges_into_the_parent_only() {
        let mut kv = Kv::new();
        kv.set("a", "0");
        kv.begin();
        kv.set("a", "1");
        kv.begin();
        kv.set("a", "2");
        kv.set("b", "3");
        assert_eq!(kv.commit(), Ok(()));
        // The inner writes are now the outer transaction's writes...
        assert_eq!(kv.get("a"), Some("2"));
        assert_eq!(kv.get("b"), Some("3"));
        // ...so rolling back the outer transaction discards them too.
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), Some("0"));
        assert_eq!(kv.get("b"), None);
    }

    #[test]
    fn delete_then_rollback_brings_the_value_back() {
        let mut kv = Kv::new();
        kv.set("a", "1");
        kv.begin();
        assert!(kv.delete("a"));
        assert_eq!(kv.get("a"), None);
        // Already deleted in this transaction.
        assert!(!kv.delete("a"));
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), Some("1"));

        // The same two levels down: the inner rollback only undoes the inner
        // delete.
        kv.begin();
        kv.set("a", "2");
        kv.begin();
        assert!(kv.delete("a"));
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), Some("2"));
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), Some("1"));
    }

    #[test]
    fn commit_and_rollback_need_an_open_transaction() {
        let mut kv = Kv::new();
        kv.set("a", "1");
        assert_eq!(kv.commit(), Err(TxError::NoTransaction));
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));
        assert_eq!(kv.get("a"), Some("1"));

        kv.begin();
        kv.set("a", "2");
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.commit(), Err(TxError::NoTransaction));
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));
        assert_eq!(kv.get("a"), Some("2"));

        kv.begin();
        kv.set("a", "3");
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));
        assert_eq!(kv.get("a"), Some("2"));
        // The failed calls left nothing half open: writes go to the store.
        kv.set("b", "4");
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));
        assert_eq!(kv.get("b"), Some("4"));
    }

    #[test]
    fn a_delete_survives_a_nested_commit() {
        let mut kv = Kv::new();
        kv.set("a", "1");
        kv.begin();
        kv.begin();
        assert!(kv.delete("a"));
        // The inner delete is merged into the outer transaction. There it
        // must keep hiding the "1" in the store: dropping it during the
        // merge would bring "1" back.
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.get("a"), None);
        assert!(!kv.delete("a"));
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.get("a"), None);
        // Committed for good: a later rollback cannot bring it back.
        kv.begin();
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), None);
        kv.set("a", "2");
        assert_eq!(kv.get("a"), Some("2"));
    }

    // Checks that the value stored under `key` still lives at `address`,
    // the address of its bytes when the test first looked.
    fn assert_not_copied(kv: &Kv, key: &str, address: *const u8, after: &str) {
        let value = kv.get(key).expect("the key should be set");
        assert!(
            value.as_ptr() == address,
            "after {after}, the value of {key:?} lives somewhere else: it was copied"
        );
    }

    #[test]
    fn values_are_borrowed_and_moved_never_copied() {
        let mut kv = Kv::new();
        kv.set("a", "untouched");
        let a = kv.get("a").unwrap().as_ptr();
        // `get` borrows from the store: the same bytes every time, not a new
        // copy (or a new leak) per call.
        assert_not_copied(&kv, "a", a, "a second `get`");

        // Neither `begin`, nor a write to another key, nor `rollback` copies
        // "a": a snapshot of the store, taken and restored, would.
        kv.begin();
        kv.set("b", "temporary");
        assert_not_copied(&kv, "a", a, "`begin`");
        assert_eq!(kv.rollback(), Ok(()));
        assert_not_copied(&kv, "a", a, "`begin` and `rollback`");
        // `commit` doesn't copy it either.
        kv.begin();
        kv.set("b", "kept");
        assert_eq!(kv.commit(), Ok(()));
        assert_not_copied(&kv, "a", a, "`begin` and `commit`");

        // A value written inside two nested transactions moves down to the
        // store as they commit; it is not copied at each level.
        kv.begin();
        kv.begin();
        kv.set("c", "moved twice");
        let c = kv.get("c").unwrap().as_ptr();
        assert_eq!(kv.commit(), Ok(()));
        assert_not_copied(&kv, "c", c, "the inner `commit`");
        assert_eq!(kv.commit(), Ok(()));
        assert_not_copied(&kv, "c", c, "the outer `commit`");

        // A rollback restores the very value it hid, not a copy of it.
        kv.begin();
        kv.set("a", "overwritten");
        assert!(kv.delete("c"));
        assert_eq!(kv.rollback(), Ok(()));
        assert_not_copied(&kv, "a", a, "a `set` and a `rollback`");
        assert_not_copied(&kv, "c", c, "a `delete` and a `rollback`");
        assert_eq!(kv.get("a"), Some("untouched"));
        assert_eq!(kv.get("c"), Some("moved twice"));
    }

    // ==== Part 2: `count` ====

    #[test]
    fn part_2_statement_example() {
        let mut kv = Kv::new();
        kv.set("a", "x");
        kv.set("b", "x");
        assert_eq!(kv.count("x"), 2);
        kv.begin();
        kv.set("a", "y");
        assert_eq!(kv.count("x"), 1);
        assert_eq!(kv.count("y"), 1);
        assert!(kv.delete("b"));
        assert_eq!(kv.count("x"), 0);
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.count("x"), 2);
        assert_eq!(kv.count("y"), 0);
    }

    #[test]
    fn count_follows_set_overwrite_and_delete() {
        let mut kv = Kv::new();
        assert_eq!(kv.count("x"), 0);
        assert_eq!(kv.count(""), 0);
        kv.set("a", "x");
        kv.set("b", "x");
        kv.set("c", "y");
        assert_eq!(kv.count("x"), 2);
        assert_eq!(kv.count("y"), 1);
        // An overwrite moves the key from one value to the other.
        kv.set("a", "y");
        assert_eq!(kv.count("x"), 1);
        assert_eq!(kv.count("y"), 2);
        // Setting the value a key already has changes no count.
        kv.set("a", "y");
        kv.set("a", "y");
        assert_eq!(kv.count("y"), 2);
        // A delete takes the key away; deleting a missing key does nothing.
        assert!(kv.delete("b"));
        assert_eq!(kv.count("x"), 0);
        assert!(!kv.delete("b"));
        assert!(!kv.delete("nope"));
        assert_eq!(kv.count("x"), 0);
        assert_eq!(kv.count("y"), 2);
        // Keys and values live in different worlds: "a" is not a value.
        assert_eq!(kv.count("a"), 0);
        kv.set("empty", "");
        assert_eq!(kv.count(""), 1);
    }

    #[test]
    fn rollback_takes_the_counts_back() {
        let mut kv = Kv::new();
        kv.set("a", "x");
        kv.set("b", "y");
        kv.begin();
        kv.set("a", "y");
        kv.set("c", "x");
        kv.set("d", "z");
        assert!(kv.delete("b"));
        assert_eq!((kv.count("x"), kv.count("y"), kv.count("z")), (1, 1, 1));
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!((kv.count("x"), kv.count("y"), kv.count("z")), (1, 1, 0));
        // A failed commit or rollback changes no count.
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));
        assert_eq!(kv.commit(), Err(TxError::NoTransaction));
        assert_eq!((kv.count("x"), kv.count("y"), kv.count("z")), (1, 1, 0));
    }

    #[test]
    fn counts_through_nested_transactions() {
        let mut kv = Kv::new();
        kv.set("a", "x");
        kv.begin();
        kv.set("b", "x");
        kv.begin();
        kv.set("c", "x");
        kv.set("a", "y");
        assert_eq!((kv.count("x"), kv.count("y")), (2, 1));
        // A commit moves writes between layers; what `get` sees, and so
        // every count, stays the same.
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!((kv.count("x"), kv.count("y")), (2, 1));
        kv.begin();
        kv.set("b", "y");
        assert_eq!((kv.count("x"), kv.count("y")), (1, 2));
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!((kv.count("x"), kv.count("y")), (2, 1));
        // The outer rollback also takes back what the inner commit merged.
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!((kv.count("x"), kv.count("y")), (1, 0));
    }

    #[test]
    fn a_tombstone_hides_the_value_below_it() {
        let mut kv = Kv::new();
        kv.set("a", "x");
        kv.begin();
        assert!(kv.delete("a"));
        assert_eq!(kv.count("x"), 0);
        kv.begin();
        kv.set("a", "x");
        assert_eq!(kv.count("x"), 1);
        assert!(kv.delete("a"));
        assert_eq!(kv.count("x"), 0);
        // The merged tombstone keeps hiding the "x" in the store.
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.count("x"), 0);
        // Rolling back the outer transaction brings it back.
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.count("x"), 1);

        // Committed all the way down, the delete is final.
        kv.begin();
        kv.begin();
        assert!(kv.delete("a"));
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.count("x"), 0);
        kv.begin();
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.count("x"), 0);
    }

    // A time budget relative to a plain `HashMap<String, String>` with no
    // transactions, which does the same writes as the `Kv`. `run` takes
    // turns, `CHUNK` steps on the `Kv` and then the same steps on the plain
    // map, and times each side. The `Kv` breaks the budget when its total so
    // far is more than `FACTOR` times the plain map's total plus `SLACK`, and
    // it also took more than `FACTOR` times as long in each of its last
    // `STREAK` chunks.
    //
    // A fixed number of seconds would fail a correct solution on a busy
    // machine. A ratio does not: a turn lasts a fraction of a millisecond, so
    // a busy machine slows both sides alike. It also pauses the test now and
    // then, and a pause adds its whole length to whichever side is running.
    // `SLACK` covers the first pauses and the totals even them out over time,
    // but a few long ones early on can still land on the `Kv`. A pause makes
    // one chunk look slow, though, rarely `STREAK` in a row, hence the second
    // condition. A design that breaks the constraints is slow in every chunk,
    // so it fails both.
    const FACTOR: u32 = 64;
    const SLACK: Duration = Duration::from_millis(500);
    const CHUNK: usize = 20;
    const STREAK: usize = 5;

    #[derive(Default)]
    struct Budget {
        kv: Duration,
        plain: Duration,
        // How many chunks in a row the `Kv` took more than `FACTOR` times as
        // long as the plain map.
        slow_chunks: usize,
    }

    impl Budget {
        fn run(
            &mut self,
            what: &str,
            steps: Range<usize>,
            mut kv_step: impl FnMut(usize),
            mut plain_step: impl FnMut(usize),
        ) {
            for start in steps.clone().step_by(CHUNK) {
                let chunk = start..steps.end.min(start + CHUNK);
                let before = Instant::now();
                chunk.clone().for_each(&mut kv_step);
                let middle = Instant::now();
                chunk.for_each(&mut plain_step);
                let (kv, plain) = (middle - before, middle.elapsed());
                self.kv += kv;
                self.plain += plain;
                self.slow_chunks = if kv > plain * FACTOR {
                    self.slow_chunks + 1
                } else {
                    0
                };
                assert!(
                    self.kv <= self.plain * FACTOR + SLACK || self.slow_chunks < STREAK,
                    "{what}: the `Kv` has taken {:?} so far, more than {FACTOR} \
                     times the {:?} that the same work took on a plain \
                     `HashMap`, and more than {FACTOR} times as long in each \
                     of its last {STREAK} chunks. Does something walk every key?",
                    self.kv,
                    self.plain,
                );
            }
        }
    }

    // `count` must not walk the keys, not even the keys that the open
    // transactions wrote, and a rollback must not count the whole store
    // again. Here a transaction rewrites all 40_000 keys: 40_000 `count` calls
    // that walk them, or 40_000 rollbacks that recount the store, take 1.6
    // billion steps. The plain map does one lookup where the `Kv` counts, and
    // writes the old values back where the `Kv` rolls back. Against it, the
    // intended solution costs a small constant factor. A scan costs a factor
    // that grows with the store, over ten thousand here, so it breaks the
    // budget after a few seconds instead of running for minutes.
    #[test]
    fn count_and_rollback_do_not_scan_the_store() {
        let keys: Vec<String> = (0..40_000).map(|i| format!("key{i}")).collect();
        let old = |i: usize| if i.is_multiple_of(4) { "hot" } else { "cold" };
        let new = |i: usize| if i.is_multiple_of(2) { "hot" } else { "cold" };
        let mut kv = Kv::new();
        let mut plain = HashMap::new();
        let mut budget = Budget::default();
        budget.run(
            "setting 40_000 keys",
            0..keys.len(),
            |i| kv.set(&keys[i], old(i)),
            |i| {
                plain.insert(keys[i].clone(), old(i).to_string());
            },
        );
        kv.begin();
        budget.run(
            "rewriting 40_000 keys in a transaction",
            0..keys.len(),
            |i| kv.set(&keys[i], new(i)),
            |i| {
                plain.insert(keys[i].clone(), new(i).to_string());
            },
        );
        budget.run(
            "40_000 calls of `count` in a transaction",
            0..keys.len(),
            |_| assert_eq!(kv.count("hot"), 20_000),
            |i| assert!(plain.contains_key(&keys[i])),
        );
        budget.run(
            "rolling back 40_000 writes",
            0..1,
            |_| assert_eq!(kv.rollback(), Ok(())),
            |_| {
                for (i, key) in keys.iter().enumerate() {
                    plain.insert(key.clone(), old(i).to_string());
                }
            },
        );
        assert_eq!(kv.count("hot"), 10_000);
        budget.run(
            "40_000 rollbacks",
            0..keys.len(),
            |_| {
                kv.begin();
                kv.set("key1", "hot");
                assert!(kv.delete("key0"));
                assert_eq!(kv.count("hot"), 10_000);
                assert_eq!(kv.rollback(), Ok(()));
            },
            // The same writes and a lookup, then the old values written back.
            |_| {
                plain.insert(keys[1].clone(), "hot".to_string());
                assert!(plain.remove(&keys[0]).is_some());
                assert!(plain.contains_key(&keys[2]));
                plain.insert(keys[1].clone(), "cold".to_string());
                plain.insert(keys[0].clone(), "hot".to_string());
            },
        );
        assert_eq!((kv.count("hot"), kv.count("cold")), (10_000, 30_000));
    }

    // The part-1 model, plus a count taken the slow way.
    struct Model {
        states: Vec<HashMap<String, String>>,
    }

    impl Model {
        fn current(&mut self) -> &mut HashMap<String, String> {
            self.states
                .last_mut()
                .expect("the model always has a state")
        }

        fn count(&mut self, value: &str) -> usize {
            self.current().values().filter(|v| *v == value).count()
        }

        fn commit(&mut self) -> Result<(), TxError> {
            if self.states.len() == 1 {
                return Err(TxError::NoTransaction);
            }
            let top = self.states.pop().expect("checked above");
            *self.current() = top;
            Ok(())
        }

        fn rollback(&mut self) -> Result<(), TxError> {
            if self.states.len() == 1 {
                return Err(TxError::NoTransaction);
            }
            self.states.pop();
            Ok(())
        }
    }

    // A tiny deterministic pseudo-random generator (Knuth's MMIX LCG), so
    // every run replays exactly the same operations.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, bound: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) % bound as u64) as usize
        }
    }

    #[test]
    fn random_operations_match_a_stack_of_copies() {
        const KEYS: [&str; 5] = ["a", "b", "c", "d", "e"];
        const VALUES: [&str; 3] = ["x", "y", "z"];
        for seed in [1, 7, 42, 2024, 31337] {
            let mut rng = Lcg(seed);
            let mut kv = Kv::new();
            let mut model = Model {
                states: vec![HashMap::new()],
            };
            for step in 0..3_000 {
                let key = KEYS[rng.below(KEYS.len())];
                let context = format!("seed {seed}, step {step}");
                match rng.below(10) {
                    0..=3 => {
                        let value = VALUES[rng.below(VALUES.len())];
                        kv.set(key, value);
                        model.current().insert(key.to_string(), value.to_string());
                    }
                    4..=5 => {
                        let expected = model.current().remove(key).is_some();
                        assert_eq!(kv.delete(key), expected, "{context}: delete({key})");
                    }
                    6..=7 => {
                        kv.begin();
                        let copy = model.current().clone();
                        model.states.push(copy);
                    }
                    8 => assert_eq!(kv.commit(), model.commit(), "{context}: commit"),
                    _ => assert_eq!(kv.rollback(), model.rollback(), "{context}: rollback"),
                }
                for key in KEYS {
                    let expected = model.current().get(key).map(String::as_str);
                    assert_eq!(kv.get(key), expected, "{context}: get({key})");
                }
                for value in VALUES {
                    let expected = model.count(value);
                    assert_eq!(kv.count(value), expected, "{context}: count({value})");
                }
            }
        }
    }
}
