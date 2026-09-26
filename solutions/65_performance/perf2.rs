// Module 5 · Performance — part 2: borrow instead of copying: `&str` and `&[impl AsRef<str>]` parameters, a `Vec<&str>` return (E0308).
//
// The other half of "where are the allocations?" is copies nobody needed.
// Two kinds of signature cause most of them:
//
//   - A parameter that is more owned than the function needs. A `&String`
//     or `&Vec<T>` parameter accepts only a borrowed `String` or `Vec<T>`. A
//     caller that holds a string literal, a slice of a bigger string, a
//     `Box<str>` or an array must first BUILD a `String` or a `Vec` (an
//     allocation and a copy) just to make the call. `&str` and `&[T]` accept
//     all of those, and a `&String` or `&Vec<T>` still coerces to them (deref
//     coercion, `45_sized_deref/deref1`). Clippy's `ptr_arg` lint states the
//     same rule as a style lint (`67_code_review/review2` grades it); here
//     the tests are the callers who cannot call you.
//   - A return type that copies what the caller already has. The words of a
//     line are already there, in the caller's line. `Vec<String>` allocates
//     and copies one `String` per word, while a `Vec<&str>` holds only a
//     (pointer, length) pair per word, pointing into the line. Lifetime
//     elision ties the result to the line (`25_lifetimes_deep`), so the
//     compiler makes sure the line outlives the words.
//
// A slice of strings needs one more step. `&[String]` refuses the
// `Vec<&str>` that a borrowing `tokens` returns, and `&[&str]` refuses a
// `Vec<String>`: a slice is one block of elements of ONE type, and turning
// every `String` into a `&str` would mean building a new vector. A function
// that is generic over the element type, bounded by `AsRef<str>`, accepts
// both of them, and arrays, and sub-slices, without anyone building anything.
//
// The tests check "no copy" with heap pointers. A borrowed word's `as_ptr()`
// is an address INSIDE the line; a copied `String` lives in an allocation of
// its own, somewhere else.
//
// The flip side of "borrow what you only read" is "take ownership of what
// you keep": a function that stores a string takes `impl Into<String>`, so a
// caller who owns one moves it in instead of paying for a copy
// (`47_type_level/builder1`). A function that only SOMETIMES needs to
// allocate returns a `Cow` (`45_sized_deref/cow1`).
//
// How interviewers probe this: "Why `&str` instead of `&String`?", "What
// does a `Vec<&str>` cost compared to a `Vec<String>`, and what does it tie
// the result to?", "How do you write one function that takes both a
// `Vec<String>` and a `&[&str]`?".

// Splits a line such as "GET /index.html HTTP/1.1" into its words: the runs
// of characters between Unicode whitespace.
fn tokens(line: &str) -> Vec<&str> {
    // `&str` takes literals, slices of longer strings and (by deref coercion)
    // a `&String`. `split_whitespace` already yields `&str` views into
    // `line`, so collecting them copies no text: the only allocation is the
    // `Vec` of (pointer, length) pairs. With one reference parameter,
    // elision gives the result `line`'s lifetime, so the words cannot
    // outlive the line they point into.
    line.split_whitespace().collect()
}

// Counts the words that have at least `min_chars` characters (chars, not
// bytes).
fn count_long(words: &[impl AsRef<str>], min_chars: usize) -> usize {
    // A slice is one block of elements of one type, so the element type is
    // the generic part: `String`, `&str` (and `Box<str>`, `Cow<str>`, ...)
    // all implement `AsRef<str>`, and `as_ref()` borrows each one as a
    // `&str` for free. Arrays and `Vec`s coerce to the slice. The spelled-out
    // form `fn count_long<S: AsRef<str>>(words: &[S], ..)` is the same thing.
    words
        .iter()
        .filter(|word| word.as_ref().chars().count() >= min_chars)
        .count()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Checks that every word is a view into `line`: its bytes start inside
    // the line's buffer (a copy would live in an allocation of its own).
    fn assert_borrowed_from(line: &str, words: &[impl AsRef<str>]) {
        let inside = line.as_bytes().as_ptr_range();
        for (i, word) in words.iter().enumerate() {
            let word = word.as_ref();
            assert!(
                inside.contains(&word.as_ptr()),
                "word {i} ({word:?}) was copied out of the line instead of borrowed"
            );
        }
    }

    #[test]
    fn tokens_splits_at_runs_of_whitespace() {
        assert_eq!(
            tokens("GET /index.html HTTP/1.1"),
            ["GET", "/index.html", "HTTP/1.1"]
        );
        assert_eq!(tokens("  a\tb\n\nc  "), ["a", "b", "c"]);
        // Unicode whitespace too: U+3000 IDEOGRAPHIC SPACE and U+00A0
        // NO-BREAK SPACE.
        assert_eq!(
            tokens("naïve\u{3000}café\u{a0}日本"),
            ["naïve", "café", "日本"]
        );
        assert!(tokens("").is_empty());
        assert!(tokens(" \t\r\n ").is_empty());
    }

    #[test]
    fn tokens_borrow_from_the_line() {
        let line = String::from("POST  /api/v1/orders\tHTTP/2");
        let words = tokens(&line);
        assert_eq!(words, ["POST", "/api/v1/orders", "HTTP/2"]);
        assert_borrowed_from(&line, &words);
        // Exactly where each word starts in the line.
        assert_eq!(words[0].as_ptr(), line.as_ptr());
        assert_eq!(words[1].as_ptr(), line[6..].as_ptr());
        assert_eq!(words[2].as_ptr(), line[21..].as_ptr());
    }

    #[test]
    fn tokens_of_a_slice_borrow_from_the_slice() {
        let request = String::from("GET /a HTTP/1.1\r\nHost: example.com\r\n");
        let (first_line, _) = request.split_once("\r\n").expect("a CRLF");
        let words = tokens(first_line);
        assert_eq!(words, ["GET", "/a", "HTTP/1.1"]);
        assert_borrowed_from(first_line, &words);
        assert_eq!(words[1].as_ptr(), request[4..].as_ptr());
    }

    #[test]
    fn tokens_from_many_lines_borrow_from_each_line() {
        let log: Vec<String> = (0..200)
            .map(|i| format!("GET /item/{i} HTTP/1.1"))
            .collect();
        let words: Vec<_> = log.iter().flat_map(|line| tokens(line)).collect();
        assert_eq!(words.len(), 600);
        for (line, words) in log.iter().zip(words.chunks(3)) {
            assert_borrowed_from(line, words);
        }
        // "/item/100" to "/item/199" have 9 chars; "/item/10" to "/item/99"
        // and every "HTTP/1.1" have 8.
        assert_eq!(count_long(&words, 9), 100);
        assert_eq!(count_long(&words, 8), 100 + 90 + 200);
    }

    #[test]
    fn count_long_takes_owned_and_borrowed_words() {
        let owned: Vec<String> = ["alpha", "be", "gamma"].map(String::from).to_vec();
        assert_eq!(count_long(&owned, 5), 2);
        assert_eq!(count_long(&owned[1..], 5), 1);

        let literals = ["alpha", "be", "gamma", "delta"];
        assert_eq!(count_long(&literals, 5), 3);
        assert_eq!(count_long(&literals[1..3], 5), 1);
        assert_eq!(count_long(&literals, 0), 4);

        let line = String::from("GET /index.html HTTP/1.1");
        assert_eq!(count_long(&tokens(&line), 4), 2);

        let none: [&str; 0] = [];
        assert_eq!(count_long(&none, 0), 0);
    }

    #[test]
    fn count_long_counts_chars_not_bytes() {
        // "héllo" is 5 chars in 6 bytes, "日本語" 3 chars in 9 bytes.
        let words = ["héllo", "日本語", "abcd"];
        assert_eq!(count_long(&words, 3), 3);
        assert_eq!(count_long(&words, 4), 2);
        assert_eq!(count_long(&words, 5), 1);
        assert_eq!(count_long(&words, 6), 0);
    }
}
