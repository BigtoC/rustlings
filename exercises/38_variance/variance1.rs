// Module 1 · Variance and PhantomData — part 1: pick the `PhantomData` marker: covariant, contravariant, `!Send`, and derives that add bounds (E0277, E0369).
//
// VARIANCE says how subtyping passes through a generic type. The only
// subtyping in Rust is between lifetimes: a `&'static str` may be used where a
// `&'a str` is expected, because a reference that is valid forever is valid
// for any shorter `'a`. For a generic type `F<T>`, the question is what
// happens to `F<&'static str>`:
//
//   - COVARIANT: it may be used as an `F<&'a str>`, the same direction.
//     `&T`, `Box<T>`, `Vec<T>`, `*const T` and `fn() -> T` are covariant.
//   - CONTRAVARIANT: the reverse, an `F<&'a str>` may be used as an
//     `F<&'static str>`. Only function ARGUMENTS are: `fn(T)`. A function that
//     accepts any `&'a str` can certainly be handed a `&'static str`.
//   - INVARIANT: neither, the lifetime must match exactly. `&mut T`, `Cell<T>`
//     (and everything else built on `UnsafeCell`), `*mut T`, `fn(T) -> T`,
//     and the type arguments of a trait object, `dyn Fn(T)` included.
//
// You never declare variance. rustc infers it from the fields: a struct is
// covariant in `T` if every use of `T` in its fields is covariant,
// contravariant if every use is contravariant, and invariant as soon as one
// use is invariant or two uses disagree. The same fields decide the auto
// traits: a struct is `Send` (or `Sync`) only if all of its fields are. That
// is why a type parameter that no field mentions is E0392 "type parameter `T`
// is never used": there would be nothing to infer from. The fix is a
// zero-sized `PhantomData<X>` field, which counts as an `X` for variance and
// for auto traits. Choosing `X` IS the design decision:
//
//   - `PhantomData<T>` says "I own a `T`": covariant, and `Send` / `Sync`
//     only when `T` is. Right for a container whose `T`s live behind a raw
//     pointer, as `Vec`'s do.
//   - `PhantomData<fn() -> T>` says "I can produce a `T`": covariant, and
//     ALWAYS `Send + Sync`, because a function pointer is.
//   - `PhantomData<fn(T)>` says "I consume `T`s": contravariant, and also
//     always `Send + Sync`.
//   - `PhantomData<*const ()>` says "stay on this thread": raw pointers are
//     neither `Send` nor `Sync`. On stable Rust a marker like this is the way
//     to opt out (`impl !Send for X {}` is E0658, unstable).
//
// The three types below started out with the wrong marker.
//
// `Id<T>` is a typed row id: an `Id<User>` cannot be passed where an
// `Id<Order>` is expected, yet at run time it is just a `u64`. With
// `PhantomData<T>` it inherits the auto traits of `T`, so an id into a table
// of `Rc`s cannot go to another thread (E0277 "`Rc<String>` cannot be sent
// between threads safely"), although there is no `Rc` in it. The same type
// has a second classic bug: `#[derive(Clone, PartialEq, ...)]` on a generic
// struct generates `impl<T: Clone> Clone for Id<T>`, with a bound on EVERY
// type parameter, whatever the fields are. So `Id<User>` is copyable,
// comparable, hashable and printable only if `User` is all of those too
// (E0369 for `==`, E0277 "the trait bound `User: Eq` is not satisfied", E0277
// "`User` doesn't implement `Debug`"). rustc's help line suggests
// `#[derive(PartialEq)]` on `User`, which is the wrong fix: a row type should
// not have to be `Hash` for its id to be. The other help line, "consider
// manually implementing `Eq` to avoid undesired bounds", is the right one.
//
// `Sink<T>` accepts `T`s and writes them out; it never stores or returns one.
// So a sink that accepts ANY `&'a str` can stand in wherever a
// `Sink<&'static str>` is needed. With `PhantomData<T>` the sink is covariant
// instead, and the test helper `widen` is rejected with "lifetime may not live
// long enough". That error has no code: it comes from the borrow checker,
// which runs after type checking, and it is the usual symptom of wrong
// variance.
//
// `LocalOnly` is a handle that only means something on the thread that
// created it, like a GUI window handle, or a guard that must be released on
// the thread that took it (std's `MutexGuard` is not `Send` for exactly that
// reason: POSIX requires a mutex to be unlocked by the thread that locked it).
// It must be neither `Send` nor `Sync`, and `PhantomData<()>` changes
// nothing, because `()` is both.
//
// How do you TEST that a type is not `Send`? An `assert_send::<X>()` call can
// only prove that it is. The honest tool is a `compile_fail` doctest, which a
// single-file exercise cannot have (the `deep-dive/src/api_surface.rs` lab
// uses them). The tests below use the trick of the `impls` crate instead:
// an inherent associated const wins over a trait's, but only when the
// inherent impl's bounds hold, so `Probe::<X>::IS_SEND` is `true` exactly when
// `X: Send`. It works for concrete types only. Inside a generic function the
// `T` has no `Send` bound, so the probe always answers `false`; that is why
// the tests wrap it in a macro and not in a function.
//
// How interviewers probe this: "Which `PhantomData` goes in a typed id, and
// why not `PhantomData<T>`? What does `PhantomData` change besides variance?
// Why is `&mut T` invariant? Is `dyn Fn(T)` contravariant? Why doesn't
// `#[derive(Clone)]` on `Id<T>` make every id cloneable?"

use std::fmt::{Display, Write};
use std::marker::PhantomData;

// ---------- `Id<T>`: a typed row id ----------

// TODO: two problems, both reported in the tests. (1) E0277 "`Rc<String>`
// cannot be sent between threads safely" (and "... shared between threads
// safely"): with `PhantomData<T>`, an `Id<T>` is exactly as `Send` and `Sync`
// as a `T`. (2) E0369 "binary operation `==` cannot be applied to type
// `Id<User>`", E0277 "the trait bound `User: Eq` is not satisfied" (and
// `User: Hash`), E0277 "`User` doesn't implement `Debug`" and E0277 "the trait
// bound `User: Clone` is not satisfied": each derive puts a bound on `T`.
// Requirements, for EVERY `T` (the tests' `User` implements nothing, and `Rc`
// is neither `Send` nor `Sync`): `Id<T>` is `Send + Sync`, covariant in `T`
// (the test helper `shrink` must keep compiling), `Copy`, `Clone`,
// `PartialEq`, `Eq`, `Hash` and `Debug`, and exactly the size of a `u64`. Two
// ids are equal when their raw numbers are, equal ids hash alike, and `{:?}`
// prints `Id(<raw>)`, e.g. `Id(7)`. Constraints: no `unsafe impl`, the `u64`
// is the only real field, and rustc's "consider annotating `User`" help is
// not an option: don't change `User` or the tests. Until you stop `Id<T>`
// from taking its auto traits and its trait impls from `T`, this exercise
// will not compile.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Id<T> {
    raw: u64,
    _row: PhantomData<T>,
}

impl<T> Id<T> {
    const fn new(raw: u64) -> Self {
        Id {
            raw,
            _row: PhantomData,
        }
    }

    const fn raw(self) -> u64 {
        self.raw
    }
}

// A minimal table that hands out typed ids.
struct Table<T> {
    rows: Vec<T>,
}

impl<T> Table<T> {
    fn new() -> Self {
        Table { rows: Vec::new() }
    }

    fn insert(&mut self, row: T) -> Id<T> {
        let id = Id::new(self.rows.len() as u64);
        self.rows.push(row);
        id
    }

    fn get(&self, id: Id<T>) -> Option<&T> {
        self.rows.get(usize::try_from(id.raw()).ok()?)
    }
}

// ---------- `Sink<T>`: consumes `T`s, never produces one ----------

// TODO: the test helper `widen` is rejected with "lifetime may not live long
// enough": with `PhantomData<T>` a `Sink` is COVARIANT in `T`, so a
// `Sink<&'a str>` could become a `Sink<&'static str>` only if `'a` were
// `'static`. Requirement: a sink for any `&'a str` can be used as a
// `Sink<&'static str>` (it never hands a `T` back, so accepting shorter-lived
// strings than it is asked to is safe), `widen` compiles unchanged, and a
// `Sink` stays the size of its `String`. Constraints: don't change `widen` or
// the other tests, and no `unsafe`. Until you give `Sink` the variance of
// something that only consumes `T`s, this exercise will not compile.
struct Sink<T> {
    out: String,
    _accepts: PhantomData<T>,
}

impl<T: Display> Sink<T> {
    fn new() -> Self {
        Sink {
            out: String::new(),
            _accepts: PhantomData,
        }
    }

    // Writes `item` as one line.
    fn send(&mut self, item: T) {
        writeln!(self.out, "{item}").expect("writing to a `String` cannot fail");
    }

    fn output(&self) -> &str {
        &self.out
    }
}

// ---------- `LocalOnly`: must stay on the thread that made it ----------

// TODO: the test `local_only_is_neither_send_nor_sync` fails: `()` is `Send`
// and `Sync`, so a `PhantomData<()>` changes nothing and `LocalOnly` is as
// `Send + Sync` as its `u32`. Requirement: `LocalOnly` is neither `Send` nor
// `Sync`, and it stays the size of a `u32` (a test checks). Constraints:
// stable Rust (`impl !Send for LocalOnly {}` is E0658 "negative impls are
// experimental"), no `unsafe`, and don't change the tests. Until you opt
// `LocalOnly` out of both auto traits, the tests will fail.
struct LocalOnly {
    slot: u32,
    _not_send: PhantomData<()>,
}

impl LocalOnly {
    fn new(slot: u32) -> Self {
        LocalOnly {
            slot,
            _not_send: PhantomData,
        }
    }

    fn slot(&self) -> u32 {
        self.slot
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::HashSet;
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::rc::Rc;
    use std::thread;

    // A row type that implements NOTHING: not `Clone`, not `PartialEq`, not
    // `Hash`, not `Debug`.
    struct User {
        name: String,
    }

    fn user(name: &str) -> User {
        User {
            name: name.to_string(),
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    // Calls `Clone::clone` through a generic bound. (A direct `.clone()` on a
    // `Copy` type is a clippy lint.)
    fn duplicate<C: Clone>(value: &C) -> C {
        value.clone()
    }

    // Hashes both values, each where it is stored, with the same hasher.
    fn same_hash<H: Hash>(a: &H, b: &H) -> bool {
        let hash = |value: &H| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };
        hash(a) == hash(b)
    }

    // ----- Variance checks. Each body is just its argument, so it compiles
    // only if rustc accepts the conversion as subtyping. Don't change them.

    // Covariance: an id minted for a table of `&'static str` rows also
    // identifies a row in a table of shorter-lived `&'a str`s.
    fn shrink<'a>(id: Id<&'static str>) -> Id<&'a str> {
        id
    }

    // Contravariance: a sink that takes any `&str` can go where a sink for
    // `&'static str` is expected.
    fn widen(sink: Sink<&str>) -> Sink<&'static str> {
        sink
    }

    // ----- The `Send` / `Sync` probe (see the comment at the top). An
    // inherent const is found first, but only if its impl's bound holds;
    // otherwise the trait's default applies.
    struct Probe<T: ?Sized>(PhantomData<T>);

    trait Fallback {
        const IS_SEND: bool = false;
        const IS_SYNC: bool = false;
    }

    impl<T: ?Sized> Fallback for Probe<T> {}

    impl<T: ?Sized + Send> Probe<T> {
        const IS_SEND: bool = true;
    }

    impl<T: ?Sized + Sync> Probe<T> {
        const IS_SYNC: bool = true;
    }

    // `(is it Send, is it Sync)` for a CONCRETE type.
    macro_rules! send_sync {
        ($t:ty) => {
            (Probe::<$t>::IS_SEND, Probe::<$t>::IS_SYNC)
        };
    }

    // ---------- `Id<T>` ----------

    #[test]
    fn ids_work_for_a_row_type_that_implements_nothing() {
        let mut users = Table::new();
        let alice = users.insert(user("alice"));
        let bob = users.insert(user("bob"));

        let again = alice; // a copy: `alice` is still usable below
        assert!(alice == again);
        assert!(alice != bob);
        assert_eq!(duplicate(&bob).raw(), bob.raw());
        assert!(same_hash(&alice, &again), "equal ids must hash alike");

        // Equal ids collapse into one entry, different ids stay apart.
        let seen: HashSet<Id<User>> = [alice, again, bob].into_iter().collect();
        assert_eq!(seen.len(), 2);

        assert_eq!(format!("{:?}", [alice, bob]), "[Id(0), Id(1)]");
        assert_eq!(users.get(alice).map(|u| u.name.as_str()), Some("alice"));
        assert_eq!(users.get(bob).map(|u| u.name.as_str()), Some("bob"));
    }

    #[test]
    fn ids_cross_threads_even_when_their_rows_cannot() {
        assert_send_sync::<Id<Rc<String>>>();

        // `Rc` is neither `Send` nor `Sync`, so this table stays on this
        // thread...
        let mut nodes = Table::new();
        let first = nodes.insert(Rc::new("first".to_string()));
        let second = nodes.insert(Rc::new("second".to_string()));

        // ...but its ids are plain numbers: a worker may pick one and send it
        // back.
        let picked = thread::spawn(move || {
            if second.raw() > first.raw() {
                second
            } else {
                first
            }
        })
        .join()
        .unwrap();

        assert_eq!(picked, second);
        assert_eq!(format!("{picked:?}"), "Id(1)");
        assert_eq!(nodes.get(picked).map(|row| row.as_str()), Some("second"));
        // The ids were copied into the thread, not moved.
        assert_eq!(nodes.get(first).map(|row| row.as_str()), Some("first"));
    }

    #[test]
    fn ids_are_covariant_and_cost_nothing() {
        let local = String::from("borrowed");
        let mut words: Table<&str> = Table::new();
        words.insert("static");
        words.insert(&local);

        let minted: Id<&'static str> = Id::new(1);
        assert_eq!(words.get(shrink(minted)), Some(&"borrowed"));

        assert_eq!(size_of::<Id<String>>(), size_of::<u64>());
        assert_eq!(size_of::<Id<User>>(), size_of::<u64>());
        assert_eq!(size_of::<Id<[u8; 1024]>>(), size_of::<u64>());
    }

    // ---------- `Sink<T>` ----------

    #[test]
    fn a_sink_writes_one_line_per_item() {
        let mut numbers = Sink::new();
        numbers.send(1);
        numbers.send(20);
        assert_eq!(numbers.output(), "1\n20\n");
        assert_eq!(size_of::<Sink<u64>>(), size_of::<String>());
    }

    #[test]
    fn a_sink_for_any_str_stands_in_for_a_static_one() {
        let owned = String::from("dynamic");
        let mut sink = Sink::new();
        sink.send(owned.as_str()); // a sink for the borrow of `owned`

        let mut registry: Vec<Sink<&'static str>> = vec![widen(sink)];

        // The sink kept only the text it wrote, never the `&str`, so `owned`
        // may go away while the sink lives on.
        drop(owned);
        registry[0].send("static");
        assert_eq!(registry[0].output(), "dynamic\nstatic\n");
    }

    // ---------- `LocalOnly` ----------

    #[test]
    fn the_probe_tells_send_and_sync_apart() {
        // Sanity checks, so you can trust the probe.
        assert_eq!(send_sync!(u32), (true, true));
        assert_eq!(send_sync!(Rc<u32>), (false, false));
        assert_eq!(send_sync!(Cell<u32>), (true, false));
        assert_eq!(send_sync!(*const u8), (false, false));
    }

    #[test]
    fn local_only_is_neither_send_nor_sync() {
        let handle = LocalOnly::new(7);
        assert_eq!(handle.slot(), 7);
        assert_eq!(size_of::<LocalOnly>(), size_of::<u32>());
        assert_eq!(
            send_sync!(LocalOnly),
            (false, false),
            "`LocalOnly` must be neither `Send` nor `Sync` (left: what it is now)"
        );
    }
}
