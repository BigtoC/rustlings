//! Lab · a Treiber stack on `AtomicPtr`: the CAS is easy, reclamation is hard
//!
//! A Treiber stack is the "hello world" of lock-free data structures: a singly
//! linked list whose top pointer is an `AtomicPtr`, changed only by
//! compare-and-swap. `36_atomics/atomics5` builds its safe cousin, a free list
//! of arena indices, and fixes ABA there with a version tag. With real heap
//! nodes the problem changes shape: the danger is no longer a stale index but
//! a stale *pointer*, to memory that may already be freed.
//!
//! A pop is three steps:
//!
//! ```text
//! 1. top  = head.load(Acquire)       pop_begin
//! 2. next = (*top).next              pop_commit: the dangerous read
//! 3. CAS(head: top -> next)          pop_commit
//! ```
//!
//! Between 1 and 2 another thread can pop `top`. If that pop freed the node,
//! step 2 is a use-after-free. If the allocator then hands the same address to
//! a newly pushed node, step 3 finds "`top`" on top again and succeeds with
//! the stale `next`: the ABA problem, now on pointers.
//!
//! # Sharpest question (second half)
//!
//! *Why is memory reclamation the hard part of a lock-free stack?* The CAS
//! makes each push and pop atomic, but it cannot stop another thread from
//! freeing the node you are about to dereference. You may free a node only
//! once no thread can still hold a pointer to it, and a lock-free structure
//! has no lock whose holder would know that. Every real answer (below) is a
//! way of finding out, or of waiting until it is safe. The first half of the
//! question, store buffering, is answered in [`crate::ordering_lab`].
//!
//! # The bug, deterministic, and the fix
//!
//! As in atomics5, pop is split into [`TreiberStack::pop_begin`] (step 1) and
//! [`TreiberStack::pop_commit`] (steps 2 and 3), so a test can run the bad
//! interleaving on one thread. (atomics5's `pop_begin` also reads the link;
//! here the read waits for `pop_commit`, so a stale ticket reaches it.)
//!
//! - [`TreiberStack::new_eager_free`] (BROKEN, `unsafe` to construct) frees
//!   each popped node at once, the natural first attempt. It passes every
//!   single-threaded test (`the_eager_free_stack_passes_single_threaded_tests`)
//!   and is undefined behavior as soon as a ticket goes stale. The
//!   `#[ignore]`d `eager_free_stale_commit_is_a_use_after_free` runs begin,
//!   pop, commit; natively it passes (debug and release, on the machine this
//!   was written on), and Miri, with either aliasing model, reports:
//!   "Undefined Behavior: constructing invalid value of type
//!   `&std::sync::atomic::Atomic<*mut treiber::Node<char>>`: encountered a
//!   dangling reference (use-after-free)".
//! - [`TreiberStack::new`], what ships, defers reclamation: a popped node goes
//!   onto a retired list and is freed only in `Drop`, when `&mut self` proves
//!   that no thread is between steps 1 and 3. The same interleaving
//!   (`deferred_reclamation_makes_the_stale_commit_safe`) then reads a retired
//!   but still allocated node, and the CAS fails because the top moved on.
//!   And because a retired node's address cannot be reused while the stack
//!   lives, pointer ABA cannot happen either
//!   (`no_aba_while_popped_nodes_stay_allocated`): no version tag needed.
//!
//! The price is memory: every popped node stays allocated until the stack is
//! dropped ([`TreiberStack::retired_len`]). Production schemes free nodes
//! while the structure lives, as soon as no thread can still reach them:
//!
//! - **Epoch-based reclamation** (the `crossbeam-epoch` crate): a thread
//!   *pins* the current global epoch before touching the structure and unpins
//!   afterwards. A retired node is tagged with the epoch it was retired in.
//!   The global epoch advances only once every pinned thread has observed the
//!   current one, and a node is freed after the epoch has moved on twice, by
//!   which time no pinned thread can still hold a pointer to it. Cheap per
//!   operation, but one thread stalled while pinned stops all reclamation.
//! - **Hazard pointers**: before dereferencing `top`, a thread publishes it in
//!   a per-thread hazard slot (then re-checks that `head` still equals it); a
//!   retiring thread frees a node only if no hazard slot names it. Bounded
//!   garbage, at the cost of a store, a fence and a re-check on every read.
//! - **Reference counting or a garbage collector**: a node lives while anyone
//!   points at it, which is why a Treiber stack in Java needs no reclamation
//!   code at all.
//!
//! The ROADMAP's optional crossbeam-epoch variant is left out: it would add
//! an external dependency to a crate that has none outside `cfg(loom)`.
//!
//! A lock makes the problem disappear: a popper holding a lock can free the
//! node at once, because no other thread can be between steps 1 and 3. So the
//! module also carries `36_atomics/atomics3`'s [`SpinLock`], model-checked by
//! loom. Its models add the other half of the ordering story: with `Relaxed`
//! on the CAS and on the unlock, mutual exclusion still holds (the CAS is
//! atomic), yet the next holder may miss the previous holder's writes, so a
//! counter bumped in two critical sections that never overlapped still loses
//! an update. Both halves are needed: `Acquire` with a `Relaxed` unlock, or a
//! `Relaxed` CAS with a `Release` unlock, lose the update too (checked with
//! loom).
//!
//! Compare with `36_atomics/atomics5` (ABA on arena indices, fixed with a
//! version tag), `36_atomics/atomics3` (the spinlock), [`crate::myarc`] (the
//! Release/Acquire edge that must precede a free) and `loom_lab` (message
//! passing under loom); [`crate::ordering_lab`] covers when Release/Acquire is
//! not enough.
//!
//! # Memory orderings
//!
//! - `push`: `Release` on the CAS that installs the node, publishing its value
//!   and its `next` link.
//! - `pop_begin`: `Acquire` load of `head`. Every write to `head` is a CAS (a
//!   read-modify-write), so it extends the release sequence of the push that
//!   installed the node, and the load synchronizes with that push whichever
//!   CAS it read from.
//! - `pop_commit`: `Acquire` on success, like atomics5; `Relaxed` also passes
//!   loom, since `pop_begin` already did the synchronizing.
//! - The retired list is `Relaxed`: it is only read with `&mut self`.
//!
//! # Invariants
//!
//! - A node's `next` is written only before the node is published, so a
//!   stale popper may read it at any time while the node is allocated.
//! - Exactly one `pop_commit` wins the CAS that unlinks a given node; the
//!   winner alone moves the value out and retires (or frees) the node.
//! - Nodes on the stack own a live value; retired nodes' values have been
//!   moved out. `Drop` drops each live value once and frees every node once
//!   (drop counters in `every_value_is_dropped_exactly_once` and
//!   `threads_push_and_pop_each_value_exactly_once`).
//! - A [`PopTicket`] borrows its stack, so `Drop` cannot run while a ticket is
//!   alive, and committing a ticket on another stack panics.
//! - `TreiberStack<T>` is `Send + Sync` exactly when `T: Send` (the doctests
//!   on [`TreiberStack`]).
//!
//! # Run it
//!
//! ```text
//! cargo test --manifest-path deep-dive/Cargo.toml treiber
//!
//! # loom: the stack and atomics3's spinlock
//! RUSTFLAGS="--cfg loom" cargo test --manifest-path deep-dive/Cargo.toml \
//!     --lib treiber::loom_tests
//!
//! # Miri: the shipped stack is clean ...
//! MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test \
//!     --manifest-path deep-dive/Cargo.toml treiber
//!
//! # ... and the eager-free stack is a use-after-free
//! cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib \
//!     treiber::tests::eager_free_stale_commit_is_a_use_after_free -- --ignored
//! ```
//!
//! # Try it
//!
//! - Weaken the `Release` in `push`, or the `Acquire` in `pop_begin`, to
//!   `Relaxed` and run the loom models: loom panics with "Causality
//!   violation: Concurrent load and mut accesses." (the popper reads `next`
//!   without being ordered after the push that initialized it). Weaken the
//!   `Acquire` in `pop_commit` instead and loom still passes.
//! - In `SpinLock::new`, change `with_orderings(Acquire, Release)` to
//!   `(Acquire, Relaxed)` or `(Relaxed, Release)`: loom's
//!   `spinlock_acquire_release_publishes_the_critical_section` fails with
//!   "spinlock: lost update".
//! - Delete the retired-list loop in `Drop`: every test still passes natively,
//!   and Miri fails the run at exit with "error: memory leaked".
//! - Delete the two `unsafe impl`s: the auto-derived `Sync` then needs
//!   `T: Sync`, so the positive-control doctest and
//!   `the_stack_is_send_and_sync_for_send_values` stop compiling, while
//!   `TreiberStack<MutexGuard<'_, u8>>` (`Sync` but not `Send`) becomes
//!   `Sync`, which would be unsound.

use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::{self, NonNull};
use std::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

// The stack's and the spinlock's atomics: std's in a normal build, loom's
// instrumented copies under `--cfg loom`, so the loom models at the bottom of
// this file check exactly this code.
#[cfg(loom)]
use loom::{
    hint,
    sync::atomic::{AtomicBool, AtomicPtr},
};
#[cfg(not(loom))]
use std::{
    hint,
    sync::atomic::{AtomicBool, AtomicPtr},
};

/// One heap node of the stack.
struct Node<T> {
    /// Moved out by the pop that unlinks the node; `ManuallyDrop` so freeing
    /// the node afterwards does not drop it a second time.
    value: ManuallyDrop<T>,
    /// The node below this one. Written before the node is published and
    /// never again, so a popper holding a stale pointer to this node may
    /// still read it, as long as the node has not been freed. It is an
    /// atomic (accessed `Relaxed`, which costs nothing over a plain field)
    /// so that loom tracks it: weaken the `Release` in `push` or the
    /// `Acquire` in `pop_begin` and loom reports a causality violation on
    /// this field.
    next: AtomicPtr<Node<T>>,
    /// Link in the list of retired nodes, written only by the one pop that
    /// unlinked this node.
    retired_next: *mut Node<T>,
}

/// A lock-free LIFO stack (R. Kent Treiber, 1986): a singly linked list of
/// heap nodes whose top is an `AtomicPtr`, changed only by compare-and-swap.
///
/// Popped nodes are not freed when they are popped: they are *retired* onto
/// a second list and freed in `Drop` (deferred reclamation). See the module
/// docs for why, and [`TreiberStack::new_eager_free`] for what goes wrong
/// otherwise.
///
/// The stack is `Send` and `Sync` whenever `T: Send`. It never hands out a
/// `&T`, only moves values in and out, so `T` does not need to be `Sync`.
/// It must not be `Sync` for a `T` that is not `Send`, though: that would
/// let a push on one thread and a pop on another move a `!Send` value
/// across threads. (That is why the `unsafe impl`s below are written out:
/// the auto-derived ones would require `T: Sync` for `Sync` and would be
/// satisfied by a `!Send` but `Sync` type.)
///
/// ```compile_fail,E0277
/// fn assert_sync<S: Sync>() {}
/// assert_sync::<deep_dive::treiber::TreiberStack<std::rc::Rc<u8>>>();
/// ```
///
/// (E0277: "`Rc<u8>` cannot be sent between threads safely". Stable rustdoc
/// only checks that this fails to compile; nightly rustdoc, which the Miri CI
/// job runs, also checks the error code.) The positive control, one type
/// argument away (`Cell` is `Send` but not `Sync`, and the stack is still
/// `Sync`):
///
/// ```
/// fn assert_sync<S: Sync>() {}
/// assert_sync::<deep_dive::treiber::TreiberStack<std::cell::Cell<u8>>>();
/// ```
pub struct TreiberStack<T> {
    head: AtomicPtr<Node<T>>,
    /// Popped nodes, kept allocated until `Drop`. A push-only lock-free
    /// list: nothing pops from it concurrently, so it has neither an ABA
    /// nor a reclamation problem of its own.
    retired: AtomicPtr<Node<T>>,
    /// `true` only for the deliberately broken
    /// [`TreiberStack::new_eager_free`].
    eager_free: bool,
    /// The stack owns `T` values (for drop check).
    _owns: PhantomData<T>,
}

// SAFETY: the stack moves `T` values between threads and never shares a
// `&T`, so `T: Send` is what both need; nodes are reached only through the
// atomics and are freed only in `Drop`, which has `&mut self` (or, for the
// `unsafe` `new_eager_free`, under its contract).
unsafe impl<T: Send> Send for TreiberStack<T> {}
// SAFETY: see above; `&TreiberStack<T>` only allows push and pop, which move
// whole `T` values, so sharing the stack is sound for any `T: Send`.
unsafe impl<T: Send> Sync for TreiberStack<T> {}

/// What [`TreiberStack::pop_begin`] saw: the node on top of the stack. It has
/// not been dereferenced yet, and by the time [`TreiberStack::pop_commit`]
/// dereferences it another thread may have popped it.
pub struct PopTicket<'s, T> {
    stack: &'s TreiberStack<T>,
    head: NonNull<Node<T>>,
}

/// [`TreiberStack::pop_commit`] lost: the top of the stack changed after
/// [`TreiberStack::pop_begin`].
#[derive(Debug, PartialEq, Eq)]
pub struct Stale;

impl<T> TreiberStack<T> {
    /// An empty stack with deferred reclamation.
    pub fn new() -> Self {
        Self::with_eager_free(false)
    }

    /// BROKEN ON PURPOSE: an empty stack that frees each popped node
    /// immediately, the way a first attempt usually does.
    ///
    /// # Safety
    ///
    /// While a [`PopTicket`] of this stack is alive, from its
    /// [`pop_begin`](Self::pop_begin) until it is committed or dropped, no
    /// other pop may run on this stack: that pop could free the node the
    /// ticket points at, and the commit would then read freed memory. On one
    /// thread you can keep that promise; with several threads popping
    /// concurrently you cannot, which is the point. The `#[ignore]`d test
    /// `eager_free_stale_commit_is_a_use_after_free` breaks the promise and
    /// Miri reports the use-after-free.
    pub unsafe fn new_eager_free() -> Self {
        Self::with_eager_free(true)
    }

    fn with_eager_free(eager_free: bool) -> Self {
        TreiberStack {
            head: AtomicPtr::new(ptr::null_mut()),
            retired: AtomicPtr::new(ptr::null_mut()),
            eager_free,
            _owns: PhantomData,
        }
    }

    /// Pushes `value` on top: allocate a node, point it at the current top,
    /// and CAS it in, retrying if another thread changed the top meanwhile.
    pub fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            value: ManuallyDrop::new(value),
            next: AtomicPtr::new(ptr::null_mut()),
            retired_next: ptr::null_mut(),
        }));
        let mut top = self.head.load(Relaxed);
        loop {
            // SAFETY: `node` came from `Box::into_raw` above and is not
            // published yet, so it is valid and only this thread can see it.
            unsafe { (*node).next.store(top, Relaxed) };
            // Release: whoever loads `node` from `head` with Acquire also
            // sees its value and its `next` link. The CAS is weak because a
            // spurious failure just costs one more trip round this loop.
            match self.head.compare_exchange_weak(top, node, Release, Relaxed) {
                Ok(_) => return,
                Err(actual) => top = actual,
            }
        }
    }

    /// Pop, step 1: snapshot the top node. `None` if the stack is empty.
    pub fn pop_begin(&self) -> Option<PopTicket<'_, T>> {
        // Acquire pairs with the Release CAS in `push`. Every change of
        // `head` is a CAS (a read-modify-write), so it continues the release
        // sequence of the push that installed the node we load, whichever
        // CAS we read from.
        let head = NonNull::new(self.head.load(Acquire))?;
        Some(PopTicket { stack: self, head })
    }

    /// Pop, step 2: read the snapshot node's `next` link and CAS `head` from
    /// the node to that link. `Err(Stale)` if the top changed since
    /// `pop_begin`; on success the value is moved out and the node retired.
    ///
    /// The read of `next` is the dangerous step of every Treiber pop: the
    /// node was the top when `pop_begin` loaded it, but nothing stopped
    /// another thread from popping it since.
    pub fn pop_commit(&self, ticket: PopTicket<'_, T>) -> Result<T, Stale> {
        assert!(
            ptr::eq(ticket.stack, self),
            "a PopTicket can only be committed on the stack that issued it"
        );
        let node = ticket.head.as_ptr();
        // SAFETY: `node` was pushed by this stack. With deferred reclamation
        // no node is freed before `Drop`, which cannot run while `ticket`
        // borrows the stack, so `node` is still allocated even if another
        // thread popped it meanwhile, and `next` never changes after the
        // push. With `new_eager_free` this holds only under that
        // constructor's contract, and breaking the contract makes this line
        // a use-after-free.
        let next = unsafe { (*node).next.load(Relaxed) };
        // Acquire on success, like atomics5's `pop_commit`. `Relaxed` would
        // also do (loom agrees): the Acquire load in `pop_begin` already
        // made the node's contents visible.
        self.head
            .compare_exchange(node, next, Acquire, Relaxed)
            .map_err(|_| Stale)?;
        // SAFETY: `node` is still allocated (see the read of `next` above),
        // so projecting to its field stays in bounds of a live allocation.
        let value_ptr = unsafe { &raw const (*node).value };
        // SAFETY: our CAS unlinked `node`, so no later `pop_begin` can return
        // it, and exactly one commit can win the CAS for a given node: we
        // alone own its value and move it out exactly once. The value's
        // write happens-before this read (the push's Release CAS,
        // synchronized by our Acquire loads).
        let value = ManuallyDrop::into_inner(unsafe { ptr::read(value_ptr) });
        if self.eager_free {
            // BROKEN ON PURPOSE: another thread may hold a ticket for `node`
            // and be about to read `node.next` (see `new_eager_free`).
            // SAFETY: `node` came from `Box::into_raw` in `push`. Only under
            // `new_eager_free`'s contract: no other ticket of this stack is
            // alive, so no one else can still reach `node`. The value was
            // moved out above, and `ManuallyDrop` keeps freeing the node from
            // dropping it again.
            unsafe { drop(Box::from_raw(node)) };
        } else {
            self.retire(node);
        }
        Ok(value)
    }

    /// Pops the top value, retrying `pop_begin` + `pop_commit` after every
    /// lost race. `None` if the stack is empty.
    pub fn pop(&self) -> Option<T> {
        loop {
            let ticket = self.pop_begin()?;
            if let Ok(value) = self.pop_commit(ticket) {
                return Some(value);
            }
        }
    }

    /// `true` if the stack had no elements when we looked.
    pub fn is_empty(&self) -> bool {
        self.head.load(Acquire).is_null()
    }

    /// How many popped nodes are still allocated, waiting for `Drop`. This is
    /// the price of the simplest safe reclamation scheme: memory grows with
    /// the number of pops until the whole stack is dropped.
    pub fn retired_len(&mut self) -> usize {
        let mut len = 0;
        let mut node = self.retired.load(Relaxed);
        while !node.is_null() {
            len += 1;
            // SAFETY: retired nodes stay allocated until `Drop`, and `&mut
            // self` means no pop is running, so no `retired_next` is being
            // written.
            node = unsafe { (*node).retired_next };
        }
        len
    }

    /// Pushes an unlinked node onto the retired list.
    fn retire(&self, node: *mut Node<T>) {
        let mut top = self.retired.load(Relaxed);
        loop {
            // SAFETY: our CAS in `pop_commit` unlinked `node`, so we are the
            // only thread that ever writes its `retired_next`; stale poppers
            // only read `next`, a different field.
            unsafe { (*node).retired_next = top };
            // Relaxed is enough: the retired list is only walked with `&mut
            // self` (`retired_len`, `Drop`), and whatever handed out that
            // `&mut` (a join, the end of a thread scope) already ordered
            // every retire before it.
            match self
                .retired
                .compare_exchange_weak(top, node, Relaxed, Relaxed)
            {
                Ok(_) => return,
                Err(actual) => top = actual,
            }
        }
    }
}

impl<T> Default for TreiberStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for TreiberStack<T> {
    fn drop(&mut self) {
        // `&mut self`: no ticket is alive (tickets borrow the stack) and no
        // pop is running, so nothing can still be reading any node. This is
        // the moment deferred reclamation was waiting for.
        let mut node = self.head.load(Relaxed);
        while !node.is_null() {
            // SAFETY: a node still on the stack: allocated by `push` through
            // `Box`, linked exactly once, value never moved out. We free it
            // exactly once, here.
            let mut boxed = unsafe { Box::from_raw(node) };
            node = boxed.next.load(Relaxed);
            // SAFETY: the value is still inside (it was never popped) and is
            // dropped exactly once, here.
            unsafe { ManuallyDrop::drop(&mut boxed.value) };
        }
        let mut node = self.retired.load(Relaxed);
        while !node.is_null() {
            // SAFETY: a retired node: popped, its value already moved out
            // (and `ManuallyDrop`, so freeing the box does not drop it
            // again), retired exactly once, so freed exactly once.
            let boxed = unsafe { Box::from_raw(node) };
            node = boxed.retired_next;
        }
    }
}

// ---------------------------------------------------------------------------
// atomics3's spinlock, for loom
// ---------------------------------------------------------------------------

/// The spinlock from `36_atomics/atomics3`, with its two orderings stored in
/// the lock so the loom models can show what each one buys.
pub struct SpinLock {
    locked: AtomicBool,
    /// Success ordering of the CAS that takes the lock.
    acquire: Ordering,
    /// Ordering of the store that releases it.
    release: Ordering,
}

impl SpinLock {
    /// atomics3's lock: `Acquire` on the winning CAS, `Release` on unlock.
    pub fn new() -> Self {
        Self::with_orderings(Acquire, Release)
    }

    /// BROKEN ON PURPOSE: `Relaxed` on both. The CAS still lets only one
    /// thread hold the lock at a time, but nothing orders one critical
    /// section before the next, so the next holder may miss the previous
    /// holder's writes.
    pub fn relaxed() -> Self {
        Self::with_orderings(Relaxed, Relaxed)
    }

    fn with_orderings(acquire: Ordering, release: Ordering) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            acquire,
            release,
        }
    }

    /// One attempt to flip the flag `false -> true`.
    pub fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, self.acquire, Relaxed)
            .is_ok()
    }

    pub fn lock(&self) {
        while !self.try_lock() {
            // Under loom this yields, which keeps the model finite.
            hint::spin_loop();
        }
    }

    pub fn unlock(&self) {
        self.locked.store(false, self.release);
    }
}

impl Default for SpinLock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::atomic::AtomicUsize;
    use std::thread;

    /// A payload that counts its drops in a shared counter.
    struct Tracked<'a> {
        id: usize,
        drops: &'a AtomicUsize,
    }

    impl Drop for Tracked<'_> {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Relaxed);
        }
    }

    fn spawn_scoped<'scope, T: Send + 'scope>(
        s: &'scope thread::Scope<'scope, '_>,
        f: impl FnOnce() -> T + Send + 'scope,
    ) -> thread::ScopedJoinHandle<'scope, T> {
        thread::Builder::new()
            .spawn_scoped(s, f)
            .expect("failed to spawn a thread")
    }

    #[test]
    fn pops_in_lifo_order() {
        let stack = TreiberStack::new();
        for i in 1..=3 {
            stack.push(i);
        }
        assert_eq!(stack.pop(), Some(3));
        stack.push(4);
        assert_eq!(stack.pop(), Some(4));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
        assert!(stack.is_empty());
    }

    #[test]
    fn an_uncontended_commit_succeeds() {
        let stack = TreiberStack::new();
        stack.push('a');
        stack.push('b');
        let ticket = stack.pop_begin().unwrap();
        assert_eq!(stack.pop_commit(ticket), Ok('b'));
        assert_eq!(stack.pop(), Some('a'));
        assert!(stack.pop_begin().is_none());
    }

    #[test]
    fn deferred_reclamation_makes_the_stale_commit_safe() {
        // The exact interleaving of the #[ignore]d use-after-free test below,
        // on the safe stack. T1 snapshots the top (node 'b'); T2 pops 'b';
        // T1 commits. 'b' is retired, not freed, so T1's read of its `next`
        // link is fine, and its CAS fails because the top moved on.
        let mut stack = TreiberStack::new();
        stack.push('a');
        stack.push('b');
        let ticket = stack.pop_begin().unwrap();
        assert_eq!(stack.pop(), Some('b'));
        assert_eq!(stack.pop_commit(ticket), Err(Stale));
        // T1 retries and gets the real top.
        assert_eq!(stack.pop(), Some('a'));
        assert_eq!(stack.retired_len(), 2);
    }

    #[test]
    fn no_aba_while_popped_nodes_stay_allocated() {
        // The Treiber-stack ABA: T1 snapshots node A; T2 pops A and pushes a
        // new value. If A had been freed, the allocator could hand A's
        // address to the new node, T1's CAS would see "A" on top again and
        // succeed with A's stale `next`. Retired nodes are still allocated,
        // so the new node cannot get A's address and the commit fails.
        let stack = TreiberStack::new();
        stack.push(1);
        stack.push(2);
        let ticket = stack.pop_begin().unwrap();
        let a = ticket.head;
        assert_eq!(stack.pop(), Some(2));
        stack.push(3);
        let new_top = stack.pop_begin().unwrap().head;
        assert_ne!(new_top, a, "a live allocation's address was reused");
        assert_eq!(stack.pop_commit(ticket), Err(Stale));
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(1));
    }

    #[test]
    #[should_panic(expected = "stack that issued it")]
    fn a_ticket_from_another_stack_is_rejected() {
        let (one, two) = (TreiberStack::new(), TreiberStack::new());
        one.push(1);
        two.push(2);
        let ticket = one.pop_begin().unwrap();
        let _ = two.pop_commit(ticket);
    }

    #[test]
    fn every_value_is_dropped_exactly_once() {
        let drops = AtomicUsize::new(0);
        {
            let mut stack = TreiberStack::new();
            for id in 0..10 {
                stack.push(Tracked { id, drops: &drops });
            }
            for expected in (6..10).rev() {
                let popped = stack.pop().unwrap();
                assert_eq!(popped.id, expected);
            }
            // The 4 popped values were dropped by us; their nodes are retired
            // but freeing them later must not drop the values again.
            assert_eq!(drops.load(Relaxed), 4);
            assert_eq!(stack.retired_len(), 4);
            // Dropping the stack drops the 6 values still on it, once each.
        }
        assert_eq!(drops.load(Relaxed), 10);
    }

    #[test]
    fn retired_nodes_wait_for_drop() {
        let mut stack = TreiberStack::new();
        for round in 0..5 {
            stack.push(round);
            stack.pop();
        }
        assert!(stack.is_empty());
        assert_eq!(
            stack.retired_len(),
            5,
            "every popped node is still allocated"
        );
    }

    #[test]
    fn the_eager_free_stack_passes_single_threaded_tests() {
        // The broken variant looks fine as long as no ticket goes stale:
        // push, pop, LIFO order and drop counts all check out. Ordinary
        // tests cannot tell the two stacks apart.
        let drops = AtomicUsize::new(0);
        {
            // SAFETY: one thread, and every pop_begin is committed before
            // the next pop starts, which is `new_eager_free`'s contract.
            let stack = unsafe { TreiberStack::new_eager_free() };
            for id in 0..5 {
                stack.push(Tracked { id, drops: &drops });
            }
            let ticket = stack.pop_begin().unwrap();
            assert_eq!(stack.pop_commit(ticket).ok().map(|t| t.id), Some(4));
            assert_eq!(stack.pop().map(|t| t.id), Some(3));
            assert_eq!(drops.load(Relaxed), 2);
        }
        assert_eq!(drops.load(Relaxed), 5);
    }

    /// THE BUG, made deterministic on one thread with the split pop API: T1
    /// snapshots the top node, T2 pops (and, on this stack, frees) it, then
    /// T1's commit reads the freed node's `next` link. Undefined behavior:
    /// natively it usually "passes", which is why it is `#[ignore]`d. Run it
    /// under Miri to see the use-after-free reported:
    ///
    /// ```text
    /// cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib \
    ///     treiber::tests::eager_free_stale_commit_is_a_use_after_free \
    ///     -- --ignored
    /// ```
    #[test]
    #[ignore = "undefined behavior on purpose; run by hand under Miri"]
    fn eager_free_stale_commit_is_a_use_after_free() {
        // SAFETY: VIOLATED ON PURPOSE. `new_eager_free` requires that no
        // other pop runs between a pop_begin and its pop_commit; the second
        // `pop` below does exactly that, so the commit reads freed memory.
        let stack = unsafe { TreiberStack::new_eager_free() };
        stack.push('a');
        stack.push('b');
        let ticket = stack.pop_begin().unwrap(); // T1: top is node 'b'
        assert_eq!(stack.pop(), Some('b')); // T2: pops and frees node 'b'
        let _ = stack.pop_commit(ticket); // T1: reads freed 'b'.next
    }

    #[test]
    fn threads_push_and_pop_each_value_exactly_once() {
        const THREADS: usize = 4;
        let per_thread = if cfg!(miri) { 20 } else { 2_000 };
        let drops = AtomicUsize::new(0);
        let mut stack = TreiberStack::new();
        let mut seen: Vec<usize> = thread::scope(|s| {
            let handles: Vec<_> = (0..THREADS)
                .map(|t| {
                    let (stack, drops) = (&stack, &drops);
                    spawn_scoped(s, move || {
                        let mut popped = Vec::new();
                        for i in 0..per_thread {
                            stack.push(Tracked {
                                id: t * per_thread + i,
                                drops,
                            });
                            // Pop about every other time, so the stack
                            // grows and shrinks while other threads race.
                            if i % 2 == 1 {
                                popped.extend(stack.pop().map(|v| v.id));
                            }
                        }
                        popped
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });
        let popped_by_threads = seen.len();
        while let Some(value) = stack.pop() {
            seen.push(value.id);
        }
        seen.sort_unstable();
        let expected: Vec<usize> = (0..THREADS * per_thread).collect();
        assert_eq!(seen, expected, "every value popped exactly once");
        assert_eq!(drops.load(Relaxed), THREADS * per_thread);
        assert_eq!(stack.retired_len(), THREADS * per_thread);
        assert!(popped_by_threads > 0);
    }

    #[test]
    fn the_stack_is_send_and_sync_for_send_values() {
        fn assert_send_sync<S: Send + Sync>() {}
        // `Cell` is Send but not Sync: the stack never shares a `&T`.
        assert_send_sync::<TreiberStack<Cell<u8>>>();
    }

    #[test]
    fn spinlock_single_threaded_state_machine() {
        let lock = SpinLock::new();
        assert!(lock.try_lock());
        assert!(!lock.try_lock());
        lock.unlock();
        assert!(lock.try_lock());
    }

    #[test]
    fn spinlock_guards_a_counter_across_threads() {
        let per_thread = if cfg!(miri) { 20 } else { 1_000 };
        let lock = SpinLock::new();
        let counter = AtomicUsize::new(0);
        thread::scope(|s| {
            for _ in 0..4 {
                spawn_scoped(s, || {
                    for _ in 0..per_thread {
                        lock.lock();
                        let seen = counter.load(Relaxed);
                        counter.store(seen + 1, Relaxed);
                        lock.unlock();
                    }
                });
            }
        });
        assert_eq!(counter.into_inner(), 4 * per_thread);
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use super::{SpinLock, TreiberStack};
    use loom::sync::Arc;
    use loom::sync::atomic::AtomicUsize;
    use loom::thread;
    use std::sync::atomic::Ordering::Relaxed;

    /// Two threads take the lock once each and bump a counter with a
    /// separate load and store.
    fn spinlock(lock: SpinLock) {
        let lock = Arc::new(lock);
        let counter = Arc::new(AtomicUsize::new(0));
        let bump = |lock: &SpinLock, counter: &AtomicUsize| {
            lock.lock();
            let seen = counter.load(Relaxed);
            counter.store(seen + 1, Relaxed);
            lock.unlock();
        };
        let (lock1, counter1) = (lock.clone(), counter.clone());
        let other = thread::spawn(move || bump(&lock1, &counter1));
        bump(&lock, &counter);
        other.join().unwrap();
        assert_eq!(counter.load(Relaxed), 2, "spinlock: lost update");
    }

    #[test]
    fn spinlock_acquire_release_publishes_the_critical_section() {
        loom::model(|| spinlock(SpinLock::new()));
    }

    /// Mutual exclusion alone is not enough: with `Relaxed` the second
    /// holder can read the counter value from before the first holder's
    /// critical section, even though the two sections never overlapped.
    #[test]
    #[should_panic(expected = "lost update")]
    fn spinlock_relaxed_loses_an_update() {
        loom::model(|| spinlock(SpinLock::relaxed()));
    }

    #[test]
    fn treiber_concurrent_push_and_pops_keep_every_value_once() {
        loom::model(|| {
            let stack = Arc::new(TreiberStack::new());
            stack.push(1);
            let stack1 = stack.clone();
            let other = thread::spawn(move || {
                stack1.push(2);
                stack1.pop()
            });
            let mine = stack.pop();
            let theirs = other.join().unwrap();
            let mut seen: Vec<i32> = [mine, theirs].into_iter().flatten().collect();
            while let Some(v) = stack.pop() {
                seen.push(v);
            }
            seen.sort_unstable();
            assert_eq!(seen, [1, 2], "a value was lost or popped twice");
        });
    }

    #[test]
    fn treiber_two_concurrent_pops_get_different_values() {
        loom::model(|| {
            let stack = Arc::new(TreiberStack::new());
            stack.push(1);
            stack.push(2);
            let stack1 = stack.clone();
            let other = thread::spawn(move || stack1.pop());
            let mine = stack.pop();
            let theirs = other.join().unwrap();
            let mut seen: Vec<i32> = [mine, theirs].into_iter().flatten().collect();
            seen.sort_unstable();
            assert_eq!(seen, [1, 2], "both pops must succeed with distinct values");
            assert!(stack.is_empty());
        });
    }
}
