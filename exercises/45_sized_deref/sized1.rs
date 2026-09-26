// Traits & Abstraction · `?Sized` and unsized types — part 5: every type parameter is `Sized` unless you opt out, `impl Trait` included (E0277).
//
// Most types have a size the compiler knows: a `u32` is 4 bytes, a `String`
// is three words. A few do not. A `str` is any number of UTF-8 bytes, a `[T]`
// any number of `T`s, and a `dyn Display` any type at all that implements
// `Display`. These are DYNAMICALLY SIZED TYPES (DSTs, or "unsized types").
// You cannot keep one in a local variable, pass one by value or return one,
// because the compiler could not reserve space for it. They only exist
// behind a pointer (`&str`, `Box<[u8]>`, `Rc<dyn Display>`), and that pointer
// carries the missing information next to the address: the length for `str`
// and `[T]`, a pointer to the vtable for `dyn Trait`. Those are the two-word
// "fat pointers" whose sizes `41_memory_layout/layout1` asked about. They are
// also the borrowed forms from the first half of this module: `String`
// derefs to `str`, `Vec<T>` to `[T]`.
//
// `Sized` is the marker trait for "has a size known at compile time". Almost
// all generic code moves values around, so Rust adds an IMPLICIT `T: Sized`
// bound to every type parameter: of functions, structs, enums, impls and
// traits, and to the associated types of a trait. `T: ?Sized` ("maybe
// sized") takes it away. The `?` works with `Sized` and nothing else
// ("bound modifier `?` can only be applied to `Sized`"). The one parameter
// that is `?Sized` from the start is `Self` inside a trait, which is why std
// can write `impl Display for str`.
//
// `join_all` below only ever touches a `T` through a `&T`, so it has no use
// for `T: Sized`, but it gets the bound anyway. Called with a `&[&str]`, it
// infers `T = str` (the type parameter is what the items POINT TO, not the
// reference), and rustc answers E0277 "the size for values of type `str`
// cannot be known at compilation time", with the note "required by an
// implicit `Sized` bound in `join_all`". A `&[&dyn Display]` gets the same
// error for `dyn std::fmt::Display`.
//
// Opting out is a promise: inside the function, a `T: ?Sized` may only be
// used behind a pointer. `fn f<T: ?Sized>(x: T)` is rejected with the same
// E0277, now for `T` (rustc's help: "function arguments must have a
// statically known size"). So `?Sized` belongs on generic code that only
// borrows (`&T`, `Box<T>`, `Rc<T>`), and std puts it there all the time:
// `HashMap::get` has `Q: ?Sized` (`borrow1`), `Deref` has
// `type Target: ?Sized` (`deref1`), and
// `impl<T: Display + ?Sized> ToString for T` covers `str` and `dyn Display`.
//
// `impl Trait` in argument position is an anonymous type parameter, so it
// gets the implicit bound as well, which is Part B. Relaxing it needs one
// piece of syntax that rustc's own suggestion leaves out.
//
// How interviewers probe it: "What does `?Sized` mean?", "Why does
// `fn f<T: Display>(x: &T)` reject a `&str`?", "Why is `Self` `?Sized` in a
// trait when a function's `T` is not?", "How are `&str` and `&dyn Display`
// represented?", and "Can you write `fn f(s: str)`?"

use std::fmt::Display;

// ---- Part A — a generic function over borrowed items ----------------------

// Joins the items' `Display` output, with `sep` between neighbors.
// TODO: the tests join `&str` items (`T = str`) and `&dyn Display` items
// (`T = dyn Display`), and rustc rejects both calls: E0277 "the size for
// values of type `str` cannot be known at compilation time" (note: "required
// by an implicit `Sized` bound in `join_all`"), and the same for
// `dyn std::fmt::Display`.
// Requirement: accept items whose type has no size known at compile time,
// while slices of `&i32` or `&String` keep working. The tests also name the
// type parameter explicitly (`join_all::<str>`, `join_all::<dyn Display>`),
// so keep the shape of the signature: `T` is what the items point to.
// Constraints: don't change the body or the tests; no `unsafe`. Until you
// relax the bound on `join_all`, this exercise will not compile.
fn join_all<T: Display>(items: &[&T], sep: &str) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(&item.to_string());
    }
    out
}

// ---- Part B — `impl Trait` in argument position ---------------------------

// A label that can be viewed as text. It is not a smart pointer, so it
// implements `AsRef<str>` and deliberately no `Deref`.
struct Label(String);

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Returns the text without its leading and trailing whitespace, as a slice
// of the caller's text (nothing is copied).
// TODO: `trimmed("  hi there \n")` fails with the same E0277 for `str`, this
// time "required by an implicit `Sized` bound in `trimmed`": the
// `impl AsRef<str>` is an anonymous type parameter, and it is `Sized`
// unless you say otherwise.
// Requirement: accept a `&str` as well as a `&String`, a `&Box<str>` and a
// `&Label` (which has `AsRef<str>` but no `Deref`), and keep returning a
// slice of the caller's text.
// Constraints: keep taking the text by reference; don't change the tests; no
// `unsafe`. rustc's suggested edit does not parse as written (the follow-up
// error is "ambiguous `+` in a type"), so read that error too. Until you
// relax the bound on the parameter, this exercise will not compile.
fn trimmed(text: &impl AsRef<str>) -> &str {
    text.as_ref().trim()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Part A -----

    #[test]
    fn joins_str_items() {
        let words: Vec<&str> = vec!["a", "b"];
        assert_eq!(join_all(&words, ","), "a,b");
        // The same call with the type parameter spelled out: `T` is `str`.
        assert_eq!(join_all::<str>(&words, " | "), "a | b");
    }

    #[test]
    fn joins_dyn_display_items() {
        let items: Vec<&dyn Display> = vec![&1, &"x"];
        assert_eq!(join_all(&items, ","), "1,x");
        let name = String::from("s");
        let mixed: [&dyn Display; 3] = [&'c', &2.5, &name];
        assert_eq!(join_all::<dyn Display>(&mixed, "; "), "c; 2.5; s");
    }

    #[test]
    fn boxed_trait_objects_join_either_way() {
        let boxed: Vec<Box<dyn Display>> = vec![Box::new(7), Box::new("seven")];
        // Borrow the object inside each box: `T = dyn Display`.
        let inner: Vec<&dyn Display> = boxed.iter().map(|b| b.as_ref()).collect();
        assert_eq!(join_all(&inner, "="), "7=seven");
        // Or borrow the boxes themselves: `T = Box<dyn Display>`, a sized
        // type. It is `Display` because std forwards `Display` through
        // `Box<T>` for any `T: Display + ?Sized` (part 6 writes such impls).
        let outer: Vec<&Box<dyn Display>> = boxed.iter().collect();
        assert_eq!(join_all(&outer, "="), "7=seven");
    }

    #[test]
    fn sized_items_still_work() {
        let nums = [1, 2, 3];
        let refs: Vec<&i32> = nums.iter().collect();
        assert_eq!(join_all(&refs, "+"), "1+2+3");
        let owned = [String::from("x"), String::from("y")];
        let refs: Vec<&String> = owned.iter().collect();
        assert_eq!(join_all::<String>(&refs, ""), "xy");
    }

    #[test]
    fn empty_and_single_item_slices() {
        assert_eq!(join_all::<str>(&[], ","), "");
        assert_eq!(join_all(&["solo"], ","), "solo");
        let unicode: Vec<&str> = vec!["é", "漢字"];
        assert_eq!(join_all(&unicode, "·"), "é·漢字");
    }

    // ----- Part B -----

    #[test]
    fn trimmed_takes_a_str_and_borrows_from_it() {
        let text = "  hi there \n";
        let t = trimmed(text);
        assert_eq!(t, "hi there");
        // A slice of the caller's text, not a copy.
        assert_eq!(t.as_ptr(), text[2..].as_ptr());
        assert_eq!(trimmed(""), "");
        assert_eq!(trimmed(" \t "), "");
    }

    #[test]
    fn trimmed_takes_owned_and_boxed_text() {
        let owned = String::from(" owned ");
        assert_eq!(trimmed(&owned), "owned");
        assert_eq!(trimmed(&owned).as_ptr(), owned[1..].as_ptr());
        let boxed: Box<str> = Box::from(" boxed ");
        // `T = Box<str>` here, and `T = str` after an explicit reborrow.
        assert_eq!(trimmed(&boxed), "boxed");
        assert_eq!(trimmed(&*boxed), "boxed");
        assert_eq!(trimmed(&*boxed).as_ptr(), boxed[1..].as_ptr());
    }

    #[test]
    fn trimmed_takes_any_as_ref_str_type() {
        // `Label` has no `Deref`, so `&label` never coerces to a `&str`.
        let label = Label(String::from("\tadmin "));
        assert_eq!(trimmed(&label), "admin");
        assert_eq!(trimmed(&label).as_ptr(), label.0[1..].as_ptr());
    }
}
