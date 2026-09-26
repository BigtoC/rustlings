// Module 2 · Graphs — part 2: a recursive `dfs(&mut self)` over `&self.adj[u]`, made iterative, and three-color cycle detection (E0502, E0308).
//
// This is the graph question with a Rust twist. Everyone writes depth-first
// search (DFS) recursively, as a method:
//
//     fn dfs(&mut self, u: usize) {
//         self.seen[u] = true;
//         for &v in &self.adj[u] {
//             if !self.seen[v] {
//                 self.dfs(v);
//             }
//         }
//     }
//
// and rustc rejects it with E0502 "cannot borrow `*self` as mutable because
// it is also borrowed as immutable". The `for` loop holds a shared borrow of
// `self.adj[u]`, which borrows the field `self.adj`, for as long as the loop
// runs. The recursive call needs `&mut self`: an exclusive borrow of ALL of
// `*self`, `adj` included. That the body only ever writes `self.seen` does
// not help, because the borrow checker works one function at a time and
// trusts signatures: calling a `&mut self` method borrows every field
// (`37_borrowck_errors/borrowck1`, part B). And the rule protects something
// real. A `&mut self` method COULD push onto `self.adj[u]`, make the `Vec`
// reallocate, and leave the loop reading freed memory: C++'s iterator
// invalidation, ruled out at compile time.
//
// There are three ways out, and an interviewer wants to hear why you pick
// one:
//
//   1. Copy the neighbor list first (`self.adj[u].clone()`, `to_vec()`, or
//      collecting it). It compiles, and it is the wrong lesson: one
//      allocation per node, to hide a design problem.
//   2. Split the borrow. Move the recursion into a function that takes the
//      fields separately, `fn visit(adj: &[Vec<usize>], seen: &mut [bool],
//      u: usize)`. Two DIFFERENT fields may be borrowed in different ways at
//      the same time, so this compiles and copies nothing. It is the right
//      answer when the recursion is known to be shallow.
//   3. Drop the recursion. Keep the nodes still to visit on your own stack, a
//      `Vec` on the heap, and in the loop use the fields directly: `self.adj`
//      and `self.seen` are disjoint places, so a shared borrow of one and a
//      write to the other do not conflict, as long as no `&mut self` method
//      is called inside the loop.
//
// Why not 2 here? Depth. A graph can be one long path, just as a BST fed
// sorted keys is one long spine, and `61_trees` showed what that does to
// recursion: one stack frame per level, until the 2 MiB stack of a test
// thread overflows and the whole test run aborts with "has overflowed its
// stack" (not a panic you could catch). A recursive DFS in a debug build gets
// there within a few tens of thousands of levels. One test below walks a
// 200_000-node path, so only a version without recursion passes.
//
// The second task is cycle detection in a DIRECTED graph. "I have seen this
// node before" does not mean a cycle: in the diamond 0 -> 1 -> 3,
// 0 -> 2 -> 3, node 3 is reached twice and there is no cycle. A cycle is an
// edge back to a node that is still on the CURRENT path. Hence three colors:
// white (not visited yet), gray (entered but not finished, so on the current
// path) and black (finished, together with everything reachable from it). An
// edge to a gray node closes a cycle; an edge to a black node is harmless.
//
// Without recursion, you have to know when a node is FINISHED: the moment the
// recursive version reaches the end of its `for` loop. A stack of bare nodes
// loses that information, so push frames instead: `(node, index of the next
// edge to follow)`. That pair is exactly what each recursive call kept in its
// stack frame (which node, and how far its loop had got). Look at the top
// frame: if its node has an edge left, advance the index and follow the
// edge; if not, the node is finished, so it turns black and its frame is
// popped.
//
// How interviewers probe it: "Your `dfs(&mut self)` does not compile. Why,
// if it only writes `seen`?", "Restructure it, recursively and
// iteratively", "What happens on a graph that is a million nodes in a
// line?", and "Detect a cycle in a directed graph. Why is a visited set not
// enough?".

// Deliberately not `Clone`: copying the graph is not the way out.
struct Graph {
    // `adj[u]` lists the targets of `u`'s edges.
    adj: Vec<Vec<usize>>,
    // `seen[u]` is set once `dfs` has reached `u`.
    seen: Vec<bool>,
}

// The three states of a node during a directed DFS (for
// `has_cycle_directed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

impl Graph {
    // `n` nodes and no edges.
    fn new(n: usize) -> Self {
        Graph {
            adj: vec![Vec::new(); n],
            seen: vec![false; n],
        }
    }

    fn len(&self) -> usize {
        self.adj.len()
    }

    // A directed edge `from -> to`.
    fn add_edge(&mut self, from: usize, to: usize) {
        assert!(to < self.len(), "node {to} does not exist");
        self.adj[from].push(to);
    }

    // An undirected edge: one directed edge each way.
    fn add_undirected_edge(&mut self, a: usize, b: usize) {
        self.add_edge(a, b);
        self.add_edge(b, a);
    }

    // Marks every node that is reachable from `start` and not marked yet, and
    // returns how many nodes this call marked (0 if `start` was already
    // seen).
    fn dfs(&mut self, start: usize) -> usize {
        // No `&mut self` call inside the loop: it reads the field `self.adj`
        // and writes the field `self.seen` directly, and two different fields
        // may be borrowed in different ways at the same time. The nodes still
        // to visit wait on a stack in a `Vec`, on the heap, so a deep graph
        // costs heap memory instead of call-stack frames. A node is marked
        // when it is pushed, so it is pushed and counted once.
        if self.seen[start] {
            return 0;
        }
        self.seen[start] = true;
        let mut marked = 1;
        let mut stack = vec![start];
        while let Some(u) = stack.pop() {
            for &v in &self.adj[u] {
                if !self.seen[v] {
                    self.seen[v] = true;
                    marked += 1;
                    stack.push(v);
                }
            }
        }
        marked
    }

    // The number of connected components of an undirected graph (one built
    // with `add_undirected_edge`). Starts from a clean `seen`.
    fn count_components(&mut self) -> usize {
        self.seen.fill(false);
        let mut components = 0;
        for u in 0..self.len() {
            if self.dfs(u) > 0 {
                components += 1;
            }
        }
        components
    }

    // Does the directed graph contain a cycle? A self-loop is a cycle of
    // length 1.
    fn has_cycle_directed(&self) -> bool {
        // Every frame on `stack` is a gray node plus the index of its next
        // edge: the recursion's call stack, made explicit. A node turns black
        // only when its frame runs out of edges (the moment a recursive DFS
        // would return from it), so gray means "on the current path", and an
        // edge to a gray node is an edge back into that path: a cycle.
        let mut color = vec![Color::White; self.len()];
        let mut stack: Vec<(usize, usize)> = Vec::new();
        for root in 0..self.len() {
            if color[root] != Color::White {
                continue;
            }
            color[root] = Color::Gray;
            stack.push((root, 0));
            while let Some(frame) = stack.last_mut() {
                let (u, next_edge) = *frame;
                match self.adj[u].get(next_edge) {
                    Some(&v) => {
                        frame.1 += 1;
                        match color[v] {
                            Color::Gray => return true,
                            Color::White => {
                                color[v] = Color::Gray;
                                stack.push((v, 0));
                            }
                            Color::Black => {}
                        }
                    }
                    None => {
                        color[u] = Color::Black;
                        stack.pop();
                    }
                }
            }
        }
        false
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // A graph with one directed edge per pair.
    fn directed(n: usize, edges: &[(usize, usize)]) -> Graph {
        let mut g = Graph::new(n);
        for &(from, to) in edges {
            g.add_edge(from, to);
        }
        g
    }

    // A graph with one undirected edge per pair.
    fn undirected(n: usize, edges: &[(usize, usize)]) -> Graph {
        let mut g = Graph::new(n);
        for &(a, b) in edges {
            g.add_undirected_edge(a, b);
        }
        g
    }

    // 0 -> 1 -> 2 -> ... -> n-1.
    fn path_graph(n: usize) -> Graph {
        let mut g = Graph::new(n);
        for u in 1..n {
            g.add_edge(u - 1, u);
        }
        g
    }

    #[test]
    fn dfs_marks_exactly_the_reachable_nodes() {
        // 4 -> 0 -> 1 -> 2, 0 -> 3, and 5 -> 6 apart.
        let mut g = directed(7, &[(0, 1), (1, 2), (0, 3), (4, 0), (5, 6)]);
        assert_eq!(g.dfs(0), 4);
        assert_eq!(g.seen, [true, true, true, true, false, false, false]);
        // Nothing new below an already seen node.
        assert_eq!(g.dfs(2), 0);
        // From 4, only 4 itself is new: 0 and everything below it are seen.
        assert_eq!(g.dfs(4), 1);
        assert_eq!(g.dfs(6), 1);
        assert_eq!(g.dfs(5), 1);
        assert!(g.seen.iter().all(|&seen| seen));
    }

    #[test]
    fn dfs_counts_each_node_once() {
        // Duplicate edges, self-loops and cycles all lead back to marked
        // nodes. Node 3 is not reachable.
        let mut g = directed(4, &[(0, 1), (0, 1), (1, 0), (1, 1), (1, 2)]);
        for (from, to) in [(2, 0), (2, 2), (3, 0)] {
            g.add_edge(from, to);
        }
        assert_eq!(g.dfs(0), 3);
        assert_eq!(g.seen, [true, true, true, false]);
        assert_eq!(g.dfs(0), 0);
    }

    #[test]
    fn counts_connected_components() {
        // LeetCode 323.
        assert_eq!(
            undirected(5, &[(0, 1), (1, 2), (3, 4)]).count_components(),
            2
        );
        assert_eq!(
            undirected(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]).count_components(),
            1
        );
        // Every isolated node is a component of its own.
        assert_eq!(undirected(4, &[]).count_components(), 4);
        assert_eq!(undirected(0, &[]).count_components(), 0);
        // A second call gives the same answer (`seen` is reset).
        let mut g = undirected(6, &[(0, 5), (5, 3), (1, 1), (2, 4), (4, 2)]);
        assert_eq!(g.count_components(), 3);
        assert_eq!(g.count_components(), 3);
    }

    #[test]
    fn dfs_walks_a_200_000_node_path_without_recursion() {
        const N: usize = 200_000;
        let mut g = path_graph(N);
        assert_eq!(g.dfs(0), N);
        assert!(g.seen.iter().all(|&seen| seen));

        // The same path with undirected edges: one component, and a search
        // from the far end reaches everything too.
        let mut g = undirected(N, &(1..N).map(|u| (u - 1, u)).collect::<Vec<_>>());
        assert_eq!(g.count_components(), 1);
        g.seen.fill(false);
        assert_eq!(g.dfs(N - 1), N);
    }

    #[test]
    fn simple_cycles_are_found() {
        assert!(directed(3, &[(0, 1), (1, 2), (2, 0)]).has_cycle_directed());
        assert!(directed(2, &[(0, 1), (1, 0)]).has_cycle_directed());
        // The cycle does not have to go through node 0.
        assert!(directed(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]).has_cycle_directed());
    }

    #[test]
    fn a_self_loop_is_a_cycle() {
        assert!(directed(3, &[(0, 1), (1, 1)]).has_cycle_directed());
        assert!(directed(1, &[(0, 0)]).has_cycle_directed());
    }

    #[test]
    fn nodes_reached_twice_in_a_dag_are_not_a_cycle() {
        // The diamond: 3 is reached through 1 and through 2.
        assert!(!directed(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]).has_cycle_directed());
        // 0 -> 1, 0 -> 2, 2 -> 1: when the search is at 2, node 1 has been
        // pushed (it is waiting on the stack) but it is not on the path
        // 0 -> 2, so the edge 2 -> 1 closes no cycle.
        assert!(!directed(3, &[(0, 1), (0, 2), (2, 1)]).has_cycle_directed());
        assert!(!directed(3, &[(0, 2), (0, 1), (2, 1)]).has_cycle_directed());
        // Two roots that share a child, and a duplicate edge.
        assert!(!directed(3, &[(1, 0), (2, 0), (2, 0)]).has_cycle_directed());
        // No edges at all, and no nodes at all.
        assert!(!directed(5, &[]).has_cycle_directed());
        assert!(!directed(0, &[]).has_cycle_directed());
    }

    #[test]
    fn a_cycle_that_node_0_cannot_reach_is_found() {
        // 0 -> 1 is harmless; 2 -> 3 -> 4 -> 2 is a cycle elsewhere.
        assert!(directed(5, &[(0, 1), (2, 3), (3, 4), (4, 2)]).has_cycle_directed());
        // A cycle reachable only from the last node.
        assert!(directed(4, &[(3, 1), (1, 2), (2, 1)]).has_cycle_directed());
    }

    #[test]
    fn a_200_000_node_path_and_cycle_need_no_recursion() {
        const N: usize = 200_000;
        let mut g = path_graph(N);
        assert!(!g.has_cycle_directed());
        // One more edge closes the whole path into a cycle.
        g.add_edge(N - 1, 0);
        assert!(g.has_cycle_directed());
    }

    #[test]
    fn has_cycle_directed_needs_only_shared_access() {
        // `g` is not `mut`: the colors live inside `has_cycle_directed`.
        let g = directed(3, &[(0, 1), (1, 2)]);
        let shared = &g;
        assert!(!shared.has_cycle_directed());
        assert!(!g.seen.iter().any(|&seen| seen), "`seen` belongs to `dfs`");
    }

    #[test]
    fn matches_a_brute_force_answer_on_small_random_graphs() {
        // A fixed LCG, so every run checks the same 3_000 graphs.
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed >> 33
        };
        let (mut cyclic, mut acyclic) = (0, 0);
        for round in 0..3_000 {
            let n = 1 + round % 7;
            let percent = 8 + round % 17;
            let mut g = Graph::new(n);
            // reach[u][v]: is there a path of one or more edges from u to v?
            let mut reach = vec![vec![false; n]; n];
            for (from, row) in reach.iter_mut().enumerate() {
                for (to, cell) in row.iter_mut().enumerate() {
                    // Self-loops are rarer than other edges.
                    let chance = if from == to { 2 } else { percent };
                    if next() % 100 < chance as u64 {
                        g.add_edge(from, to);
                        *cell = true;
                    }
                }
            }
            // Floyd-Warshall on booleans (the transitive closure).
            for k in 0..n {
                for u in 0..n {
                    for v in 0..n {
                        if reach[u][k] && reach[k][v] {
                            reach[u][v] = true;
                        }
                    }
                }
            }
            // A directed cycle exists exactly when some node reaches itself.
            let expected = (0..n).any(|u| reach[u][u]);
            assert_eq!(
                g.has_cycle_directed(),
                expected,
                "graph with edges {:?}",
                g.adj
            );
            if expected {
                cyclic += 1;
            } else {
                acyclic += 1;
            }
        }
        // Make sure the generator produced plenty of both kinds.
        assert!(
            cyclic > 500 && acyclic > 500,
            "{cyclic} cyclic, {acyclic} acyclic"
        );
    }
}
