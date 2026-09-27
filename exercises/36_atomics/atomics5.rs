// Atomics - atomic operations and memory ordering, part 5: the ABA problem.
//
// A CAS checks that the BITS are unchanged, not that nothing happened. For
// the counter in part 4 that is fine: 7 means the same thing whenever you
// read it. It is not fine when the atomic names the top of a linked
// structure. While you are paused between your read and your CAS, the head
// can go from A to B and back to A. Your CAS still sees A, succeeds, and
// commits a decision that was based on a world that no longer exists. That
// is the ABA problem.
//
// Here is a lock-free free list (a Treiber stack) of slot indices over a
// fixed arena. `next[i]` is the slot below slot `i`, and `head` is the top.
// To pop, read the head (0) and the slot below it (1), then CAS the head
// from 0 to 1. Now a bad interleaving:
//
//   T1: reads head = 0 and next[0] = 1, then gets paused.
//   T2: pops 0, pops 1 (and keeps using slot 1), pushes 0 back.
//       The head is 0 again, but next[0] is now 2.
//   T1: CAS(head: 0 -> 1) succeeds, because the head IS 0.
//
// Slot 1, which T2 is still using, is now the top of the free list, and the
// next pop hands it out a second time. The standard fix is a VERSION TAG
// (also called a stamp, or a tagged counter): `head` packs `(tag, idx)` into
// one `AtomicU64`, and every successful CAS writes a new tag. After the
// A-B-A the index matches but the tag does not, so T1's stale CAS fails and
// it retries with a fresh snapshot. Java's `AtomicStampedReference` is the
// same idea. The limit: a 32-bit tag wraps after 2^32 updates, so a thread
// that stays paused for exactly 2^32 updates (or a multiple) could still be
// fooled. Far-fetched with 32 bits, realistic with 8 or 16, which is why
// tags are made as wide as the atomic word allows.
//
// This is safe Rust because the arena never frees anything: a stale index is
// still in bounds, so ABA here is a logic bug (a double allocation), not
// undefined behavior. A Treiber stack of heap nodes on `AtomicPtr` is
// worse: T1 reads `next` through a pointer to a node that T2 may already
// have freed (a use-after-free), and the allocator can hand that same
// address out again for a new node (the A-B-A). So the real problem there
// is safe memory reclamation, solved with hazard pointers or epoch-based
// reclamation (`crossbeam-epoch`), not with tags alone. Tagging a pointer
// is also harder: a 64-bit pointer has only a few spare bits (unused high
// address bits, alignment bits), and std has no stable 128-bit atomic for a
// double-width CAS. The `unsafe` pointer version is the Treiber-stack lab,
// deep-dive/src/treiber.rs.
//
// To make the bad interleaving deterministic, `pop` is split into
// `pop_begin` (the snapshot) and `pop_commit` (the CAS). The tests run T2's
// operations between the two calls, on a single thread.
//
// Interviewers ask you to construct this interleaving, then how tags fix it,
// what their limits are, and why garbage collection or epoch-based
// reclamation removes ABA from a pointer-based stack that allocates a fresh
// node on every push: a node's memory cannot be reused while any thread
// still holds a reference to it.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Marks the end of the list ("no slot").
const NIL: u32 = u32::MAX;

/// `head` packs a version tag (high 32 bits) and a slot index (low 32 bits).
fn pack(tag: u32, idx: u32) -> u64 {
    (u64::from(tag) << 32) | u64::from(idx)
}

/// Splits a head word back into `(tag, idx)`.
fn unpack(word: u64) -> (u32, u32) {
    ((word >> 32) as u32, word as u32)
}

/// The word to CAS into `head`, in place of `observed`, so that `new_idx`
/// becomes the top of the list. Both `push` and `pop_commit` use it.
fn next_head(observed: u64, new_idx: u32) -> u64 {
    // TODO: this ignores `observed` and always writes tag 0, so the tag
    // never changes, and after an A-B-A the head compares equal again. The
    // tests `aba_stale_commit_is_rejected`, `aba_never_relists_a_slot_in_use`
    // and `a_thousand_scripted_interleavings_never_double_allocate` fail: the
    // stale `pop_commit` returns `Ok(0)` instead of `Err(Stale)`, and slot 1
    // ends up both in use and on the free list. Give every successful CAS on
    // `head` a new version: the word you return must hold `new_idx` and a
    // tag that has moved on from the one in `observed`. Requirements:
    //   - change only this function: keep `pack` / `unpack`, `push`,
    //     `pop_begin`, `pop_commit` and the tests as they are (re-checking
    //     `next[idx]` inside `pop_commit` is not a fix: it is its own
    //     check-then-act race);
    //   - the tag must keep working past `u32::MAX` updates: tests run in
    //     debug mode, where integer overflow panics;
    //   - no `Mutex`, no `unsafe`.
    // Until you make every successful CAS write a new tag, the tests will
    // fail.
    let _ = observed;
    pack(0, new_idx)
}

/// What `pop_begin` saw: the whole head word, the slot on top and the slot
/// below it.
#[derive(Debug)]
struct PopTicket {
    observed: u64,
    idx: u32,
    next: u32,
}

/// `pop_commit` lost: the head changed after `pop_begin`.
#[derive(Debug, PartialEq, Eq)]
struct Stale;

/// A lock-free LIFO free list of the slot indices `0..n` of a fixed arena.
struct FreeList {
    head: AtomicU64,
    next: Vec<AtomicU32>,
}

impl FreeList {
    /// All `n` slots start out free, listed in order: 0 -> 1 -> ... -> n - 1.
    fn new(n: u32) -> Self {
        let next = (0..n)
            .map(|i| AtomicU32::new(if i + 1 < n { i + 1 } else { NIL }))
            .collect();
        let top = if n == 0 { NIL } else { 0 };
        FreeList {
            head: AtomicU64::new(pack(0, top)),
            next,
        }
    }

    /// Pop, step 1: snapshot the head and the link below it. `None` if the
    /// list is empty.
    fn pop_begin(&self) -> Option<PopTicket> {
        // Acquire pairs with the Release in `push`, so the `next` link that
        // the pusher wrote is visible to us.
        let observed = self.head.load(Ordering::Acquire);
        let (_, idx) = unpack(observed);
        if idx == NIL {
            return None;
        }
        // In bounds even if `observed` is already stale: the arena never
        // shrinks, which is what keeps ABA a logic bug here instead of UB.
        let next = self.next[idx as usize].load(Ordering::Relaxed);
        Some(PopTicket {
            observed,
            idx,
            next,
        })
    }

    /// Pop, step 2: make `ticket.next` the new top, but only if the head is
    /// still exactly the word `pop_begin` saw. A single STRONG CAS: a
    /// spurious failure here would wrongly report `Stale`.
    fn pop_commit(&self, ticket: PopTicket) -> Result<u32, Stale> {
        let new = next_head(ticket.observed, ticket.next);
        self.head
            .compare_exchange(ticket.observed, new, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| ticket.idx)
            .map_err(|_| Stale)
    }

    /// Takes a free slot, retrying after every lost race.
    fn pop(&self) -> Option<u32> {
        loop {
            let ticket = self.pop_begin()?;
            if let Ok(idx) = self.pop_commit(ticket) {
                return Some(idx);
            }
        }
    }

    /// Gives a slot back. The caller must own `idx` (it came from `pop`).
    fn push(&self, idx: u32) {
        let mut observed = self.head.load(Ordering::Relaxed);
        loop {
            let (_, top) = unpack(observed);
            self.next[idx as usize].store(top, Ordering::Relaxed);
            // Release publishes the `next` link above to the next popper.
            match self.head.compare_exchange_weak(
                observed,
                next_head(observed, idx),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(actual) => observed = actual,
            }
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::thread;

    // Pops until the list is empty, but at most one more time than there are
    // slots: a corrupted (even cyclic) list then fails the test instead of
    // hanging it.
    fn drain(list: &FreeList) -> Vec<u32> {
        let limit = list.next.len() + 1;
        let mut out = Vec::new();
        while out.len() < limit {
            match list.pop() {
                Some(idx) => out.push(idx),
                None => break,
            }
        }
        out
    }

    // Some earlier traffic first (pop 0, push it back), so the tag is no
    // longer its initial value. Then T1 takes its snapshot (head 0, next 1),
    // and T2 pops 0, pops 1 and pushes 0 back. Returns T1's now-stale
    // ticket; T2 still holds slot 1.
    fn aba_interleaving(list: &FreeList) -> PopTicket {
        assert_eq!(list.pop(), Some(0));
        list.push(0);
        let stale = list.pop_begin().expect("the list is not empty");
        assert_eq!((stale.idx, stale.next), (0, 1));
        assert_eq!(list.pop(), Some(0));
        assert_eq!(list.pop(), Some(1));
        list.push(0);
        // A-B-A: the top is slot 0 again, but the link below it is now 2.
        assert_eq!(unpack(list.head.load(Ordering::Relaxed)).1, 0);
        assert_eq!(list.next[0].load(Ordering::Relaxed), 2);
        stale
    }

    #[test]
    fn pops_in_lifo_order() {
        let list = FreeList::new(3);
        assert_eq!(list.pop(), Some(0));
        assert_eq!(list.pop(), Some(1));
        list.push(0);
        list.push(1);
        assert_eq!(drain(&list), [1, 0, 2]);
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn an_empty_list_has_nothing_to_pop() {
        let list = FreeList::new(0);
        assert!(list.pop_begin().is_none());
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn an_uncontended_commit_succeeds() {
        // Nothing happened between begin and commit, so the commit must win.
        let list = FreeList::new(3);
        let ticket = list.pop_begin().unwrap();
        assert_eq!(list.pop_commit(ticket), Ok(0));
        assert_eq!(drain(&list), [1, 2]);
    }

    #[test]
    fn a_commit_after_a_plain_pop_is_stale() {
        // The head moved from 0 to 1: this fails even without tags.
        let list = FreeList::new(3);
        let ticket = list.pop_begin().unwrap();
        assert_eq!(list.pop(), Some(0));
        assert_eq!(list.pop_commit(ticket), Err(Stale));
        assert_eq!(drain(&list), [1, 2]);
    }

    #[test]
    fn aba_stale_commit_is_rejected() {
        let list = FreeList::new(3);
        let stale = aba_interleaving(&list);
        assert_eq!(
            list.pop_commit(stale),
            Err(Stale),
            "the head went 0 -> 1 -> 2 -> 0: same index, but it is not the \
             head T1 saw, so T1's commit must fail"
        );
        // T1 retries with a fresh snapshot and gets the real top: slot 0.
        assert_eq!(list.pop(), Some(0));
        assert_eq!(drain(&list), [2]);
    }

    #[test]
    fn aba_never_relists_a_slot_in_use() {
        // Whatever the commit returns, every slot must end up either owned by
        // exactly one party or on the free list, never both.
        let list = FreeList::new(3);
        let stale = aba_interleaving(&list);
        let mut owned = vec![1]; // T2 is still using slot 1.
        owned.extend(list.pop_commit(stale).ok()); // T1's slot, if it got one.
        let free = drain(&list);
        for slot in &owned {
            assert!(
                !free.contains(slot),
                "slot {slot} is in use AND on the free list {free:?}, so it \
                 would be handed out twice"
            );
        }
        let mut all: Vec<u32> = free.iter().chain(&owned).copied().collect();
        all.sort_unstable();
        assert_eq!(all, [0, 1, 2], "every slot exactly once, free or owned");
    }

    #[test]
    fn the_tag_wraps_around_instead_of_overflowing() {
        // Fast-forward the tag to its last value. The next updates must wrap
        // it (debug builds panic on integer overflow) and keep working.
        let list = FreeList::new(2);
        list.head.store(pack(u32::MAX, 0), Ordering::Relaxed);
        assert_eq!(list.pop(), Some(0));
        list.push(0);
        let (tag, top) = unpack(list.head.load(Ordering::Relaxed));
        assert_eq!(top, 0);
        assert_ne!(tag, u32::MAX, "a pop and a push must change the tag");
        assert_eq!(drain(&list), [0, 1]);
    }

    #[test]
    fn a_thousand_scripted_interleavings_never_double_allocate() {
        // The same begin / "T2 runs" / commit pattern, with T2's pops and
        // pushes chosen by a fixed pseudo-random sequence: deterministic, on
        // one thread, and full of A-B-A shapes beyond the one above.
        let list = FreeList::new(4);
        let mut t2_holds: Vec<u32> = Vec::new();
        let mut seed: u32 = 0x2545_f491;
        let mut rand_below = |n: usize| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16) as usize % n
        };
        for _ in 0..1_000 {
            let Some(ticket) = list.pop_begin() else {
                // Empty: T2 must hold all four slots. Give one back.
                let slot = t2_holds.pop().expect("empty list, yet T2 holds < 4");
                list.push(slot);
                continue;
            };
            for _ in 0..=rand_below(4) {
                if rand_below(2) == 0 {
                    t2_holds.extend(list.pop());
                } else if !t2_holds.is_empty() {
                    let slot = t2_holds.swap_remove(rand_below(t2_holds.len()));
                    list.push(slot);
                }
            }
            if let Ok(slot) = list.pop_commit(ticket) {
                assert!(
                    !t2_holds.contains(&slot),
                    "T1's commit handed out slot {slot}, which T2 still holds"
                );
                list.push(slot);
            }
        }
        for slot in t2_holds {
            list.push(slot);
        }
        let mut free = drain(&list);
        free.sort_unstable();
        assert_eq!(free, [0, 1, 2, 3], "every slot must be free exactly once");
    }

    #[test]
    fn eight_threads_share_64_slots_without_double_allocation() {
        const SLOTS: u32 = 64;
        let list = FreeList::new(SLOTS);
        let in_use: Vec<AtomicBool> = (0..SLOTS).map(|_| AtomicBool::new(false)).collect();
        let double_allocations = thread::scope(|s| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        let mut doubles = 0;
                        for _ in 0..5_000 {
                            // Hold two slots at once and give them back in
                            // the order we took them: lots of A-B-A on the
                            // head for the other threads' snapshots.
                            let held = [list.pop(), list.pop()];
                            for &slot in held.iter().flatten() {
                                if in_use[slot as usize].swap(true, Ordering::AcqRel) {
                                    doubles += 1;
                                }
                            }
                            for &slot in held.iter().flatten() {
                                in_use[slot as usize].store(false, Ordering::Release);
                                list.push(slot);
                            }
                        }
                        doubles
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).sum::<u32>()
        });
        assert_eq!(double_allocations, 0, "a slot was handed out twice");
        let mut free = drain(&list);
        free.sort_unstable();
        assert!(
            free.iter().copied().eq(0..SLOTS),
            "the list must drain to 0..64 exactly once each, got {free:?}"
        );
    }
}
