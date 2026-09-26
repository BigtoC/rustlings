// Quiz 5 · Send and Sync — classify fourteen std types as Send and Sync, Send only, Sync only, or neither, and know the rule behind each answer.
//
// "Is `Mutex<Cell<i32>>` Sync? Is `RwLock<Cell<i32>>`? Is a `MutexGuard`
// Send?" Concurrency interviews fire these one after another, and a yes or a
// no is only half an answer: the interviewer wants the RULE behind it. The
// concurrency modules met several of these types one at a time
// (`30_send_sync`, `51_scoped_threads`, `53_lock_hazards`, `54_channels`,
// `56_async_bounds`); this quiz asks about fourteen of them at once.
//
// The definitions, from `std::marker`:
//
//   - `T: Send` means a value of type `T` can be MOVED to another thread:
//     its ownership may pass from one thread to another.
//   - `T: Sync` means a `&T` can be shared with another thread, so several
//     threads may use one `T` through shared references at the same time. By
//     definition, `T` is Sync exactly when `&T` is Send.
//
// Both are auto traits. A struct, enum, tuple or closure is Send (or Sync)
// when all of its fields (a closure's captures) are, and nobody has to write
// an impl for it. The types that manage sharing themselves (cells, locks,
// reference counts, guards, channels, raw pointers) have explicit impls
// instead, and most of those impls carry a condition on the type parameter:
// "`Foo<T>` is Sync if `T` is ...". (Trait objects follow a rule of their
// own, which Q14 asks about.) Those conditions are the quiz. For each type,
// ask what it would let two threads do at the same time, and what `T` must
// allow for that to be safe. Send and Sync are separate questions: answer
// both for every type.
//
// How the tests check you: each test asks the compiler whether the type is
// Send and whether it is Sync (with a small probe in the tests module), and
// compares that with your answer. The failure message names the type and
// asks a leading question, but never shows the right answer. Nothing stops
// you from printing the probe's result instead of thinking, as with any quiz
// that is checked at run time. It just teaches you nothing.
//
// How interviewers probe it: "Why is `Rc` not Send when `Arc` is?", "Can a
// type be Sync but not Send? Name one.", "What must a type guarantee before
// you write `unsafe impl Sync` for it?", "Why is `Box<dyn Error>` a poor
// error type for code that crosses threads?" (`35_error_design` asks for
// `Box<dyn Error + Send + Sync>`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// The placeholder: not answered yet.
    Unanswered,
    /// The type is Send and Sync.
    SendAndSync,
    /// The type is Send but not Sync.
    SendOnly,
    /// The type is Sync but not Send.
    SyncOnly,
    /// The type is neither Send nor Sync.
    Neither,
}

// Every answer applies one row of this table (`T: ?Sized` omitted):
//
//   type                  Send if            Sync if
//   Cell<T>, RefCell<T>   T: Send            never
//   Mutex<T>              T: Send            T: Send
//   RwLock<T>             T: Send            T: Send + Sync
//   Rc<T>                 never              never
//   Arc<T>                T: Send + Sync     T: Send + Sync
//   &T                    T: Sync            T: Sync
//   &mut T                T: Send            T: Sync
//   MutexGuard<'_, T>     never              T: Sync
//   mpsc::Sender<T>       T: Send            T: Send (since Rust 1.72)
//   mpsc::Receiver<T>     T: Send            never
//   *const T, *mut T      never              never
//   Box<T>                T: Send            T: Sync
//
// Each "never" is an explicit negative impl, such as
// `impl<T: ?Sized> !Sync for Cell<T> {}`, which only std can write on stable
// Rust. Your own types get the same effect from a field that is not Send or
// Sync, such as a `PhantomData<*const ()>` (`38_variance/variance1`).

// ---- Cells and locks ----

// Q1: `Cell<i32>`
// Moving a `Cell` moves the only way to reach it, so it is Send when its
// content is. Sharing `&Cell` would let two threads `set` it with no
// synchronization, so it is never Sync.
const CELL: Answer = Answer::SendOnly;
// Q2: `Mutex<Cell<i32>>`
// The lock hands the `Cell` (as `&mut Cell`) to one thread at a time, which
// is no different from moving it there, so a `Mutex` only asks `T: Send`, for
// both traits. This is how a `Mutex` makes a `!Sync` type shareable.
const MUTEX_OF_CELL: Answer = Answer::SendAndSync;
// Q3: `RwLock<Cell<i32>>`
// Readers on several threads hold `&Cell` at the same time, so sharing an
// `RwLock` also needs `T: Sync`, and `Cell` is not (`53_lock_hazards`).
const RWLOCK_OF_CELL: Answer = Answer::SendOnly;
// Q4: `Mutex<Rc<i32>>`
// Both traits need `Rc: Send`, and it is not. Whoever holds the lock can
// clone the `Rc` and keep the clone outside it, and then two threads update
// one non-atomic count. A lock protects its content, not the other clones.
const MUTEX_OF_RC: Answer = Answer::Neither;

// ---- Shared ownership and references ----

// Q5: `Arc<Cell<i32>>`
// Every clone of an `Arc` gives out `&T`, and dropping the last one drops the
// `T` on whichever thread that happens. So both traits need `T: Send + Sync`,
// and `Cell` is not Sync.
const ARC_OF_CELL: Answer = Answer::Neither;
// Q6: `Arc<Mutex<RefCell<i32>>>`
// `RefCell<i32>` is Send, so `Mutex<RefCell<i32>>` is Send and Sync, so the
// `Arc` around it is Send and Sync too: each layer asks only what the layer
// inside it provides.
const ARC_OF_MUTEX_OF_REFCELL: Answer = Answer::SendAndSync;
// Q7: `&Cell<i32>`
// `&T` is Send exactly when `T` is Sync (that is the definition of Sync),
// and a shared `&&T` is only another way to reach `&T`, so it is Sync on the
// same condition. `Cell` is not Sync.
const SHARED_REF_TO_CELL: Answer = Answer::Neither;
// Q8: `&mut Cell<i32>`
// Sending a `&mut T` hands over exclusive access, like moving the `T`, so it
// needs `T: Send`. A shared `&&mut T` only allows reading, as a `&T` does, so
// Sync needs `T: Sync`.
const MUT_REF_TO_CELL: Answer = Answer::SendOnly;

// ---- Guards and channels ----

// Q9: `MutexGuard<'_, u32>`
// Dropping a guard unlocks the mutex, and with POSIX threads only the thread
// that locked a mutex may unlock it, so a guard never changes threads
// (`56_async_bounds`). A shared `&MutexGuard` only gives `&T`, so the guard
// is Sync when `T` is.
const GUARD_OF_U32: Answer = Answer::SyncOnly;
// Q10: `MutexGuard<'_, Cell<i32>>`
// Still never Send, and a shared `&MutexGuard<Cell<i32>>` would give two
// threads `&Cell`, so not Sync either, even though `Mutex<Cell<i32>>` itself
// is Sync.
const GUARD_OF_CELL: Answer = Answer::Neither;
// Q11: `mpsc::Sender<String>`
// Since Rust 1.72, `Sender<T>` is Sync when `T` is Send (std's `mpsc` has
// been a port of `crossbeam-channel` since 1.67), so threads may share one
// `&Sender` instead of each cloning their own (`54_channels`).
const SENDER: Answer = Answer::SendAndSync;
// Q12: `mpsc::Receiver<String>`
// The receiving end can move to another thread, but only one thread may
// receive at a time: `Receiver` is never Sync. Several consumers share it as
// `Arc<Mutex<Receiver<T>>>` (`54_channels`).
const RECEIVER: Answer = Answer::SendOnly;

// ---- Raw pointers and trait objects ----

// Q13: `*const u8`
// The compiler knows nothing about who owns the data behind a raw pointer or
// who else uses it, so raw pointers are neither, as a conservative default. A
// type built on them opts in with `unsafe impl Send` after checking its own
// invariants, as `Arc` does.
const CONST_PTR: Answer = Answer::Neither;
// Q14: `Box<dyn Fn() + Send>`
// A `Box` is Send or Sync exactly when its content is, and a trait object has
// only the auto traits its type names (directly or through a supertrait):
// `dyn Fn() + Send` promises Send, not Sync. It is the same reason a
// `Box<dyn Error>` cannot cross threads (`35_error_design`).
const BOXED_SEND_CLOSURE: Answer = Answer::SendOnly;

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::marker::PhantomData;
    use std::rc::Rc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, Mutex, MutexGuard, RwLock};

    // The probe. `Probe::<T>::IS_SEND` names an associated constant, and rustc
    // looks for it in `Probe`'s inherent impls before it looks at traits. The
    // inherent impl below exists only when `T: Send`, so for a Send type it
    // wins and says `true`; for any other type rustc falls back to the
    // trait's default, `false`. It only works for concrete types, which is
    // all these tests need: in a generic function, rustc would decide from
    // `T`'s bounds alone, before any real type is known. Outside a quiz, you
    // would check a type with `fn assert_send<T: Send>() {}` and state the
    // negative cases as `compile_fail` doctests.
    struct Probe<T: ?Sized>(PhantomData<T>);

    trait NotSend {
        const IS_SEND: bool = false;
    }

    impl<T: ?Sized> NotSend for Probe<T> {}

    impl<T: ?Sized + Send> Probe<T> {
        const IS_SEND: bool = true;
    }

    trait NotSync {
        const IS_SYNC: bool = false;
    }

    impl<T: ?Sized> NotSync for Probe<T> {}

    impl<T: ?Sized + Sync> Probe<T> {
        const IS_SYNC: bool = true;
    }

    // What the compiler says about `$t`, as an `Answer`.
    macro_rules! actual {
        ($t:ty) => {
            classify(Probe::<$t>::IS_SEND, Probe::<$t>::IS_SYNC)
        };
    }

    fn classify(send: bool, sync: bool) -> Answer {
        match (send, sync) {
            (true, true) => Answer::SendAndSync,
            (true, false) => Answer::SendOnly,
            (false, true) => Answer::SyncOnly,
            (false, false) => Answer::Neither,
        }
    }

    fn check(name: &str, answer: Answer, actual: Answer, nudge: &str) {
        assert!(answer != Answer::Unanswered, "{name} is not answered yet");
        assert!(answer == actual, "{name} is wrong: {nudge}");
    }

    // ---- Cells and locks ----

    #[test]
    fn q1_cell() {
        check(
            "CELL",
            CELL,
            actual!(Cell<i32>),
            "may a `Cell` change threads? may two threads call `set` on the \
             same one?",
        );
    }

    #[test]
    fn q2_mutex_of_cell() {
        check(
            "MUTEX_OF_CELL",
            MUTEX_OF_CELL,
            actual!(Mutex<Cell<i32>>),
            "how many threads can reach the `Cell` inside at the same time?",
        );
    }

    #[test]
    fn q3_rwlock_of_cell() {
        check(
            "RWLOCK_OF_CELL",
            RWLOCK_OF_CELL,
            actual!(RwLock<Cell<i32>>),
            "what do readers get, and how many of them at the same time?",
        );
    }

    #[test]
    fn q4_mutex_of_rc() {
        check(
            "MUTEX_OF_RC",
            MUTEX_OF_RC,
            actual!(Mutex<Rc<i32>>),
            "the lock guards this `Rc`; does it guard its clones?",
        );
    }

    // ---- Shared ownership and references ----

    #[test]
    fn q5_arc_of_cell() {
        check(
            "ARC_OF_CELL",
            ARC_OF_CELL,
            actual!(Arc<Cell<i32>>),
            "sending one clone of an `Arc` lets two threads reach what?",
        );
    }

    #[test]
    fn q6_arc_of_mutex_of_refcell() {
        check(
            "ARC_OF_MUTEX_OF_REFCELL",
            ARC_OF_MUTEX_OF_REFCELL,
            actual!(Arc<Mutex<RefCell<i32>>>),
            "what does each layer need from the layer inside it?",
        );
    }

    #[test]
    fn q7_shared_ref_to_cell() {
        check(
            "SHARED_REF_TO_CELL",
            SHARED_REF_TO_CELL,
            actual!(&Cell<i32>),
            "sending a `&T` shares the `T`; sharing a `&&T` shares what?",
        );
    }

    #[test]
    fn q8_mut_ref_to_cell() {
        check(
            "MUT_REF_TO_CELL",
            MUT_REF_TO_CELL,
            actual!(&mut Cell<i32>),
            "sending a `&mut T` hands over exclusive access; how many threads \
             can use the `Cell` then? and what does a `&&mut T` allow?",
        );
    }

    // ---- Guards and channels ----

    #[test]
    fn q9_guard_of_u32() {
        check(
            "GUARD_OF_U32",
            GUARD_OF_U32,
            actual!(MutexGuard<'_, u32>),
            "which thread unlocks the mutex when a moved guard is dropped? \
             what does a `&MutexGuard` give access to?",
        );
    }

    #[test]
    fn q10_guard_of_cell() {
        check(
            "GUARD_OF_CELL",
            GUARD_OF_CELL,
            actual!(MutexGuard<'_, Cell<i32>>),
            "compare it with the guard of a `u32`: what does a shared \
             `&MutexGuard<Cell<i32>>` give access to?",
        );
    }

    #[test]
    fn q11_sender() {
        check(
            "SENDER",
            SENDER,
            actual!(Sender<String>),
            "can a `Sender` move to another thread? can several threads send \
             through one shared `&Sender`?",
        );
    }

    #[test]
    fn q12_receiver() {
        check(
            "RECEIVER",
            RECEIVER,
            actual!(Receiver<String>),
            "can the receiving end move to another thread? could two threads \
             call `recv` through one shared `&Receiver`?",
        );
    }

    // ---- Raw pointers and trait objects ----

    #[test]
    fn q13_const_ptr() {
        check(
            "CONST_PTR",
            CONST_PTR,
            actual!(*const u8),
            "what does the compiler know about who owns, or who else \
             touches, the byte behind a raw pointer?",
        );
    }

    #[test]
    fn q14_boxed_send_closure() {
        check(
            "BOXED_SEND_CLOSURE",
            BOXED_SEND_CLOSURE,
            actual!(Box<dyn Fn() + Send>),
            "a trait object is only what its type spells out; which auto \
             traits does `dyn Fn() + Send` promise?",
        );
    }
}
