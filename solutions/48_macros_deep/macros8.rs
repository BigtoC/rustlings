// Traits & Abstraction · Declarative macros — part 4: generating items: unit newtypes, and one `#[test]` per table row ("no rules expected", no E-code).
//
// A macro can expand to ITEMS as well as expressions: structs, `impl`
// blocks, functions, whole modules. That is how std avoids writing the same
// impl over and over: core implements `Add` for every primitive number type
// with one invocation, `add_impl! { usize u8 u16 .. f64 f128 }`, where the
// macro's single arm is `($($t:ty)*) => ($( impl Add for $t { .. } )*)`. An
// interviewer who asks "how would you implement this trait for ten types
// without copy-paste?" wants to hear exactly that (or "a blanket impl", when
// one generic impl can cover them all).
//
// Three tools do the work here:
//
//   - An `ident` fragment can NAME a new item: `struct $name(f64);` defines
//     a struct called whatever the caller wrote, and `fn $name()` a
//     function. Items are not hygienic in `macro_rules!` (only local
//     variables and labels are, parts 2 and 3), so the caller sees them
//     under exactly that name. The flip side: two expansions that both define a
//     fixed name, say a helper `fn check()`, clash with E0428.
//   - `stringify!(..)` turns tokens into a `&'static str` literal at compile
//     time, so `stringify!($name)` is the unit's name as text, and
//     `concat!` glues literals together.
//   - A `meta` fragment captures the inside of an attribute. With
//     `$(#[$attr:meta])*` in the matcher and `$(#[$attr])*` in the
//     transcriber, a macro passes the caller's attributes (`#[should_panic]`,
//     `#[ignore]`, doc comments) through to the item it generates.
//
// Part A generates one newtype per unit. `Meters` and `Seconds` both wrap an
// `f64`, but they are different types, so adding meters to seconds does not
// compile, and each costs exactly as much as the `f64` inside it.
//
// Part B generates tests from a table. One test per row, each with its own
// name, is better than one test that loops over the rows: a failure names
// the row that broke, and one bad row does not hide the rows after it. The
// crates `test-case` and `rstest` do this with procedural macros; with
// `macro_rules!` it takes one arm.
//
// Both stubs below accept only an empty call, so both invocations fail with
// "no rules expected ..". They deliberately do not accept their input and
// expand to nothing: a `test_cases!` that swallowed its table would make
// those tests disappear, and an empty test suite passes.
//
// Interviewers ask: "How do you implement a trait for many types without
// copy-paste?", "Is a struct defined inside a `macro_rules!` expansion
// hygienic?", "What does `stringify!` give you?" and "How do you write
// table-driven tests in Rust?".

// ---- Part A — one newtype per unit --------------------------------------

// One repetition per name. Inside it, `$name` is an `ident`, so it can name
// the new struct and appear in its impls, and `stringify!($name)` turns it
// into the text the `Display` impl prints. The trait paths are absolute, so
// the expansion works in any module, whatever that module imports.
macro_rules! newtype_units {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
            struct $name(f64);

            impl ::std::ops::Add for $name {
                type Output = Self;

                fn add(self, rhs: Self) -> Self {
                    Self(self.0 + rhs.0)
                }
            }

            impl ::std::fmt::Display for $name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "{} {}", self.0, stringify!($name))
                }
            }
        )+
    };
}

newtype_units!(Meters, Seconds);

// ---- Part B — one test per table row ------------------------------------

// Given, complete: the functions the tables in the tests check.

// Lowercase ASCII words joined by single dashes; everything that is not an
// ASCII letter or digit separates words.
fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    for word in title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if !slug.is_empty() {
            slug.push('-');
        }
        slug.extend(word.chars().map(|c| c.to_ascii_lowercase()));
    }
    slug
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

// The function comes first, then one repetition per row. A row's attributes
// are a nested repetition of `meta` fragments, re-emitted on that row's test
// next to `#[test]`. Each row expands to exactly one function, named by the
// row's `ident`, so every row is its own test with its own name, and a
// failure reports which row broke. Nothing else is generated, so two tables
// can live in one module.
#[cfg(test)]
macro_rules! test_cases {
    ($function:ident; $($(#[$attr:meta])* $name:ident: $input:expr => $expected:expr;)+) => {
        $(
            #[test]
            $(#[$attr])*
            fn $name() {
                assert_eq!($function($input), $expected);
            }
        )+
    };
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::mem::size_of;

    // ---- Part A ----

    #[test]
    fn meters_add_up_to_meters() {
        assert_eq!(Meters(1.5) + Meters(2.0), Meters(3.5));
        let total: Meters = Meters(0.25) + Meters(0.5) + Meters(0.25);
        assert_eq!(total, Meters(1.0));
    }

    #[test]
    fn a_unit_is_copy() {
        let lap = Seconds(20.0);
        let three_laps = lap + lap + lap;
        assert_eq!(three_laps, Seconds(60.0));
        // `lap` was copied into each addition, not moved.
        assert_eq!(lap, Seconds(20.0));
    }

    #[test]
    fn a_unit_displays_its_own_name() {
        assert_eq!(Seconds(2.0).to_string(), "2 Seconds");
        assert_eq!(Meters(0.5).to_string(), "0.5 Meters");
        assert_eq!(format!("[{}]", Meters(-3.25)), "[-3.25 Meters]");
    }

    #[test]
    fn units_compare_and_debug_print() {
        assert!(Meters(1.0) < Meters(2.0));
        assert!(Seconds(9.5) > Seconds(-1.0));
        assert_eq!(format!("{:?}", Seconds(1.5)), "Seconds(1.5)");
        assert_eq!(format!("{:?}", Meters(2.0)), "Meters(2.0)");
    }

    #[test]
    fn each_unit_is_its_own_zero_cost_type() {
        assert_ne!(TypeId::of::<Meters>(), TypeId::of::<Seconds>());
        assert_eq!(size_of::<Meters>(), size_of::<f64>());
        assert_eq!(size_of::<Seconds>(), size_of::<f64>());
    }

    // A second invocation, with a trailing comma, from a module that has no
    // `use` lines.
    mod more_units {
        newtype_units!(Grams, Liters,);

        #[test]
        fn units_can_be_declared_in_any_module() {
            assert_eq!((Grams(250.0) + Grams(750.0)).to_string(), "1000 Grams");
            assert_eq!(Liters(0.75) + Liters(0.25), Liters(1.0));
            assert_eq!(format!("{:?}", Liters(1.5)), "Liters(1.5)");
        }
    }

    // ---- Part B ----

    // The tables get a module of their own, where dead code is an error
    // again (this course allows it everywhere else). A generated function
    // that is not a `#[test]` is never called by anything, so it is dead,
    // and the build stops.
    #[deny(dead_code)]
    mod tables {
        use crate::{slugify, word_count};
        use std::panic;

        test_cases! {
            slugify;
            lowercases_letters: "Hello" => "hello";
            joins_words_with_one_dash: "Rust  macros, in depth" => "rust-macros-in-depth";
            trims_separators_at_both_ends: "  --2024!  " => "2024";
            #[should_panic(expected = "left == right")]
            a_wrong_expectation_fails: "Hello" => "Hello";
        }

        test_cases! {
            word_count;
            counts_words: "one two  three" => 3;
            an_empty_text_has_no_words: "" => 0;
        }

        #[test]
        fn a_row_with_a_wrong_expectation_really_fails() {
            // Calling a row's function runs that row's check, so this one
            // must panic: the generated test compares, it does not just call
            // the function.
            assert!(panic::catch_unwind(a_wrong_expectation_fails).is_err());
        }
    }
}
