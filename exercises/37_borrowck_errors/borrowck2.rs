// Module 1 · Borrow-checker errors — part 2: temporaries and locals (E0716, E0515).
//
// Every reference needs a living owner: the borrow checker rejects any borrow
// that is still used after the value it points into has been dropped. Inside a
// single function there are two classic ways to get that wrong, and both are
// interview favorites because the real fix is a DESIGN decision (who owns the
// data?), not a syntax tweak. Note the difference in the diagnostics: for E0716
// rustc's `help:` spells out the fix for you, but for E0515 there is no
// mechanical fix, and that is where an interviewer will push.
//
// 1. Temporaries — E0716 "temporary value dropped while borrowed". A value
//    created in the middle of an expression and never bound to a name (the
//    `String` returned by `to_lowercase()`, `format!(..)`, `String::from(..)`)
//    is a TEMPORARY. It is dropped at the end of the enclosing statement (the
//    `;`), so a borrow of it that you keep in a `let` dangles on the very next
//    line. Rust extends a temporary's life only in a few SYNTACTIC positions,
//    chiefly when it is the operand of a `&` that is itself the `let`
//    initializer, or of a `&` nested (at any depth) in the initializer's tuple,
//    array or braced-struct literal, tuple-struct or tuple-variant constructor
//    such as `Some(..)` (Rust 1.89+), `as` cast, or the tail of its block,
//    `if`/`else` branch or `match` arm:
//
//        let s = &String::from("x");          // OK: the String now lives to the end of the block
//        let s = String::from("x").as_str();  // E0716, because `s` is used below:
//        println!("{s}");                     // the String was freed at the `;` above
//
//    The second line borrows the String through a METHOD CALL (`as_str(&self)`
//    auto-refs its receiver), and a method receiver is not an extending
//    position. Neither is `(&String::from("x")).as_str()` or an ordinary
//    function argument such as `std::convert::identity(&String::from("x"))`.
//    The rule looks at where the temporary sits in the syntax, not at what the
//    borrow checker could prove. And the error always needs a LATER use: if `s`
//    were never read, the borrow would end immediately (NLL, see
//    `24_ownership_model/ownership3`) and nothing would be reported. The fix is
//    to give the value a name: a `let` binding lives until the end of its scope.
//
// 2. Locals — E0515 "cannot return reference to local variable" (or "cannot
//    return value referencing local variable" when the borrow is buried inside
//    something else, like the result of `split`). Every local that a function
//    still owns when it returns is dropped right then, so a returned reference
//    must point into data that outlives the call: data the CALLER owns,
//    reached through one of the inputs, or `'static` data such as a string
//    literal (a `-> &'a str` function may always return `""`). A lifetime
//    annotation cannot change that: `-> &'a str` is a promise that the result
//    borrows from whatever carries `'a`, and a local `String` carries no
//    caller lifetime at all. (Leave out the annotations on a two-input function
//    and rustc stops earlier, at E0106 "missing lifetime specifier", without
//    ever borrow-checking the body. That is why `shout_longest` below spells
//    out its `'a`. See `16_lifetimes/lifetimes1` and
//    `25_lifetimes_deep/lifetimes5`.)
//
// The interview question is some form of "this function builds a string; how
// do I return it as a `&str`?" You don't. Pick one of these, in roughly this
// order:
//   (1) Return an owned `String`. The new text needs an owner, and now the
//       caller is that owner.
//   (2) Borrow from an input instead. If the answer is a piece of an argument,
//       slice the argument; don't copy it into a local first.
//   (3) `Cow<'a, str>`, for when you USUALLY hand back the input unchanged and
//       only sometimes build new text. (Mentioned here, covered later.)
//   (4) Write into a caller-provided buffer, e.g.
//       `fn shout_into(a: &str, b: &str, out: &mut String)`, so the caller owns
//       the allocation and can reuse it (the `fmt::Write` / `io::Write` style).
// Interviewers will push back on these non-answers: `Box::leak` /
// `String::leak` (they compile, because the leaked text really does live
// forever, but every call leaks memory that is never freed), `unsafe` pointer
// tricks (undefined behavior: the memory IS freed), and `.clone()` (no help
// at all: the clone is one more value the function owns and drops).

// Part A (E0716). Normalize a CSV header line into column names: lowercase
// the whole line, split it on `,`, and trim each name. Empty fields are kept.
fn header_columns(line: &str) -> Vec<String> {
    // TODO: rustc rejects this with E0716 "temporary value dropped while
    // borrowed". The lowercased `String` is a temporary that nothing owns, so
    // it is freed at the `;` of this `let`, yet every `&str` in `names` points
    // into it and is read on the next line. Restructure the code so the text
    // that `names` borrows is still alive when `names` is used.
    // Constraints: keep the behavior (lowercase, split on `,`, trim, keep
    // empty fields) and the `Vec<String>` return type; no `.clone()`, no
    // `Box::leak`/`String::leak`, no `unsafe`; don't change the tests.
    // Until you keep the lowercased text alive for as long as `names` borrows
    // it, this exercise will not compile.
    let names: Vec<&str> = line.to_lowercase().split(',').map(str::trim).collect();
    names.iter().map(|s| s.to_string()).collect()
}

// Part B (E0515, a reference to a local). Shout the longer of two words
// (longer by `len()`, `a` wins a tie): uppercase it and add a `!`.
//
// TODO: rustc rejects `&shouted` below with E0515 "cannot return reference to
// local variable `shouted`". `shouted` is a `String` this function owns, so it
// is dropped the moment the function returns and any `&str` into it would
// dangle. No lifetime annotation can rescue it: `-> &'a str` promises that the
// result points into `a` or `b`, and the shouted text lives in neither. Change
// the design (the signature is yours to change) so the caller receives the new
// text safely. Constraints: keep the behavior; no `Box::leak`/`String::leak`,
// no `unsafe`; don't change the tests. Until you stop returning a borrow of
// this function's own local, this exercise will not compile.
fn shout_longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    let longest = if a.len() >= b.len() { a } else { b };
    let shouted = format!("{}!", longest.to_uppercase());
    &shouted
}

// Part C (E0515, a value that borrows a local). Return the first word of `s`,
// ignoring surrounding whitespace (words are separated by spaces), or `""`
// when there is no word at all.
fn first_word<'a>(s: &'a str) -> &'a str {
    // TODO: rustc rejects this with E0515 "cannot return value referencing
    // local variable `trimmed`". The slice that `split` yields points into
    // `trimmed`, a `String` this function owns and drops on return. Unlike
    // `shout_longest`, the signature is RIGHT this time: the first word really
    // is a piece of the caller's `s`, so the result can borrow from the input.
    // Constraints: keep the `&str` return type; allocate nothing (no `String`,
    // `to_string`/`to_owned`, `format!`, `Box::leak`/`String::leak`); no
    // `unsafe`; don't change the tests (one of them checks that the result
    // points INTO the input). Until you make the returned slice borrow from
    // `s` instead of from a local, this exercise will not compile.
    let trimmed = s.trim().to_string();
    trimmed.split(' ').next().unwrap_or("")
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    // ----- Part A: `header_columns` -----

    #[test]
    fn header_is_lowercased_split_and_trimmed() {
        assert_eq!(header_columns("Name, AGE ,City"), ["name", "age", "city"]);
    }

    #[test]
    fn header_with_a_single_column() {
        assert_eq!(header_columns("  ID  "), ["id"]);
    }

    #[test]
    fn lowercasing_is_unicode_aware() {
        // `to_lowercase` follows Unicode, not just ASCII.
        assert_eq!(header_columns("ÉCOLE,Straße"), ["école", "straße"]);
    }

    #[test]
    fn empty_fields_are_kept() {
        // `split` reports the (empty) text between adjacent separators and
        // after a trailing one. A header with blank columns still has them.
        assert_eq!(header_columns("a,,b,"), ["a", "", "b", ""]);
    }

    #[test]
    fn an_empty_line_is_one_empty_column() {
        // `split` always yields at least one item, the text before the first
        // separator, so `""` gives ONE empty column, not zero columns.
        assert_eq!(header_columns(""), [""]);
    }

    // ----- Part B: `shout_longest` -----
    //
    // These comparisons compile whether `shout_longest` returns `&str` or
    // `String`, because `String` can be compared with `&str` directly.

    #[test]
    fn shouts_the_longer_word() {
        assert_eq!(shout_longest("hi", "hello"), "HELLO!");
        assert_eq!(shout_longest("hello", "hi"), "HELLO!");
    }

    #[test]
    fn a_tie_goes_to_the_first_word() {
        assert_eq!(shout_longest("abc", "xyz"), "ABC!");
    }

    #[test]
    fn two_empty_words_still_shout() {
        assert_eq!(shout_longest("", ""), "!");
    }

    #[test]
    fn the_shout_is_new_text_found_in_neither_input() {
        // Uppercasing does not even map one char to one char ("ß" becomes
        // "SS"). The result is not a substring of `a` or `b`, so there is
        // nothing in the caller's data that a `&str` could point at.
        assert_eq!(shout_longest("straße", "abc"), "STRASSE!");
    }

    #[test]
    fn the_shout_is_handed_back_as_an_owned_string() {
        // `Box::leak`/`String::leak` would also satisfy `-> &'a str` and pass
        // every test above, by leaking a little memory on EVERY call, forever.
        // A `Cow` would always be `Cow::Owned` here (the input never comes back
        // unchanged), so it only adds noise. `&dyn Any` lets this test compile
        // for any 'static return type and then check at run time that the
        // caller really received an owned `String`.
        let shouted = shout_longest("hi", "hello");
        let any: &dyn Any = &shouted;
        assert!(
            any.is::<String>(),
            "hand the caller an owned `String`, not a borrowed or leaked `&str`"
        );
    }

    // ----- Part C: `first_word` -----

    #[test]
    fn first_word_skips_leading_whitespace() {
        assert_eq!(first_word("  hello world"), "hello");
        assert_eq!(first_word("hello   big world"), "hello");
    }

    #[test]
    fn a_single_word_is_the_first_word() {
        assert_eq!(first_word("rust"), "rust");
        assert_eq!(first_word("\trust  \n"), "rust");
        // Trailing whitespace is not always a space: a line read with
        // `read_line` still ends in its `\n`.
        assert_eq!(first_word("rust\n"), "rust");
    }

    #[test]
    fn no_word_gives_an_empty_str() {
        // Empty and whitespace-only input must not panic.
        assert_eq!(first_word(""), "");
        assert_eq!(first_word("   "), "");
    }

    #[test]
    fn the_word_points_into_the_input() {
        // `-> &'a str` promises a window INTO the caller's string, so check
        // that literally. A copied, leaked or `String`-returning version has
        // the right text but lives in a different allocation, and fails here.
        let input = String::from("  hello world");
        let word = first_word(&input);
        assert_eq!(word, "hello");
        // "hello" starts two bytes in, right after the two leading spaces.
        assert_eq!(word.as_ptr(), input[2..].as_ptr());

        let input = String::from("one");
        let word = first_word(&input);
        assert_eq!(word.as_ptr(), input.as_ptr());
        assert_eq!(word.len(), input.len());
    }
}
