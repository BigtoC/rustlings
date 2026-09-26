// Traits & Abstraction · Deref, Borrow and Cow — part 2: `Deref` is not inheritance, because trait impls do not follow it (E0277, E0599).
//
// Coming from an object-oriented language, it is tempting to model "an
// `Admin` IS-A `User`" with `impl Deref for Admin { type Target = User; .. }`.
// At first it seems to work: `admin.name()` finds `User::name` by method
// lookup, a `&Admin` coerces to a `&User` wherever a `&User` is expected, and
// even the field access `admin.email` reaches through. But `Deref` feeds only
// METHOD LOOKUP and COERCION (part 1), and neither of them is subtyping:
//
//   - A trait bound is checked against the type you actually pass. A generic
//     `fn welcome<G: Greet>(who: &G)` called with `&admin` needs
//     `Admin: Greet`, and rustc does not go looking for `User: Greet` behind
//     the `Deref`: E0277 "the trait bound `Admin: Greet` is not satisfied".
//   - Turning a `&Admin` into a `&dyn Greet` (or a `Box<Admin>` into a
//     `Box<dyn Greet>`) needs a vtable for `Admin`'s OWN `Greet` impl, and
//     there is none: the same E0277, "required for the cast from `&Admin` to
//     `&dyn Greet`".
//   - Worse, `admin.greet()` DOES compile. Method lookup finds no `greet` for
//     `Admin`, derefs, and calls `User::greet`, so the admin quietly greets
//     as a plain user. The "override" you had in mind never happens, and
//     nothing warns you.
//   - Every inherent method that `User` gains later becomes an `Admin` method
//     too, and a method of the same name on `Admin` silently shadows it (the
//     "collisions" the `Deref` docs warn about).
//
// Rust's answer is composition plus explicit delegation. An `Admin` HAS a
// `User`: it implements the traits it should have itself, forwarding to the
// inner user where that is the right behavior, and it hands the inner value
// out through an accessor when a caller needs the `User` itself. It is a few
// more lines, and each of them says exactly what an `Admin` can do.
// (Generic code shares behavior through traits: default methods and bounds,
// see `15_traits` and `32_dispatch`.)
//
// How interviewers probe it: "Why is `Deref` for inheritance an
// anti-pattern?" (a code-review favorite), "Why does `admin.greet()` compile
// when `welcome(&admin)` does not?", and "How do you share behavior between
// types without inheritance?"

trait Greet {
    fn greet(&self) -> String;
}

// Deliberately not `Clone`: an account is not something to copy around.
#[derive(Debug)]
struct User {
    name: String,
    email: String,
    nickname: Option<String>,
}

impl User {
    fn new(name: &str, email: &str) -> Self {
        User {
            name: name.to_string(),
            email: email.to_string(),
            nickname: None,
        }
    }

    fn with_nickname(mut self, nickname: &str) -> Self {
        self.nickname = Some(nickname.to_string());
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn email(&self) -> &str {
        &self.email
    }
}

impl Greet for User {
    // Friends are greeted by their nickname, when they have one.
    fn greet(&self) -> String {
        let shown = self.nickname.as_deref().unwrap_or(&self.name);
        format!("hi {shown}")
    }
}

// An administrator: a user account plus an access level.
#[derive(Debug)]
struct Admin {
    user: User,
    level: u8,
}

impl Admin {
    fn new(user: User, level: u8) -> Self {
        Admin { user, level }
    }

    fn level(&self) -> u8 {
        self.level
    }
}

// Composition instead of `Deref`: `Admin` has its own `Greet` impl, so trait
// bounds (`welcome`) and trait objects (`&dyn Greet`, `Box<dyn Greet>`) find
// it, and its greeting delegates to the user's own `greet`, so the user's
// rules (the nickname) still apply. `user()` lends the inner account out
// explicitly for the APIs that need a `&User`.
impl Greet for Admin {
    fn greet(&self) -> String {
        format!("{} (admin level {})", self.user.greet(), self.level)
    }
}

impl Admin {
    fn user(&self) -> &User {
        &self.user
    }
}

// Generic code: works for any type that implements `Greet`.
fn welcome<G: Greet>(who: &G) -> String {
    format!("welcome! {}", who.greet())
}

// Dynamic dispatch over a mixed list (see `32_dispatch`).
fn greet_all(people: &[&dyn Greet]) -> Vec<String> {
    people.iter().map(|person| person.greet()).collect()
}

// An API that really does want a `User`.
fn mailing_label(user: &User) -> String {
    format!("{} <{}>", user.name(), user.email())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::marker::PhantomData;
    use std::ops::Deref;
    use std::ptr;

    // Test-only trait detection, as in part 1: `Probe::<T>::DEREF_TARGET` is
    // `Some(TypeId of the target)` when the concrete type `T` implements
    // `Deref`, and `None` when it does not.
    struct Probe<T: ?Sized>(PhantomData<T>);

    trait Fallback {
        const DEREF_TARGET: Option<TypeId> = None;
    }
    impl<T: ?Sized> Fallback for Probe<T> {}

    impl<T: ?Sized + Deref> Probe<T>
    where
        T::Target: 'static,
    {
        const DEREF_TARGET: Option<TypeId> = Some(TypeId::of::<T::Target>());
    }

    fn alice() -> User {
        User::new("alice", "alice@example.com")
    }

    fn root() -> Admin {
        Admin::new(User::new("root", "root@example.com"), 9)
    }

    #[test]
    fn a_user_greets_by_name_or_nickname() {
        assert_eq!(alice().greet(), "hi alice");
        assert_eq!(alice().with_nickname("al").greet(), "hi al");
        assert_eq!(welcome(&alice()), "welcome! hi alice");
    }

    #[test]
    fn an_admin_greets_with_its_level() {
        assert_eq!(root().greet(), "hi root (admin level 9)");
        let ops = Admin::new(User::new("ops", "ops@example.com"), 0);
        assert_eq!(ops.greet(), "hi ops (admin level 0)");
    }

    #[test]
    fn an_admin_greets_the_way_its_user_does() {
        // The user's own rule (nickname first) still applies: delegate to the
        // user's greeting instead of rebuilding it from the name.
        let boss = Admin::new(
            User::new("root", "root@example.com").with_nickname("boss"),
            255,
        );
        assert_eq!(boss.greet(), "hi boss (admin level 255)");
    }

    #[test]
    fn generic_code_accepts_an_admin() {
        assert_eq!(welcome(&root()), "welcome! hi root (admin level 9)");
    }

    #[test]
    fn users_and_admins_share_one_dyn_list() {
        let user = alice();
        let admin = root();
        let people: Vec<&dyn Greet> = vec![&user, &admin];
        assert_eq!(greet_all(&people), ["hi alice", "hi root (admin level 9)"]);
    }

    #[test]
    fn a_box_dyn_greet_can_own_an_admin() {
        let people: Vec<Box<dyn Greet>> = vec![Box::new(root()), Box::new(alice())];
        let greetings: Vec<String> = people.iter().map(|p| p.greet()).collect();
        assert_eq!(greetings, ["hi root (admin level 9)", "hi alice"]);
    }

    #[test]
    fn the_admin_lends_out_its_own_user() {
        let admin = root();
        let user: &User = admin.user();
        // A borrow of the account inside the admin, not some other `User`.
        assert!(ptr::eq(user, &admin.user));
        assert_eq!(user.name(), "root");
        assert_eq!(mailing_label(admin.user()), "root <root@example.com>");
        assert_eq!(admin.level(), 9);
    }

    #[test]
    fn an_admin_is_not_a_smart_pointer_to_a_user() {
        assert_eq!(
            Probe::<Admin>::DEREF_TARGET,
            None,
            "`Admin` must not implement `Deref`: it is not a pointer to a \
             `User`. Implement the traits it needs and add a `user()` accessor"
        );
        // A real smart pointer, for comparison.
        assert_eq!(Probe::<Box<User>>::DEREF_TARGET, Some(TypeId::of::<User>()));
    }
}
