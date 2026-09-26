// Traits & Abstraction · Declarative macros — part 3: `$crate` paths in an exported macro, and where a macro's names resolve (E0425, clippy::crate_in_macro_def).
//
// `#[macro_export]` puts a macro at the ROOT of its crate, whichever module
// defines it: inside this crate it is `crate::percent!`, and another crate
// calls it as `your_crate::percent!`. (Upstream `21_macros/macros3` used
// `#[macro_use]` instead, which only extends the macro's textual scope
// within this crate.) Exporting is the easy half. The hard half is that the
// expansion is pasted into the CALLER'S module, far from where it was
// written, and every name in it has to mean the same thing there.
//
// `macro_rules!` hygiene is "mixed site". Local variables and labels
// resolve at the macro's DEFINITION: that is what kept part 2's `first` and
// `rest` apart from the caller's variables. Everything else (functions,
// types, modules, traits, other macros) resolves at the CALL SITE, as if the
// caller had typed the expansion; even `self::` and `super::` count from the
// caller's module. So `clamp_percent($e)` looks for a `clamp_percent` in the
// caller's module. In a module that has not imported one, that is E0425
// "cannot find function `clamp_percent` in this scope". In a module that has
// a `clamp_percent` of its own, it compiles and silently calls the wrong
// function. rustc's help, "consider importing this function", is the wrong
// fix for a macro: it makes every caller import the macro's internals.
//
// The expansion is also privacy-checked at the call site: a macro can only
// name what its caller could name. That is why `util` and `clamp_percent`
// are `pub` while `Percent`'s field is private (callers cannot build a
// `Percent` directly, not even through the macro), and why library macros
// often call `#[doc(hidden)] pub` helpers in a module named something like
// `__private`.
//
// `$crate` is a special metavariable that exists for exactly this problem:
// it expands to a path to the root of the crate that DEFINES the macro.
// Inside that crate it means `crate`; in any other crate it means
// `::your_crate`. std's own macros use it everywhere: `assert_eq!` calls
// `$crate::panicking::assert_failed`, and `vec![]` expands to
// `$crate::vec::Vec::new()`. Plain `crate::` in a macro is a trap: it means
// the crate the macro is CALLED from, so it works in every test you can
// write inside this crate and breaks for every other crate. Clippy flags it
// with the warn-by-default lint `crate_in_macro_def`, and rustlings runs
// clippy with `-D warnings` for this exercise.
//
// Interviewers ask: "Why does `$crate` exist?", "Which names in a
// `macro_rules!` expansion are hygienic, and which resolve at the call
// site?", "How does an exported macro call a helper that users shouldn't
// use?" and "What does `#[macro_export]` do to where the macro lives?".

pub mod util {
    // A percentage, always in 0..=100. The field is private, so outside this
    // module a `Percent` can only come from `clamp_percent`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Percent(u8);

    impl Percent {
        pub fn get(self) -> u8 {
            self.0
        }
    }

    // Clamps any `i64` into 0..=100.
    pub fn clamp_percent(value: i64) -> Percent {
        // The clamped value fits in a `u8`, so the cast cannot truncate.
        Percent(value.clamp(0, 100) as u8)
    }

    // TODO: Every call of `percent!` outside this module fails with E0425
    // "cannot find function `clamp_percent` in this scope", starting with
    // `report::describe` below. In a module that has a `clamp_percent` of
    // its own it would compile and call THAT function (a test checks). Make
    // the expansion call this module's `clamp_percent` from any module of
    // this crate AND from any other crate that calls `your_crate::percent!`.
    // Requirements:
    //   - don't touch the callers: no imports at the call sites, no
    //     re-export at the crate root (one test calls the macro from a
    //     module with no `use` lines at all), and don't edit the tests;
    //   - keep going through `clamp_percent`: the expansion cannot build a
    //     `Percent` itself, because its field is private;
    //   - rustlings runs clippy with `-D warnings` for this exercise, so a
    //     path that is right only inside this crate still fails.
    // Until you name the helper by a path that works from every caller, this
    // exercise will not compile.
    #[macro_export]
    macro_rules! percent {
        ($e:expr) => {
            clamp_percent($e)
        };
    }
}

// Given, complete: a caller in another module.
mod report {
    use crate::percent;

    pub fn describe(score: i64) -> String {
        format!("{}%", percent!(score).get())
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_negative_values_to_zero() {
        assert_eq!(percent!(-5).get(), 0);
        assert_eq!(percent!(-1).get(), 0);
    }

    #[test]
    fn keeps_values_in_range() {
        assert_eq!(percent!(42).get(), 42);
        assert_eq!(percent!(0).get(), 0);
        assert_eq!(percent!(100).get(), 100);
    }

    #[test]
    fn clamps_large_values_to_one_hundred() {
        assert_eq!(percent!(1_000).get(), 100);
        assert_eq!(percent!(101).get(), 100);
    }

    #[test]
    fn handles_the_extremes_of_i64() {
        assert_eq!(percent!(i64::MAX).get(), 100);
        assert_eq!(percent!(i64::MIN).get(), 0);
    }

    #[test]
    fn takes_any_i64_expression() {
        let done: i64 = 3;
        let total: i64 = 4;
        assert_eq!(percent!(done * 100 / total).get(), 75);
        assert_eq!(percent!(if done > total { 200 } else { 50 }).get(), 50);
    }

    #[test]
    fn the_report_uses_the_macro() {
        assert_eq!(report::describe(250), "100%");
        assert_eq!(report::describe(64), "64%");
        assert_eq!(report::describe(-3), "0%");
    }

    // Callers two modules down, where nothing is in scope unless it is
    // imported right there: neither `callers` nor `without_imports` has a
    // `use` line, so the expansion must name everything by a path that works
    // from any module.
    mod callers {
        mod without_imports {
            #[test]
            fn works_from_a_module_that_imports_nothing() {
                assert_eq!(crate::percent!(-5).get(), 0);
                assert_eq!(crate::percent!(55).get(), 55);
                assert_eq!(crate::percent!(1_000).get(), 100);
            }
        }

        // A caller that happens to have a function with the same name.
        mod with_its_own_clamp_percent {
            use crate::util::{self, Percent};

            fn clamp_percent(_value: i64) -> Percent {
                util::clamp_percent(77)
            }

            #[test]
            fn the_macro_does_not_call_the_callers_function() {
                assert_eq!(crate::percent!(-5).get(), 0);
                assert_eq!(crate::percent!(1_000).get(), 100);
                // A direct call in this module does reach the local function.
                assert_eq!(clamp_percent(-5).get(), 77);
            }
        }
    }
}
