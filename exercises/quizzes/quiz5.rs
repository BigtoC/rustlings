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

// TODO: every answer below is `Answer::Unanswered`, so each of the fourteen
// tests fails with "<NAME> is not answered yet". Replace each one with
// `SendAndSync`, `SendOnly`, `SyncOnly` or `Neither`, and for each be ready to
// say which condition on the type parameter (if any) decides it. Predict all
// fourteen first and run second: a guess you had to flip teaches you nothing.
// (`Cell` and `RefCell` are from `std::cell`, `Rc` from `std::rc`, the channel
// ends from `std::sync::mpsc`, everything else from `std::sync`.) Don't change
// the tests. Until every answer is right, the tests will fail.

// ---- Cells and locks ----

// Q1: `Cell<i32>`
const CELL: Answer = Answer::Unanswered;
// Q2: `Mutex<Cell<i32>>`
const MUTEX_OF_CELL: Answer = Answer::Unanswered;
// Q3: `RwLock<Cell<i32>>`
const RWLOCK_OF_CELL: Answer = Answer::Unanswered;
// Q4: `Mutex<Rc<i32>>`
const MUTEX_OF_RC: Answer = Answer::Unanswered;

// ---- Shared ownership and references ----

// Q5: `Arc<Cell<i32>>`
const ARC_OF_CELL: Answer = Answer::Unanswered;
// Q6: `Arc<Mutex<RefCell<i32>>>`
const ARC_OF_MUTEX_OF_REFCELL: Answer = Answer::Unanswered;
// Q7: `&Cell<i32>`
const SHARED_REF_TO_CELL: Answer = Answer::Unanswered;
// Q8: `&mut Cell<i32>`
const MUT_REF_TO_CELL: Answer = Answer::Unanswered;

// ---- Guards and channels ----

// Q9: `MutexGuard<'_, u32>`
const GUARD_OF_U32: Answer = Answer::Unanswered;
// Q10: `MutexGuard<'_, Cell<i32>>`
const GUARD_OF_CELL: Answer = Answer::Unanswered;
// Q11: `mpsc::Sender<String>`
const SENDER: Answer = Answer::Unanswered;
// Q12: `mpsc::Receiver<String>`
const RECEIVER: Answer = Answer::Unanswered;

// ---- Raw pointers and trait objects ----

// Q13: `*const u8`
const CONST_PTR: Answer = Answer::Unanswered;
// Q14: `Box<dyn Fn() + Send>`
const BOXED_SEND_CLOSURE: Answer = Answer::Unanswered;

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
