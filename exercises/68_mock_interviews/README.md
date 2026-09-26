# Module 5 · Mock Interviews: Timed, Statement-First Problems With Part-2 Follow-Ups

> The last module of the course, after the review round of `67_code_review`.
> Almost everything before it trains you to solve a problem once a TODO has
> named it. A live-coding round starts from a problem statement and an empty
> editor, with a clock running. These four exercises are that round: a
> statement, the signatures with `todo!()` bodies, and nothing else. All
> **std**, **100% safe**, **stable** Rust, edition 2024.
>
> In this module the `// TODO` **is** the problem statement. It says what to
> build and under which constraints, never how, and every struct starts
> without fields. The tests come in two groups: the examples from the
> statement, then, below a banner, the follow-ups an interviewer asks once
> the examples pass. Press `h` only after the round: the hint is the
> interviewer's rubric plus a sketch of a reference answer, not a
> walkthrough.

## Core Ideas

- **Clarify first, and out loud.** Every statement ends with "what the
  interviewer tells you if you ask": the input domain (any UTF-8? the empty
  string?), the limits, the exact semantics of the corner cases. In a real
  round you get those answers only by asking, and asking is graded.
- **Commit to a design before you type, and say why.** The trie's children
  map, the edit distance table, where a transaction's writes live until it
  commits. Name one alternative and what it would cost; that trade-off is
  most of the design score.
- **Every operation gets a cost, in terms of its input.** "O(length of the
  word), whatever the number of words", "O(n * m) time and O(min(n, m))
  memory", "BEGIN is O(1) and ROLLBACK is O(keys written)". Complexity
  stated before the interviewer asks counts for more than complexity
  extracted by questions.
- **The follow-ups are the edge cases you did not ask about.** The empty
  prefix, a limit of 0 or `usize::MAX`, a word with a 'ü' in it, a delete
  inside a nested transaction that is then committed. The tests below each
  banner are those questions. Try to predict them before you read them.
- **Rust is part of the grade.** Borrow instead of copying (`get` returns a
  `&str` into the store), move instead of cloning (a committed value is the
  `String` that was stored), know what a `char` is and what it is not, and
  get past the borrow checker with a better structure, not with `.clone()`.
- **Part 2 grades part 1.** The follow-up extends your own code. A design
  that keeps the right information makes `count` a few lines; one that does
  not has to be reworked under the clock.

## How to Run a Round

1. Open the exercise and read only the statement, not the tests. Start a
   timer for its time box.
2. Spend the first minutes on questions and a plan: the representation, the
   cost of every operation, the edge cases. Write them down as comments if
   nobody is listening.
3. Write the code. Test it the way you would in a shared editor: a few
   calls in `main`, or your own `#[test]` functions.
4. Run the exercise. Fix what the examples show, then read the follow-up
   tests that fail, as if the interviewer had just asked them.
5. When the tests pass or the timer runs out, press `h` and grade yourself
   against the rubric. Note where you ran out of time; that is the part to
   drill.
6. For a second attempt, delete your solution and start again a few days
   later, faster.

## What Interviewers Grade

| Dimension     | Strong signal                                                                  | Weak signal                                                  |
| ------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------ |
| Understanding | asks about the input domain and corner cases before coding                     | codes the happy path, then discovers the edge cases in tests |
| Design        | picks a representation, names an alternative and what each costs               | a representation that happened to come to mind first         |
| Complexity    | states time and memory per operation unprompted, and how to improve them       | "it's fast", or a cost that is off by a factor of `n`        |
| Correctness   | handles the empty input, the limits, non-ASCII text, errors                    | passes the example and stops                                 |
| Rust fluency  | borrows where it can, moves where it must, no `clone` to silence the checker   | `.clone()` and `.unwrap()` wherever the compiler complained  |
| Testing       | writes a test for each edge case it mentioned, or checks against a slow model  | runs the given example once                                  |
| Communication | thinks out loud, keeps the interviewer oriented, adapts to hints               | long silences, then a finished answer                        |

## The Four Rounds

| Round               | Time   | Problem                                            | The Rust-specific trap                                                                |
| ------------------- | ------ | -------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `set_trie`          | 25 min | autocomplete: insert, lookup, prefix, suggestions  | a `[_; 26]` child array only knows 'a'..='z'; `suggest` has to return owned `String`s |
| `set_edit_distance` | 20 min | Levenshtein distance                               | bytes vs `char`s vs letters on screen; two rows instead of the full table             |
| `set_kv_tx`         | 35 min | key-value store with nested BEGIN/COMMIT/ROLLBACK  | `get` borrows from whichever layer holds the value; commit moves, never clones        |
| `set_kv_tx_part2`   | 15 min | add COUNT(value) that stays right across rollbacks | updating the counts while holding a `&str` borrowed from the store                    |

## Exercise Path

1. **set_trie** — Implement an autocomplete `Trie` from scratch: `insert`
   (returns whether the word was new), `contains`, `starts_with` and
   `suggest(prefix, limit)` in lexicographic order. The follow-ups check a
   prefix that leads nowhere, a word that is its own prefix, limits of 0
   and `usize::MAX`, the empty prefix and the empty word, and words such as
   "über", "日本語" and "new york" (a `[_; 26]` array panics on them). A
   model test checks random words and queries against a sorted set.
2. **set_edit_distance** — Implement the Levenshtein distance over `char`s
   in O(n * m) time and O(min(n, m)) memory. The follow-ups check the empty
   strings, that swapping two neighbors costs 2, that "café" and "cafe" are
   1 apart (a byte-based version says 2), that a precomposed and a
   decomposed "café" are 2 apart (a `char` is not a letter on screen), and
   3_000-char inputs. A model test compares 2_000 random pairs with the full
   table.
3. **set_kv_tx** — Implement an in-memory store with nested transactions:
   `set`, `get` (returning a `&str`), `delete`, `begin`, `commit` (into the
   parent transaction) and `rollback`, with `TxError::NoTransaction` when no
   transaction is open. The follow-ups check that a delete survives a
   nested commit, writes after a delete, a thousand nested levels, and
   heap-pointer identity: `begin`, `commit` and `rollback` must not copy the
   store or its values. A time budget catches a copy per `begin` that
   shares its values through `Rc` and so keeps every pointer. A model test
   replays 15_000 random operations against a stack of full copies.
4. **set_kv_tx_part2** — The follow-up round: copy your `Kv` from
   `set_kv_tx` over the stubs (only the signatures are given, so the file
   does not spoil part 1) and add `count(value)`, the number of keys whose
   visible value is `value`. It does not compile until `count` exists
   (E0599, "`Kv` is not an iterator": rustc only knows `Iterator::count`).
   The tests count through overwrites, deletes, rollbacks, nested commits
   and tombstones, and one test gives `count` and `rollback` a time budget
   that a scan cannot meet, whether it walks the store or only the keys
   that the open transactions wrote.

## Related Modules

- `60_lru_cache` — the most common "design a data structure" round, built
  in three steps. `59_arena` and `27_data_structures` cover the ownership
  side of linked structures, including why a deep recursive structure needs
  an iterative `Drop`.
- `63_slices_strings/window2` — bytes vs `char`s and char boundaries, the
  point behind the "über" and "café" follow-ups.
- `39_drop_raii/raii2` — a single transaction that rolls back in `Drop` by
  restoring a snapshot. `set_kv_tx` is the nested version, where a
  snapshot per `begin` is exactly what the interviewer rules out. Wrapping
  `begin` in such a guard is a good follow-up to discuss.
- `37_borrowck_errors/borrowck1` — split borrows through fields, which is
  how `set_kv_tx_part2` updates its counts while holding a `&str` from the
  store.
- `67_code_review` — the review round that comes before this one.
- `61_trees` and `62_graphs` — more live-coding problems in the guided
  format: an `Option<Box<Node>>` binary search tree, then BFS, iterative
  DFS, topological sort, union-find and grid searches. A trie is a tree
  too, and the same iterative techniques apply to it.

## Further Reading

- The LeetCode versions: [208. Implement Trie](https://leetcode.com/problems/implement-trie-prefix-tree/), [1268. Search Suggestions System](https://leetcode.com/problems/search-suggestions-system/) and [72. Edit Distance](https://leetcode.com/problems/edit-distance/)
- [Trie](https://en.wikipedia.org/wiki/Trie), [Levenshtein distance](https://en.wikipedia.org/wiki/Levenshtein_distance) and [the Wagner-Fischer algorithm](https://en.wikipedia.org/wiki/Wagner%E2%80%93Fischer_algorithm) (Wikipedia)
- [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) (iterates in key order) and [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html) with its [`Entry` API](https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html)
- [`char`](https://doc.rust-lang.org/std/primitive.char.html) ("a Unicode scalar value") and [`str::chars`](https://doc.rust-lang.org/std/primitive.str.html#method.chars), whose docs warn that a `char` is not a grapheme cluster
- [Unicode Normalization Forms (UAX #15)](https://unicode.org/reports/tr15/), for why "é" can be one `char` or two
- [SQLite: SAVEPOINT, RELEASE and ROLLBACK TO](https://www.sqlite.org/lang_savepoint.html) and [PostgreSQL: SAVEPOINT](https://www.postgresql.org/docs/current/sql-savepoint.html), the nested transactions of real databases
- [Tech Interview Handbook: coding interview rubrics](https://www.techinterviewhandbook.org/coding-interview-rubrics/)
