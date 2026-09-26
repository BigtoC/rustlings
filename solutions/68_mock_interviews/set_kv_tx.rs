// Module 5 · Mock interviews — part 3: a key-value store with nested transactions in 35 minutes (the stubs panic, so the tests fail).
//
// The third timed round, in the format of `set_trie`: the `// TODO` below is
// the statement, the struct has no fields yet, and every method is a stub
// that panics. Start the timer, sketch the design and the cost of each
// operation before you type, and press `h` for the rubric only afterwards.
// The tests below the banner are the follow-ups.
//
// "An in-memory database with BEGIN, COMMIT and ROLLBACK" is one of the most
// common backend interview problems, and `set_kv_tx_part2` is the follow-up
// that usually comes next. It is not an algorithm puzzle: every operation is
// a few lines. It is graded as a design exercise. Where do a transaction's
// writes live until it commits? What does a DELETE inside a transaction
// write, given that the key may still exist in the layer below? What does
// COMMIT of an inner transaction do to the outer one? And what do BEGIN,
// COMMIT and ROLLBACK cost when the store holds ten million keys and the
// transaction touched two? (`39_drop_raii/raii2` rolled back a single
// transaction by restoring a snapshot. Here that is what gets ruled out.)
//
// Rust adds its own questions. `get` returns a `&str` that borrows from the
// store, so the lookup must hand out a reference into whichever layer holds
// the value, with no copy and no leak. A value can move from an inner
// transaction to an outer one without being copied. And a state that has to
// be restored is a state you can keep instead of clone.

// Problem (35 minutes). Implement `Kv`, an in-memory store of `String`
// keys and values with nested transactions:
//   - `set(key, value)` stores `value` under `key`; `get(key)` returns the
//     current value; `delete(key)` removes `key` and returns whether it was
//     there;
//   - `begin()` opens a transaction, nested inside the current one if there
//     is one. Every write goes to the innermost open transaction, and every
//     read sees all the writes made so far, committed or not;
//   - `rollback()` throws away every write of the innermost transaction;
//     `commit()` merges its writes into the transaction around it, or into
//     the store if it is the outermost one. Both close the innermost
//     transaction. With no transaction open, both return
//     `Err(TxError::NoTransaction)` and change nothing.
// Example: set a = 10; begin; set a = 20; begin; delete a; get a is None;
// rollback; get a is "20"; commit; get a is "20"; rollback is
// `Err(TxError::NoTransaction)`.
// Constraints: `begin` must not copy the store, and `commit` and `rollback`
// must take time proportional to the number of keys the transaction wrote,
// not to the size of the store. No operation copies a stored value: `get`
// borrows it, and after a `commit` or a `rollback` every value still lives
// in the allocation it was stored in, not in a clone.
//
// The reference answer: a base map plus one layer of pending writes per open
// transaction, where a layer maps a key to `Some(value)` for a write and to
// `None` (a tombstone) for a delete. `begin` pushes an empty layer, and
// `rollback` pops one. `commit` pops the top layer and moves its entries
// into the layer below (a tombstone included, since it must keep shadowing
// the value below that one), or applies them to the base. `get` looks from
// the innermost layer outward, and the first layer that mentions the key
// decides: O(depth) per read, O(written keys) per commit or rollback.

use std::collections::HashMap;

// Everything that `commit` and `rollback` can fail with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxError {
    // `commit` or `rollback` while no transaction is open.
    NoTransaction,
}

// A pending write: `Some(value)` for a `set`, `None` for a `delete`.
type Layer = HashMap<String, Option<String>>;

struct Kv {
    // The committed state. It never holds tombstones: a delete outside any
    // transaction removes the key for real.
    base: HashMap<String, String>,
    // One layer per open transaction, innermost last.
    layers: Vec<Layer>,
}

impl Kv {
    fn new() -> Self {
        Kv {
            base: HashMap::new(),
            layers: Vec::new(),
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        let (key, value) = (key.to_string(), value.to_string());
        match self.layers.last_mut() {
            Some(layer) => {
                layer.insert(key, Some(value));
            }
            None => {
                self.base.insert(key, value);
            }
        }
    }

    // The innermost layer that mentions `key` decides, and a tombstone there
    // means "deleted", whatever the layers below say. The `&str` points into
    // that layer's `String` (or the base's), so nothing is copied.
    fn get(&self, key: &str) -> Option<&str> {
        for layer in self.layers.iter().rev() {
            if let Some(entry) = layer.get(key) {
                return entry.as_deref();
            }
        }
        self.base.get(key).map(String::as_str)
    }

    // Inside a transaction the key may still exist in a layer below (or in
    // the base), which this transaction must not touch, so the delete is
    // recorded as a tombstone that shadows it.
    fn delete(&mut self, key: &str) -> bool {
        if self.get(key).is_none() {
            return false;
        }
        match self.layers.last_mut() {
            Some(layer) => {
                layer.insert(key.to_string(), None);
            }
            None => {
                self.base.remove(key);
            }
        }
        true
    }

    // O(1): an empty layer, nothing copied.
    fn begin(&mut self) {
        self.layers.push(Layer::new());
    }

    // `extend` moves every entry of the popped layer into its parent: an
    // inner write replaces the parent's write for the same key, and an inner
    // tombstone keeps shadowing whatever lies below the parent. Only the
    // outermost commit turns tombstones into real removals.
    fn commit(&mut self) -> Result<(), TxError> {
        let top = self.layers.pop().ok_or(TxError::NoTransaction)?;
        match self.layers.last_mut() {
            Some(parent) => parent.extend(top),
            None => {
                for (key, entry) in top {
                    match entry {
                        Some(value) => self.base.insert(key, value),
                        None => self.base.remove(&key),
                    };
                }
            }
        }
        Ok(())
    }

    // The layers below were never touched, so dropping the top layer is the
    // whole rollback.
    fn rollback(&mut self) -> Result<(), TxError> {
        self.layers.pop().ok_or(TxError::NoTransaction)?;
        Ok(())
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

    // The value of every key in `keys`, as `get` sees it.
    fn snapshot(kv: &Kv, keys: &[&str]) -> Vec<Option<String>> {
        keys.iter()
            .map(|key| kv.get(key).map(String::from))
            .collect()
    }

    // ---- The example from the statement ----

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
    fn set_get_and_delete_without_transactions() {
        let mut kv = Kv::new();
        assert_eq!(kv.get("a"), None);
        kv.set("a", "1");
        kv.set("b", "2");
        assert_eq!(kv.get("a"), Some("1"));
        kv.set("a", "one");
        assert_eq!(kv.get("a"), Some("one"));
        assert!(kv.delete("a"));
        assert_eq!(kv.get("a"), None);
        assert!(!kv.delete("a"));
        assert!(!kv.delete("never set"));
        assert_eq!(kv.get("b"), Some("2"));
        // The empty string is a key and a value like any other.
        kv.set("", "");
        assert_eq!(kv.get(""), Some(""));
    }

    #[test]
    fn rollback_restores_the_state_at_begin() {
        let keys = ["a", "b", "c", "d"];
        let mut kv = Kv::new();
        kv.set("a", "1");
        kv.set("b", "2");
        kv.set("c", "3");
        let before = snapshot(&kv, &keys);

        kv.begin();
        kv.set("a", "one");
        assert!(kv.delete("b"));
        kv.set("d", "4");
        kv.set("d", "four");
        assert_eq!(
            snapshot(&kv, &keys),
            [
                Some("one".into()),
                None,
                Some("3".into()),
                Some("four".into())
            ]
        );
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(snapshot(&kv, &keys), before);
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

    // ==== Follow-ups: what the interviewer asks once the examples pass ====

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

    #[test]
    fn writes_after_a_delete_in_the_same_transaction() {
        let mut kv = Kv::new();
        kv.set("a", "1");
        kv.begin();
        assert!(kv.delete("a"));
        kv.set("a", "2");
        assert_eq!(kv.get("a"), Some("2"));
        assert_eq!(kv.rollback(), Ok(()));
        assert_eq!(kv.get("a"), Some("1"));

        // A key created and deleted inside one transaction leaves no trace.
        kv.begin();
        kv.set("n", "new");
        assert!(kv.delete("n"));
        assert_eq!(kv.commit(), Ok(()));
        assert_eq!(kv.get("n"), None);
        assert!(!kv.delete("n"));
        assert_eq!(kv.get("a"), Some("1"));
    }

    #[test]
    fn a_thousand_nested_transactions() {
        let mut kv = Kv::new();
        kv.set("depth", "0");
        for depth in 1..=1_000 {
            kv.begin();
            kv.set("depth", &depth.to_string());
            kv.set(&format!("key{depth}"), "x");
        }
        assert_eq!(kv.get("depth"), Some("1000"));
        assert_eq!(kv.get("key1"), Some("x"));
        // Unwind one level at a time: each rollback exposes the value the
        // level below wrote.
        for depth in (1..=1_000).rev() {
            assert_eq!(kv.rollback(), Ok(()));
            let expected = (depth - 1).to_string();
            assert_eq!(kv.get("depth"), Some(expected.as_str()));
            assert_eq!(kv.get(&format!("key{depth}")), None);
        }
        assert_eq!(kv.rollback(), Err(TxError::NoTransaction));

        // And the other way: commit every level, innermost first.
        for depth in 1..=1_000 {
            kv.begin();
            kv.set("depth", &depth.to_string());
        }
        for _ in 0..1_000 {
            assert_eq!(kv.commit(), Ok(()));
        }
        assert_eq!(kv.get("depth"), Some("1000"));
        assert_eq!(kv.commit(), Err(TxError::NoTransaction));
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
                     times the {:?} that the same writes took on a plain \
                     `HashMap`, and more than {FACTOR} times as long in each \
                     of its last {STREAK} chunks. Does something copy the store?",
                    self.kv,
                    self.plain,
                );
            }
        }
    }

    // The constraints, timed. A copy of the store per `begin` passes every
    // test above if the copy shares its values instead of cloning them (with
    // an `Rc<str>` per value, every pointer stays where it was). It is still a
    // copy of the whole map: here, 30_000 entries per `begin`, 30_000 times.
    // Against the plain map, which just writes each key, the intended
    // solution costs a small constant factor. A copy costs a factor that
    // grows with the store, about a thousand here, so it breaks the budget
    // after a few seconds instead of running for minutes.
    #[test]
    fn begin_commit_and_rollback_never_copy_the_store() {
        let keys: Vec<String> = (0..30_000).map(|i| format!("key{i}")).collect();
        let mut kv = Kv::new();
        let mut plain = HashMap::new();
        let mut budget = Budget::default();
        budget.run(
            "setting 30_000 keys",
            0..keys.len(),
            |i| kv.set(&keys[i], "old"),
            |i| {
                plain.insert(keys[i].clone(), "old".to_string());
            },
        );
        budget.run(
            "30_000 transactions rolled back",
            0..keys.len(),
            |_| {
                kv.begin();
                kv.set("key0", "new");
                assert!(kv.delete("key1"));
                assert_eq!(kv.rollback(), Ok(()));
            },
            // The same two writes, then the two old values written back.
            |_| {
                plain.insert(keys[0].clone(), "new".to_string());
                assert!(plain.remove(&keys[1]).is_some());
                plain.insert(keys[0].clone(), "old".to_string());
                plain.insert(keys[1].clone(), "old".to_string());
            },
        );
        assert_eq!(kv.get("key0"), Some("old"));
        assert_eq!(kv.get("key1"), Some("old"));
        budget.run(
            "30_000 transactions committed",
            0..keys.len(),
            |i| {
                kv.begin();
                kv.set(&keys[i], "new");
                assert_eq!(kv.commit(), Ok(()));
            },
            |i| {
                plain.insert(keys[i].clone(), "new".to_string());
            },
        );
        assert_eq!(kv.get("key0"), Some("new"));
        assert_eq!(kv.get("key29999"), Some("new"));
    }

    // The simplest correct model: a stack of full copies of the store. The
    // last one is the current state; `begin` pushes a copy of it. Every
    // `begin` is O(n), which is fine in a test and exactly what the real
    // store must not do.
    struct Model {
        states: Vec<HashMap<String, String>>,
    }

    impl Model {
        fn current(&mut self) -> &mut HashMap<String, String> {
            self.states
                .last_mut()
                .expect("the model always has a state")
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
            }
        }
    }
}
