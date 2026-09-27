//! Lab · the UB zoo: undefined behavior, case by case, under Miri
//!
//! Undefined behavior (UB) is not "a crash" or "a garbage value". It is a
//! broken promise: the compiler optimizes on the assumption that UB never
//! happens, so a program that commits it has no meaning at all, not even in
//! the lines that ran before the bad one. Safe Rust cannot commit UB as long
//! as the `unsafe` code under it is sound; that is the whole guarantee.
//! `unsafe` switches none of the rules off. It unlocks operations the
//! compiler cannot check (dereferencing a raw pointer, calling an
//! `unsafe fn`, accessing a `static mut`, reading a union field,
//! implementing an `unsafe trait`) and makes *you* responsible for the rules
//! they can break.
//!
//! Every case below is an `#[ignore]`d `ub_*` test with a `fixed_*` twin (a
//! few cases share one; `BadCell`'s fix is `GoodCell`). Plain `cargo test`
//! runs the twins and the other demonstrations, never a `ub_*` test. Run a
//! `ub_*` test natively and it usually *passes*: that is the lesson. A
//! passing test shows what one build did on one machine, nothing more. Miri
//! interprets the program on Rust's abstract machine and stops at the first
//! UB it detects, naming the rule. The `deep-dive-miri` CI job runs every
//! non-ignored test (so every twin) under both aliasing models, and
//! `deep-dive/scripts/check_ub_zoo.sh` runs each `ub_*` test alone and
//! checks that Miri still reports *its* diagnostic, which the
//! `// miri-expect:` line above the test records.
//!
//! Compare with the safe side of the same rules: `38_variance/variance1`
//! (picking a `PhantomData` marker), `40_interior_mutability/cell1` (`Cell`,
//! the sound way to write through `&`; the module's README shows why every
//! cell must be invariant, with the very exploit `BadCell` allows),
//! `41_memory_layout/layout1` and its README (a `bool` is only ever 0 or 1,
//! the niche that makes `Option<bool>` one byte), `36_atomics/atomics1` (an
//! atomic counter instead of a racy one), `36_atomics/atomics4`
//! (check-then-act: a race condition without UB) and `39_drop_raii/raii4`
//! (an `Rc` cycle leaks in safe code). Among the labs, `raw_vec` initializes
//! every slot before reading it and drops only `[0, len)`, and `myarc`
//! explains how a `Relaxed` decrement would turn into a use-after-free.
//!
//! # The zoo
//!
//! Each test's doc comment says what it does, what Miri reports, what a
//! native debug run did on the author's machine (rustc 1.96,
//! aarch64-apple-darwin; UB has no portable outcome) and the command that
//! runs it. Miri is quoted from nightly 1.99 (2026-07-23); every diagnostic
//! was checked again on nightly 1.101 (2026-09-26), and the one that changed
//! says so. "SB" is Stacked Borrows, Miri's default aliasing model; "TB" is
//! Tree Borrows (`-Zmiri-tree-borrows`).
//!
//! - **Aliasing** (a live `&mut T` is unique, a live `&T` is read-only):
//!   `ub_two_mut_from_one_raw_pointer`, `ub_split_at_mut_overlapping`,
//!   `ub_split_at_mut_via_index` (SB only), `ub_write_through_shared_ref`.
//! - **Invalid values** (every value must be valid for its type, even one
//!   you never look at): `ub_transmute_2_to_bool`,
//!   `ub_assume_init_uninit_integer`, `ub_set_len_then_assign_drops_garbage`.
//! - **Dangling and misaligned pointers**: `ub_use_after_free`,
//!   `ub_let_underscore_of_unsafe_block_reads`, `ub_unaligned_read`.
//! - **Concurrency**: `ub_data_race`.
//! - **Library UB**: `ub_lib_invalid_utf8_unseen` (Miri sees nothing),
//!   `ub_lib_invalid_utf8_then_chars`, `ub_get_unchecked_out_of_bounds`,
//!   `ub_lib_set_len_past_capacity`.
//! - **Not UB, but reported**: `ub_leak_not_ub_but_reported`.
//! - **Variance** (the `unsound_covariant_cell` part): [`BadCell`] and
//!   `ub_bad_cell_use_after_free`, fixed by [`GoodCell`].
//!
//! # Sharpest question: which UB needs `unsafe`, and is a data race UB?
//!
//! All of it needs `unsafe` somewhere. Every case here has an `unsafe`
//! block, either in the test or, for `BadCell`, in a library whose safe API
//! is unsound, which is the only way safe code can reach UB. Three kinds to
//! name in an interview: breaking the aliasing rules (two live `&mut` to one
//! place, a write through `&T`), producing an invalid value (a `bool` that
//! is 2, an uninitialized integer, a dangling reference), and accessing
//! freed or misaligned memory.
//!
//! A **data race** is UB. It is two accesses to the same location from
//! different threads, at least one a write and at least one non-atomic, with
//! no happens-before between them. Safe Rust rules it out at compile time: a
//! `&mut` is unique, and shared mutation needs a `Sync` type (an atomic, a
//! `Mutex`), so writing one takes a `static mut`, a raw pointer or a false
//! `unsafe impl Sync`. A **race condition** is a logic bug whose outcome
//! depends on timing, like check-then-act on an atomic (`atomics4`). It is
//! not UB, safe Rust allows it, and Miri accepts it:
//! `race_condition_without_ub` loses an update on every run and Miri has
//! nothing to say. The two overlap without either containing the other:
//! lost updates from a racy `+= 1` are both, and a data-race-free program
//! can still be full of race conditions.
//!
//! # Library UB vs language UB
//!
//! *Language UB* breaks a rule of the abstract machine: the Reference's list
//! (aliasing, invalid values, dangling or misaligned accesses, data races,
//! ...). *Library UB* breaks the documented contract of an `unsafe fn` or a
//! type's invariant: `from_utf8_unchecked` requires valid UTF-8, `set_len`
//! requires `new_len <= capacity()`. The Reference only asks a `str` to be
//! initialized, so a `&str` holding invalid UTF-8 is fine at the language
//! level until code that trusts the invariant turns it into language UB.
//! That is why Miri does not see `ub_lib_invalid_utf8_unseen` at all, and
//! catches `ub_lib_invalid_utf8_then_chars` only when `chars()` decodes the
//! bytes into an invalid `char`. It is still a bug in the caller: the
//! library may rely on its invariant anywhere, in any future version.
//!
//! Debug builds help. Since Rust 1.78, std checks many `unsafe`
//! preconditions at run time whenever *your* crate is built with debug
//! assertions. A violation prints "unsafe precondition(s) violated: ..." and
//! "This Undefined Behavior check is optional, and cannot be relied on for
//! safety.", then aborts the process, so a test cannot catch it (the whole
//! test binary dies). std tags each check in its source (read in the
//! nightly std): `check_library_ub` guards a library precondition and runs
//! under Miri too; `check_language_ub` guards an operation that is language
//! UB anyway, so std turns it off under Miri and lets Miri report the UB:
//!
//! - `char::from_u32_unchecked`, which `chars()` reaches above, is
//!   `check_language_ub`: Miri reports the invalid `char` itself.
//! - `set_len(capacity + 1)` is pure library UB, since only a `usize` field
//!   changes. Its check is `check_library_ub`, so Miri reports the abort.
//! - `get_unchecked(3)` on a 3-element slice changed sides in 2026. Nightly
//!   1.99 (2026-07-23) tags it `check_language_ub`, so Miri skips the check
//!   and reports the `assume(index < len)` that std makes right after it.
//!   Nightly 1.101 (2026-09-26) tags it `check_library_ub` ("Hitting the
//!   `assume` provides worse const-eval and Miri diagnostics"), so Miri
//!   reports the same abort as a native debug run. Which checks exist, and
//!   how they are tagged, is up to std and can change in any release.
//! - Release builds (debug assertions off) have no such checks. See "Try it"
//!   for what Miri makes of `--release`.
//!
//! # Stacked Borrows vs Tree Borrows
//!
//! Rust has no official aliasing model yet; Miri implements two candidates,
//! and Tree Borrows is usually the more permissive one. Under SB, every byte
//! has a "borrow stack" of the references allowed to use it, and creating a
//! `&mut` from a pointer pops everything above that pointer, so each
//! reference derived from it earlier is gone. Miri re-checks (*retags*) a
//! reference whenever it is created, copied or moved, so merely moving a
//! popped reference into a tuple is UB, before any read or write through
//! it. Under TB, creating a `&mut` only counts as a read of its bytes, and
//! the UB comes at the first read or write through a reference that a
//! conflicting access has disabled. `ub_split_at_mut_via_index` cuts both
//! halves out of whole-slice `&mut` reborrows: SB rejects it, and TB accepts
//! it, because the halves are disjoint and the first was not written before
//! the second reborrow. `ub_split_at_mut_overlapping` is UB under both, at
//! different points. The `deep-dive-miri` CI job runs every lab under both
//! models with `-Zmiri-strict-provenance` (no integer-to-pointer casts), so
//! code that relies on either model's leniency fails it.
//!
//! # `let _ = *p` reads nothing; `let _ = unsafe { *p }` reads
//!
//! A `_` pattern binds nothing, so `let _ = *p;` names the place `*p` and
//! never loads from it, and the Reference makes *accessing* a dangling place
//! UB, not naming one. Miri accepts it after a free
//! (`let_underscore_on_a_dangling_place_reads_nothing`). Wrap the place in a
//! block and that changes: a block is a value expression, so in
//! `let _ = unsafe { *p };` the `*p` is copied out to become the block's
//! value, and that copy is the read
//! (`ub_let_underscore_of_unsafe_block_reads`).
//!
//! # Invariants
//!
//! - Every `ub_*` test is `#[ignore]`d and has a `// miri-expect:` line (or
//!   one per model) that `scripts/check_ub_zoo.sh` checks. `PASS` means Miri
//!   is expected to report nothing.
//! - Every other test passes natively and under Miri, with both aliasing
//!   models and strict provenance.
//! - [`split_at_mut`] returns disjoint halves derived from one raw pointer.
//!   [`GoodCell`] is invariant in `T`, never lends out a reference to its
//!   contents, and drops every value it stored exactly once.
//! - Every `unsafe` block has a `// SAFETY:` comment. In a deliberately
//!   broken variant it starts with "VIOLATED" and names the broken rule.
//!
//! # Run it
//!
//! ```text
//! # The twins and demonstrations (the ub_* tests are ignored):
//! cargo test --manifest-path deep-dive/Cargo.toml ub_zoo
//! cargo +nightly miri test --manifest-path deep-dive/Cargo.toml ub_zoo
//!
//! # A name filter skips the doctests; the BadCell / GoodCell pair
//! # (nightly rustdoc also checks the E0597):
//! cargo test --manifest-path deep-dive/Cargo.toml --doc ub_zoo
//! cargo +nightly test --manifest-path deep-dive/Cargo.toml --doc ub_zoo
//!
//! # One UB case, alone, under Miri (prepend
//! # MIRIFLAGS=-Zmiri-tree-borrows for Tree Borrows):
//! cargo +nightly miri test --manifest-path deep-dive/Cargo.toml --lib -- \
//!     --ignored --exact ub_zoo::tests::ub_use_after_free
//!
//! # Every UB case under both models, each checked against its expected
//! # diagnostic (what CI runs):
//! bash deep-dive/scripts/check_ub_zoo.sh
//! ```
//!
//! # Try it
//!
//! - Run a UB case natively:
//!   `cargo test --manifest-path deep-dive/Cargo.toml --lib -- --ignored
//!   --exact ub_zoo::tests::ub_transmute_2_to_bool` passes. Most of the zoo
//!   does. Run them one at a time: the precondition checks and the
//!   misaligned-pointer check abort the whole test binary.
//! - Run `ub_split_at_mut_via_index` under Miri with and without
//!   `MIRIFLAGS=-Zmiri-tree-borrows`: SB reports it, TB passes it.
//! - Give [`GoodCell`] `BadCell`'s covariant marker (`PhantomData<T>`
//!   instead of `PhantomData<Cell<T>>`) and run `cargo test --manifest-path
//!   deep-dive/Cargo.toml --doc ub_zoo`: the `compile_fail` doctest now
//!   compiles, so it fails.
//! - Make the first call in `fixed_split_at_mut` use
//!   [`split_at_mut_overlapping`] and run the twin under both models. SB
//!   reports UB at the function's return, which would turn the
//!   `deep-dive-miri` job red. TB passes it: after `right[0]` writes the
//!   shared element, the twin never touches it through `left`.
//! - Prepend `MIRIFLAGS=-Zmiri-ignore-leaks` to the Miri command for
//!   `ub_leak_not_ub_but_reported`: it passes, because a leak is not UB.
//! - Add `--release` to the Miri command for `ub_lib_set_len_past_capacity`.
//!   Miri ignores the opt-level (it warns "Miri does not support
//!   optimizations"), so what `--release` changes here is that debug
//!   assertions are off: the check is gone, and Miri reports nothing, since
//!   no memory past the capacity is ever touched. Do the same for
//!   `ub_get_unchecked_out_of_bounds`: with no check, Miri reaches std's
//!   `assume(index < len)` and reports "`assume` called with `false`", on
//!   either nightly.
//! - Make a `ub_*` test sound (transmute `1` instead of `2`) or add a new
//!   `ub_*` test without a `// miri-expect:` line, then run the guard script:
//!   it fails and names the test.

use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;

// ---------- Aliasing: three hand-rolled `split_at_mut`s ----------

/// A sound hand-rolled `split_at_mut`, like the one in std.
///
/// Both halves are derived from one raw pointer and cover disjoint ranges,
/// so writing through one never touches memory the other may use. The
/// borrow checker cannot see that disjointness through a single `&mut [T]`,
/// which is why this needs `unsafe` (compare the safe split borrows in
/// `37_borrowck_errors/borrowck1`).
///
/// # Panics
///
/// If `mid > s.len()`.
pub fn split_at_mut<T>(s: &mut [T], mid: usize) -> (&mut [T], &mut [T]) {
    let len = s.len();
    assert!(mid <= len, "mid {mid} is past the end of a slice of {len}");
    let p = s.as_mut_ptr();
    // SAFETY: `p` points to `len` initialized `T`s borrowed mutably from `s`
    // for the returned lifetime, and `mid <= len`, so `[0, mid)` is in bounds.
    // Nothing else uses `s` while the halves live.
    let left = unsafe { slice::from_raw_parts_mut(p, mid) };
    // SAFETY: `mid <= len`, so `p + mid` is in bounds or one past the end.
    let tail = unsafe { p.add(mid) };
    // SAFETY: `[mid, len)` is in bounds and disjoint from `left`, and both
    // halves are derived from the same pointer `p`, so neither invalidates
    // the other.
    let right = unsafe { slice::from_raw_parts_mut(tail, len - mid) };
    (left, right)
}

/// UNSOUND ON PURPOSE: an off-by-one `split_at_mut` whose halves share
/// element `mid`. The signature promises two unique borrows; the body hands
/// out two `&mut` to one element. `ub_split_at_mut_overlapping` shows Miri
/// catching it under both aliasing models.
///
/// # Panics
///
/// If `mid >= s.len()`.
pub fn split_at_mut_overlapping<T>(s: &mut [T], mid: usize) -> (&mut [T], &mut [T]) {
    let len = s.len();
    assert!(mid < len, "mid {mid} must index into a slice of {len}");
    let p = s.as_mut_ptr();
    // SAFETY: VIOLATED. `[0, mid]` is in bounds, but it overlaps `right`
    // below at index `mid`: two live `&mut` to one element.
    let left = unsafe { slice::from_raw_parts_mut(p, mid + 1) };
    // SAFETY: `mid < len`, so `p + mid` is in bounds.
    let tail = unsafe { p.add(mid) };
    // SAFETY: VIOLATED. See `left`: this half starts at the shared element.
    let right = unsafe { slice::from_raw_parts_mut(tail, len - mid) };
    (left, right)
}

/// UNSOUND ON PURPOSE under Stacked Borrows, accepted by Tree Borrows: both
/// halves are cut out of `&mut` reborrows of the *whole* slice.
///
/// Under SB, creating the second whole-slice `&mut` pops the first one (and
/// `left`, derived from it) off the borrow stack, so moving `left` into the
/// returned tuple, which retags it, is already UB. Under TB, creating a
/// `&mut` only counts as a read, which the not-yet-written `left`
/// tolerates, and the halves are disjoint. Write the
/// version both models accept: [`split_at_mut`]. Writing the reborrows
/// implicitly, as `&mut (*p)[..mid]`, trips rustc's deny-by-default
/// `dangerous_implicit_autorefs` lint.
pub fn split_at_mut_via_index<T>(s: &mut [T], mid: usize) -> (&mut [T], &mut [T]) {
    let p: *mut [T] = s;
    // SAFETY: `p` comes from a live `&mut [T]`; this reborrow is the only
    // reference in use so far.
    let left = unsafe { &mut (&mut *p)[..mid] };
    // SAFETY: VIOLATED under Stacked Borrows: this second `&mut` to the whole
    // slice invalidates `left`. Tree Borrows accepts it.
    let right = unsafe { &mut (&mut *p)[mid..] };
    (left, right)
}

// ---------- Invalid values ----------

/// The checked way to turn a byte into a `bool` (the fix for
/// `ub_transmute_2_to_bool`). A `bool` has exactly two valid bit patterns,
/// 0 and 1; the other 254 are a niche that `Option<bool>` uses for `None`
/// (see the `41_memory_layout` README). `transmute` would have to check
/// that, and it doesn't.
pub fn bool_from_byte(byte: u8) -> Option<bool> {
    match byte {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

// ---------- Variance: the `unsound_covariant_cell` part ----------

/// UNSOUND ON PURPOSE: a heap cell that can be written through `&self`, with
/// the wrong variance.
///
/// `set(&self, T)` writes through a shared reference, so `BadCell<T>` must
/// be invariant in `T`, like every std cell. Its fields are a `NonNull<T>`
/// and a `PhantomData<T>`, though, and both are covariant, so rustc infers a
/// covariant `BadCell`: a `&BadCell<&'static str>` may be used as a
/// `&BadCell<&'a str>` for any shorter `'a`. Safe code can then `set` a
/// short-lived `&str` into a cell that promises `&'static str`, and read it
/// back after its owner is gone:
///
/// ```no_run
/// use deep_dive::ub_zoo::BadCell;
///
/// let cell: BadCell<&'static str> = BadCell::new("static");
/// {
///     let s = String::from("short-lived");
///     cell.set(&s);
/// }
/// println!("{}", cell.get());
/// ```
///
/// That compiles, and `no_run` keeps the doctest from running it: under
/// Miri, `ub_bad_cell_use_after_free` reports "constructing invalid value of
/// type &str: encountered a dangling reference (use-after-free)". It is also
/// the positive control for the `compile_fail` doctest on [`GoodCell`]: the
/// two differ only in the type name. Every `unsafe` block below is fine on
/// its own; the bug is the variance, which no `unsafe` block mentions.
/// Soundness is a property of the whole API.
pub struct BadCell<T> {
    ptr: NonNull<T>,
    // The bug: `PhantomData<T>` is covariant (see `GoodCell` for the fix).
    _marker: PhantomData<T>,
}

impl<T> BadCell<T> {
    pub fn new(value: T) -> Self {
        BadCell {
            ptr: NonNull::from(Box::leak(Box::new(value))),
            _marker: PhantomData,
        }
    }

    /// Swaps the new value in, then drops the old one, like `Cell::set`. If
    /// the old value's `Drop` calls back into the cell, it finds a valid
    /// value there. `*ptr = value` would drop the old value in place first,
    /// while the cell still holds it.
    pub fn set(&self, value: T) {
        // SAFETY: `ptr` came from `Box::leak` and stays valid until `drop`.
        // The cell never hands out a reference into the box, so nothing
        // aliases the write. `NonNull` makes the type `!Sync`, so no other
        // thread can call `set` at the same time.
        let old = unsafe { self.ptr.as_ptr().replace(value) };
        drop(old);
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: `ptr` is valid and initialized (see `set`), and `T: Copy`,
        // so reading leaves the cell's value in place.
        unsafe { self.ptr.as_ptr().read() }
    }
}

impl<T> Drop for BadCell<T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `Box::leak` in `new` and is reclaimed only
        // here, once.
        drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
    }
}

/// The fix: [`BadCell`] with one marker changed. `PhantomData<Cell<T>>`
/// makes `GoodCell` invariant in `T`, since everything built on
/// `UnsafeCell` is, and still says "owns a `T`" (which the drop checker
/// only consults once a `Drop` impl uses `#[may_dangle]`, as `Vec`'s does).
/// An inline `UnsafeCell<T>` would also do; `PhantomData<*mut T>` and
/// `PhantomData<fn(T) -> T>` are invariant too, but drop the ownership
/// claim.
///
/// The exploit from [`BadCell`] no longer compiles: `set` now needs a
/// `&'static str`, and `s` dies at the end of the block (E0597, "`s` does
/// not live long enough"):
///
/// ```compile_fail,E0597
/// use deep_dive::ub_zoo::GoodCell;
///
/// let cell: GoodCell<&'static str> = GoodCell::new("static");
/// {
///     let s = String::from("short-lived");
///     cell.set(&s);
/// }
/// println!("{}", cell.get());
/// ```
///
/// Stable rustdoc only checks that this fails to compile. Nightly rustdoc
/// also checks the error code, and `cargo +nightly miri test` runs doctests
/// with nightly rustdoc, so the `deep-dive-miri` job pins E0597. The
/// positive control is the `no_run` doctest on [`BadCell`]: the same lines
/// with `BadCell` compile, so the only thing failing here is the variance.
pub struct GoodCell<T> {
    ptr: NonNull<T>,
    // The fix: `Cell<T>` is invariant in `T`.
    _marker: PhantomData<Cell<T>>,
}

impl<T> GoodCell<T> {
    pub fn new(value: T) -> Self {
        GoodCell {
            ptr: NonNull::from(Box::leak(Box::new(value))),
            _marker: PhantomData,
        }
    }

    /// Replaces the value, then drops the old one (see [`BadCell::set`]).
    pub fn set(&self, value: T) {
        // SAFETY: as in `BadCell::set`: `ptr` is valid until `drop`, no
        // reference into the box exists, and the type is `!Sync`.
        let old = unsafe { self.ptr.as_ptr().replace(value) };
        drop(old);
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: `ptr` is valid and initialized, and `T: Copy`.
        unsafe { self.ptr.as_ptr().read() }
    }
}

impl<T> Drop for GoodCell<T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `Box::leak` in `new` and is reclaimed only
        // here, once.
        drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::UnsafeCell;
    use std::hint::black_box;
    use std::mem::{self, MaybeUninit};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;

    /// A payload that counts its drops, to prove "dropped exactly once".
    struct Tracked<'a> {
        drops: &'a Cell<usize>,
        value: u32,
    }

    impl<'a> Tracked<'a> {
        fn new(drops: &'a Cell<usize>, value: u32) -> Self {
            Tracked { drops, value }
        }
    }

    impl Drop for Tracked<'_> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    /// Eight bytes whose first one is guaranteed to sit at a multiple of 4.
    #[repr(align(4))]
    struct Aligned4([u8; 8]);

    // ---------- Aliasing ----------

    /// Two `&mut` made from one raw pointer, and the first one used again
    /// after a write through the second. Each `&mut` claims to be the only
    /// way to reach `x` while it is live, so the two claims contradict each
    /// other. Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_two_mut_from_one_raw_pointer`
    // miri-expect(sb): Undefined Behavior: attempting a read access using
    // miri-expect(tb): Undefined Behavior: read access through <.* is forbidden
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_two_mut_from_one_raw_pointer() {
        let mut x = 0u32;
        let p = &raw mut x;
        // SAFETY: VIOLATED. `a` is live until its last use below ...
        let a = unsafe { &mut *p };
        // SAFETY: VIOLATED. ... so `b` is a second live `&mut` to `x`.
        let b = unsafe { &mut *p };
        *b = 2;
        *a += 1;
        black_box(x);
    }

    /// The fix: let each `&mut` die before the next one is made (or write
    /// through the raw pointer itself).
    #[test]
    fn fixed_reborrow_one_at_a_time() {
        let mut x = 0u32;
        let p = &raw mut x;
        {
            // SAFETY: `p` points to the live `x`, and `a` is the only
            // reference to it until the end of this block.
            let a = unsafe { &mut *p };
            *a += 1;
        }
        {
            // SAFETY: as above; `a` is dead, so `b` is unique.
            let b = unsafe { &mut *p };
            *b += 1;
        }
        // SAFETY: `p` is valid, and no reference to `x` is live.
        unsafe { *p += 1 };
        assert_eq!(x, 3);
    }

    /// [`split_at_mut_overlapping`] hands out two `&mut` to element 2. Under
    /// SB the function is UB before it returns: moving `left` into the
    /// returned tuple retags it, and creating `right` has already
    /// invalidated it at the shared element. Under TB, creating `right` is
    /// only a read, so the UB comes later: reading `left[2]` after the write
    /// through `right[0]`. Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_split_at_mut_overlapping`
    // miri-expect(sb): Undefined Behavior: trying to retag .* borrow stack
    // miri-expect(tb): Undefined Behavior: read access through <.* is forbidden
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_split_at_mut_overlapping() {
        let mut v = [1, 2, 3, 4];
        let (left, right) = split_at_mut_overlapping(&mut v, 2);
        right[0] = 30;
        left[2] += 1;
        black_box(&v);
    }

    /// [`split_at_mut_via_index`] is UB under Stacked Borrows only: the
    /// second whole-slice reborrow pops `left` off the borrow stack, so the
    /// retag when `left` is returned finds its tag gone, before this test
    /// touches either half. Tree Borrows accepts it, writes included (see
    /// the module docs; prepend `MIRIFLAGS=-Zmiri-tree-borrows` to the
    /// command below). Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_split_at_mut_via_index`
    // miri-expect(sb): Undefined Behavior: trying to retag .* borrow stack
    // miri-expect(tb): PASS
    #[test]
    #[ignore = "UB on purpose (Stacked Borrows only): run it alone under Miri"]
    fn ub_split_at_mut_via_index() {
        let mut v = [1, 2, 3, 4];
        let (left, right) = split_at_mut_via_index(&mut v, 2);
        left[0] = 10;
        right[0] = 30;
        black_box(&v);
    }

    /// The fix for both: [`split_at_mut`]. The same writes, on disjoint
    /// halves, give the same result as std's `split_at_mut`.
    #[test]
    fn fixed_split_at_mut() {
        let mut v = [1, 2, 3, 4];
        let (left, right) = split_at_mut(&mut v, 2);
        right[0] = 30;
        left[1] += 10;
        assert_eq!(v, [1, 12, 30, 4]);

        let mut w = [1, 2, 3, 4];
        let (l, r) = w.split_at_mut(2);
        r[0] = 30;
        l[1] += 10;
        assert_eq!(w, v);

        // The edges: an empty left half and an empty right half.
        let (l, r) = split_at_mut(&mut v, 0);
        assert!(l.is_empty() && r.len() == 4);
        let (l, r) = split_at_mut(&mut v, 4);
        assert!(l.len() == 4 && r.is_empty());
    }

    /// The `assert!` is what keeps [`split_at_mut`] sound for safe callers:
    /// without it, `mid = 5` would make `left` cover memory past the end of
    /// the slice before anything else could go wrong.
    #[test]
    #[should_panic(expected = "mid 5 is past the end of a slice of 4")]
    fn split_at_mut_panics_past_the_end() {
        let mut v = [1, 2, 3, 4];
        let _ = split_at_mut(&mut v, 5);
    }

    /// A write through a pointer cast from `&T`. The bytes behind a shared
    /// reference are immutable while it is live, unless they sit inside an
    /// `UnsafeCell`. Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_write_through_shared_ref`
    // miri-expect(sb): Undefined Behavior: .*grants SharedReadOnly permission
    // miri-expect(tb): Undefined Behavior: write access through .* is forbidden
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    #[allow(
        invalid_reference_casting,
        reason = "deny-by-default because this is UB: the case under test"
    )]
    fn ub_write_through_shared_ref() {
        let x = 1u32;
        let r = &x;
        // SAFETY: VIOLATED. `r` is a shared reference to memory outside any
        // `UnsafeCell`, so nothing may write through it.
        unsafe { *(r as *const u32 as *mut u32) = 2 };
        black_box(x);
    }

    /// The fix: interior mutability. `Cell` (built on `UnsafeCell`) is the
    /// one sound way to mutate behind `&`, and `UnsafeCell::get` is what it
    /// does inside.
    #[test]
    fn fixed_write_through_cell() {
        let x = Cell::new(1u32);
        let r = &x;
        r.set(2);
        assert_eq!(x.get(), 2);

        let y = UnsafeCell::new(1u32);
        let r = &y;
        // SAFETY: the value sits inside an `UnsafeCell`, and no reference to
        // the value itself (only `r`, to the cell) exists while we write, so
        // writing through `get()` is allowed.
        unsafe { *r.get() = 2 };
        assert_eq!(y.into_inner(), 2);
    }

    // ---------- Invalid values ----------

    /// A `bool` must be 0 or 1. Producing a 2 is UB the moment it exists,
    /// even if nothing branches on it. Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_transmute_2_to_bool`
    // miri-expect: Undefined Behavior: .*type bool: encountered 0x02
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    #[allow(
        clippy::transmute_int_to_bool,
        reason = "the invalid transmute is the case under test"
    )]
    fn ub_transmute_2_to_bool() {
        // SAFETY: VIOLATED. 2 is not a valid `bool`.
        let b: bool = unsafe { mem::transmute::<u8, bool>(black_box(2)) };
        black_box(b);
    }

    /// The fix: check the byte ([`bool_from_byte`]), or use `byte != 0`
    /// for C's "non-zero is true".
    #[test]
    fn fixed_bool_from_byte() {
        assert_eq!(bool_from_byte(0), Some(false));
        assert_eq!(bool_from_byte(1), Some(true));
        assert_eq!(bool_from_byte(2), None);
        assert!(black_box(2u8) != 0);
    }

    /// Every bit pattern is a valid `u32`, but uninitialized memory is not a
    /// bit pattern: it is UB for integers too (the Reference: an integer
    /// "must be initialized"). rustc's `invalid_value` lint and clippy's
    /// deny-by-default `uninit_assumed_init` both flag it. Natively (debug):
    /// passes, with whatever the stack held.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_assume_init_uninit_integer`
    // miri-expect: Undefined Behavior: .*type u32: encountered uninitialized
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    #[allow(
        invalid_value,
        clippy::uninit_assumed_init,
        reason = "assume_init on uninitialized memory is the case under test"
    )]
    fn ub_assume_init_uninit_integer() {
        // SAFETY: VIOLATED. Nothing was written to the `MaybeUninit`.
        let x: u32 = unsafe { MaybeUninit::<u32>::uninit().assume_init() };
        black_box(x);
    }

    /// The fix: write the value, then `assume_init`.
    #[test]
    fn fixed_assume_init_after_write() {
        let mut slot = MaybeUninit::<u32>::uninit();
        slot.write(7);
        // SAFETY: `write` initialized the value above.
        let x = unsafe { slot.assume_init() };
        assert_eq!(x, 7);
    }

    /// `set_len(2)` on an empty `Vec<String>` claims two initialized
    /// strings, and `v[0] = ..` drops the "old" one, reading a pointer and a
    /// capacity that were never written (dropping `v` later does the same to
    /// `v[1]`). Clippy's deny-by-default `uninit_vec` flags the pattern.
    /// Natively (debug): passes, which means both garbage capacities read as
    /// 0, an empty `String` whose drop frees nothing; any other value makes
    /// the drop free a garbage pointer.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_set_len_then_assign_drops_garbage`
    // miri-expect: Undefined Behavior: reading memory .* is uninitialized
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    #[allow(
        clippy::uninit_vec,
        reason = "set_len over uninitialized elements is the case under test"
    )]
    fn ub_set_len_then_assign_drops_garbage() {
        let mut v: Vec<String> = Vec::with_capacity(2);
        // SAFETY: VIOLATED. `set_len` requires `0..2` to be initialized.
        unsafe { v.set_len(2) };
        v[0] = String::from("assigning drops the old value first");
        black_box(&v);
    }

    /// The fix: write into `spare_capacity_mut()` (a `&mut
    /// [MaybeUninit<T>]`, so no old value is dropped), then `set_len`. The
    /// drop counter proves both values are dropped exactly once.
    #[test]
    fn fixed_spare_capacity_mut_then_set_len() {
        let drops = Cell::new(0);
        let mut v: Vec<Tracked> = Vec::with_capacity(2);
        let spare = v.spare_capacity_mut();
        spare[0].write(Tracked::new(&drops, 1));
        spare[1].write(Tracked::new(&drops, 2));
        // SAFETY: 2 <= capacity, and elements 0 and 1 were written above.
        unsafe { v.set_len(2) };
        assert_eq!(v[1].value, 2);
        assert_eq!(drops.get(), 0);
        drop(v);
        assert_eq!(drops.get(), 2);
    }

    // ---------- Dangling and misaligned pointers ----------

    /// A read through a pointer to a freed `Box`. Natively (debug): passes,
    /// usually reading the old value or allocator bookkeeping.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_use_after_free`
    // miri-expect: Undefined Behavior: memory access failed: .* been freed
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_use_after_free() {
        let p = Box::into_raw(Box::new(7u32));
        // SAFETY: `p` came from `Box::into_raw` and is reclaimed only here.
        drop(unsafe { Box::from_raw(p) });
        // SAFETY: VIOLATED. The allocation was freed on the line above.
        let v = unsafe { *p };
        black_box(v);
    }

    /// The fix: finish reading before the free. The drop counter proves
    /// the box's contents are dropped exactly once.
    #[test]
    fn fixed_read_before_free() {
        let drops = Cell::new(0);
        let p = Box::into_raw(Box::new(Tracked::new(&drops, 7)));
        // SAFETY: `p` came from `Box::into_raw` and has not been freed yet.
        let value = unsafe { (*p).value };
        // SAFETY: `p` is reclaimed exactly once and not used afterwards.
        drop(unsafe { Box::from_raw(p) });
        assert_eq!(value, 7);
        assert_eq!(drops.get(), 1);
    }

    /// `let _ = *p;` does not read: a `_` pattern binds nothing, so the
    /// place is named and never loaded from. Miri accepts this, in both
    /// aliasing models, even though `p` dangles.
    #[test]
    fn let_underscore_on_a_dangling_place_reads_nothing() {
        let p = Box::into_raw(Box::new(7u32));
        // SAFETY: `p` came from `Box::into_raw` and is reclaimed only here.
        drop(unsafe { Box::from_raw(p) });
        // SAFETY: `p` dangles, but `let _ = <place>` performs no access,
        // and only accessing a dangling place is UB.
        unsafe {
            let _ = *p;
        }
    }

    /// The same statement with the `unsafe` block moved to the right-hand
    /// side does read: a block is a value expression, so `*p` is copied out
    /// of the freed allocation to become the block's value. Natively
    /// (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_let_underscore_of_unsafe_block_reads`
    // miri-expect: Undefined Behavior: memory access failed: .* been freed
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_let_underscore_of_unsafe_block_reads() {
        let p = Box::into_raw(Box::new(7u32));
        // SAFETY: `p` came from `Box::into_raw` and is reclaimed only here.
        drop(unsafe { Box::from_raw(p) });
        // SAFETY: VIOLATED. The block's value is a copy of `*p`: a read of
        // freed memory.
        let _ = unsafe { *p };
    }

    /// A `u32` load from an address that is 1 past a multiple of 4. A `*p`
    /// read or write needs a pointer aligned for its type, even on hardware
    /// that handles unaligned loads. Natively (debug): rustc's inserted
    /// alignment check aborts the test binary with "misaligned pointer
    /// dereference"; a release build just loads.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_unaligned_read`
    // miri-expect: Undefined Behavior: .*but alignment 4 is required
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_unaligned_read() {
        let buf = Aligned4([1, 2, 3, 4, 5, 6, 7, 8]);
        let p = buf.0.as_ptr().wrapping_add(1).cast::<u32>();
        // SAFETY: VIOLATED. `p` is in bounds but not aligned for `u32`.
        let v = unsafe { *p };
        black_box(v);
    }

    /// The fix: `read_unaligned`, or no pointer at all.
    #[test]
    fn fixed_read_unaligned() {
        let buf = Aligned4([1, 2, 3, 4, 5, 6, 7, 8]);
        let p = buf.0.as_ptr().wrapping_add(1).cast::<u32>();
        // SAFETY: bytes 1..5 of `buf` are initialized and in bounds, and
        // `read_unaligned` has no alignment requirement.
        let v = unsafe { p.read_unaligned() };
        assert_eq!(v, u32::from_ne_bytes([2, 3, 4, 5]));

        let bytes: [u8; 4] = buf.0[1..5].try_into().unwrap();
        assert_eq!(u32::from_ne_bytes(bytes), v);
    }

    // ---------- Concurrency ----------

    /// Two threads increment a `static mut` with no synchronization: a data
    /// race. Miri tracks happens-before with vector clocks, so it reports
    /// the race whichever order the threads happen to run in. Natively
    /// (debug): passes; `HITS` ends at 2, or at 1 if an update is lost.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_data_race`
    // miri-expect: Undefined Behavior: Data race detected between
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_data_race() {
        static mut HITS: u32 = 0;
        let other = thread::Builder::new()
            .spawn(|| {
                // SAFETY: VIOLATED. The main thread increments `HITS` too,
                // with no lock or atomic ordering the two accesses.
                unsafe { HITS += 1 };
            })
            .expect("spawn a thread");
        // SAFETY: VIOLATED. The same race, from this side.
        unsafe { HITS += 1 };
        other.join().unwrap();
        // SAFETY: the other thread has been joined, so this read races
        // with nothing. Reading by value takes no reference to the static.
        black_box(unsafe { HITS });
    }

    /// The fix: an atomic read-modify-write (`36_atomics/atomics1`).
    /// `Relaxed` is enough for a counter, and joining the scope orders both
    /// increments before the final read.
    #[test]
    fn fixed_atomic_counter() {
        let hits = AtomicU32::new(0);
        thread::scope(|s| {
            thread::Builder::new()
                .spawn_scoped(s, || hits.fetch_add(1, Ordering::Relaxed))
                .expect("spawn a thread");
            hits.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(hits.into_inner(), 2);
    }

    /// A race condition without a data race. Check-then-act on an atomic
    /// (`36_atomics/atomics4`): load, then store `seen + 1`. The second
    /// thread runs its whole increment inside that window, and the scope
    /// joins it before the store, so the update is lost on every run. Every
    /// access is atomic, so this is a logic bug but not UB, and Miri runs
    /// this test without a complaint.
    #[test]
    fn race_condition_without_ub() {
        let hits = AtomicU32::new(0);
        let seen = hits.load(Ordering::SeqCst);
        thread::scope(|s| {
            thread::Builder::new()
                .spawn_scoped(s, || hits.fetch_add(1, Ordering::SeqCst))
                .expect("spawn a thread");
        });
        hits.store(seen + 1, Ordering::SeqCst);
        // Two increments, one lost.
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    // ---------- Library UB vs language UB ----------

    /// Library UB that Miri cannot see. `F4 90 80 80` would encode
    /// U+110000, one past the last code point, so it is not UTF-8. The
    /// `&str` breaks `from_utf8_unchecked`'s contract, but the Reference only
    /// requires a `str` to be initialized, and `len()` and `as_bytes()` never
    /// look at the encoding. Miri: the test passes. (`black_box` hides the
    /// bytes from rustc's deny-by-default `invalid_from_utf8_unchecked`
    /// lint, which rejects an invalid literal.) Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_lib_invalid_utf8_unseen`
    // miri-expect: PASS
    #[test]
    #[ignore = "library UB on purpose: run it alone under Miri"]
    fn ub_lib_invalid_utf8_unseen() {
        let bytes = black_box([0xF4, 0x90, 0x80, 0x80]);
        // SAFETY: VIOLATED. `bytes` is not valid UTF-8.
        let s = unsafe { std::str::from_utf8_unchecked(&bytes) };
        black_box(s.len());
        black_box(s.as_bytes());
    }

    /// The same `&str`, then `chars()`. The decoder trusts the UTF-8
    /// invariant, decodes the four bytes to 0x110000 and makes a `char` of
    /// it: library UB has become language UB. Natively (debug): std's
    /// precondition check on `char::from_u32_unchecked` aborts the test
    /// binary.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_lib_invalid_utf8_then_chars`
    // miri-expect: Undefined Behavior: constructing invalid value of type char
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_lib_invalid_utf8_then_chars() {
        let bytes = black_box([0xF4, 0x90, 0x80, 0x80]);
        // SAFETY: VIOLATED. `bytes` is not valid UTF-8.
        let s = unsafe { std::str::from_utf8_unchecked(&bytes) };
        black_box(s.chars().next());
    }

    /// The fix for both: validate. `from_utf8` says where the bytes stop
    /// being UTF-8, and `from_utf8_lossy` replaces each bad byte with
    /// U+FFFD.
    #[test]
    fn fixed_from_utf8_checked() {
        let bytes = black_box([0xF4, 0x90, 0x80, 0x80]);
        let err = std::str::from_utf8(&bytes).unwrap_err();
        assert_eq!(err.valid_up_to(), 0);
        assert_eq!(err.error_len(), Some(1));
        assert_eq!(String::from_utf8_lossy(&bytes), "\u{FFFD}".repeat(4));
        assert_eq!(std::str::from_utf8(b"ok"), Ok("ok"));
    }

    /// `get_unchecked` past the end. Natively (debug): std's precondition
    /// check aborts the test binary with "unsafe precondition(s) violated:
    /// slice::get_unchecked requires that the index is within the slice".
    /// Under Miri it depends on the nightly (see the module docs): nightly
    /// 1.99 skips the check and reports "Undefined Behavior: `assume` called
    /// with `false`"; nightly 1.101 runs the check and reports the same
    /// abort as a native run. The `miri-expect` line accepts both.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_get_unchecked_out_of_bounds`
    // miri-expect: `assume` called with `false`|get_unchecked requires
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_get_unchecked_out_of_bounds() {
        let v = [1u32, 2, 3];
        // SAFETY: VIOLATED. Index 3 is out of bounds for 3 elements.
        let x = unsafe { *v.get_unchecked(black_box(3)) };
        black_box(x);
    }

    /// The fix: `get`, which returns `None`, or `get_unchecked` only behind
    /// a bounds check.
    #[test]
    fn fixed_get_returns_none() {
        let v = [1u32, 2, 3];
        assert_eq!(v.get(3), None);
        let i = black_box(2);
        if i < v.len() {
            // SAFETY: `i < v.len()` was checked on the line above.
            assert_eq!(unsafe { *v.get_unchecked(i) }, 3);
        }
    }

    /// `set_len` past the capacity. At the language level only a `usize`
    /// field changes, so this is pure library UB. std tags that precondition
    /// check `check_library_ub`, so it stays on under Miri, which then
    /// reports the abort. Natively (debug): the same message, and the test
    /// binary aborts.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_lib_set_len_past_capacity`
    // miri-expect: unsafe precondition\(s\) violated: Vec::set_len requires
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_lib_set_len_past_capacity() {
        let mut v: Vec<u8> = Vec::with_capacity(2);
        let too_long = v.capacity() + 1;
        // SAFETY: VIOLATED. `new_len` must be at most `capacity()`.
        unsafe { v.set_len(black_box(too_long)) };
        black_box(v.len());
        // SAFETY: 0 <= capacity, and no element needs to be initialized.
        unsafe { v.set_len(0) };
    }

    /// The fix: `reserve` first, write the new elements, then `set_len`.
    #[test]
    fn fixed_reserve_then_set_len() {
        let mut v: Vec<u8> = Vec::with_capacity(2);
        let want = v.capacity() + 1;
        v.reserve(want);
        for slot in &mut v.spare_capacity_mut()[..want] {
            slot.write(7);
        }
        // SAFETY: `reserve` made `want <= capacity()`, and elements
        // `0..want` were written above.
        unsafe { v.set_len(want) };
        assert_eq!(v, vec![7; want]);
    }

    // ---------- Not UB, but reported ----------

    /// A leak is not UB: `mem::forget` and `Box::leak` are safe functions,
    /// and an `Rc` cycle leaks in 100% safe code (`39_drop_raii/raii4`). Miri
    /// still reports memory that is unreachable at exit, because that is
    /// usually a bug: the test itself passes, then Miri fails the run.
    /// `-Zmiri-ignore-leaks` turns the check off. Natively (debug): passes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_leak_not_ub_but_reported`
    // miri-expect: error: memory leaked: alloc[0-9]+
    #[test]
    #[ignore = "a leak on purpose: run it alone under Miri"]
    fn ub_leak_not_ub_but_reported() {
        let leaked: &'static mut u32 = Box::leak(Box::new(41));
        *leaked += 1;
        black_box(leaked);
    }

    /// The fix: give the allocation back, with `Box::from_raw`, when the
    /// leaked `&'static mut` is no longer used.
    #[test]
    fn fixed_leak_reclaimed() {
        let drops = Cell::new(0);
        let leaked: &mut Tracked = Box::leak(Box::new(Tracked::new(&drops, 41)));
        leaked.value += 1;
        let p: *mut Tracked = leaked;
        // SAFETY: `p` came from `Box::leak`, the `&mut` it came from is not
        // used again, and it is reclaimed exactly once.
        let reclaimed = unsafe { Box::from_raw(p) };
        assert_eq!(reclaimed.value, 42);
        drop(reclaimed);
        assert_eq!(drops.get(), 1);
    }

    // ---------- Variance: the `unsound_covariant_cell` part ----------

    /// The exploit from [`BadCell`]'s docs, in 100% safe code: covariance
    /// lets `set` store a `&str` that dies at the end of the block, and
    /// `get` hands it back as `&'static str`. Natively (debug): the
    /// allocator gave `to_owned` the freed block back, so its copy's source
    /// and destination overlapped, and std's precondition check on
    /// `copy_nonoverlapping` aborted the test binary. Another allocator
    /// might just copy stale bytes.
    /// Run (in deep-dive/): `cargo +nightly miri test --lib -- --ignored
    /// --exact ub_zoo::tests::ub_bad_cell_use_after_free`
    // miri-expect: Undefined Behavior: .*dangling reference \(use-after-free\)
    #[test]
    #[ignore = "UB on purpose: run it alone under Miri"]
    fn ub_bad_cell_use_after_free() {
        let cell: BadCell<&'static str> = BadCell::new("static");
        {
            let s = String::from("short-lived");
            cell.set(&s);
        }
        let dangling: &'static str = cell.get();
        black_box(dangling.to_owned());
    }

    /// Why the exploit compiles: this function only type-checks because
    /// `BadCell` is covariant in `T`. The same function for `GoodCell` is
    /// "lifetime may not live long enough".
    #[test]
    fn bad_cell_is_covariant() {
        fn shrink<'a>(cell: &'a BadCell<&'static str>) -> &'a BadCell<&'a str> {
            cell
        }
        let cell = BadCell::new("static");
        assert_eq!(shrink(&cell).get(), "static");
    }

    /// `GoodCell` still works for every legitimate use: here a borrow that
    /// outlives the cell.
    #[test]
    fn good_cell_stores_borrows_that_outlive_it() {
        let s = String::from("long-lived");
        let cell = GoodCell::new("static");
        cell.set(s.as_str());
        assert_eq!(cell.get(), "long-lived");
    }

    /// `set` drops the replaced value right away, and dropping the cell
    /// drops the last one: every value exactly once.
    #[test]
    fn good_cell_drops_every_value_exactly_once() {
        let drops = Cell::new(0);
        let cell = GoodCell::new(Tracked::new(&drops, 1));
        cell.set(Tracked::new(&drops, 2));
        assert_eq!(drops.get(), 1);
        cell.set(Tracked::new(&drops, 3));
        assert_eq!(drops.get(), 2);
        drop(cell);
        assert_eq!(drops.get(), 3);
    }
}
