// Quiz 4 · Method resolution and borrowck verdicts — name the impl you mean with fully qualified syntax (E0034, E0790), then predict rustc's verdict on ten borrowing snippets.
//
// Two kinds of rapid-fire question come up in almost every Rust interview
// loop: "which function does this call run?" and "does this compile, and if
// not, what does rustc say?". This quiz, placed after `47_type_level`, asks
// both.
//
// Part A is a fix-it. rustc resolves a METHOD call `x.fly()` from the type of
// `x`. For each receiver type it tries (`x` itself, then `&x` and `&mut x`,
// then the same again after each deref), it looks at inherent methods first
// and at the methods of traits in scope second. So an inherent `fly` hides a
// trait's `fly` with the same receiver, while two traits that both offer a
// `fly` (and no inherent one) leave the call ambiguous. A PATH call is
// resolved from the path you write instead. `Type::f(..)` also tries inherent
// items first. `Trait::f(..)` fixes the trait, but the type is still inferred,
// which only works when the arguments (or the type the result must have) pin
// `Self` down. The fully qualified form `<Type as Trait>::f(..)` names both,
// so it always works, for methods, associated functions and associated
// constants alike. `42_coherence/coherence3` and `43_assoc_types/assoc2`
// already used these paths; here you meet the two errors that force you to
// write one. Part A must compile before the Part B tests can run.
//
// Part B is the quiz proper: ten snippets, and for each one the verdict rustc
// gives. Six are about TWO-PHASE BORROWS, which the course has not named yet.
// A two-phase borrow is a `&mut` borrow that starts out merely RESERVED: until
// it is first used, the borrowed place may still be read through shared
// borrows, but not written or mutably borrowed a second time. rustc makes only
// some implicit `&mut` borrows two-phase, and which ones is what those six
// questions are about. The other four revisit what `37_borrowck_errors`,
// `39_drop_raii` and `33_closures` drilled.
//
// How interviewers probe it: "Two traits in scope both define `fly`; how do
// you call each one?", "A type has an inherent method and a trait method with
// the same name; which one does `x.m()` run?", "What is a two-phase borrow,
// and which `&mut` borrows get one?".

// ---- Part A: call the impl you mean -----------------------------------------

trait Pilot {
    fn fly(&self) -> String;
}

trait Wizard {
    fn fly(&self) -> String;
}

struct Human {
    name: String,
}

impl Pilot for Human {
    fn fly(&self) -> String {
        format!("This is your captain, {}, speaking.", self.name)
    }
}

impl Wizard for Human {
    fn fly(&self) -> String {
        format!("{} rises into the air on a broomstick.", self.name)
    }
}

// What `h` says over the intercom: the `Pilot` side of `fly`.
fn cockpit_announcement(h: &Human) -> String {
    // TODO: rustc rejects `h.fly()` with E0034 "multiple applicable items in
    // scope": `Human` has no inherent `fly`, and its `Pilot` and `Wizard`
    // impls both provide one, so method-call syntax cannot choose. Call the
    // `Pilot` implementation explicitly. Requirements: keep the signature; no
    // renaming or removing a trait, a method or an impl (the tests use the
    // `Wizard` side too); read rustc's `help:` lines critically, since `h` is
    // already a reference. Until you name the trait in the call, this exercise
    // will not compile.
    h.fly()
}

trait Animal {
    fn baby_name() -> String;
}

struct Dog;

impl Dog {
    // An inherent associated function with the same name as the trait's.
    fn baby_name() -> String {
        String::from("Spot")
    }
}

impl Animal for Dog {
    fn baby_name() -> String {
        String::from("puppy")
    }
}

struct Cat;

impl Animal for Cat {
    fn baby_name() -> String {
        String::from("kitten")
    }
}

// What `T`'s `Animal` impl calls a young one.
fn baby_name_of<T: Animal>() -> String {
    // TODO: rustc rejects `Animal::baby_name()` with E0790 "cannot call
    // associated function on trait without specifying the corresponding
    // `impl` type": `baby_name` has no `self` parameter and does not return
    // `Self`, so nothing in the call says whose impl to run. Its `help:`
    // offers the concrete impls it can see. Requirements: keep the signature;
    // every `T: Animal` must get the answer from its OWN impl (the tests also
    // use an animal defined in the tests), so no hard-coded type and no
    // matching on type names. Until you say which impl to call, this exercise
    // will not compile.
    Animal::baby_name()
}

// ---- Part B: will it compile? -----------------------------------------------
//
// Each snippet is a function marked `#[cfg(any())]`. An empty `any()` is
// always false, so rustc parses these functions but never type-checks or
// borrow-checks them, and the file compiles whatever they contain. For each
// snippet, choose the `Verdict` rustc gives: `Compiles`, or the code of the
// error it reports. A snippet that fails reports one kind of error only
// (sometimes at more than one place).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// The placeholder: not answered yet.
    Unanswered,
    /// rustc accepts the snippet.
    Compiles,
    /// E0382 "borrow of moved value" or "use of moved value".
    E0382,
    /// E0499 "cannot borrow `x` as mutable more than once at a time".
    E0499,
    /// E0502 "cannot borrow `x` as immutable because it is also borrowed as
    /// mutable" (or the other way around).
    E0502,
    /// E0505 "cannot move out of `x` because it is borrowed".
    E0505,
    /// E0506 "cannot assign to `x` because it is borrowed".
    E0506,
    /// E0515 "cannot return reference to local variable `x`" (or "returns a
    /// value referencing data owned by the current function").
    E0515,
    /// E0597 "`x` does not live long enough".
    E0597,
    /// E0716 "temporary value dropped while borrowed".
    E0716,
    /// "lifetime may not live long enough": a borrow-checker error that has
    /// no error code.
    LifetimeMayNotLiveLongEnough,
}

// TODO: every `Q*` constant below is `Verdict::Unanswered`, so each `q*` test
// fails with "Qn is not answered yet". Replace each one with the verdict you
// predict from reading the snippet above it. Each test hashes your answer and
// compares it with the hash of the right one, so a failure says WHICH answer
// is wrong but never what the right one is. Answer all ten before you run
// anything: a guess you had to flip teaches you nothing. Afterwards, for each
// answer you got wrong, change that snippet's `#[cfg(any())]` to
// `#[cfg(all())]` (always true), read rustc's own explanation, and change it
// back. Don't change the snippets or the tests. Until every verdict is right,
// the tests will fail.

// Q1. Push the length.
#[cfg(any())]
fn q1() {
    let mut v: Vec<usize> = vec![1, 2, 3];
    v.push(v.len());
    println!("{v:?}");
}

const Q1: Verdict = Verdict::Unanswered;

// Q2. Push what you pop.
#[cfg(any())]
fn q2() {
    let mut v = vec![1, 2, 3];
    v.push(v.pop().unwrap_or(0));
    println!("{v:?}");
}

const Q2: Verdict = Verdict::Unanswered;

// Q3. Overwrite the last element of a `Vec`.
#[cfg(any())]
fn q3() {
    let mut v = vec![1, 2, 3];
    v[v.len() - 1] = 7;
    println!("{v:?}");
}

const Q3: Verdict = Verdict::Unanswered;

// Q4. The same with an array.
#[cfg(any())]
fn q4() {
    let mut a = [1, 2, 3];
    a[a.len() - 1] = 7;
    println!("{a:?}");
}

const Q4: Verdict = Verdict::Unanswered;

// Q5. Swap in a copy of itself.
#[cfg(any())]
fn q5() {
    let mut v = vec![1, 2, 3];
    let old = std::mem::replace(&mut v, v.clone());
    println!("{old:?} {v:?}");
}

const Q5: Verdict = Verdict::Unanswered;

// Q6. Push the length, through a `&mut` binding.
#[cfg(any())]
fn q6() {
    let mut v: Vec<usize> = vec![1, 2, 3];
    let r = &mut v;
    r.push(r.len());
    println!("{v:?}");
}

const Q6: Verdict = Verdict::Unanswered;

// Q7. Borrow a temporary.
#[cfg(any())]
fn q7() {
    let greeting = &String::from("hello");
    println!("{greeting}");
}

const Q7: Verdict = Verdict::Unanswered;

// Q8. Borrow from a temporary through a method.
#[cfg(any())]
fn q8() {
    let greeting = String::from("hello").as_str();
    println!("{greeting}");
}

const Q8: Verdict = Verdict::Unanswered;

// Q9. Get or insert.
#[cfg(any())]
fn q9(map: &mut std::collections::HashMap<u32, Vec<u32>>, key: u32) -> &mut Vec<u32> {
    if let Some(list) = map.get_mut(&key) {
        return list;
    }
    map.insert(key, Vec::new());
    map.get_mut(&key).unwrap()
}

const Q9: Verdict = Verdict::Unanswered;

// Q10. A closure that returns part of its argument.
#[cfg(any())]
fn q10() {
    let first_word = |s: &str| -> &str { s.split(' ').next().unwrap_or("") };
    println!("{}", first_word("hello world"));
}

const Q10: Verdict = Verdict::Unanswered;

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Part A ----

    // With a single trait bound in play, a method call has one candidate.
    fn wizard_line<W: Wizard>(w: &W) -> String {
        w.fly()
    }

    // An animal that the code above has never heard of.
    struct Duck;

    impl Animal for Duck {
        fn baby_name() -> String {
            String::from("duckling")
        }
    }

    #[test]
    fn the_cockpit_announcement_comes_from_the_pilot_impl() {
        let ada = Human {
            name: String::from("Ada"),
        };
        assert_eq!(
            cockpit_announcement(&ada),
            "This is your captain, Ada, speaking."
        );
        let grace = Human {
            name: String::from("Grace"),
        };
        assert_eq!(
            cockpit_announcement(&grace),
            "This is your captain, Grace, speaking."
        );
    }

    #[test]
    fn humans_can_still_fly_as_wizards() {
        let ada = Human {
            name: String::from("Ada"),
        };
        assert_eq!(wizard_line(&ada), "Ada rises into the air on a broomstick.");
    }

    #[test]
    fn every_animal_answers_from_its_own_impl() {
        assert_eq!(baby_name_of::<Cat>(), "kitten");
        assert_eq!(baby_name_of::<Duck>(), "duckling");
    }

    #[test]
    fn a_dog_answers_from_its_animal_impl() {
        // `Dog`'s inherent function is still there, and still says "Spot"...
        assert_eq!(Dog::baby_name(), "Spot");
        // ...but `baby_name_of` asks the `Animal` impl.
        assert_eq!(baby_name_of::<Dog>(), "puppy");
    }

    // ---- Part B ----

    // FNV-1a (64 bits) of "<question>=<answer>". Each test compares it with
    // the hash of the right answer, so a failure names the question and never
    // the answer.
    fn fingerprint(question: &str, answer: Verdict) -> u64 {
        format!("{question}={answer:?}")
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            })
    }

    fn check(question: &str, answer: Verdict, expected: u64, nudge: &str) {
        assert!(
            answer != Verdict::Unanswered,
            "{question} is not answered yet"
        );
        assert!(
            fingerprint(question, answer) == expected,
            "{question} is wrong: {nudge}"
        );
    }

    #[test]
    fn q1_push_the_length() {
        check(
            "Q1",
            Q1,
            0x00d8_acc8_0366_9604,
            "what does `push` borrow for its receiver, and when does that \
             borrow start to count?",
        );
    }

    #[test]
    fn q2_push_what_you_pop() {
        check(
            "Q2",
            Q2,
            0x68a8_562a_5659_23d2,
            "what may happen to `v` while the receiver's borrow is only \
             reserved?",
        );
    }

    #[test]
    fn q3_overwrite_the_last_element_of_a_vec() {
        check(
            "Q3",
            Q3,
            0x3c80_f1d2_c3b1_00de,
            "what does `v[..] = 7` call on a `Vec`, and when is its `&mut v` \
             taken?",
        );
    }

    #[test]
    fn q4_overwrite_the_last_element_of_an_array() {
        check(
            "Q4",
            Q4,
            0x73f4_ac81_afcd_fe25,
            "is indexing an array a method call?",
        );
    }

    #[test]
    fn q5_swap_in_a_copy_of_itself() {
        check(
            "Q5",
            Q5,
            0xacd2_2f9b_4726_d8f8,
            "what kind of `&mut` borrow is the first argument, and when is \
             the second one evaluated?",
        );
    }

    #[test]
    fn q6_push_the_length_through_a_mut_binding() {
        check(
            "Q6",
            Q6,
            0x39f8_4de6_0024_7d8b,
            "what does calling `push` through `r` borrow, and is that borrow \
             two-phase?",
        );
    }

    #[test]
    fn q7_borrow_a_temporary() {
        check(
            "Q7",
            Q7,
            0x9015_eae6_11b0_8ce6,
            "how long does the `String` live?",
        );
    }

    #[test]
    fn q8_borrow_from_a_temporary_through_a_method() {
        check(
            "Q8",
            Q8,
            0x8df8_7b56_58ff_57c0,
            "how long does the `String` live, and what does `greeting` point \
             into?",
        );
    }

    #[test]
    fn q9_get_or_insert() {
        check(
            "Q9",
            Q9,
            0x5ae4_d917_3659_7c7f,
            "how long must the borrow from `get_mut` last once it might be \
             returned?",
        );
    }

    #[test]
    fn q10_a_closure_that_returns_part_of_its_argument() {
        check(
            "Q10",
            Q10,
            0x686b_7b3f_a108_7d51,
            "where does a `let`-bound closure get its signature from?",
        );
    }
}
