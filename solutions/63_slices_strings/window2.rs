// Module 5 · Slices and strings — part 2: a Unicode-safe sliding window over `&str`, without char-boundary panics.
//
// "Longest substring without repeating characters" (LeetCode 3) is THE
// sliding-window question. Grow a window to the right one character at a
// time. When the new character already occurs inside the window, move the
// window's start to just past that earlier occurrence. Remember where every
// character was last seen, and the whole scan is O(n).
//
// The textbook solution indexes an array by the character's code, and in C++
// or Java that "character" is a byte or a UTF-16 unit. Ported to Rust as
// bytes, it compiles and passes every ASCII test. Then it meets "éã". A `&str`
// is UTF-8: 'é' is the two bytes C3 A9 and 'ã' is C3 A3. The byte scan sees
// the lead byte C3 twice, moves the window's start to byte 1, the middle of
// 'é', and the final `&s[best]` panics:
//
//     start byte index 1 is not a char boundary; it is inside 'é'
//     (bytes 0..2 of string)
//
// Three facts about `str` explain it:
//
//   - `s.len()` counts BYTES, not characters: `"éã".len()` is 4.
//   - Range indexing `&s[a..b]` takes BYTE offsets, and both ends must fall on
//     a char boundary or it panics at run time. `s.get(a..b)` returns `None`
//     instead, and `s.is_char_boundary(i)` asks first. Plain `s[i]` does not
//     even compile: E0277 "the type `str` cannot be indexed by `{integer}`",
//     because "the i-th character" is not an O(1) question in UTF-8.
//   - `s.char_indices()` yields `(byte_offset, char)` pairs. Every offset it
//     gives you IS a boundary, and so is `offset + c.len_utf8()`, the offset
//     just past that character. Slice only with those and nothing can panic.
//
// What counts as a "character" is an interview question of its own. Here it
// is a `char`, a Unicode scalar value, so "longest" means the most `char`s,
// not the most bytes: "🐍abc" (4 chars, 7 bytes) beats "🦀🐍" (2 chars, 8
// bytes). A `char` is not always one letter on screen either: "é" can also be
// written as 'e' followed by U+0301, a combining accent, which is two `char`s.
// Splitting text into what users see as characters (grapheme clusters) needs
// the `unicode-segmentation` crate; std has no grapheme API.
//
// A `[usize; 256]` table works for bytes. There are 1,112,064 Unicode scalar
// values, so a table indexed by `char` would need over a million slots to
// track a few dozen characters: use a `HashMap<char, _>`, which only holds
// the characters the input contains. And when "only if it is inside the
// window" is the second half of an `if let`, write it as a let chain
// (`if let Some(x) = .. && cond`), stable since Rust 1.88 in edition 2024.
//
// The result is a `&str` borrowed from the input: elision gives it the
// lifetime of `s`, the only reference parameter. No copy, no `String`. The
// tests check that the answer is a slice of the input, at the right offset.
//
// How interviewers probe this: "What does your solution do with 'éã'?", "Is
// `s.len()` the number of characters?", "Why can't I write `s[i]`?".

use std::collections::HashMap;

// Returns the longest substring of `s` in which no character repeats, as a
// slice of `s`. Length is counted in `char`s. When several substrings share
// the longest length, the LEFTMOST one wins. `""` gives `""`.
fn longest_unique_substring(s: &str) -> &str {
    // For every char seen so far: its position counted in chars, and the byte
    // offset just past it. Both come from `char_indices`, so every offset
    // stored here is a char boundary.
    let mut last_seen: HashMap<char, (usize, usize)> = HashMap::new();
    // The window starts at byte `start`, which is char number `start_pos`.
    let (mut start, mut start_pos) = (0, 0);
    let (mut best, mut best_len) = (0..0, 0);
    for (pos, (i, c)) in s.char_indices().enumerate() {
        let end = i + c.len_utf8();
        // `insert` hands back the previous entry, so each char costs one hash
        // lookup. The let chain's second condition matters: an occurrence
        // that is already behind the window must not move its start back
        // ("abba").
        if let Some((prev_pos, prev_end)) = last_seen.insert(c, (pos, end))
            && prev_pos >= start_pos
        {
            (start, start_pos) = (prev_end, prev_pos + 1);
        }
        // Length in chars. Only a strictly longer window replaces the best,
        // so the leftmost one keeps a tie.
        let len = pos + 1 - start_pos;
        if len > best_len {
            (best, best_len) = (start..end, len);
        }
    }
    // Both ends of `best` came from `char_indices`, so this cannot panic, and
    // the result borrows from `s` instead of copying it.
    &s[best]
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // The byte offset of `part` inside `whole`. Panics unless `part` is a
    // slice of `whole` itself: the same buffer, and in bounds. (An empty input
    // has no buffer to point into, so any empty answer counts as offset 0.)
    fn offset_in(whole: &str, part: &str) -> usize {
        if whole.is_empty() && part.is_empty() {
            return 0;
        }
        let (start, at) = (whole.as_ptr().addr(), part.as_ptr().addr());
        assert!(
            start <= at && at + part.len() <= start + whole.len(),
            "{part:?} is not a slice of {whole:?}: return a borrow of the input"
        );
        at - start
    }

    // Checks the answer's text AND where it sits in the input.
    fn check(s: &str, expected: &str, offset: usize) {
        let got = longest_unique_substring(s);
        assert_eq!(got, expected, "input {s:?}");
        assert_eq!(
            offset_in(s, got),
            offset,
            "input {s:?}: right text, wrong place (the LEFTMOST window wins)"
        );
    }

    // A slow but obviously correct model: from every start, extend while the
    // characters stay unique, and keep the first of the longest.
    fn model(s: &str) -> &str {
        let chars: Vec<(usize, char)> = s.char_indices().collect();
        let (mut best_len, mut best) = (0, 0..0);
        for (a, &(start, _)) in chars.iter().enumerate() {
            let mut seen = HashSet::new();
            let mut end = start;
            for &(i, c) in &chars[a..] {
                if !seen.insert(c) {
                    break;
                }
                end = i + c.len_utf8();
            }
            if seen.len() > best_len {
                (best_len, best) = (seen.len(), start..end);
            }
        }
        &s[best]
    }

    // A tiny deterministic pseudo-random generator (a 64-bit LCG), so every
    // run checks exactly the same inputs.
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 33
    }

    #[test]
    fn ascii_classics() {
        check("abcabcbb", "abc", 0);
        check("bbbbb", "b", 0);
        check("pwwkew", "wke", 2);
        check("dvdf", "vdf", 1);
        check("abcabcd", "abcd", 3);
        check(" ", " ", 0);
    }

    #[test]
    fn a_repeat_outside_the_window_does_not_move_its_start_back() {
        // At the second 'a' the window is "b": the first 'a' is already
        // behind it, so the start must stay where it is.
        check("abba", "ab", 0);
        check("tmmzuxt", "mzuxt", 2);
    }

    #[test]
    fn empty_input_gives_empty_output() {
        assert_eq!(longest_unique_substring(""), "");
    }

    #[test]
    fn two_accented_letters_are_two_different_chars() {
        // 'é' is C3 A9 and 'ã' is C3 A3: different chars, same first byte.
        check("éã", "éã", 0);
        check("ãé", "ãé", 0);
        check("ééa", "éa", 2);
        check("çãé", "çãé", 0);
    }

    #[test]
    fn emoji_are_one_char_of_four_bytes() {
        check("🦀🦀", "🦀", 0);
        check("a🦀b🦀c", "a🦀b", 0);
        // Counted in chars, "🐍abc" (4 chars, 7 bytes) is longer than "🦀🐍"
        // (2 chars, 8 bytes).
        check("🦀🐍🐍abcc", "🐍abc", 8);
        check("🦀🐍🦈🐍🦀", "🦀🐍🦈", 0);
    }

    #[test]
    fn three_byte_characters() {
        check("日本語の日本", "日本語の", 0);
        // "sumomo mo momo mo momo no uchi": 3 bytes per char, so the answer
        // starts 8 chars in, at byte 24.
        check("すもももももももものうち", "ものうち", 24);
    }

    #[test]
    fn a_combining_accent_is_a_char_of_its_own() {
        // 'e' + U+0301 renders as one "é" but is two chars, so the window
        // "ae\u{301}" has three chars and ends before the second 'e'.
        check("ae\u{301}e", "ae\u{301}", 0);
    }

    #[test]
    fn matches_a_brute_force_model() {
        // One-, two-, three- and four-byte characters, with repeats.
        let alphabet = ['a', 'b', 'c', 'é', 'ã', 'ç', '日', '本', '🦀', '🐍'];
        let mut state = 2024;
        for _ in 0..3_000 {
            let len = next(&mut state) % 14;
            let s: String = (0..len)
                .map(|_| alphabet[(next(&mut state) as usize) % alphabet.len()])
                .collect();
            let expected = model(&s);
            check(&s, expected, offset_in(&s, expected));
        }
    }
}
