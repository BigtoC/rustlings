// Module 1 · Borrow-checker errors — part 1: two `&mut` at once, split borrows (E0499).
//
// This module is interview prep. Every exercise reproduces a borrow-checker
// error that interviewers like to put on a whiteboard. Read the error first —
// rustc names exactly which two borrows collide and where — then fix the
// DESIGN. `.clone()`, `unsafe` and `RefCell` can all make an error like this go
// away, and none of them is the answer the interviewer is waiting for.
//
// E0499 is the "exclusive" half of the rule from `24_ownership_model/ownership3`:
// a place may have at most ONE live `&mut` at a time. With non-lexical lifetimes
// a borrow lives until its last use, so E0499 fires when a second `&mut` to the
// same (or an overlapping) place is created while the first one is still going
// to be used.
//
// Part A — indexing. The borrow checker reasons about PLACES, not values. It
// never evaluates `i` or `j` (they only exist at run time), so `&mut tickets[i]`
// and `&mut tickets[j]` are both borrows of the place `tickets[_]` — rustc even
// prints it that way — and they conflict. Even `&mut arr[0]` next to
// `&mut arr[1]` is rejected: an index is an expression the checker does not
// look inside, however literal it is. On a `Vec` it is coarser still, because
// `v[i]` is sugar for the method call `*v.index_mut(i)`, which borrows the whole
// `Vec`. And the checker is right to refuse: nothing stops a caller from
// passing `i == j`, and two live `&mut` to one element would be undefined
// behavior.
//
// "Give me three ways to get two `&mut` into one `Vec`" is the classic follow-up.
// All three are slice methods, which a `Vec` reaches through `DerefMut`:
//
//   1. `split_at_mut(mid)` cuts a slice into two `&mut [T]` halves, `[..mid]`
//      and `[mid..]`, that cannot overlap, so you may hold one `&mut` into each.
//      It is the oldest tool (rustc's own help line suggests it), it panics if
//      `mid > len`, and ordering the indices, `i == j` and the bounds checks
//      are all your job.
//   2. `get_disjoint_mut([i, j])` (stable since Rust 1.86) checks at run time
//      that every index is in bounds and that no two are equal, and only then
//      returns `Ok([&mut T; N])`; otherwise you get a `GetDisjointMutError`
//      telling you which check failed. (`HashMap::get_disjoint_mut` exists too,
//      but it PANICS on duplicate keys.)
//   3. Don't hold two `&mut` at all: let the slice's own methods do the
//      aliasing-sensitive work — `swap(i, j)`, `rotate_left(k)`, `sort()`, and
//      friends — so no two `&mut` ever reach your code. But `swap` moves WHOLE
//      elements, which is exactly why it is not enough below: only the titles
//      may move, every id stays put.
//
// (Slice patterns are disjoint too: `if let [first, second, ..] = tickets` binds
// two `&mut` at once, because a pattern's positions are fixed by its shape,
// which the checker does see.)
//
// Why not `Vec<RefCell<Ticket>>`? It compiles, but it only moves the same check
// to run time: every access pays for a borrow flag, every user of the type has
// to write `.borrow_mut()`, and `i == j` becomes a "RefCell already borrowed"
// panic (the `BorrowMutError` case from `31_debugging/debugging4`) instead of a
// compile error. Interior mutability is for aliasing you cannot rule out
// statically — shared ownership, graphs. Two different slots of one slice CAN
// be proven disjoint, so prove it.
//
// Part B — the same error, hidden behind methods. The borrow checker works one
// function at a time and trusts only SIGNATURES. `fn buffer_mut(&mut self) ->
// &mut String` promises "the result borrows all of `*self`" (elision ties the
// output lifetime to `&mut self`), whatever field the body happens to touch.
// So two accessor calls are two `&mut *ed`, and the second one is E0499. Field
// paths are different: `ed.buffer` and `ed.history` are DISJOINT places, and
// the checker tracks fields precisely, so a `&mut` to each may be live at the
// same time. That is a "split borrow". When the fields are private to another
// module, the idiomatic API is a single method that borrows `self` ONCE and
// hands out the pieces as a tuple of `&mut`s.
//
// How interviewers probe this: "Why do `&mut v[0]` and `&mut v[1]` conflict when
// they are obviously different?", "Why do field borrows work but getters
// don't?", "Would you reach for `RefCell` here?". This exercise is all three.

use std::mem;

// ---- Part A — two `&mut` into one slice -----------------------------------

// Deliberately NOT `Clone`: copying tickets around is not the way out.
#[derive(Debug, PartialEq)]
struct Ticket {
    id: u32,
    title: String,
}

#[derive(Debug, PartialEq)]
enum SwapError {
    // `i == j`: there is no second ticket to swap with.
    SameIndex,
    // `i` or `j` is not a valid index into the slice.
    OutOfBounds,
}

// Exchange the titles of `tickets[i]` and `tickets[j]`, leaving every `id` in
// its original slot.
fn swap_titles(tickets: &mut [Ticket], i: usize, j: usize) -> Result<(), SwapError> {
    // TODO: rustc rejects this with E0499 "cannot borrow `tickets[_]` as mutable
    // more than once at a time": `a` is still used on the `swap` line when `b` is
    // created, and to the checker both are borrows of the same place.
    // Restructure it so no two live `&mut` can alias, and turn bad input into an
    // `Err` instead of a panic:
    //   - `i == j`                        -> `Err(SwapError::SameIndex)`
    //   - `i` or `j` is past the end      -> `Err(SwapError::OutOfBounds)`
    //   - otherwise swap the two TITLES   -> `Ok(())`   (the ids stay put)
    // An `Err` must leave the slice untouched. No `.clone()` (the tests check
    // that the `String`s are moved, not copied), no `unsafe`, no `RefCell`, and
    // don't edit the tests. Until you replace the two `&mut tickets[_]` with
    // borrows the checker can prove disjoint, this exercise will not compile.
    let a = &mut tickets[i];
    let b = &mut tickets[j];
    mem::swap(&mut a.title, &mut b.title);
    Ok(())
}

// ---- Part B — split borrows through methods -------------------------------

// Deliberately NOT `Clone` either.
#[derive(Debug, Default)]
struct Editor {
    buffer: String,
    history: Vec<String>,
}

impl Editor {
    fn buffer_mut(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn history_mut(&mut self) -> &mut Vec<String> {
        &mut self.history
    }
}

// Move whatever is in the buffer into the history as its newest entry.
fn commit(ed: &mut Editor) {
    // TODO: rustc rejects this with E0499 "cannot borrow `*ed` as mutable more
    // than once at a time": each accessor's `&mut self` borrows the WHOLE editor
    // for as long as its result lives, and `buffer` is still live when
    // `history_mut` is called. Restructure `commit` so the checker can see that
    // you touch two DIFFERENT fields. Requirement: the buffer's text becomes the
    // newest history entry — moved, not copied (the tests compare heap
    // pointers) — and the buffer is left empty. No `.clone()`, no `unsafe`, no
    // `RefCell`, and don't edit the tests. Until you stop borrowing the whole
    // editor twice at once, this exercise will not compile.
    let buffer = ed.buffer_mut();
    let history = ed.history_mut();
    history.push(mem::take(buffer));
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(id: u32, title: &str) -> Ticket {
        Ticket {
            id,
            title: title.to_string(),
        }
    }

    // A fresh board every time: `Ticket` is not `Clone`, so "the unchanged
    // board" is simply a second call to this function.
    fn board() -> Vec<Ticket> {
        vec![
            ticket(1, "Fix login"),
            ticket(2, "Write docs"),
            ticket(3, "Ship v2"),
        ]
    }

    // ---- Part A ----

    #[test]
    fn swaps_titles_and_leaves_ids_in_place() {
        let mut tickets = board();
        assert_eq!(swap_titles(&mut tickets, 0, 2), Ok(()));
        // The ids did NOT move, so a whole-element `tickets.swap(0, 2)` fails here.
        assert_eq!(
            tickets,
            vec![
                ticket(1, "Ship v2"),
                ticket(2, "Write docs"),
                ticket(3, "Fix login"),
            ]
        );
    }

    #[test]
    fn works_when_i_is_greater_than_j() {
        let mut tickets = board();
        assert_eq!(swap_titles(&mut tickets, 2, 0), Ok(()));
        assert_eq!(
            tickets,
            vec![
                ticket(1, "Ship v2"),
                ticket(2, "Write docs"),
                ticket(3, "Fix login"),
            ]
        );
    }

    #[test]
    fn works_for_neighbors_at_either_end() {
        // Adjacent indices are where an off-by-one in a `split_at_mut` solution
        // shows up: the split point must fall exactly between the two tickets.
        let mut tickets = board();
        assert_eq!(swap_titles(&mut tickets, 2, 1), Ok(()));
        assert_eq!(swap_titles(&mut tickets, 0, 1), Ok(()));
        assert_eq!(
            tickets,
            vec![
                ticket(1, "Ship v2"),
                ticket(2, "Fix login"),
                ticket(3, "Write docs"),
            ]
        );
    }

    #[test]
    fn same_index_is_rejected_and_nothing_changes() {
        let mut tickets = board();
        for i in 0..tickets.len() {
            assert_eq!(swap_titles(&mut tickets, i, i), Err(SwapError::SameIndex));
        }
        assert_eq!(tickets, board());
    }

    #[test]
    fn out_of_bounds_is_an_error_not_a_panic() {
        let mut tickets = board();
        assert_eq!(swap_titles(&mut tickets, 0, 3), Err(SwapError::OutOfBounds));
        assert_eq!(swap_titles(&mut tickets, 3, 0), Err(SwapError::OutOfBounds));
        assert_eq!(
            swap_titles(&mut tickets, 1, usize::MAX),
            Err(SwapError::OutOfBounds)
        );
        assert_eq!(tickets, board());

        // An empty slice has no valid index at all.
        let mut empty: Vec<Ticket> = Vec::new();
        assert_eq!(swap_titles(&mut empty, 0, 1), Err(SwapError::OutOfBounds));

        // Not tested on purpose: `i == j` AND out of bounds (say `5, 5`). Which
        // error wins is a design choice, and the two idiomatic fixes differ:
        // `get_disjoint_mut` checks bounds first, a hand-rolled `i == j` check
        // usually comes first.
    }

    #[test]
    fn titles_are_moved_not_copied() {
        let mut tickets = board();
        let first = tickets[0].title.as_ptr();
        let last = tickets[2].title.as_ptr();
        assert_eq!(swap_titles(&mut tickets, 0, 2), Ok(()));
        // Swapping two `String`s exchanges their headers (pointer, capacity,
        // length); the heap bytes never move. A `.clone()` would allocate new
        // buffers while the originals are still alive, so the addresses would
        // not match.
        assert_eq!(tickets[0].title.as_ptr(), last);
        assert_eq!(tickets[2].title.as_ptr(), first);
    }

    // ---- Part B ----

    #[test]
    fn commit_moves_the_buffer_into_history() {
        let mut ed = Editor::default();
        // One accessor call at a time is fine: this borrow ends with the statement.
        ed.buffer_mut().push_str("hello");
        commit(&mut ed);
        assert_eq!(ed.history, vec!["hello".to_string()]);
        assert!(ed.buffer.is_empty());
    }

    #[test]
    fn commits_keep_their_order() {
        let mut ed = Editor::default();
        ed.buffer_mut().push_str("first");
        commit(&mut ed);
        ed.buffer_mut().push_str("second");
        commit(&mut ed);
        assert_eq!(ed.history, vec!["first".to_string(), "second".to_string()]);
        assert_eq!(ed.buffer, "");
    }

    #[test]
    fn commit_moves_the_text_instead_of_copying_it() {
        let mut ed = Editor::default();
        ed.buffer_mut().push_str("draft");
        let heap = ed.buffer.as_ptr();
        commit(&mut ed);
        // Moving a `String` moves only its header. A copy (`clone`, `to_string`,
        // `collect`, ...) would sit in a new allocation at a different address.
        assert_eq!(ed.history[0].as_ptr(), heap);
        assert!(ed.buffer.is_empty());
    }
}
