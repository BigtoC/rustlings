// Traits & Abstraction · Declarative macros — part 1: a `hashmap!` with a trailing comma, pre-sized by a recursive `count!` ("no rules expected", no E-code).
//
// A `macro_rules!` macro is a list of ARMS, `(matcher) => { transcriber };`.
// rustc tries the arms from top to bottom against the tokens of the call and
// expands the FIRST arm whose matcher accepts all of them: the transcriber is
// pasted in place of the call, with every `$name` replaced by what it
// captured. When no arm accepts the call, there is no error code, just
// "no rules expected `x`", pointing at the first token that no arm could
// match. Both macros below have a single arm that accepts an empty call and
// nothing else, so every call with arguments stops at its first token.
//
// A metavariable is `$name:fragment`. The fragment says what it captures:
// `expr` one expression, `ident` one identifier, `ty` a type, `literal`,
// `path`, `pat`, `block` and so on, and `tt` one TOKEN TREE: a single token
// (`a`, `+`, `"s"`, `5`) or a whole bracketed group, `( .. )`, `[ .. ]` or
// `{ .. }`, whatever is inside it. Repetition works like a regex:
// `$( .. ),*` is zero or more, separated by commas, `+` is one or more and
// `?` at most one. `$(,)?` after a repetition is the usual way to allow a
// trailing comma. In the transcriber, `$( .. )*` repeats once per capture,
// so a metavariable captured inside a repetition must be used inside one,
// at the same depth (otherwise: "variable `k` is still repeating at this
// depth").
//
// There is no stable way to ask "how many?" in a transcriber: the
// metavariable expression `${count($x)}` is still unstable (E0658). The
// classic answer is recursion. One arm peels off the first token tree and
// adds 1 to a recursive call on the rest; a base arm turns `()` into 0. The
// result is plain arithmetic on constants, so it works in a `const` and as
// an array length. `hashmap!` can count its pairs this way because a
// captured `$k:expr`, passed on to another macro, arrives as ONE opaque token
// tree, however long the expression was. Each recursive step is one level
// of macro expansion, and rustc stops at its recursion limit (128 levels by
// default), so a recursive `count!` fails on inputs of about 128 tokens
// (fewer when it is called from inside other macros).
//
// std's `vec!` has the shape you are about to write: a separate arm for
// `()`, and an arm for `$($x:expr),+ $(,)?` (plus one for `vec![x; n]`).
// The empty arm is not decoration. With a single `$( .. ),*` arm for
// everything, an empty call expands to `let mut map = ..; map`, and rustc
// warns "variable does not need to be mutable" (`unused_mut`) at every
// `hashmap!{}` in the caller's code.
//
// Interviewers ask: "Write a `vec!`- or `hashmap!`-style macro that accepts a
// trailing comma", "What does `tt` match, and why is it the most flexible
// fragment?", "How do you count a macro's arguments?" and "Why does `vec!`
// have a separate arm for `()`?".

use std::collections::HashMap;

// Two arms, tried top to bottom. The first peels off one token tree (`tt`
// also matches a whole bracketed group) and adds 1 to a recursive call on the
// rest; the base arm turns the empty input into 0. The expansion is a sum of
// `usize` constants, so it works in a `const` and as an array length.
macro_rules! count {
    () => {
        0usize
    };
    ($head:tt $($tail:tt)*) => {
        1usize + count!($($tail)*)
    };
}

// The empty arm stays separate, so an empty call never declares an unused
// `let mut`. The second arm takes one or more pairs plus an optional trailing
// comma. `count!` sees each captured key as one token tree, so it counts the
// pairs at compile time and the map is allocated once, with room for every
// pair written. Each `$k` and `$v` is pasted exactly once, in order, so every
// expression runs once and its value is moved into the map. The inner braces
// make the expansion one block expression, and the absolute path
// `::std::collections::HashMap` resolves the same in every module.
macro_rules! hashmap {
    () => {
        ::std::collections::HashMap::new()
    };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut map = ::std::collections::HashMap::with_capacity(count!($($k)*));
        $(
            map.insert($k, $v);
        )+
        map
    }};
}

// Given, complete: a caller.
fn status_names() -> HashMap<u16, &'static str> {
    hashmap! {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    // Not `Clone`, so a macro can only move it into the map.
    #[derive(Debug)]
    struct Payload(Vec<u8>);

    #[test]
    fn count_counts_tokens() {
        assert_eq!(count!(a b c d), 4);
        assert_eq!(count!(x), 1);
        assert_eq!(count!(), 0);
        assert_eq!(count!(+ - 5 "s" 'c'), 5);
    }

    #[test]
    fn a_bracketed_group_counts_as_one_token_tree() {
        assert_eq!(count!(x (y z) [1, 2] "s"), 4);
        assert_eq!(count!({ a b c } () [[]]), 3);
    }

    #[test]
    fn count_is_a_usize_constant_expression() {
        const FOUR: usize = count!(a b c d);
        // An array length must be a constant.
        let buffer = [0u8; count!(w x y)];
        assert_eq!(FOUR, 4);
        assert_eq!(buffer.len(), 3);
        let as_usize: usize = count!(p q);
        assert_eq!(as_usize, 2);
    }

    #[test]
    fn count_handles_longer_inputs() {
        assert_eq!(
            count!(a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 b0 b1 b2 b3 b4 b5 b6 b7 b8 b9),
            20
        );
        assert_eq!(count!(0 1 2 3 4 5 6 7 8 9 10 11), 12);
    }

    #[test]
    fn builds_a_map_from_pairs() {
        let map = hashmap! { "a" => 1, "b" => 2, "c" => 3 };
        assert_eq!(map.len(), 3);
        assert_eq!(map["a"], 1);
        assert_eq!(map["b"], 2);
        assert_eq!(map["c"], 3);
        let single = hashmap! { 'x' => true };
        assert_eq!(single.len(), 1);
        assert_eq!(single.get(&'x'), Some(&true));
    }

    #[test]
    fn accepts_a_trailing_comma() {
        let map = hashmap! { "a" => 1, "b" => 2, "c" => 3, };
        assert_eq!(map.len(), 3);
        let single = hashmap! { 7 => "seven", };
        assert_eq!(single[&7], "seven");
    }

    #[test]
    fn keys_and_values_can_be_any_expressions() {
        let base = 10;
        let map = hashmap! {
            format!("k{}", base) => base * 2,
            String::from("sum") => [1, 2, 3].iter().sum::<i32>(),
            "lit".to_string() => if base > 5 { 1 } else { 0 },
        };
        assert_eq!(map["k10"], 20);
        assert_eq!(map["sum"], 6);
        assert_eq!(map["lit"], 1);
    }

    #[test]
    fn an_empty_call_gives_an_empty_map_that_has_not_allocated() {
        let empty: HashMap<&str, i32> = hashmap! {};
        assert!(empty.is_empty());
        assert_eq!(empty.capacity(), 0);
    }

    // What `HashMap` gives when asked for room for exactly `n` entries.
    fn capacity_for(n: usize) -> usize {
        HashMap::<&str, i32>::with_capacity(n).capacity()
    }

    #[test]
    fn the_map_is_presized_for_the_number_of_pairs() {
        let map = hashmap! { "a" => 1, "b" => 2, "c" => 3 };
        assert!(map.capacity() >= 3);
        // Room for three pairs, not for every token between the braces.
        assert_eq!(map.capacity(), capacity_for(3));
    }

    #[test]
    fn the_map_is_presized_for_every_pair_written() {
        // Five pairs, one key. A map that grows on demand is sized for the
        // one entry it ends up holding; a pre-sized map reserved room for all
        // five pairs before the first insert.
        let same_key = hashmap! { "k" => 1, "k" => 2, "k" => 3, "k" => 4, "k" => 5 };
        assert_eq!(same_key.len(), 1);
        assert_eq!(same_key["k"], 5, "a later pair replaces an earlier one");
        assert_eq!(
            same_key.capacity(),
            capacity_for(5),
            "the map must be pre-sized for exactly the 5 pairs written"
        );
    }

    #[test]
    fn keys_and_values_are_evaluated_once_in_order() {
        let log = RefCell::new(Vec::new());
        let note = |label: &'static str| {
            log.borrow_mut().push(label);
            label
        };
        let map = hashmap! {
            note("k1") => note("v1"),
            note("k2") => note("v2"),
            note("k3") => note("v3"),
        };
        assert_eq!(*log.borrow(), ["k1", "v1", "k2", "v2", "k3", "v3"]);
        assert_eq!(map["k2"], "v2");
    }

    #[test]
    fn values_are_moved_in_not_copied() {
        let payload = Payload(vec![1, 2, 3]);
        let heap = payload.0.as_ptr();
        let map = hashmap! { "blob" => payload, "empty" => Payload(Vec::new()) };
        assert_eq!(map["blob"].0.as_ptr(), heap);
        assert!(map["empty"].0.is_empty());
    }

    #[test]
    fn status_names_are_built_with_the_macro() {
        let names = status_names();
        assert_eq!(names.len(), 3);
        assert_eq!(names[&404], "Not Found");
        assert_eq!(names.get(&418), None);
    }

    // No `use` lines here: whatever `hashmap!` expands to must name
    // everything by a path that works from any module.
    mod without_imports {
        #[test]
        fn works_where_hash_map_is_not_imported() {
            let map = hashmap! { 1 => "one", 2 => "two" };
            assert_eq!(map.get(&2), Some(&"two"));
            assert_eq!(count!(p q r), 3);
        }
    }
}
