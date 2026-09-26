// Module 5 · Mock interviews — part 2: Levenshtein distance over chars in 20 minutes, in O(min(n, m)) memory (the stub panics, so the tests fail).
//
// The second timed round, in the same format as `set_trie`: the `// TODO`
// below is the statement, and the only given code is the signature. Start
// the timer, say your plan and its cost out loud before you type, run the
// tests when you think you are done, and press `h` for the rubric only
// afterwards. The tests below the banner are the follow-ups.
//
// Edit distance is one of the dynamic programming problems that interviewers
// reach for most often. The recurrence is short enough to derive on a
// whiteboard, so the round is graded on everything around it: stating what
// a cell of the table means and deriving the recurrence from that meaning
// instead of reciting it, getting the first row and column right, the time
// and memory cost and how to cut the memory, and one question that has no
// single right answer in general but has one in every program: what is a
// character?

// Problem (20 minutes). Implement `edit_distance(a, b)`: the Levenshtein
// distance between `a` and `b`, that is, the smallest number of
// single-character insertions, deletions and substitutions that turn `a`
// into `b`.
// Examples: "kitten" to "sitting" is 3 (k -> s, e -> i, then insert g);
// "flaw" to "lawn" is 2; "" to "abc" is 3; a string to itself is 0.
// What the interviewer tells you if you ask: a character is a `char` (a
// Unicode scalar value), not a byte; swapping two neighbors counts as two
// edits (this is Levenshtein, not Damerau-Levenshtein); the strings can be
// thousands of characters long. Aim for O(n * m) time, and expect the
// follow-up "can you do it in O(min(n, m)) extra memory?", so you might as
// well answer it right away.
//
// The reference answer: the Wagner-Fischer table, one row at a time. Cell
// `j` of row `i` is the distance between the first `i` chars of the longer
// string and the first `j` chars of the shorter one. A row only reads the
// row above it, so two rows as long as the SHORTER string (plus one) are
// enough: O(n * m) time, O(min(n, m)) memory for the rows.

use std::mem;

fn edit_distance(a: &str, b: &str) -> usize {
    // Iterate the longer string and keep a row per char of the shorter one.
    // Counting the chars costs O(n + m), nothing next to the table.
    let (long, short) = if a.chars().count() >= b.chars().count() {
        (a, b)
    } else {
        (b, a)
    };
    // The inner loop needs the short string's chars by position, so collect
    // them once; `str` has no O(1) "n-th char" (UTF-8 is variable width).
    let short: Vec<char> = short.chars().collect();

    // Row 0: turning "" into the first `j` chars of `short` takes `j`
    // insertions.
    let mut prev: Vec<usize> = (0..=short.len()).collect();
    let mut curr = vec![0; short.len() + 1];
    for (i, lc) in long.chars().enumerate() {
        // Column 0: turning the first `i + 1` chars of `long` into "" takes
        // `i + 1` deletions.
        curr[0] = i + 1;
        for (j, &sc) in short.iter().enumerate() {
            // Keep or substitute `lc`, delete it, or insert `sc` after it.
            let substitute = prev[j] + usize::from(lc != sc);
            let delete = prev[j + 1] + 1;
            let insert = curr[j] + 1;
            curr[j + 1] = substitute.min(delete).min(insert);
        }
        // The row just computed is the row above the next one. Swapping
        // reuses both buffers: no allocation inside the loop.
        mem::swap(&mut prev, &mut curr);
    }
    prev[short.len()]
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- The examples from the statement ----

    #[test]
    fn statement_examples() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("flaw", "lawn"), 2);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("same", "same"), 0);
    }

    #[test]
    fn classic_pairs_in_both_directions() {
        for (a, b, distance) in [
            ("kitten", "sitting", 3),
            ("intention", "execution", 5),
            ("horse", "ros", 3),
            ("abc", "yabd", 2),
            ("sunday", "saturday", 3),
            ("a", "b", 1),
            ("a", "", 1),
        ] {
            assert_eq!(edit_distance(a, b), distance, "{a:?} -> {b:?}");
            assert_eq!(edit_distance(b, a), distance, "{b:?} -> {a:?}");
        }
    }

    // ==== Follow-ups: what the interviewer asks once the examples pass ====

    #[test]
    fn empty_strings() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("", "日本語"), 3);
    }

    #[test]
    fn swapping_two_neighbors_costs_two() {
        // Levenshtein has no transposition: two substitutions (or a delete
        // and an insert). Damerau-Levenshtein would say 1.
        assert_eq!(edit_distance("ab", "ba"), 2);
        assert_eq!(edit_distance("form", "from"), 2);
    }

    #[test]
    fn a_character_is_a_char_not_a_byte() {
        // 'é' is two bytes in UTF-8, so a byte-based version says 2 here.
        assert_eq!(edit_distance("café", "cafe"), 1);
        assert_eq!(edit_distance("naïve", "naive"), 1);
        // A four-byte char: a byte-based version says 4 and 3.
        assert_eq!(edit_distance("🎉a", "a"), 1);
        assert_eq!(edit_distance("日本語", "日本"), 1);
        assert_eq!(edit_distance("日本語", "日本人"), 1);
    }

    #[test]
    fn a_char_is_not_a_letter_on_screen_either() {
        // Both print as "café". The first ends in one char, 'é' (U+00E9);
        // the second in two, 'e' and a combining acute accent (U+0301). Over
        // chars the distance is 2 (substitute, then insert the accent); a
        // byte-based version says 3. Making the two equal takes Unicode
        // normalization, and counting "e" plus its accent as one letter takes
        // grapheme clusters. std has neither.
        let composed = "caf\u{e9}";
        let decomposed = "cafe\u{301}";
        assert_eq!(composed.chars().count(), 4);
        assert_eq!(decomposed.chars().count(), 5);
        assert_eq!(edit_distance(composed, decomposed), 2);
        assert_eq!(edit_distance(decomposed, composed), 2);
    }

    #[test]
    fn long_strings() {
        // 3_000 x 3_000 chars: nine million cells, a fraction of a second
        // even in a debug build.
        let abab = "ab".repeat(1_500);
        let baba = "ba".repeat(1_500);
        // Delete the leading 'a', then append an 'a'.
        assert_eq!(edit_distance(&abab, &baba), 2);
        let a = "a".repeat(3_000);
        let b = "b".repeat(3_000);
        assert_eq!(edit_distance(&a, &b), 3_000);
        // Very different lengths.
        let long = "xy".repeat(50_000);
        assert_eq!(edit_distance(&long, "yx"), 99_998);
        assert_eq!(edit_distance("", &long), 100_000);
    }

    // The textbook full table, O(n * m) memory, to check against.
    fn full_table(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut d = vec![vec![0; b.len() + 1]; a.len() + 1];
        for (i, row) in d.iter_mut().enumerate() {
            row[0] = i;
        }
        for (j, cell) in d[0].iter_mut().enumerate() {
            *cell = j;
        }
        for i in 1..=a.len() {
            for j in 1..=b.len() {
                let cost = usize::from(a[i - 1] != b[j - 1]);
                d[i][j] = (d[i - 1][j - 1] + cost)
                    .min(d[i - 1][j] + 1)
                    .min(d[i][j - 1] + 1);
            }
        }
        d[a.len()][b.len()]
    }

    // A tiny deterministic pseudo-random generator (Knuth's MMIX LCG), so
    // every run checks exactly the same inputs.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, bound: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) % bound as u64) as usize
        }

        // Up to `max_len` chars from a small alphabet with one-, two- and
        // four-byte chars, so that the random strings share a lot.
        fn string(&mut self, max_len: usize) -> String {
            const ALPHABET: [char; 4] = ['a', 'b', 'é', '🎉'];
            let len = self.below(max_len + 1);
            (0..len)
                .map(|_| ALPHABET[self.below(ALPHABET.len())])
                .collect()
        }
    }

    #[test]
    fn random_pairs_match_the_full_table() {
        let mut rng = Lcg(42);
        for _ in 0..2_000 {
            let a = rng.string(9);
            let b = rng.string(9);
            assert_eq!(edit_distance(&a, &b), full_table(&a, &b), "{a:?} -> {b:?}");
        }
    }
}
