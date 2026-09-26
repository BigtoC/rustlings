# Module 2 · Graphs: BFS, Iterative DFS, Topological Order, Union-Find, Grids

> Module 2 (data structures), after `61_trees`. Graph and grid problems are
> the most common category in algorithm rounds. The algorithms themselves
> are the same in every language, so this module keeps them short and
> drills what is different in Rust: an adjacency list is the arena of
> `59_arena` (nodes are indices), a recursive `dfs(&mut self)` does not
> compile, deep recursion aborts the process, a `usize` coordinate cannot
> step to -1, and edition 2024 changed what a returned `impl Iterator`
> borrows. All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error (or the
> failing test) and the requirements but **not** the fix, as in an
> interview. Press `h` when you want the full answer.

## Core Ideas

- **Nodes are indices.** A graph is `Vec<Vec<usize>>`: `adj[u]` lists the
  targets of `u`'s edges. An edge is a `Copy` number that owns and borrows
  nothing, so cycles are free, the graph is `Send + Sync`, and there is
  nothing to leak (compare `Rc<RefCell<Node>>` in
  `26_smart_pointers_deep/smartptr2`). This is the arena of `59_arena`
  without removal.
- **A `&mut self` call borrows every field.** A recursive `dfs(&mut self, u)`
  that loops over `&self.adj[u]` is E0502, even though it only writes
  `self.seen`: the checker trusts the signature, not the body
  (`37_borrowck_errors/borrowck1`, part B). Touching the fields directly is a
  split borrow and compiles.
- **Depth again.** A graph can be one long path, the worst case of the
  sorted BST in `61_trees`: a recursive walk needs one stack frame per node,
  and in a debug build a recursive DFS overflows a test thread's 2 MiB stack
  within a few tens of thousands of levels, which aborts the whole run. An
  explicit stack is a `Vec` on the heap.
- **Work after the loop needs frames.** A recursive DFS does something
  when a node is finished (color it black, emit it in post-order). A stack
  of bare nodes loses that moment; a stack of `(node, next_edge_index)`
  frames keeps it, because that pair is what each recursive call stored.
- **`usize` does not go below 0.** `r - 1` at row 0 panics in debug builds
  and tests, and wraps in release. `checked_add_signed` over a table of
  `(isize, isize)` directions, plus `then_some` for the bounds, never
  computes a coordinate that does not exist.
- **The signature decides what a returned `impl Trait` borrows.** In
  edition 2024 a return-position `impl Trait` captures every generic
  parameter in scope, including the lifetime of `&self`. `+ use<>` says the
  hidden type captures none of them.

## Three Ways Out of the Recursive-DFS E0502

| Fix                                                                | Compiles | 200_000-node path | Cost                          |
| ------------------------------------------------------------------ | -------- | ----------------- | ----------------------------- |
| Clone the neighbor list (`self.adj[u].clone()`) and recurse        | yes      | stack overflow    | an allocation per node        |
| Split borrow: `fn visit(adj: &[Vec<usize>], seen: &mut [bool], u)` | yes      | stack overflow    | none; fine for shallow graphs |
| Iterative: an explicit stack, fields used directly in the loop     | yes      | fine              | one `Vec`                     |

Taking the list out with `std::mem::take(&mut self.adj[u])` and putting it
back after the loop is a fourth way to end the borrow of `self`. It
compiles and copies nothing, and the recursion still overflows.

## Directed Cycles: Why Three Colors

| Approach                                                         | Diamond `0→1→3`, `0→2→3` | `0→1`, `0→2`, `2→1` | `0→1→2→0` |
| ---------------------------------------------------------------- | ------------------------ | ------------------- | --------- |
| "Seen before" means a cycle (two colors)                         | false positive           | false positive      | found     |
| Gray when pushed, black once its edges are pushed                | correct                  | false positive      | missed    |
| Gray on entry, black when its frame runs out of edges            | correct                  | correct             | found     |
| Kahn's algorithm (`graph1`): some node never reaches in-degree 0 | correct                  | correct             | found     |

For an **undirected** graph, every edge is two directed edges, so the
three-color test would report each edge as a cycle. There, a cycle is an
edge to an already visited node other than the parent you came from (or use
union-find: an edge whose ends already share a root closes a cycle,
LeetCode 684).

## Edition 2024 and `+ use<>`

| Return type                             | Edition 2021 captures                   | Edition 2024 captures                                          |
| --------------------------------------- | --------------------------------------- | -------------------------------------------------------------- |
| `impl Iterator<Item = u8>`              | the type parameters only                | the type parameters and every lifetime in scope, `&self`'s too |
| `impl Iterator<Item = u8> + '_`         | the type parameters and the elided `'_` | the same as without `+ '_`                                     |
| `impl Iterator<Item = u8> + use<>`      | nothing                                 | nothing                                                        |
| `impl Iterator<Item = u8> + use<'a, T>` | exactly `'a` and `T`                    | exactly `'a` and `T`                                           |

Precise capturing (`use<..>`) is stable since Rust 1.82, in every edition.

- The E0502 that `grid2` starts with comes with the note "this call may
  capture more lifetimes than intended, because Rust 2024 has adjusted the
  `impl Trait` lifetime capture rules". In edition 2021 the same code
  compiles.
- `use<..>` is a promise the compiler checks. If the hidden type does borrow
  `self` (say the closure reads `self.cols` instead of a copy), `+ use<>` is
  E0700 "hidden type for `impl Iterator<..>` captures lifetime that does not
  appear in bounds", the same error edition 2021 gave for a missing `+ '_`.
- Today every type parameter in scope must be listed: `use<>` on a generic
  function is an error, "`impl Trait` must mention all type parameters in
  scope in `use<...>`". Only lifetimes can be left out.
- `+ 'static` also compiles in `grid2`: an iterator that outlives every
  lifetime cannot be holding a borrow of `self`. `use<>` says exactly which
  parameters are captured, and it is what rustc's help line suggests.
- `cargo fix --edition` inserts `use<..>` bounds for you (the
  `impl_trait_overcaptures` lint), so migrated code keeps its 2021 meaning.

## Exercise Path

1. **graph1** — Five empty bodies (E0308), three classics. `bfs` returns
   `Vec<Option<usize>>` distances from a `VecDeque`, marking nodes when they
   are queued; `shortest_path` records parents and walks back with a loop.
   `topo_order` is Kahn's algorithm with a `BinaryHeap<Reverse<usize>>`, so
   the order is the lexicographically smallest, and a cycle is an `Err` that
   counts the nodes left over. `Dsu::find` compresses the whole path in two
   loops (a 200_000-node chain rules out recursion), and `union` goes by
   size and reports whether it merged, which the given Kruskal `mst_weight`
   relies on.
2. **graph2** — The centerpiece. A recursive `dfs(&mut self)` iterating
   `&self.adj[u]` is E0502 "cannot borrow `*self` as mutable because it is
   also borrowed as immutable": make it iterative, using the fields directly
   (a 200_000-node path rules out the split-borrow recursion too).
   `has_cycle_directed` is empty (E0308): three colors, with a stack of
   `(node, next_edge_index)` frames so a node turns black only when it is
   finished. A brute-force check on 3_000 random graphs catches the
   shortcuts.
3. **grid1** — The given `neighbors(r, c, rows, cols)` builds `(r - 1, c)`,
   and every test that touches row 0 or column 0 panics with "attempt to
   subtract with overflow". Rewrite it with `checked_add_signed` and
   `then_some`. A cell is never its own neighbor (so no `saturating_sub`),
   and coordinates past `i32::MAX` and `isize::MAX` work (so no `as` casts).
   The given `min_steps` is a grid BFS that starts in a corner.
4. **grid2** — Number of islands (LeetCode 200). `count_islands` sinks land
   while it iterates `self.neighbors(r, c)`, and the edition-2024 capture
   rules make that E0502 (plus E0502 and E0597 in a test that mutates and
   drops the grid while an iterator is alive): add `+ use<>`. Then
   `grid[(r, c)]` is E0608 until you implement `Index<(usize, usize)>` and
   `IndexMut`, with a check per axis so `(0, cols)` panics instead of
   reaching `(1, 0)`.

Related: `61_trees` walks a tree with an explicit stack too (`bst1..4`),
`44_trait_contracts/contracts3` builds a min-heap with `Reverse` for
Dijkstra, and `63_slices_strings/window1` meets `usize` underflow again as
`len() - 1` on an empty slice. The overflow rules themselves are in
`31_debugging/debugging2`.

## Further Reading

- [Modeling graphs in Rust using vector indices](https://smallcultfollowing.com/babysteps/blog/2015/04/06/modeling-graphs-in-rust-using-vector-indices/) (Niko Matsakis)
- [`petgraph`](https://docs.rs/petgraph): the graph crate, built on index-based adjacency lists
- [`VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html), [`BinaryHeap` as a min-heap](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html#min-heap) and [`std::cmp::Reverse`](https://doc.rust-lang.org/std/cmp/struct.Reverse.html)
- [`usize::checked_add_signed`](https://doc.rust-lang.org/std/primitive.usize.html#method.checked_add_signed) and [`bool::then_some`](https://doc.rust-lang.org/std/primitive.bool.html#method.then_some)
- [`std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) and [`IndexMut`](https://doc.rust-lang.org/std/ops/trait.IndexMut.html)
- [Thread stack size](https://doc.rust-lang.org/std/thread/index.html#stack-size) in the `std::thread` docs
- [Changes to `impl Trait` in Rust 2024](https://blog.rust-lang.org/2024/09/05/impl-trait-capture-rules.html) (Rust blog) and the edition guide's [RPIT lifetime capture rules](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html)
- The Reference on [`impl Trait` capturing](https://doc.rust-lang.org/reference/types/impl-trait.html#capturing) and [precise capturing](https://doc.rust-lang.org/reference/types/impl-trait.html#precise-capturing)
- [RFC 3498: lifetime capture rules 2024](https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html) and [RFC 3617: precise capturing](https://rust-lang.github.io/rfcs/3617-precise-capturing.html)
- [Disjoint-set data structure](https://en.wikipedia.org/wiki/Disjoint-set_data_structure) (Wikipedia): path compression, union by size and rank, and the inverse Ackermann bound
- LeetCode [200. Number of Islands](https://leetcode.com/problems/number-of-islands/), [210. Course Schedule II](https://leetcode.com/problems/course-schedule-ii/), [323. Number of Connected Components in an Undirected Graph](https://leetcode.com/problems/number-of-connected-components-in-an-undirected-graph/), [684. Redundant Connection](https://leetcode.com/problems/redundant-connection/), [1091. Shortest Path in Binary Matrix](https://leetcode.com/problems/shortest-path-in-binary-matrix/) and [1584. Min Cost to Connect All Points](https://leetcode.com/problems/min-cost-to-connect-all-points/)
