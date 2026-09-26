// Module 5 · Mock interviews — part 1: an autocomplete trie in 25 minutes, statement first (the stubs panic, so the tests fail).
//
// The course ends where a live-coding round begins: a problem statement and
// an empty editor. Until now every exercise told you which error to fix and
// which tool fits. In this module nothing does. Its four files are timed
// problems in the format of a real round. Each one opens with the statement
// (the `// TODO` below) and gives only the signatures, with stub bodies that
// panic. Every design choice is yours, down to the fields of the struct.
//
// How to run the round:
//   1. Start a timer for the time box in the statement.
//   2. Before you type, say (or write down) the design you picked, why, and
//      the time and memory cost of every operation. Ask yourself the
//      clarifying questions an interviewer expects to hear; the statement
//      lists the answers you would get.
//   3. Write the code, then run the tests. The first group checks the
//      examples from the statement. The group below the banner holds the
//      follow-ups an interviewer asks once the examples pass: the edge cases
//      you should have asked about. Try to predict them before you read
//      them.
//   4. When the tests pass or the timer runs out, press `h`. The hint is the
//      interviewer's rubric plus a sketch of a reference answer, not a
//      walkthrough.
//
// Autocomplete is a favorite warm-up because it is small enough for 25
// minutes and still has a real design decision in it. What gets graded: how
// a node stores its children and what that choice costs, the cost of each
// operation in terms of the length of the word rather than the number of
// words, the order of the suggestions, and what the code does with input you
// did not expect.

// TODO: Problem (25 minutes). Implement `Trie`, a prefix tree that stores a
// set of words for an autocomplete box:
//   - `insert(word)` stores `word` and returns `true` if it was new, `false`
//     if it was already stored (like `HashSet::insert`);
//   - `contains(word)` says whether `word` itself was inserted;
//   - `starts_with(prefix)` says whether at least one stored word starts
//     with `prefix`;
//   - `suggest(prefix, limit)` returns the stored words that start with
//     `prefix`, in lexicographic order (the order of `str`'s `Ord`), each
//     one once, and no more than `limit` of them.
// Example: after inserting "car", "card", "care", "cat" and "dog",
// `suggest("car", 2)` is `["car", "card"]`, `suggest("ca", 9)` is
// `["car", "card", "care", "cat"]`, `starts_with("ca")` is true, and
// `contains("ca")` is false.
// What the interviewer tells you if you ask: a word is any `&str` (any
// UTF-8, and the empty string is a word too), every word is a prefix of
// itself, and `limit` can be any `usize`. `insert`, `contains` and
// `starts_with` must take time proportional to the length of their
// argument, however many words are stored.
// Pick the fields (and any helper types) yourself; keep the signatures and
// the tests. Until you replace every `todo!()`, the tests will fail.

// Your representation: add the fields, and any helper types you need.
struct Trie {}

impl Trie {
    fn new() -> Self {
        todo!()
    }

    fn insert(&mut self, word: &str) -> bool {
        todo!()
    }

    fn contains(&self, word: &str) -> bool {
        todo!()
    }

    fn starts_with(&self, prefix: &str) -> bool {
        todo!()
    }

    fn suggest(&self, prefix: &str, limit: usize) -> Vec<String> {
        todo!()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn trie_of(words: &[&str]) -> Trie {
        let mut trie = Trie::new();
        for word in words {
            trie.insert(word);
        }
        trie
    }

    // ---- The examples from the statement ----

    #[test]
    fn statement_example() {
        let trie = trie_of(&["car", "card", "care", "cat", "dog"]);
        assert_eq!(trie.suggest("car", 2), ["car", "card"]);
        assert_eq!(trie.suggest("car", 9), ["car", "card", "care"]);
        assert_eq!(trie.suggest("ca", 9), ["car", "card", "care", "cat"]);
        assert!(trie.starts_with("ca"));
        assert!(trie.starts_with("do"));
        assert!(!trie.starts_with("cb"));
        assert!(trie.contains("car"));
        assert!(trie.contains("dog"));
        // A prefix of a word is not a word, and neither is a longer string.
        assert!(!trie.contains("ca"));
        assert!(!trie.contains("cards"));
    }

    #[test]
    fn insert_says_whether_the_word_is_new() {
        let mut trie = Trie::new();
        assert!(trie.insert("card"));
        assert!(!trie.insert("card"));
        // "car" runs along the path of "card", but it is a new word.
        assert!(trie.insert("car"));
        assert!(!trie.insert("car"));
        assert!(trie.insert("care"));
        // Each word is suggested once, however often it was inserted.
        assert_eq!(trie.suggest("ca", 9), ["car", "card", "care"]);
    }

    // ==== Follow-ups: what the interviewer asks once the examples pass ====

    #[test]
    fn a_prefix_that_leads_nowhere_suggests_nothing() {
        let trie = trie_of(&["car", "card", "care", "cat", "dog"]);
        let none: Vec<String> = Vec::new();
        assert_eq!(trie.suggest("x", 5), none);
        assert_eq!(trie.suggest("cb", 5), none);
        // Past the end of a stored word.
        assert_eq!(trie.suggest("cards", 5), none);
        assert_eq!(trie.suggest("dogs", 5), none);
        assert!(!trie.starts_with("dogs"));
        assert!(!trie.starts_with("x"));
    }

    #[test]
    fn a_whole_word_is_a_prefix_of_itself() {
        let trie = trie_of(&["car", "card", "care", "cat", "dog", "a", "ab"]);
        assert!(trie.starts_with("dog"));
        assert!(trie.starts_with("card"));
        assert_eq!(trie.suggest("dog", 5), ["dog"]);
        assert_eq!(trie.suggest("card", 5), ["card"]);
        // A one-letter word, and a word that is a prefix of another one.
        assert!(trie.contains("a"));
        assert_eq!(trie.suggest("a", 5), ["a", "ab"]);
    }

    #[test]
    fn the_limit_caps_the_suggestions() {
        let trie = trie_of(&["car", "card", "care", "cat", "dog"]);
        let none: Vec<String> = Vec::new();
        assert_eq!(trie.suggest("ca", 0), none);
        assert_eq!(trie.suggest("", 0), none);
        assert_eq!(trie.suggest("ca", 1), ["car"]);
        assert_eq!(trie.suggest("ca", 3), ["car", "card", "care"]);
        assert_eq!(trie.suggest("ca", 4), ["car", "card", "care", "cat"]);
        assert_eq!(trie.suggest("ca", 5), ["car", "card", "care", "cat"]);
        // Any `usize` is a valid limit, so don't allocate `limit` slots up
        // front.
        let all = ["car", "card", "care", "cat", "dog"];
        assert_eq!(trie.suggest("", usize::MAX), all);
    }

    #[test]
    fn the_empty_prefix_and_the_empty_word() {
        let mut trie = Trie::new();
        let none: Vec<String> = Vec::new();
        // No word at all: nothing starts with "", and nothing is suggested.
        assert!(!trie.starts_with(""));
        assert!(!trie.contains(""));
        assert_eq!(trie.suggest("", 5), none);

        trie.insert("dog");
        trie.insert("cat");
        assert!(trie.starts_with(""));
        assert!(!trie.contains(""));
        assert_eq!(trie.suggest("", 5), ["cat", "dog"]);

        // The empty string is a word like any other, and the smallest one.
        assert!(trie.insert(""));
        assert!(!trie.insert(""));
        assert!(trie.contains(""));
        assert_eq!(trie.suggest("", 5), ["", "cat", "dog"]);
        assert_eq!(trie.suggest("", 1), [""]);

        let mut only_empty = Trie::new();
        only_empty.insert("");
        assert!(only_empty.starts_with(""));
        assert!(!only_empty.starts_with("a"));
        assert_eq!(only_empty.suggest("", 5), [""]);
    }

    #[test]
    fn words_are_any_utf8_not_only_lowercase_ascii() {
        let words = [
            "über",
            "uber",
            "ubiquitous",
            "zebra",
            "日本",
            "日本語",
            "Zoe",
            "new york",
        ];
        let trie = trie_of(&words);
        assert!(trie.contains("über"));
        assert!(trie.contains("日本語"));
        assert!(trie.contains("new york"));
        assert!(!trie.contains("ü"));
        assert!(trie.starts_with("üb"));
        assert!(trie.starts_with("日"));
        assert!(!trie.starts_with("zoe"));
        assert_eq!(trie.suggest("ü", 9), ["über"]);
        assert_eq!(trie.suggest("u", 9), ["uber", "ubiquitous"]);
        assert_eq!(trie.suggest("日本", 9), ["日本", "日本語"]);
        assert_eq!(trie.suggest("new ", 9), ["new york"]);
        // `str`'s order compares the UTF-8 bytes, which sorts like the code
        // points: 'Z' (U+005A) < 'n' < 'u' < 'z' (U+007A) < 'ü' (U+00FC) <
        // '日' (U+65E5).
        assert_eq!(
            trie.suggest("", 99),
            [
                "Zoe",
                "new york",
                "uber",
                "ubiquitous",
                "zebra",
                "über",
                "日本",
                "日本語"
            ]
        );
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

        // A string of 0 to `max_len` chars, from an alphabet with one-,
        // two- and three-byte chars, so that many words share prefixes.
        fn word(&mut self, max_len: usize) -> String {
            const ALPHABET: [char; 5] = ['a', 'b', 'é', 'Z', '日'];
            let len = self.below(max_len + 1);
            (0..len)
                .map(|_| ALPHABET[self.below(ALPHABET.len())])
                .collect()
        }
    }

    #[test]
    fn random_words_match_a_sorted_set() {
        for seed in [1, 7, 42, 2024] {
            let mut rng = Lcg(seed);
            let mut trie = Trie::new();
            // The model: a sorted set of the words, searched the slow way.
            let mut model = BTreeSet::new();
            for _ in 0..200 {
                let word = rng.word(5);
                assert_eq!(
                    trie.insert(&word),
                    model.insert(word.clone()),
                    "seed {seed}: insert({word:?})"
                );
            }
            for _ in 0..400 {
                let prefix = rng.word(4);
                let limit = rng.below(8);
                let matching = || model.iter().filter(|w| w.starts_with(prefix.as_str()));
                let expected: Vec<String> = matching().take(limit).cloned().collect();
                assert_eq!(
                    trie.suggest(&prefix, limit),
                    expected,
                    "seed {seed}: suggest({prefix:?}, {limit})"
                );
                let all: Vec<String> = matching().cloned().collect();
                assert_eq!(trie.suggest(&prefix, usize::MAX), all, "seed {seed}");
                assert_eq!(
                    trie.starts_with(&prefix),
                    !all.is_empty(),
                    "seed {seed}: starts_with({prefix:?})"
                );
                assert_eq!(
                    trie.contains(&prefix),
                    model.contains(&prefix),
                    "seed {seed}: contains({prefix:?})"
                );
            }
        }
    }
}
