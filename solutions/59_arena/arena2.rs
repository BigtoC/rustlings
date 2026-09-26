// Module 2 · Arenas — part 2: generational ids, so a stale id gets `None` instead of the slot's next value (E0308).
//
// Part 1's tree never removes a node, so an id names the same node forever.
// Real arenas do remove things: connections close, timers fire, game
// entities die. The freed slot has to be REUSED, or a program that keeps
// inserting and removing grows without bound. But reuse breaks the promise.
// Say slot 3 held Alice's connection. It closes, slot 3 goes to Bob's
// connection, and a timer that still holds "3" now writes to Bob. That is
// the index version of a use-after-free. It is memory-safe (the slot holds a
// valid value of the right type, and indexing is bounds-checked), and in one
// way that makes it worse: nothing crashes, the wrong value is silently
// used. It is the ABA problem in another form: the index reads "3" before
// and after, and a bare index cannot tell that the slot changed hands in
// between (`36_atomics/atomics5` meets the same problem in a lock-free free
// list).
//
// The standard fix is a GENERATIONAL index. Every slot carries a generation
// counter, and an id is the pair `(index, generation)`. Removing a value
// bumps its slot's generation, so every id minted for that value stops
// matching: `get` compares one integer and returns `None` for a stale id.
// This is the id scheme of `slotmap`, `thunderdome` and `generational-arena`,
// and of the entity ids in ECS game engines (a Bevy `Entity` is an index
// plus a generation). The widely used `slab` crate, by contrast, hands out
// plain `usize` keys and cannot tell a stale key from a live one.
//
// Here a slot is an enum, `Occupied { generation, value }` or
// `Vacant { generation }`, and a vacant slot holds the generation its NEXT
// value will get. Vacant indices go on a free list (`free`, used as a
// stack), and the given `insert` pops from it before it grows `slots`, so
// remove-then-insert reuses a slot instead of allocating a new one. `remove`
// has to move the `T` out of a slot that it only reaches through
// `&mut self`: that is `24_ownership_model/ownership4` and `ownership5`
// again (moving the field out directly is E0507). Replace the WHOLE slot
// with a vacant one and take the value out of the old slot, which is then
// yours.
//
// A generation is a finite counter. A `u32` has 2^32 values, so one slot can
// hold 2^32 values over its life. When the value of generation `u32::MAX`
// is removed, `generation + 1` panics in a debug build ("attempt to add with
// overflow") and wraps to 0 in a release build (where overflow checks are
// off by default), so the slot reissues generations 0, 1, 2, .. and its
// oldest ids match again. Libraries differ here: `slotmap` accepts the
// wrap-around and documents the spurious match as incredibly unlikely. The
// strict answer, and the one this exercise asks for, is to RETIRE the slot:
// leave it vacant and never put it on the free list again, which wastes one
// slot per 2^32 values it held. (A `u64` generation cannot wrap in practice:
// 2^64 reuses at a billion per second take about 585 years.)
//
// What generations do NOT catch: an id from ANOTHER slab that happens to
// have the same index and generation. That is a logic bug again, still not
// undefined behavior. Libraries answer it with a distinct key type per
// collection (`slotmap`'s `new_key_type!`) or a slab id inside every key.
//
// How interviewers probe it: "What is the stale-index problem, and how do
// generational indices detect it? Why not `Vec::remove` or `swap_remove`?
// Why not just never reuse a slot? What happens when the generation counter
// overflows? How does an ECS name its entities?"

use std::mem;

// A handle to a value in a `Slab`: which slot, and which occupant of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Id {
    index: usize,
    generation: u32,
}

enum Slot<T> {
    Occupied { generation: u32, value: T },
    // `generation` is the generation the slot's NEXT value will get.
    Vacant { generation: u32 },
}

struct Slab<T> {
    slots: Vec<Slot<T>>,
    // Indices of vacant slots that `insert` may reuse, most recent last.
    free: Vec<usize>,
    // The number of occupied slots.
    len: usize,
}

impl<T> Slab<T> {
    fn new() -> Self {
        Slab {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    // Stores `value` and returns its id. A slot on the free list is reused
    // (with the generation it was left with) before `slots` grows.
    fn insert(&mut self, value: T) -> Id {
        let id = match self.free.pop() {
            Some(index) => {
                let Slot::Vacant { generation } = self.slots[index] else {
                    panic!("slot {index} is on the free list but occupied: was it freed twice?");
                };
                self.slots[index] = Slot::Occupied { generation, value };
                Id { index, generation }
            }
            None => {
                self.slots.push(Slot::Occupied {
                    generation: 0,
                    value,
                });
                Id {
                    index: self.slots.len() - 1,
                    generation: 0,
                }
            }
        };
        self.len += 1;
        id
    }

    // The value `id` names, if it is still in the slab.
    fn get(&self, id: Id) -> Option<&T> {
        // `slots.get` turns an out-of-range index into `None` instead of a
        // panic, and the match guard rejects a vacant slot or an occupant
        // from another generation, so only the id minted for THIS value gets
        // through.
        match self.slots.get(id.index)? {
            Slot::Occupied { generation, value } if *generation == id.generation => Some(value),
            _ => None,
        }
    }

    // Like `get`, but the value can be changed in place.
    fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        // The same checks as `get`, on a mutable borrow of the slot.
        match self.slots.get_mut(id.index)? {
            Slot::Occupied { generation, value } if *generation == id.generation => Some(value),
            _ => None,
        }
    }

    // Takes the value `id` names out of the slab. Every id for it goes stale.
    fn remove(&mut self, id: Id) -> Option<T> {
        // Validate first with `get`: a rejected id returns here, before
        // anything changes, so a second `remove` cannot free the slot again.
        // (The shared borrow ends with the statement.)
        self.get(id)?;
        // `checked_add` is `None` only at `u32::MAX`. Such a slot is retired:
        // it stays vacant and never goes back on the free list.
        let next = id.generation.checked_add(1);
        let vacant = Slot::Vacant {
            generation: next.unwrap_or(id.generation),
        };
        // `mem::replace` swaps the whole slot for the vacant one and hands
        // back the old slot BY VALUE, so the value can be moved out of it
        // without E0507. No other slot moves, so every other id stays valid.
        let Slot::Occupied { value, .. } = mem::replace(&mut self.slots[id.index], vacant) else {
            unreachable!("`get` just found this slot occupied");
        };
        if next.is_some() {
            self.free.push(id.index);
        }
        self.len -= 1;
        Some(value)
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Deliberately not `Clone`: the only way to get one back out of the slab
    // is to move the original.
    #[derive(Debug, PartialEq)]
    struct Token(String);

    fn token(name: &str) -> Token {
        Token(name.to_string())
    }

    #[test]
    fn get_and_get_mut_find_live_values() {
        let mut slab = Slab::new();
        let a = slab.insert(token("a"));
        let b = slab.insert(token("b"));
        assert_eq!(slab.len(), 2);
        assert_eq!(slab.get(a), Some(&token("a")));
        assert_eq!(slab.get(b), Some(&token("b")));

        // `get_mut` reaches the stored value itself.
        slab.get_mut(b).expect("`b` is live").0.push('!');
        assert_eq!(slab.get(b), Some(&token("b!")));
        assert_eq!(slab.get(a), Some(&token("a")));
    }

    #[test]
    fn remove_hands_back_the_value_exactly_once() {
        let mut slab = Slab::new();
        let a = slab.insert(token("a"));
        let b = slab.insert(token("b"));
        let buffer = slab.get(a).expect("`a` is live").0.as_ptr();

        let removed = slab.remove(a).expect("`a` is live");
        assert_eq!(removed, token("a"));
        // The very value that went in (same heap buffer), not a rebuilt one.
        assert_eq!(removed.0.as_ptr(), buffer);
        assert_eq!(slab.len(), 1);

        // After that, `a` names nothing.
        assert_eq!(slab.get(a), None);
        assert!(slab.get_mut(a).is_none());
        assert_eq!(slab.remove(a), None);
        assert_eq!(slab.len(), 1);
        assert_eq!(slab.get(b), Some(&token("b")));
    }

    #[test]
    fn a_stale_id_does_not_reach_the_slots_next_value() {
        let mut slab = Slab::new();
        let alice = slab.insert(token("alice"));
        assert!(slab.remove(alice).is_some());
        let bob = slab.insert(token("bob"));

        // The slot was reused, with a new generation.
        assert_eq!(bob.index, alice.index, "the free slot should be reused");
        assert_ne!(bob.generation, alice.generation);
        assert_eq!(slab.slots.len(), 1);

        // Alice's old id can neither see, change nor remove Bob's value.
        assert_eq!(slab.get(alice), None);
        assert!(slab.get_mut(alice).is_none());
        assert_eq!(slab.remove(alice), None);
        assert_eq!(slab.get(bob), Some(&token("bob")));
        assert_eq!(slab.len(), 1);
    }

    #[test]
    fn removing_values_keeps_every_other_id_valid() {
        let mut slab = Slab::new();
        let ids = ["a", "b", "c", "d", "e"].map(|name| slab.insert(token(name)));

        // Remove from the middle and from the front. `Vec::remove` would
        // shift the later values down and `swap_remove` would move the last
        // one: either way, ids that were never removed would break.
        assert_eq!(slab.remove(ids[2]), Some(token("c")));
        assert_eq!(slab.remove(ids[0]), Some(token("a")));
        for (id, name) in [(ids[1], "b"), (ids[3], "d"), (ids[4], "e")] {
            assert_eq!(slab.get(id), Some(&token(name)), "{id:?}");
        }
        assert_eq!(slab.len(), 3);

        // The two holes are filled before `slots` grows.
        let x = slab.insert(token("x"));
        let y = slab.insert(token("y"));
        let mut reused = [x.index, y.index];
        reused.sort_unstable();
        assert_eq!(reused, [ids[0].index, ids[2].index]);
        assert_eq!(slab.slots.len(), 5);
        assert_eq!(slab.len(), 5);
        assert_eq!(slab.get(x), Some(&token("x")));
        assert_eq!(slab.get(y), Some(&token("y")));
        assert_eq!(slab.get(ids[4]), Some(&token("e")));
    }

    #[test]
    fn removing_twice_frees_the_slot_once() {
        let mut slab = Slab::new();
        let a = slab.insert(token("a"));
        assert!(slab.remove(a).is_some());
        assert_eq!(slab.remove(a), None, "the second remove finds nothing");
        assert_eq!(slab.len(), 0);

        // If the second `remove` had put the index on the free list again,
        // both inserts below would be given the same slot.
        let x = slab.insert(token("x"));
        let y = slab.insert(token("y"));
        assert_ne!(x.index, y.index);
        assert_eq!(slab.get(x), Some(&token("x")));
        assert_eq!(slab.get(y), Some(&token("y")));
        assert_eq!(slab.len(), 2);
    }

    #[test]
    fn ids_that_were_never_handed_out_are_none() {
        let mut slab: Slab<Token> = Slab::new();
        let nothing_yet = Id {
            index: 0,
            generation: 0,
        };
        assert_eq!(slab.get(nothing_yet), None);
        assert_eq!(slab.remove(nothing_yet), None);

        let a = slab.insert(token("a"));
        // Out of range, and a generation this slot has not reached yet.
        let out_of_range = Id {
            index: 7,
            generation: 0,
        };
        let future = Id {
            index: a.index,
            generation: a.generation + 1,
        };
        for id in [out_of_range, future] {
            assert_eq!(slab.get(id), None, "{id:?}");
            assert!(slab.get_mut(id).is_none(), "{id:?}");
            assert_eq!(slab.remove(id), None, "{id:?}");
        }
        assert_eq!(slab.len(), 1);
        assert_eq!(slab.get(a), Some(&token("a")));

        // Once `a` is removed, the vacant slot holds `future`'s generation.
        // That still names no value, and removing it must not free the slot
        // a second time.
        assert!(slab.remove(a).is_some());
        assert_eq!(slab.get(future), None);
        assert_eq!(slab.remove(future), None);
        let x = slab.insert(token("x"));
        let y = slab.insert(token("y"));
        assert_ne!(x.index, y.index);
        assert_eq!(slab.len(), 2);
    }

    #[test]
    fn ten_thousand_insert_remove_cycles_reuse_a_single_slot() {
        let mut slab = Slab::new();
        let first = slab.insert(token("0"));
        assert!(slab.remove(first).is_some());

        let mut handed_out = HashSet::from([first]);
        let mut previous = first;
        for i in 1..10_000 {
            let name = i.to_string();
            let id = slab.insert(token(&name));
            assert!(handed_out.insert(id), "{id:?} was handed out twice");
            // The id from the previous round is stale by now.
            assert_eq!(slab.get(previous), None, "{previous:?} should be stale");
            assert_eq!(slab.remove(id), Some(token(&name)));
            previous = id;
        }
        assert_eq!(slab.slots.len(), 1, "the one slot should be reused");
        assert_eq!(slab.len(), 0);
        assert_eq!(slab.get(first), None);
    }

    #[test]
    fn a_slot_out_of_generations_is_retired() {
        // Four billion reuses would take a while, so the test builds a slab
        // whose only slot is already at its last generation.
        let last = Id {
            index: 0,
            generation: u32::MAX,
        };
        let mut slab = Slab {
            slots: vec![Slot::Occupied {
                generation: u32::MAX,
                value: token("last"),
            }],
            free: Vec::new(),
            len: 1,
        };
        assert_eq!(slab.get(last), Some(&token("last")));

        // Removing it works (no "attempt to add with overflow")...
        assert_eq!(slab.remove(last), Some(token("last")));
        assert_eq!(slab.get(last), None);
        assert_eq!(slab.len(), 0);

        // ...but slot 0 has no generation left for another value. Wrapping
        // around to 0 would revive the ids of generation 0, and staying at
        // `u32::MAX` would revive `last` itself. So the next insert must get
        // a new slot.
        let next = slab.insert(token("next"));
        assert_ne!(next.index, 0, "a slot with no generations left was reused");
        assert_eq!(slab.slots.len(), 2);
        assert_eq!(slab.get(next), Some(&token("next")));
        assert_eq!(slab.get(last), None);
        let wrapped = Id {
            index: 0,
            generation: 0,
        };
        assert_eq!(slab.get(wrapped), None);
    }
}
