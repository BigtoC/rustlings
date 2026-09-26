// Module 2 · Graphs — part 3: grid neighbors that cannot underflow at row or column 0 (it compiles; the corner tests panic).
//
// Grids are the other half of graph questions: islands, mazes, flood fill,
// rotting oranges. A grid is a graph whose edges are implicit: the neighbors
// of cell `(r, c)` are the cells one step up, down, left and right that lie
// inside the grid. Nobody builds an adjacency list for it; you compute the
// neighbors of a cell when you need them.
//
// In Python or Java the neighbor function is `(r - 1, c)` and friends plus a
// bounds check afterwards: a row of `-1` is simply out of range. In Rust,
// grid coordinates are `usize` (they index a `Vec`), and a `usize` cannot be
// negative. At row 0, `r - 1` does not produce a value to check. In debug
// builds and tests, where overflow checks are on, it PANICS with "attempt to
// subtract with overflow". In release builds the checks are off, it wraps to
// `usize::MAX`, and only then fails the bounds check, by accident. The given
// `neighbors` has exactly that bug, and a search that starts in the top-left
// corner, as most of them do, hits it on its first step. (`63_slices_strings/
// window1` meets the same underflow as `len() - 1` on an empty slice.)
//
// The fix is to never compute a coordinate that might not exist:
//
//   - `usize::checked_add_signed` (stable since Rust 1.66) adds an `isize`
//     offset to a `usize`, and returns `None` instead of going below 0 or
//     above `usize::MAX`. Over a table of `(isize, isize)` direction pairs,
//     one expression covers all four directions (or all eight, with the
//     diagonals).
//   - `bool::then_some` (stable since 1.62) turns the bounds check into an
//     `Option`: `(nr < rows && nc < cols).then_some((nr, nc))`. With `?` on
//     the checked additions, a `filter_map` closure then drops every
//     direction that leaves the grid.
//   - Other answers interviewers accept: `r.checked_sub(1)`, an explicit
//     `if r > 0` for each direction, or `r.wrapping_sub(1)` followed by the
//     `< rows` check (the wrapped value, `usize::MAX`, fails it). The one to
//     avoid is casting to `i32` or `isize` and back: an `as` cast truncates or
//     wraps without a word, and nothing limits a grid to `i32::MAX` rows.
//
// `neighbors` returns `impl Iterator<Item = (usize, usize)>`: a lazy
// iterator, with no `Vec` allocated for every cell. Its `move` closure copies
// four `usize`s (`r`, `c`, `rows`, `cols`), so the iterator borrows nothing.
// Remember that for part 4, where the same code becomes a method on `&self`.
// The given `min_steps` is a grid BFS built on it: part 1's BFS, with the
// neighbors computed instead of stored.
//
// How interviewers probe it: "What does your neighbor function do at
// (0, 0)?", "And in a release build?", "Now add the diagonals".

use std::collections::VecDeque;

// The cells next to `(r, c)` (up, down, left and right) that lie inside a
// grid of `rows` x `cols` cells, in any order. `(r, c)` itself must be inside
// the grid.
fn neighbors(r: usize, c: usize, rows: usize, cols: usize) -> impl Iterator<Item = (usize, usize)> {
    // TODO: `corner_cells_have_two_neighbors` and every other test that
    // touches row 0 or column 0 panic with "attempt to subtract with
    // overflow": at `r == 0`, `r - 1` has no `usize` value. Return the
    // in-bounds neighbors without ever computing a coordinate below 0.
    // Requirements:
    //   - a corner cell has 2 neighbors, an edge cell 3, an inner cell 4, and
    //     the only cell of a 1x1 grid none;
    //   - never the cell itself and never the same neighbor twice (clamping
    //     with `saturating_sub` gets both wrong);
    //   - correct for any `usize` coordinates, grids with more than
    //     `i32::MAX` or `isize::MAX` rows included (one test uses them; the
    //     function never allocates the grid): no `as` casts;
    //   - keep the signature, and don't change the tests.
    // Until no coordinate can go below 0, the tests will fail.
    [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)]
        .into_iter()
        .filter(move |&(nr, nc)| nr < rows && nc < cols)
}

// The fewest steps from `start` to `goal` through open cells ('.'), moving up,
// down, left or right, or `None` if `goal` cannot be reached. Walls are '#'.
// All rows have the same length, and `start` is inside the maze.
fn min_steps(maze: &[&str], start: (usize, usize), goal: (usize, usize)) -> Option<usize> {
    let rows = maze.len();
    let cols = maze.first().map_or(0, |row| row.len());
    let is_open = |(r, c): (usize, usize)| maze[r].as_bytes()[c] == b'.';
    // One flag per cell, row after row; a cell is marked when it is queued.
    let mut seen = vec![false; rows * cols];
    seen[start.0 * cols + start.1] = true;
    let mut queue = VecDeque::from([(start, 0)]);
    while let Some((cell, steps)) = queue.pop_front() {
        if cell == goal {
            return Some(steps);
        }
        for next in neighbors(cell.0, cell.1, rows, cols) {
            let index = next.0 * cols + next.1;
            if !seen[index] && is_open(next) {
                seen[index] = true;
                queue.push_back((next, steps + 1));
            }
        }
    }
    None
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // The neighbors, sorted, so the order `neighbors` yields them in does not
    // matter.
    fn sorted(cells: impl Iterator<Item = (usize, usize)>) -> Vec<(usize, usize)> {
        let mut cells: Vec<_> = cells.collect();
        cells.sort_unstable();
        cells
    }

    #[test]
    fn an_inner_cell_has_four_neighbors() {
        assert_eq!(
            sorted(neighbors(1, 1, 3, 3)),
            [(0, 1), (1, 0), (1, 2), (2, 1)]
        );
        assert_eq!(
            sorted(neighbors(1, 2, 3, 4)),
            [(0, 2), (1, 1), (1, 3), (2, 2)]
        );
    }

    #[test]
    fn corner_cells_have_two_neighbors() {
        // A grid of 3 rows and 4 columns.
        assert_eq!(sorted(neighbors(0, 0, 3, 4)), [(0, 1), (1, 0)]);
        assert_eq!(sorted(neighbors(0, 3, 3, 4)), [(0, 2), (1, 3)]);
        assert_eq!(sorted(neighbors(2, 0, 3, 4)), [(1, 0), (2, 1)]);
        assert_eq!(sorted(neighbors(2, 3, 3, 4)), [(1, 3), (2, 2)]);
    }

    #[test]
    fn edge_cells_have_three_neighbors() {
        assert_eq!(sorted(neighbors(0, 1, 3, 4)), [(0, 0), (0, 2), (1, 1)]);
        assert_eq!(sorted(neighbors(1, 0, 3, 4)), [(0, 0), (1, 1), (2, 0)]);
        assert_eq!(sorted(neighbors(2, 2, 3, 4)), [(1, 2), (2, 1), (2, 3)]);
        assert_eq!(sorted(neighbors(1, 3, 3, 4)), [(0, 3), (1, 2), (2, 3)]);
    }

    #[test]
    fn the_only_cell_of_a_1x1_grid_has_no_neighbors() {
        assert_eq!(neighbors(0, 0, 1, 1).count(), 0);
    }

    #[test]
    fn a_single_row_and_a_single_column() {
        assert_eq!(sorted(neighbors(0, 0, 1, 5)), [(0, 1)]);
        assert_eq!(sorted(neighbors(0, 2, 1, 5)), [(0, 1), (0, 3)]);
        assert_eq!(sorted(neighbors(0, 4, 1, 5)), [(0, 3)]);
        assert_eq!(sorted(neighbors(0, 0, 5, 1)), [(1, 0)]);
        assert_eq!(sorted(neighbors(3, 0, 5, 1)), [(2, 0), (4, 0)]);
        assert_eq!(sorted(neighbors(4, 0, 5, 1)), [(3, 0)]);
    }

    #[test]
    fn every_cell_matches_a_brute_force_scan() {
        let shapes: [(usize, usize); 6] = [(1, 1), (1, 4), (4, 1), (2, 2), (4, 5), (6, 3)];
        for (rows, cols) in shapes {
            for r in 0..rows {
                for c in 0..cols {
                    // Every cell of the grid at distance exactly 1: never the
                    // cell itself, and each neighbor once.
                    let expected: Vec<(usize, usize)> = (0..rows)
                        .flat_map(|nr| (0..cols).map(move |nc| (nr, nc)))
                        .filter(|&(nr, nc)| nr.abs_diff(r) + nc.abs_diff(c) == 1)
                        .collect();
                    assert_eq!(
                        sorted(neighbors(r, c, rows, cols)),
                        expected,
                        "neighbors({r}, {c}) in a {rows}x{cols} grid"
                    );
                }
            }
        }
    }

    #[test]
    fn coordinates_beyond_i32_and_isize_work() {
        // `neighbors` never allocates the grid, so it may be as large as
        // `usize` allows.
        let big = 3_000_000_000; // more than `i32::MAX`
        assert_eq!(
            sorted(neighbors(big, 7, 4_000_000_000, 10)),
            [(big - 1, 7), (big, 6), (big, 8), (big + 1, 7)]
        );
        let past_isize = usize::MAX / 2 + 1; // one more than `isize::MAX`
        assert_eq!(
            sorted(neighbors(past_isize, 0, usize::MAX, 1)),
            [(past_isize - 1, 0), (past_isize + 1, 0)]
        );
        let max = usize::MAX;
        assert_eq!(
            sorted(neighbors(max - 1, max - 1, max, max)),
            [(max - 2, max - 1), (max - 1, max - 2)]
        );
    }

    #[test]
    fn a_maze_solved_from_the_top_left_corner() {
        let maze = [
            ".#.....", //
            ".#.###.", //
            "...#...", //
            "##.#.#.", //
            "....#..", //
        ];
        assert_eq!(min_steps(&maze, (0, 0), (4, 6)), Some(14));
        assert_eq!(min_steps(&maze, (4, 6), (0, 0)), Some(14));
        assert_eq!(min_steps(&maze, (0, 0), (4, 0)), Some(8));
        assert_eq!(min_steps(&maze, (0, 0), (0, 0)), Some(0));
    }

    #[test]
    fn a_walled_off_goal_cannot_be_reached() {
        let maze = [
            "..#.", //
            "..#.", //
            "###.", //
            "....", //
        ];
        assert_eq!(min_steps(&maze, (0, 0), (3, 3)), None);
        assert_eq!(min_steps(&maze, (3, 0), (0, 3)), Some(6));
        assert_eq!(min_steps(&maze, (0, 0), (1, 1)), Some(2));
    }
}
