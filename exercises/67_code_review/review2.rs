// Module 5 · Code review — part 2: an idiomatic refactor graded by clippy (ptr_arg, needless_range_loop, manual_map, unnecessary_unwrap, len_zero, needless_return, manual_clamp).
//
// The other kind of review comment: the code is correct, the tests pass, and
// an experienced Rust reviewer still would not approve it as it is. None of
// this is about taste. Each idiom below removes a way for the NEXT edit to go
// wrong, or lets more callers use the function. Interviewers use this round
// to find out whether you know WHY an idiom exists, not whether you can
// silence a linter. Be ready to say, for each change, what it buys:
//
//   - `&Vec<T>` and `&String` parameters (`ptr_arg`). A `&[T]` or `&str`
//     accepts everything the old parameter did (a `&Vec<T>` coerces to
//     `&[T]`, and a `&String` to `&str`), plus arrays, sub-slices and string
//     literals, without making the caller build a `Vec` or a `String`. It
//     also removes a hop: a `&Vec<T>` points at the vector's (pointer,
//     capacity, length) header, which points at the elements. The borrowed
//     form is one fat pointer straight to the data.
//   - Index loops (`needless_range_loop`). `for i in 0..v.len()` and `v[i]`
//     say "a position" when you mean "each element". An iterator cannot index
//     out of bounds, so there is no bounds check to hope the optimizer
//     removes, and no off-by-one to make in the range.
//   - A `match` that rebuilds an `Option` (`manual_map`). `Option::map` says
//     "transform the value if there is one" in one call.
//   - `is_some()` followed by `unwrap()` (`unnecessary_unwrap`). The check and
//     the extraction are two separate statements, so a later edit can move or
//     change one without the other, and the `unwrap` starts to panic.
//     `if let`, `match` and the `unwrap_or` family check and extract in ONE
//     step.
//   - `len() == 0` (`len_zero`). `is_empty()` states the intent. It is also
//     the convention clippy enforces on your own public types: a public `len`
//     without an `is_empty` is the `len_without_is_empty` lint.
//   - A trailing `return x;` (`needless_return`). A block's last expression IS
//     its value. Keep `return` for early exits, so that it stands out.
//   - A hand-written clamp (`manual_clamp`). `Ord::clamp` says it in one word.
//     It does behave differently in one case: it PANICS if `min > max`, where
//     the hand-written version silently returns one of the bounds. That is why
//     clippy only suggests it when both bounds are constants it can check.
//
// There is a trap, too: a refactor that changes behavior. The tests pin the
// current behavior, including a tie and some very large numbers, and a quick
// rewrite with the "obvious" iterator method gets one of them wrong. Run the
// tests after every change.

// Ratings shown in the lobby are clamped to this range.
const MIN_RATING: i32 = 0;
const MAX_RATING: i32 = 100;

struct Player {
    name: String,
    nickname: Option<String>,
    rating: i32,
    scores: Vec<u32>,
}

// TODO: This file compiles and every test passes, but the exercise runs
// clippy with `-D warnings` (it sets `strict_clippy`), and clippy rejects it
// with eight errors from seven lints: `ptr_arg` (twice, "writing `&Vec`
// instead of `&[_]` involves a new object where a slice will do", and the
// same for `&String`), `needless_range_loop` ("the loop variable `i` is only
// used to index `players`"), `manual_map`, `unnecessary_unwrap`, `len_zero`,
// `needless_return` and `manual_clamp`. Refactor the functions below until
// clippy is clean. Requirements:
//   - keep every function's behavior: the tests pin it (including ties and
//     very large scores), and they must keep passing unchanged;
//   - changing a parameter type is allowed (it is one of the fixes), as long
//     as every call in the tests still compiles as written;
//   - no `#[allow(..)]` or `#![allow(..)]`, and no clippy config: silencing a
//     lint is not a review answer;
//   - for each change, be ready to say what the new version buys.
// Until you clear all eight clippy errors, this exercise will not pass.

// The player whose name matches `name`, ignoring ASCII case.
fn find_player<'a>(players: &'a Vec<Player>, name: &String) -> Option<&'a Player> {
    players
        .iter()
        .find(|player| player.name.eq_ignore_ascii_case(name))
}

// The player with the highest rating. On a tie, the one listed first wins.
// `None` if there are no players.
fn top_player(players: &[Player]) -> Option<&Player> {
    if players.len() == 0 {
        return None;
    }
    let mut best = &players[0];
    for i in 1..players.len() {
        if players[i].rating > best.rating {
            best = &players[i];
        }
    }
    Some(best)
}

// The mean of the player's scores, or `None` before the first game.
fn average_score(player: &Player) -> Option<f64> {
    if player.scores.is_empty() {
        return None;
    }
    let mut total: u64 = 0;
    for &score in &player.scores {
        total += u64::from(score);
    }
    return Some(total as f64 / player.scores.len() as f64);
}

// The lobby shows nicknames in capitals.
fn lobby_tag(player: &Player) -> Option<String> {
    match &player.nickname {
        Some(nickname) => Some(nickname.to_uppercase()),
        None => None,
    }
}

// The name to greet the player with: the nickname if there is one, otherwise
// the real name.
fn display_name(player: &Player) -> &str {
    let nickname = player.nickname.as_deref();
    if nickname.is_some() {
        nickname.unwrap()
    } else {
        &player.name
    }
}

// The rating shown in the lobby: the player's rating, clamped to
// `MIN_RATING..=MAX_RATING`.
fn shown_rating(player: &Player) -> i32 {
    if player.rating < MIN_RATING {
        MIN_RATING
    } else if player.rating > MAX_RATING {
        MAX_RATING
    } else {
        player.rating
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn player(name: &str, nickname: Option<&str>, rating: i32, scores: &[u32]) -> Player {
        Player {
            name: name.to_string(),
            nickname: nickname.map(String::from),
            rating,
            scores: scores.to_vec(),
        }
    }

    fn roster() -> Vec<Player> {
        vec![
            player("Alice", Some("ace"), 87, &[90, 85]),
            player("Bob", None, 92, &[70]),
            player("Carol", Some("cc"), 92, &[]),
            player("Dave", None, -3, &[10, 20, 31]),
        ]
    }

    #[test]
    fn find_player_matches_the_name_ignoring_ascii_case() {
        let players = roster();
        let queries = [
            ("Bob", Some(1)),
            ("bob", Some(1)),
            ("BOB", Some(1)),
            ("aLiCe", Some(0)),
            ("Bobby", None),
            ("", None),
        ];
        for (query, expected) in queries {
            // Queries arrive as `String`s, say from a web request.
            let query = query.to_string();
            let found = find_player(&players, &query);
            // Where in the roster the result points: a reference INTO the
            // roster, not a copy.
            let index = found.map(|p| {
                let position = players.iter().position(|q| ptr::eq(p, q));
                position.expect("a reference into the roster")
            });
            assert_eq!(index, expected, "query: {query:?}");
        }
        let nobody: Vec<Player> = Vec::new();
        let bob = String::from("Bob");
        assert!(find_player(&nobody, &bob).is_none());

        // ASCII case only, as the comment says: "O" and "o" match, but "Ë"
        // and "ë" are different letters.
        let zoe = vec![player("Zoë", None, 0, &[])];
        let ascii_caps = String::from("ZOë");
        assert!(find_player(&zoe, &ascii_caps).is_some());
        let full_caps = String::from("ZOË");
        assert!(find_player(&zoe, &full_caps).is_none());
    }

    #[test]
    fn top_player_has_the_highest_rating() {
        let players = vec![
            player("Alice", None, 10, &[]),
            player("Bob", None, 30, &[]),
            player("Carol", None, 20, &[]),
        ];
        let top = top_player(&players).expect("three players");
        assert!(ptr::eq(top, &players[1]));
    }

    #[test]
    fn a_tie_goes_to_the_player_listed_first() {
        // Bob and Carol both have 92.
        let players = roster();
        let top = top_player(&players).expect("four players");
        assert_eq!(
            top.name, "Bob",
            "Bob and Carol tie; the one listed first wins"
        );
        assert!(ptr::eq(top, &players[1]));

        let everyone_tied: Vec<Player> = ["P", "Q", "R"]
            .iter()
            .map(|name| player(name, None, 50, &[]))
            .collect();
        let top = top_player(&everyone_tied).expect("three players");
        assert!(ptr::eq(top, &everyone_tied[0]));
    }

    #[test]
    fn top_player_of_one_and_of_none() {
        let alone = vec![player("Alice", None, -7, &[])];
        assert!(ptr::eq(top_player(&alone).expect("one player"), &alone[0]));
        assert!(top_player(&[]).is_none());
    }

    #[test]
    fn average_score_is_the_mean() {
        let players = roster();
        assert_eq!(average_score(&players[0]), Some(87.5));
        assert_eq!(average_score(&players[1]), Some(70.0));
        assert_eq!(average_score(&players[2]), None);
        assert_eq!(average_score(&players[3]), Some(61.0 / 3.0));
    }

    #[test]
    fn average_score_of_huge_scores_does_not_overflow() {
        // The sum is about 1.3e10, far above `u32::MAX`.
        let whale = player("Whale", None, 0, &[u32::MAX, u32::MAX, u32::MAX]);
        assert_eq!(average_score(&whale), Some(f64::from(u32::MAX)));
    }

    #[test]
    fn lobby_tag_is_the_nickname_in_capitals() {
        let players = roster();
        assert_eq!(lobby_tag(&players[0]).as_deref(), Some("ACE"));
        assert_eq!(lobby_tag(&players[1]), None);
        assert_eq!(lobby_tag(&players[2]).as_deref(), Some("CC"));
        // Unicode-aware, not ASCII-only.
        let zoe = player("Zoe", Some("zoë"), 0, &[]);
        assert_eq!(lobby_tag(&zoe).as_deref(), Some("ZOË"));
    }

    #[test]
    fn display_name_prefers_the_nickname_and_borrows_it() {
        let players = roster();
        let alice = display_name(&players[0]);
        assert_eq!(alice, "ace");
        // Borrowed from the player, not a new string.
        let nickname = players[0].nickname.as_ref().expect("Alice has one");
        assert_eq!(alice.as_ptr(), nickname.as_ptr());
        let bob = display_name(&players[1]);
        assert_eq!(bob, "Bob");
        assert_eq!(bob.as_ptr(), players[1].name.as_ptr());
    }

    #[test]
    fn shown_rating_is_clamped_to_the_range() {
        let cases = [
            (i32::MIN, 0),
            (-3, 0),
            (0, 0),
            (1, 1),
            (42, 42),
            (99, 99),
            (100, 100),
            (101, 100),
            (i32::MAX, 100),
        ];
        for (rating, shown) in cases {
            let p = player("P", None, rating, &[]);
            assert_eq!(shown_rating(&p), shown, "rating {rating}");
        }
    }
}
