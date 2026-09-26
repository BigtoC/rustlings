// Module 2 · Graphs — part 1: BFS distances and paths, Kahn's topological order, union-find (E0308).
//
// A graph in Rust is almost always an ADJACENCY LIST: the nodes are the
// numbers `0..n`, and `adj[u]` is a `Vec<usize>` holding every `v` with an
// edge `u -> v`. That is the arena of `59_arena` again. An edge is a plain
// number: it owns nothing and borrows nothing, so cycles cost nothing, the
// graph is `Send + Sync`, and the borrow checker never sees the links. (Nodes
// that point at each other through `Rc<RefCell<Node>>` would leak on the
// first cycle, see `26_smart_pointers_deep/smartptr2`.)
//
// This exercise runs through three classics that interviewers expect you to
// write from memory. The algorithms are the same in every language; each has
// a Rust-shaped detail:
//
// BFS (breadth-first search) reaches nodes in order of their distance from
// the source, so the first time it reaches a node it has found a shortest
// path (every edge counts 1). The queue is a `VecDeque`: `push_back` and
// `pop_front` are O(1), while `Vec::remove(0)` shifts every element left. A
// node is marked when it is QUEUED, not when it is popped, so it enters the
// queue once. "Not reached" is `None`: a `usize` cannot be `-1`, and a
// sentinel such as `usize::MAX` is one forgotten check away from an overflow.
// To return the path itself, record every node's PARENT (the node it was
// discovered from), then walk back from the destination. That walk visits
// the path backwards, so reverse it once at the end.
//
// A topological order lists the nodes so that every edge `u -> v` puts `u`
// before `v` (build steps, course prerequisites). Kahn's algorithm counts
// each node's incoming edges (its in-degree). A node with in-degree 0 is
// ready: output it, and decrement the in-degree of each node it points to,
// which may make those ready in turn. If the graph has a cycle, the nodes on
// it, and every node behind them, never become ready, so the output comes up
// short: that IS the cycle check. A DAG usually has many valid orders.
// Releasing the SMALLEST ready node every time gives the one
// lexicographically smallest order, so the answer is unique. `BinaryHeap` is
// a max-heap; `std::cmp::Reverse` turns it into a min-heap
// (`44_trait_contracts/contracts3`).
//
// Union-find (a disjoint-set union, DSU) keeps `0..n` split into disjoint
// sets. Every node points at a parent, and the root of each set points at
// itself: `find` follows the parents up to the root, `union` hangs one root
// under the other. Two tricks make both nearly O(1) amortized (the inverse
// Ackermann function, at most 4 for any `n` you will meet). PATH COMPRESSION:
// once `find` knows the root, it points every node it walked through straight
// at it. UNION BY SIZE: the smaller tree goes under the root of the bigger
// one, which keeps every tree at most log2(n) deep even without compression.
// Look at the signature, `find(&mut self, ..)`: a lookup that rewrites links
// needs exclusive access. (A `find(&self)` would need `Cell<usize>` parents,
// see `40_interior_mutability`.) The given `mst_weight` is Kruskal's
// algorithm on top of it: take the edges cheapest first, and keep each one
// that joins two different sets.
//
// How interviewers probe it: "Why a `VecDeque`?", "Return the path, not just
// its length", "Order these tasks, or say that it cannot be done", "Why does
// `find` take `&mut self`, and what do path compression and union by size buy
// you?".

// ---- Part A: breadth-first search ------------------------------------------

// The number of edges on a shortest path from `src` to each node, or `None`
// for a node that `src` cannot reach. `adj[u]` lists the targets of `u`'s
// edges.
fn bfs(adj: &[Vec<usize>], src: usize) -> Vec<Option<usize>> {
    // TODO: The empty body fails with E0308 "mismatched types": expected
    // `Vec<Option<usize>>`, found `()`. Return one entry per node: `Some(0)`
    // for `src`, `Some(d)` for a node whose shortest path from `src` has `d`
    // edges, and `None` for a node that `src` cannot reach. Requirements:
    //   - edges are directed (an edge `u -> v` is `v` in `adj[u]`), and
    //     `adj` may contain self-loops and duplicate edges;
    //   - O(V + E): every node enters the queue at most once, and taking the
    //     next node out of the queue is O(1);
    //   - no sentinel distances such as `usize::MAX`, no recursion, and don't
    //     change the tests.
    // Until you return the distances, this exercise will not compile.
}

// A shortest path from `src` to `dst`, as the list of nodes on it with both
// ends included, or `None` if `src` cannot reach `dst`.
fn shortest_path(adj: &[Vec<usize>], src: usize, dst: usize) -> Option<Vec<usize>> {
    // TODO: E0308 again: expected `Option<Vec<usize>>`, found `()`. Return
    // `[src, .., dst]` along a path with the fewest edges (if there are
    // several, any one of them will do), `Some(vec![src])` when
    // `src == dst`, and `None` when `dst` is unreachable. Requirements:
    //   - every consecutive pair on the path is an edge of `adj`, followed in
    //     its direction;
    //   - O(V + E) in total, building the path included: one test asks for a
    //     path through 100_000 nodes, so no recursion and no `insert(0, ..)`
    //     in a loop;
    //   - don't change the tests.
    // Until you return the path, this exercise will not compile.
}

// ---- Part B: topological order (Kahn's algorithm) --------------------------

// Returned when the edges contain a cycle. `unordered` counts the nodes that
// could not be placed: the nodes on a cycle, plus every node that can only be
// reached through one.
#[derive(Debug, PartialEq, Eq)]
struct CycleError {
    unordered: usize,
}

// Orders the nodes `0..n` so that every edge `(u, v)` puts `u` before `v`.
// Of all the valid orders, returns the lexicographically smallest.
fn topo_order(n: usize, edges: &[(usize, usize)]) -> Result<Vec<usize>, CycleError> {
    // TODO: E0308 again: expected `Result<Vec<usize>, CycleError>`, found
    // `()`. Use Kahn's in-degree counting, described above. Requirements:
    //   - the answer is unique: at every step, place the SMALLEST node whose
    //     predecessors have all been placed, in O((V + E) log V) overall;
    //   - a node without edges is ready from the start, a duplicate edge
    //     counts twice, and a self-loop `(u, u)` is a cycle;
    //   - a cycle gives `Err(CycleError { unordered })`, counting every node
    //     that could not be placed;
    //   - no recursion (one test orders a 200_000-node chain), and don't
    //     change the tests.
    // Until you return the order, this exercise will not compile.
}

// ---- Part C: union-find ----------------------------------------------------

// `parent[x] == x` exactly when `x` is the root of its set. `size[r]` is the
// number of nodes in the set whose root is `r`; the entries of non-roots are
// out of date and never read.
struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl Dsu {
    // `n` singleton sets: every node is its own root.
    fn new(n: usize) -> Self {
        Dsu {
            parent: (0..n).collect(),
            size: vec![1; n],
        }
    }

    // The root of the set that contains `x`.
    fn find(&mut self, x: usize) -> usize {
        // TODO: E0308: expected `usize`, found `()`. Return the root of `x`'s
        // set, and compress the path on the way. Requirements:
        //   - FULL path compression: when `find(x)` returns, every node on the
        //     path from `x` to the root (`x` included) has the root as its
        //     parent. A test checks this after a single call on a hand-built
        //     chain of 200_000 nodes;
        //   - no recursion: that chain is 200_000 levels deep;
        //   - O(length of the path), without allocating; don't change the
        //     tests.
        // Until you return the root, this exercise will not compile.
    }

    // Merges the sets of `a` and `b`. Returns `false`, and changes nothing,
    // if they were in the same set already.
    fn union(&mut self, a: usize, b: usize) -> bool {
        // TODO: E0308: expected `bool`, found `()`. Requirements:
        //   - `true` if two different sets were merged, `false` if `a` and
        //     `b` already had the same root (which includes `a == b`);
        //   - union by size: the root of the smaller set goes directly under
        //     the root of the bigger one, whatever the argument order (either
        //     way on a tie), and `size` is kept right at the surviving root;
        //   - find the roots with `find`, so the paths get compressed; don't
        //     change the tests.
        // Until you return whether two sets were merged, this exercise will
        // not compile.
    }

    // The number of nodes in `x`'s set.
    fn set_size(&mut self, x: usize) -> usize {
        let root = self.find(x);
        self.size[root]
    }
}

// Kruskal's algorithm (given): the total weight of a minimum spanning tree of
// the undirected graph on `0..n` with these `(a, b, weight)` edges, or `None`
// if the graph is not connected. It relies on `union` reporting whether it
// merged: an edge inside one set would close a cycle, so it is skipped.
fn mst_weight(n: usize, edges: &[(usize, usize, u64)]) -> Option<u64> {
    let mut by_weight = edges.to_vec();
    by_weight.sort_unstable_by_key(|&(_, _, weight)| weight);
    let mut dsu = Dsu::new(n);
    let mut total = 0;
    let mut merges = 0;
    for (a, b, weight) in by_weight {
        if dsu.union(a, b) {
            total += weight;
            merges += 1;
        }
    }
    // A spanning tree of `n` nodes has `n - 1` edges (and none for n <= 1).
    (merges + 1 >= n).then_some(total)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // An adjacency list with one directed edge per pair.
    fn directed(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
        let mut adj = vec![Vec::new(); n];
        for &(u, v) in edges {
            adj[u].push(v);
        }
        adj
    }

    // An adjacency list with an edge in each direction per pair.
    fn undirected(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
        let mut adj = vec![Vec::new(); n];
        for &(u, v) in edges {
            adj[u].push(v);
            adj[v].push(u);
        }
        adj
    }

    // 0 -> 1 -> 2 -> ... -> n-1.
    fn path_graph(n: usize) -> Vec<Vec<usize>> {
        (0..n)
            .map(|u| if u + 1 < n { vec![u + 1] } else { Vec::new() })
            .collect()
    }

    // Checks that `path` goes from `src` to `dst` along edges of `adj` and
    // has exactly `edges` edges.
    fn assert_path(adj: &[Vec<usize>], path: &[usize], src: usize, dst: usize, edges: usize) {
        assert_eq!(path.first(), Some(&src), "{path:?} should start at {src}");
        assert_eq!(path.last(), Some(&dst), "{path:?} should end at {dst}");
        assert_eq!(path.len(), edges + 1, "{path:?} should have {edges} edges");
        for pair in path.windows(2) {
            assert!(
                adj[pair[0]].contains(&pair[1]),
                "{path:?} uses {} -> {}, which is not an edge",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn bfs_distances_on_an_undirected_graph() {
        //   0 - 1 - 2
        //   |       |
        //   3 - 4 - 5 - 6
        let adj = undirected(7, &[(0, 1), (1, 2), (0, 3), (3, 4), (4, 5), (2, 5), (5, 6)]);
        assert_eq!(bfs(&adj, 0), [0, 1, 2, 1, 2, 3, 4].map(Some));
        assert_eq!(bfs(&adj, 6), [4, 3, 2, 3, 2, 1, 0].map(Some));
        assert_eq!(bfs(&adj, 4), [2, 3, 2, 1, 0, 1, 2].map(Some));
    }

    #[test]
    fn bfs_follows_edge_directions_and_unreachable_is_none() {
        // 3 -> 0 -> 1 -> 2, and 4 on its own.
        let adj = directed(5, &[(0, 1), (1, 2), (3, 0)]);
        assert_eq!(bfs(&adj, 0), [Some(0), Some(1), Some(2), None, None]);
        assert_eq!(bfs(&adj, 3), [Some(1), Some(2), Some(3), Some(0), None]);
        assert_eq!(bfs(&adj, 4), [None, None, None, None, Some(0)]);
    }

    #[test]
    fn bfs_counts_the_fewest_edges_not_the_first_route_found() {
        // A depth-first walk from 0 follows 0 -> 1 -> 2 -> 3 -> 4 before it
        // ever tries the direct edge 0 -> 4.
        let mut adj = directed(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (0, 4)]);
        assert_eq!(bfs(&adj, 0), [0, 1, 2, 3, 1].map(Some));
        // The same trap for a walk that takes the LAST edge first, as a stack
        // does: 0 -> 5 -> 4 is the short way, 0 -> 1 -> 2 -> 3 -> 4 the long
        // one.
        let lifo_trap = directed(6, &[(0, 5), (0, 1), (1, 2), (2, 3), (3, 4), (5, 4)]);
        assert_eq!(bfs(&lifo_trap, 0), [0, 1, 2, 3, 2, 1].map(Some));
        // Self-loops and duplicate edges change nothing.
        adj[0].push(1);
        adj[2].push(2);
        adj[4].push(4);
        adj[3].push(4);
        assert_eq!(bfs(&adj, 0), [0, 1, 2, 3, 1].map(Some));
    }

    #[test]
    fn shortest_paths_across_a_grid_are_valid_and_shortest() {
        // A 4x4 grid of nodes (node = 4 * row + col) with edges between
        // horizontal and vertical neighbors: there are many shortest paths
        // between two corners, and any of them is accepted.
        let mut edges = Vec::new();
        for row in 0..4 {
            for col in 0..4 {
                let node = 4 * row + col;
                if col < 3 {
                    edges.push((node, node + 1));
                }
                if row < 3 {
                    edges.push((node, node + 4));
                }
            }
        }
        let adj = undirected(16, &edges);
        for dst in 0..16 {
            let path = shortest_path(&adj, 0, dst).expect("the grid is connected");
            // From the corner (0, 0), a shortest path has row + col edges.
            assert_path(&adj, &path, 0, dst, dst / 4 + dst % 4);
        }
        let path = shortest_path(&adj, 15, 0).expect("the grid is connected");
        assert_path(&adj, &path, 15, 0, 6);
    }

    #[test]
    fn shortest_path_takes_the_short_way() {
        // 0 -> 1 -> 2 -> 3 -> 4 and 0 -> 5 -> 4: exactly one shortest path.
        let adj = directed(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (0, 5), (5, 4)]);
        assert_eq!(shortest_path(&adj, 0, 4), Some(vec![0, 5, 4]));
        assert_eq!(shortest_path(&adj, 1, 4), Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn a_path_from_a_node_to_itself_is_that_node() {
        let adj = directed(3, &[(0, 1), (1, 1), (1, 2), (2, 0)]);
        assert_eq!(shortest_path(&adj, 0, 0), Some(vec![0]));
        assert_eq!(shortest_path(&adj, 1, 1), Some(vec![1]));
    }

    #[test]
    fn an_unreachable_destination_has_no_path() {
        // 1 -> 0 and 2 -> 3: node 0 has no outgoing edges.
        let adj = directed(4, &[(1, 0), (2, 3)]);
        assert_eq!(shortest_path(&adj, 0, 1), None);
        assert_eq!(shortest_path(&adj, 0, 3), None);
        assert_eq!(shortest_path(&adj, 1, 3), None);
        assert_eq!(shortest_path(&adj, 1, 0), Some(vec![1, 0]));
    }

    #[test]
    fn a_path_through_100_000_nodes() {
        const N: usize = 100_000;
        let adj = path_graph(N);
        let dist = bfs(&adj, 0);
        assert_eq!(dist[N - 1], Some(N - 1));
        assert!(dist.iter().copied().eq((0..N).map(Some)));
        let path = shortest_path(&adj, 0, N - 1).expect("the last node is reachable");
        assert!(
            path.iter().copied().eq(0..N),
            "the path should be 0, 1, 2, .."
        );
        assert_eq!(shortest_path(&adj, N - 1, 0), None);
    }

    #[test]
    fn topo_order_is_the_lexicographically_smallest() {
        // 5 -> 2 -> 3 -> 1, 5 -> 0, 4 -> 0, 4 -> 1. Many orders are valid,
        // such as [5, 4, 2, 3, 1, 0] or [4, 5, 2, 0, 3, 1] (a plain FIFO
        // queue gives the latter); only one is the smallest.
        let edges = [(5, 2), (5, 0), (4, 0), (4, 1), (2, 3), (3, 1)];
        assert_eq!(topo_order(6, &edges), Ok(vec![4, 5, 0, 2, 3, 1]));
        // A FIFO queue gives [1, 2, 3, 0] here.
        assert_eq!(topo_order(4, &[(2, 0), (1, 3)]), Ok(vec![1, 2, 0, 3]));
    }

    #[test]
    fn nodes_without_edges_take_their_place_in_the_order() {
        // Nodes 0 and 6 have no edges at all.
        let edges = [(5, 2), (5, 1), (2, 3), (4, 3)];
        assert_eq!(topo_order(7, &edges), Ok(vec![0, 4, 5, 1, 2, 3, 6]));
        assert_eq!(topo_order(3, &[]), Ok(vec![0, 1, 2]));
        assert_eq!(topo_order(0, &[]), Ok(vec![]));
    }

    #[test]
    fn duplicate_edges_count_twice() {
        assert_eq!(topo_order(3, &[(2, 0), (2, 0), (1, 0)]), Ok(vec![1, 2, 0]));
    }

    #[test]
    fn a_three_cycle_is_an_error() {
        let edges = [(0, 1), (1, 2), (2, 0)];
        assert_eq!(topo_order(3, &edges), Err(CycleError { unordered: 3 }));
    }

    #[test]
    fn nodes_behind_a_cycle_cannot_be_placed_either() {
        // 0 -> 1 <-> 2 -> 3, and 4 -> 5 on the side: 0, 4 and 5 can be
        // placed, 1 and 2 are on the cycle, and 3 waits for 2 forever.
        let edges = [(0, 1), (1, 2), (2, 1), (2, 3), (4, 5)];
        assert_eq!(topo_order(6, &edges), Err(CycleError { unordered: 3 }));
        // A self-loop is a cycle of one node.
        assert_eq!(
            topo_order(3, &[(0, 1), (1, 1)]),
            Err(CycleError { unordered: 1 })
        );
    }

    #[test]
    fn a_200_000_node_chain() {
        const N: usize = 200_000;
        // The edges point downwards, (i + 1) -> i, so the only order is
        // N - 1, N - 2, .., 0.
        let mut edges: Vec<(usize, usize)> = (0..N - 1).map(|i| (i + 1, i)).collect();
        let order = topo_order(N, &edges).expect("a chain has no cycle");
        assert!(order.iter().copied().eq((0..N).rev()));
        // Closing the chain into one big cycle leaves every node unordered.
        edges.push((0, N - 1));
        assert_eq!(topo_order(N, &edges), Err(CycleError { unordered: N }));
    }

    #[test]
    fn union_reports_whether_it_merged() {
        let mut dsu = Dsu::new(6);
        assert!(dsu.union(0, 1));
        assert!(!dsu.union(1, 0), "0 and 1 are already in one set");
        assert!(!dsu.union(2, 2), "a node is always in its own set");
        assert!(dsu.union(2, 3));
        assert!(dsu.union(1, 3));
        // One root went under the other, so either 1 or 3 is now two links
        // below the root: `parent[x]` is not always the root, `find` is.
        assert!(!dsu.union(3, 0));
        assert!(!dsu.union(1, 2));
        assert!(!dsu.union(0, 2));
        assert_eq!(dsu.find(0), dsu.find(3));
        assert_ne!(dsu.find(0), dsu.find(4));
        assert_eq!(dsu.find(4), 4, "an untouched node is its own root");
        assert_eq!(
            [0, 1, 2, 3, 4, 5].map(|x| dsu.set_size(x)),
            [4, 4, 4, 4, 1, 1]
        );
    }

    #[test]
    fn union_by_size_keeps_the_bigger_root() {
        let mut dsu = Dsu::new(8);
        for x in 1..4 {
            assert!(dsu.union(0, x));
        }
        let big = dsu.find(0);
        // The singleton goes directly under the big set's root, whichever
        // argument it is.
        assert!(dsu.union(4, 0));
        assert_eq!(dsu.parent[4], big, "4 should hang directly under the root");
        assert!(dsu.union(2, 5));
        assert_eq!(dsu.parent[5], big, "5 should hang directly under the root");
        assert_eq!(dsu.set_size(5), 6);
        // A set of 2 joins the set of 6: the root of the 6 survives.
        assert!(dsu.union(6, 7));
        assert!(dsu.union(7, 1));
        assert_eq!(dsu.find(6), big);
        assert_eq!(dsu.find(7), big);
        assert_eq!(dsu.set_size(6), 8);
    }

    #[test]
    fn find_compresses_the_whole_path_in_one_call() {
        // Built by hand, since union by size never makes one: the chain
        // 199_999 -> 199_998 -> .. -> 1 -> 0.
        const N: usize = 200_000;
        let mut dsu = Dsu {
            parent: (0..N).map(|x| x.saturating_sub(1)).collect(),
            size: vec![1; N],
        };
        dsu.size[0] = N;
        assert_eq!(dsu.find(N - 1), 0);
        assert!(
            dsu.parent.iter().all(|&p| p == 0),
            "after one find, every node on the path should point at the root"
        );
        assert_eq!(dsu.find(N / 2), 0);
        assert_eq!(dsu.set_size(12_345), N);
    }

    #[test]
    fn unions_in_a_line_keep_every_tree_shallow() {
        const N: usize = 1 << 12;
        // How many parent links lead from `x` to its root, read straight from
        // the fields (no compression).
        fn depth(dsu: &Dsu, mut x: usize) -> usize {
            let mut hops = 0;
            while dsu.parent[x] != x {
                x = dsu.parent[x];
                hops += 1;
            }
            hops
        }
        // Join 0-1, 1-2, 2-3, .. in both argument orders. Hanging one root
        // under the other without looking at the sizes makes a chain in one
        // of the two orders.
        for flipped in [false, true] {
            let mut dsu = Dsu::new(N);
            for i in 0..N - 1 {
                let merged = if flipped {
                    dsu.union(i + 1, i)
                } else {
                    dsu.union(i, i + 1)
                };
                assert!(merged);
            }
            let deepest = (0..N).map(|x| depth(&dsu, x)).max();
            assert!(
                deepest <= Some(12),
                "union by size keeps trees at most log2(n) = 12 deep, got {deepest:?} (flipped: {flipped})"
            );
            assert_eq!(dsu.set_size(0), N);
        }
    }

    #[test]
    fn kruskal_finds_the_minimum_spanning_tree_weight() {
        // The cheapest tree uses 1-2 (1), 1-3 (2), 3-4 (2) and 0-2 (3).
        let edges = [
            (0, 1, 4),
            (0, 2, 3),
            (1, 2, 1),
            (1, 3, 2),
            (2, 3, 4),
            (3, 4, 2),
            (4, 0, 7),
        ];
        assert_eq!(mst_weight(5, &edges), Some(8));
        // Of two parallel edges, the cheaper one is used.
        assert_eq!(mst_weight(2, &[(0, 1, 5), (1, 0, 2)]), Some(2));
        assert_eq!(mst_weight(1, &[]), Some(0));
    }

    #[test]
    fn kruskal_reports_a_disconnected_graph() {
        assert_eq!(mst_weight(4, &[(0, 1, 1), (2, 3, 1)]), None);
        // A self-loop joins nothing: node 2 is still on its own.
        assert_eq!(mst_weight(3, &[(0, 1, 5), (2, 2, 1)]), None);
    }
}
