// Traits & Abstraction · Deref, Borrow and Cow — part 1: `Deref` for a validated newtype, and why not `DerefMut` (E0308, E0599, E0614).
//
// A newtype such as `struct Username(String)` exists to hold an INVARIANT:
// `Username::new` checks the text once, so every `Username` in the program is
// known to be valid, and no function that receives one has to check again.
// The price is that the wrapper hides everything `str` can do. A caller who
// only wants to print, compare or search the name hits a wall, because a
// `&Username` is not a `&str`: E0308 "mismatched types ... expected `&str`,
// found `&Username`", E0599 "no method named `len` found for struct
// `Username`", and for `*name`, E0614 "type `Username` cannot be
// dereferenced".
//
// `std::ops::Deref` removes the wall. An `impl Deref for T` with
// `type Target = U` buys three things:
//   - `*v` means `*Deref::deref(&v)`: a place of type `U`, so `&*v` is a `&U`.
//   - DEREF COERCION: where the compiler already knows it needs a `&U`, it
//     accepts a `&T` and inserts the `deref` calls itself, as many as it
//     takes: `&Box<Username>` -> `&Username` -> `&str`.
//   - METHOD LOOKUP: `v.len()` looks for `len` on `T`, then on `U` (then on
//     `U`'s own target, and so on), so every `&self` method of `str` becomes
//     callable on a `Username`.
// This is how `String` (whose target is `str`), `Vec<T>` (`[T]`), `PathBuf`
// (`Path`), `Box<T>`, `Rc<T>` and `Arc<T>` all feel like their contents. Note
// the targets: the BORROWED form of the data, not the struct that stores it.
//
// Coercion only happens at a COERCION SITE, a place whose type is already
// fixed: a function or method argument, a `let` with a type annotation, a
// struct field, a return value. It does NOT happen where the compiler would
// first have to pick a type. A generic `fn shout<S: AsRef<str>>(text: S)`
// sees a `&Username` and asks for `Username: AsRef<str>`; `name == "root"`
// asks for a `PartialEq<&str>` impl on `Username`; a `match` on string
// literals wants a `&str` scrutinee; `format!("{name}")` asks for
// `Username: Display`. Method lookup does reach `str`'s trait methods
// (`name.to_string()` works), but `Username` itself never gains `str`'s trait
// IMPLS, so a bound, an operator or a `format!` argument never sees them. In
// those places you deref by hand, `&*name`, as some of the tests below do (or
// you implement the extra traits too, when the type really should have them).
//
// `DerefMut` is the other half: `&mut T` -> `&mut U`. For a type whose whole
// point is an invariant it is a hole. With a `&mut str` in hand,
// `name.make_ascii_uppercase()` turns a valid lowercase name into an invalid
// one without `Username::new` ever seeing it (with a `&mut String`, anything
// goes). `String` itself implements `DerefMut` because its only invariant,
// valid UTF-8, is one that every safe `&mut str` method preserves.
//
// How interviewers probe it: "Where does deref coercion apply, and where does
// it not?", "Why `Target = str` and not `String`?", "Should this type also be
// `DerefMut`?", and the rule of thumb from the `Deref` docs: implement it only
// when the type transparently behaves like its target, the deref is cheap and
// never fails, and nobody will be surprised by the implicit calls. (The older
// Rust API Guidelines are stricter, "only smart pointers implement `Deref`",
// so be ready to say which rule you follow for a newtype, and why.)

// Why `Username::new` rejected a name.
#[derive(Debug, PartialEq, Eq)]
enum UsernameError {
    // Not 3 to 16 bytes long.
    Length(usize),
    // Not `a-z`, `0-9` or `_`, or a first character that is not a letter.
    BadChar(char),
}

// A login name: 3 to 16 characters from `a-z`, `0-9` and `_`, starting with a
// letter. The only way to get one is `Username::new`, which checks all of it.
#[derive(Debug, PartialEq, Eq)]
struct Username(String);

impl Username {
    fn new(raw: &str) -> Result<Self, UsernameError> {
        if !(3..=16).contains(&raw.len()) {
            return Err(UsernameError::Length(raw.len()));
        }
        for (i, c) in raw.chars().enumerate() {
            let allowed = match c {
                'a'..='z' => true,
                '0'..='9' | '_' => i > 0,
                _ => false,
            };
            if !allowed {
                return Err(UsernameError::BadChar(c));
            }
        }
        Ok(Username(raw.to_string()))
    }
}

// TODO: the tests use a `Username` as text, and rustc rejects every such use:
// E0308 "mismatched types ... expected `&str`, found `&Username`" (also for
// `&Box<Username>` and `&Rc<Username>`), E0599 "no method named `len` found
// for struct `Username`" (and `starts_with`, `split`, `to_uppercase`,
// `as_ptr`; on a `Box<Username>` it reads "the method `len` exists ... but
// its trait bounds were not satisfied", because the only `len` rustc can
// find is `ExactSizeIterator::len`), and E0614 "type `Username` cannot be
// dereferenced".
// Requirement: implement `Deref` for `Username`, so that a `&Username` coerces
// to a `&str` and `str`'s methods can be called on it. The target must be the
// borrowed text, `str`, not the `String` that stores it (a test checks the
// target type), and the `&str` must point into the name's own buffer (a test
// compares heap pointers).
// Constraints: callers must NOT be able to change the text through a
// `Username` (a test checks that too); don't change `Username::new`, `greet`,
// `shout` or the tests; no `unsafe`, no `.clone()`, no `Box::leak`.
// Until you let a `Username` be seen as a `str`, this exercise will not
// compile.

// Stand-ins for the many std and third-party functions that take text.
fn greet(name: &str) -> String {
    format!("hello, {name}")
}

// Generic over its argument, so the compiler must find a type that satisfies
// the bound. No coercion happens here.
fn shout<S: AsRef<str>>(text: S) -> String {
    format!("{}!", text.as_ref().to_uppercase())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::marker::PhantomData;
    use std::ops::{Deref, DerefMut};
    use std::rc::Rc;

    // ----- test-only trait detection -----
    //
    // Inherent associated items take precedence over trait ones, but only
    // when the inherent impl's bounds hold. So for a concrete type `T`,
    // `Probe::<T>::DEREF_TARGET` is the `TypeId` of `T`'s deref target if
    // `T: Deref`, and `None` otherwise (the `impls` crate uses the same
    // trick). Likewise `DEREF_MUT_TARGET` for `DerefMut`.
    struct Probe<T: ?Sized>(PhantomData<T>);

    trait Fallback {
        const DEREF_TARGET: Option<TypeId> = None;
        const DEREF_MUT_TARGET: Option<TypeId> = None;
    }
    impl<T: ?Sized> Fallback for Probe<T> {}

    impl<T: ?Sized + Deref> Probe<T>
    where
        T::Target: 'static,
    {
        const DEREF_TARGET: Option<TypeId> = Some(TypeId::of::<T::Target>());
    }

    impl<T: ?Sized + DerefMut> Probe<T>
    where
        T::Target: 'static,
    {
        const DEREF_MUT_TARGET: Option<TypeId> = Some(TypeId::of::<T::Target>());
    }

    fn name(raw: &str) -> Username {
        Username::new(raw).unwrap()
    }

    // ----- validation (given code, already working) -----

    #[test]
    fn new_checks_the_rules() {
        assert!(Username::new("alice").is_ok());
        assert!(Username::new("bob_99").is_ok());
        assert_eq!(Username::new("Bad"), Err(UsernameError::BadChar('B')));
        assert_eq!(Username::new("ab"), Err(UsernameError::Length(2)));
        assert_eq!(Username::new("9lives"), Err(UsernameError::BadChar('9')));
        assert_eq!(Username::new("émile"), Err(UsernameError::BadChar('é')));
        assert_eq!(
            Username::new("far_too_long_a_name"),
            Err(UsernameError::Length(19))
        );
    }

    // ----- coercion sites -----

    #[test]
    fn a_username_goes_where_a_str_is_expected() {
        let name = name("alice");
        // A function argument: `&Username` -> `&str`.
        assert_eq!(greet(&name), "hello, alice");
        // A `let` with a type annotation.
        let text: &str = &name;
        assert_eq!(text, "alice");
    }

    #[test]
    fn coercion_also_applies_to_return_values_and_fields() {
        fn display_name(name: &Username) -> &str {
            name
        }
        struct Label<'a> {
            text: &'a str,
        }

        let name = name("erin");
        assert_eq!(display_name(&name), "erin");
        let label = Label { text: &name };
        assert_eq!(label.text, "erin");
    }

    #[test]
    fn coercion_chains_through_smart_pointers() {
        // `&Box<Username>` -> `&Username` -> `&str`: two `deref` calls.
        let boxed = Box::new(name("carol"));
        assert_eq!(greet(&boxed), "hello, carol");
        let shared = Rc::new(name("dave"));
        assert_eq!(greet(&shared), "hello, dave");
    }

    // ----- method lookup -----

    #[test]
    fn str_methods_can_be_called_on_a_username() {
        let name = name("bob_99");
        assert_eq!(name.len(), 6);
        assert!(name.starts_with("bob"));
        assert_eq!(name.split('_').collect::<Vec<_>>(), ["bob", "99"]);
        // `to_uppercase` builds a new `String`; the name is left as it was.
        assert_eq!(name.to_uppercase(), "BOB_99");
        assert_eq!(greet(&name), "hello, bob_99");

        // Method lookup goes through the smart pointer as well.
        let boxed = Box::new(name);
        assert_eq!(boxed.len(), 6);
    }

    // ----- where coercion does not reach -----

    #[test]
    fn without_a_coercion_site_you_deref_by_hand() {
        let name = name("root");
        // An operator: `name == "root"` needs a `PartialEq<&str>` impl.
        assert_eq!(&*name, "root");
        // A pattern: string-literal patterns match a `&str`, not a `Username`.
        let role = match &*name {
            "root" | "admin" => "staff",
            _ => "member",
        };
        assert_eq!(role, "staff");
        // A generic parameter: `shout(&name)` needs `Username: AsRef<str>`.
        assert_eq!(shout(&*name), "ROOT!");
        // A trait impl: `format!("{name}")` needs `Username: Display`.
        assert_eq!(format!("[{}]", &*name), "[root]");
    }

    // ----- how `Deref` is implemented -----

    #[test]
    fn deref_borrows_the_text_instead_of_copying_it() {
        let name = name("frank");
        let text: &str = &name;
        // The `&str` is a window into the `String` inside the `Username`: no
        // copy, no leak, no allocation.
        assert_eq!(text.as_ptr(), name.0.as_ptr());
        assert_eq!(text.len(), name.0.len());
        assert_eq!(name.as_ptr(), name.0.as_ptr());
    }

    #[test]
    fn the_deref_target_is_str() {
        assert_eq!(
            Probe::<Username>::DEREF_TARGET,
            Some(TypeId::of::<str>()),
            "`Username` should implement `Deref` with `Target = str`: deref to \
             the borrowed text, as `String` does, not to the `String` inside"
        );
    }

    #[test]
    fn a_username_is_read_only() {
        // With `DerefMut`, `name.make_ascii_uppercase()` would compile and
        // leave an uppercase, invalid name behind.
        assert_eq!(
            Probe::<Username>::DEREF_MUT_TARGET,
            None,
            "`Username` must not implement `DerefMut`: a `&mut str` would let \
             callers break the rules that `Username::new` checked"
        );
        // For comparison: `String` has no rules beyond UTF-8 to protect, so it
        // is `DerefMut` to `str`.
        assert_eq!(Probe::<String>::DEREF_MUT_TARGET, Some(TypeId::of::<str>()));
    }
}
