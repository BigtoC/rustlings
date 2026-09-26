// Module 2 · Graphs — part 4: count islands while a neighbor iterator is alive: edition-2024 `impl Trait` capture, `+ use<>`, and `(row, col)` indexing (E0502, E0597, E0608).
//
// LeetCode 200, "Number of Islands": count the groups of land cells ('1')
// that touch up, down, left or right. The standard answer scans the grid, and
// at every land cell it counts one more island and flood-fills it, SINKING
// each of its cells to water ('0') so that it is never counted again. The
// grid itself is the visited set, at no extra cost.
//
// The neighbors come from part 3's code, now a method: `grid.neighbors(r, c)`.
// Its body copies `rows` and `cols` into locals, and its `move` closure copies
// those, so the iterator it returns holds four numbers and no reference. Yet
// `count_islands`, which writes `self.cells` while it walks the neighbors of
// a cell, is rejected with E0502 "cannot borrow `self.cells` as mutable
// because it is also borrowed as immutable", plus a note: "this call may
// capture more lifetimes than intended, because Rust 2024 has adjusted the
// `impl Trait` lifetime capture rules".
//
// A return-position `impl Trait` is an OPAQUE type: callers see its bounds
// (`Iterator<Item = (usize, usize)>`), never the real, hidden type. So the
// signature has to say what the hidden type may borrow, and the borrow
// checker trusts the signature, not the body. The rule changed with the
// edition:
//
//   - Up to edition 2021, the hidden type could use every type parameter in
//     scope, but only the lifetimes named in its bounds. This `neighbors`
//     compiled, and a body that DID borrow `self` needed `+ '_` to say so
//     (E0700 otherwise).
//   - In edition 2024 (RFC 3498), it captures EVERY generic parameter in
//     scope, lifetimes included, and that includes the anonymous lifetime of
//     `&self`. A caller must assume that the iterator borrows `*self` for as
//     long as the iterator is alive, and the `for` loop keeps it alive while
//     its body writes `self.cells`.
//
// The new default is usually right: an iterator over a collection does
// borrow it. When the hidden type borrows nothing, say so with a precise
// capturing bound (stable since Rust 1.82): `+ use<..>` lists exactly the
// generic parameters the hidden type may use. The compiler holds the body to
// that promise. If the hidden type still borrowed `self`, it would report
// E0700 "hidden type for `impl Iterator<Item = (usize, usize)>` captures
// lifetime that does not appear in bounds". (A current limit: every type
// parameter in scope must be listed, "`impl Trait` must mention all type
// parameters in scope in `use<...>`"; only lifetimes may be left out.)
// `cargo fix --edition` adds these bounds for you when it moves a crate from
// 2021 to 2024.
//
// The tests also index the grid with a pair: `grid[(r, c)]`. The expression
// `a[i]` means `*a.index(i)`, or `*a.index_mut(i)` when it is assigned to or
// borrowed mutably. Those are the `std::ops::Index` and `IndexMut` traits,
// and the index can be any type, a tuple included. The trap is the layout:
// the cells are stored row after row in one `Vec`, so `(r, c)` lives at
// `r * cols + c`, and `(0, cols)`, one column too far, lands on `(1, 0)`
// without the `Vec`'s own bounds check noticing. Each coordinate needs its
// own check.
//
// How interviewers probe it: "Count islands while mutating the grid you are
// iterating over", "What does `+ use<>` do on a return-position `impl
// Trait`, and what changed in edition 2024?", "What does `grid[(r, c)]`
// desugar to?".

const LAND: u8 = b'1';
const WATER: u8 = b'0';

// Up, down, left and right, as (row, column) offsets.
const DIRECTIONS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

// Land and water cells, stored row after row: cell `(r, c)` is
// `cells[r * cols + c]`. Deliberately not `Clone`.
#[derive(Debug, PartialEq, Eq)]
struct Grid {
    rows: usize,
    cols: usize,
    cells: Vec<u8>,
}

impl Grid {
    // One string per row, '1' for land and '0' for water.
    fn parse(lines: &[&str]) -> Grid {
        let rows = lines.len();
        let cols = lines.first().map_or(0, |line| line.len());
        let mut cells = Vec::with_capacity(rows * cols);
        for line in lines {
            assert_eq!(line.len(), cols, "every row must have the same length");
            assert!(
                line.bytes().all(|b| b == LAND || b == WATER),
                "only '0' and '1' are allowed"
            );
            cells.extend_from_slice(line.as_bytes());
        }
        Grid { rows, cols, cells }
    }

    // The cells next to `(r, c)` inside the grid (part 3's function, as a
    // method).
    fn neighbors(&self, r: usize, c: usize) -> impl Iterator<Item = (usize, usize)> {
        // TODO: `count_islands` fails with E0502 "cannot borrow `self.cells`
        // as mutable because it is also borrowed as immutable", with the
        // edition-2024 capture note. The test
        // `the_neighbor_iterator_does_not_borrow_the_grid` is rejected too:
        // E0502 for `grid.cells`, and E0597 "`small` does not live long
        // enough".
        // Change THIS signature so that callers know the returned iterator
        // borrows nothing from `self`. Requirements:
        //   - keep `neighbors(&self, r, c)` returning an `impl Iterator`, and
        //     leave `count_islands` and the tests as they are;
        //   - no collecting into a `Vec`, no copying the grid, no `unsafe`.
        // Until the returned iterator no longer borrows `self`, this exercise
        // will not compile.
        let (rows, cols) = (self.rows, self.cols);
        DIRECTIONS.into_iter().filter_map(move |(dr, dc)| {
            let nr = r.checked_add_signed(dr)?;
            let nc = c.checked_add_signed(dc)?;
            (nr < rows && nc < cols).then_some((nr, nc))
        })
    }

    // LeetCode 200: counts the islands, and sinks them all (afterwards every
    // cell is water).
    fn count_islands(&mut self) -> usize {
        // Don't change this function: rustc reports the E0502 here, but the
        // fix belongs in the signature of `neighbors`.
        let mut islands = 0;
        let mut stack = Vec::new();
        for start in 0..self.cells.len() {
            if self.cells[start] != LAND {
                continue;
            }
            islands += 1;
            // Sink the whole island. The stack is a `Vec`, not recursion: a
            // 400x400 island would take up to 160_000 nested calls.
            self.cells[start] = WATER;
            stack.push((start / self.cols, start % self.cols));
            while let Some((r, c)) = stack.pop() {
                for (nr, nc) in self.neighbors(r, c) {
                    let index = nr * self.cols + nc;
                    if self.cells[index] == LAND {
                        self.cells[index] = WATER;
                        stack.push((nr, nc));
                    }
                }
            }
        }
        islands
    }
}

// TODO: The tests read and write cells as `grid[(r, c)]`, which fails with
// E0608 "cannot index into a value of type `Grid`": nothing tells rustc what
// indexing a `Grid` with a `(usize, usize)` means. Make `grid[(r, c)]` work,
// both for reading a `u8` and for assigning one. Requirements:
//   - `grid[(r, c)]` is the cell at row `r` and column `c`, stored at
//     `cells[r * cols + c]`;
//   - panic when `r >= rows` or `c >= cols`, even when `r * cols + c` is a
//     valid index into `cells`: `(0, cols)` must not quietly reach `(1, 0)`;
//   - don't change `Grid` or the tests, no `unsafe`.
// Until indexing a `Grid` with a pair works, this exercise will not compile.

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leetcode_example_1_is_one_island() {
        let mut grid = Grid::parse(&["11110", "11010", "11000", "00000"]);
        assert_eq!(grid.count_islands(), 1);
    }

    #[test]
    fn leetcode_example_2_has_three_islands() {
        let mut grid = Grid::parse(&["11000", "11000", "00100", "00011"]);
        assert_eq!(grid.count_islands(), 3);
    }

    #[test]
    fn water_only_and_empty_grids_have_no_islands() {
        assert_eq!(Grid::parse(&["000", "000"]).count_islands(), 0);
        assert_eq!(Grid::parse(&[]).count_islands(), 0);
    }

    #[test]
    fn diagonal_cells_are_separate_islands() {
        assert_eq!(Grid::parse(&["101", "010", "101"]).count_islands(), 5);
        assert_eq!(Grid::parse(&["1001", "0110"]).count_islands(), 3);
        // A single row and a single column.
        assert_eq!(Grid::parse(&["1101011"]).count_islands(), 3);
        assert_eq!(Grid::parse(&["1", "1", "0", "1"]).count_islands(), 2);
    }

    #[test]
    fn counting_sinks_every_island() {
        let mut grid = Grid::parse(&["11000", "11000", "00100", "00011"]);
        assert_eq!(grid.count_islands(), 3);
        assert!(grid.cells.iter().all(|&cell| cell == WATER));
        assert_eq!(grid.count_islands(), 0);
    }

    #[test]
    fn large_grids_need_no_recursion() {
        // One island of 160_000 cells.
        let row = "1".repeat(400);
        let mut grid = Grid::parse(&vec![row.as_str(); 400]);
        assert_eq!(grid.count_islands(), 1);
        // Stripes: every other row is land, so 200 islands.
        let land = "1".repeat(400);
        let water = "0".repeat(400);
        let lines: Vec<&str> = (0..400)
            .map(|r| {
                if r % 2 == 0 {
                    land.as_str()
                } else {
                    water.as_str()
                }
            })
            .collect();
        assert_eq!(Grid::parse(&lines).count_islands(), 200);
    }

    #[test]
    fn neighbors_stay_inside_the_grid() {
        let grid = Grid::parse(&["000", "000", "000"]);
        let mut corner: Vec<_> = grid.neighbors(0, 0).collect();
        corner.sort_unstable();
        assert_eq!(corner, [(0, 1), (1, 0)]);
        assert_eq!(grid.neighbors(0, 1).count(), 3);
        assert_eq!(grid.neighbors(1, 1).count(), 4);
        assert_eq!(Grid::parse(&["1"]).neighbors(0, 0).count(), 0);
    }

    #[test]
    fn the_neighbor_iterator_does_not_borrow_the_grid() {
        let mut grid = Grid::parse(&["111", "111"]);
        let around = grid.neighbors(0, 1);
        // `around` is still alive while the grid changes. That is only
        // allowed if the iterator borrows nothing from `grid`.
        grid.cells.fill(WATER);
        let mut cells: Vec<_> = around.collect();
        cells.sort_unstable();
        assert_eq!(cells, [(0, 0), (0, 2), (1, 1)]);
        // It can even outlive the grid it came from.
        let around = {
            let small = Grid::parse(&["11", "11"]);
            small.neighbors(1, 1)
        };
        assert_eq!(around.count(), 2);
    }

    #[test]
    fn indexing_reads_and_writes_cells() {
        let mut grid = Grid::parse(&["100", "001"]);
        let every_cell = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)];
        assert_eq!(every_cell.map(|cell| grid[cell]), *b"100001");
        // Two writes join everything into one island.
        for cell in [(1, 1), (0, 1)] {
            grid[cell] = LAND;
        }
        assert_eq!(grid.cells, b"110011");
        assert_eq!(grid.count_islands(), 1);
        assert_eq!(every_cell.map(|cell| grid[cell]), [WATER; 6]);
    }

    #[test]
    #[should_panic]
    fn reading_one_column_past_the_end_panics() {
        let grid = Grid::parse(&["10", "01"]);
        // `(0, 2)` would be `cells[2]`, the first cell of row 1: a valid index
        // into the `Vec`, but not a cell of row 0.
        let _cell: u8 = grid[(0, grid.cols)];
    }

    #[test]
    #[should_panic]
    fn writing_one_column_past_the_end_panics() {
        let mut grid = Grid::parse(&["10", "01"]);
        let cols = grid.cols;
        grid[(0, cols)] = LAND;
    }
}
