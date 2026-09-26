// Traits & Abstraction · Associated types — part 1: `type Node`, not `Graph<N, E>` (E0107, E0220).
//
// A trait can be generic in two ways. A type PARAMETER, as in
// `trait Graph<N, E>`, is an input: one type may implement `Graph<u32, u32>`
// AND `Graph<String, f64>`, one impl per choice of `N` and `E`, so every use
// of the trait has to say which one it means. An ASSOCIATED TYPE, as in
// `trait Graph { type Node; }`, is an output: each type implements the trait
// at most ONCE, and that one impl fixes `Node`. The type alone determines it,
// so generic code can simply name it: `G::Node`.
//
// That is why `Iterator` has `type Item` rather than being `Iterator<T>`. A
// `vec::IntoIter<u8>` yields `u8` and nothing else, and with a parameter,
// every function that takes "some iterator" would need an extra `T` in its
// signature. Graphs are the classic interview version of the same
// question. A graph type knows its node and edge types, and the algorithms
// written against it should not have to carry them around:
//
//   - with parameters: `fn degree<G: Graph<N, E>, N, E>(g: &G, node: &N)`,
//     two extra parameters in every signature, forever;
//   - with associated types: `fn degree<G: Graph>(g: &G, node: &G::Node)`.
//
// Associated types also give you more to say in bounds:
//
//   - `G: Graph<Edge = u32>` is an associated type CONSTRAINT: any graph at
//     all, as long as its edge type is `u32`.
//   - `where G::Edge: Ord` puts a bound on a projection: whatever the edge
//     type is, it must be `Ord`.
//   - A bound written on the associated type itself, `type Node: PartialEq;`,
//     holds wherever `G: Graph` holds, so generic code may compare nodes
//     without repeating it. A bound on a trait PARAMETER is not carried along
//     like that: with `trait Graph<N: PartialEq, E>`, every
//     `G: Graph<N, E>` in a signature must restate `N: PartialEq` or get
//     E0277 "can't compare `N` with `N`".
//
// Below, the functions and the tests are written against the associated-type
// design, but the trait still has parameters. rustc's help for the resulting
// E0107 is "add missing generic arguments", i.e. write `Graph<N, E>` in every
// signature. That is the design this exercise moves AWAY from.
//
// How interviewers probe this: "Why is `Iterator::Item` an associated type
// and not a type parameter?", "When do you pick an associated type over a
// generic parameter?", and "What is the difference between
// `I: Iterator<Item = u32>` and `where I::Item: Copy`?".

use std::collections::HashMap;

// TODO: rustc accepts this trait and both impls; it rejects every signature
// that uses the trait the new way. `G: Graph` is E0107 "missing generics for
// trait `Graph`" and `Graph<Edge = u32>` is E0107 "trait takes 2 generic
// arguments but 0 generic arguments were supplied". Every `G::Node` or
// `G::Edge` is E0220 "associated type `Node` not found for `G`", and the tests'
// `<AdjList as Graph>::Node` is E0576 "cannot find associated type `Node` in
// trait `Graph`". Redesign the trait so that each graph type determines its
// node and edge types. Requirements:
//   - the trait takes no type parameters, and the functions and tests below
//     must compile unchanged (so don't follow rustc's "add missing generic
//     arguments" help), with the names `Node` and `Edge`;
//   - nodes must still be comparable with `==`, and a function generic over
//     any `G: Graph` must get that for free: the tests' `leads_to` helper
//     compares `G::Node`s without writing a bound of its own;
//   - keep `neighbors` and the default `edge` as they are, apart from the
//     types in their signatures.
// Until you remove the trait's type parameters (and rewrite the two impls
// below to match), this exercise will not compile.
trait Graph<N: PartialEq, E> {
    // Every edge that leaves `node`, as `(target, weight)` pairs, in a fixed
    // order. A node that is not in the graph has no edges.
    fn neighbors(&self, node: &N) -> Vec<(N, E)>;

    // The weight of the edge `from -> to`, if the graph has one.
    fn edge(&self, from: &N, to: &N) -> Option<E> {
        self.neighbors(from)
            .into_iter()
            .find(|(target, _)| target == to)
            .map(|(_, weight)| weight)
    }
}

// A directed, weighted graph. `adj[&from]` lists `(to, weight)` pairs in the
// order the edges were added.
struct AdjList {
    adj: HashMap<u32, Vec<(u32, u32)>>,
}

impl AdjList {
    fn new() -> Self {
        AdjList {
            adj: HashMap::new(),
        }
    }

    fn add_edge(&mut self, from: u32, to: u32, weight: u32) {
        self.adj.entry(from).or_default().push((to, weight));
    }
}

// TODO: Once the trait has changed, this header is E0107 "trait takes 0
// generic arguments but 2 generic arguments were supplied", followed by E0046
// "not all trait items implemented, missing: `Node`, `Edge`". Make the impl
// itself say which node and edge types an `AdjList` has (both `u32`); the
// method body stays as it is. Until you make the impl match the new trait,
// this exercise will not compile.
impl Graph<u32, u32> for AdjList {
    fn neighbors(&self, node: &u32) -> Vec<(u32, u32)> {
        self.adj.get(node).cloned().unwrap_or_default()
    }
}

// A board of `height` rows by `width` columns, with cells addressed as
// `(row, col)`. Each cell has an edge to the cells above, below, left and
// right of it, and an edge's weight is the cost of the cell it ENTERS.
// `costs` is stored row by row.
struct Grid {
    width: usize,
    height: usize,
    costs: Vec<u32>,
}

impl Grid {
    fn new(width: usize, costs: Vec<u32>) -> Self {
        assert!(
            width > 0 && costs.len().is_multiple_of(width),
            "ragged grid"
        );
        Grid {
            width,
            height: costs.len() / width,
            costs,
        }
    }
}

// TODO: The same E0107 and E0046 as the impl above, for a graph whose nodes
// are `(usize, usize)` cells and whose edges are `u32` costs. Rewrite this
// impl header the same way. Until you make it match the new trait, this
// exercise will not compile.
impl Graph<(usize, usize), u32> for Grid {
    fn neighbors(&self, &(row, col): &(usize, usize)) -> Vec<((usize, usize), u32)> {
        if row >= self.height || col >= self.width {
            return Vec::new();
        }
        let mut cells = Vec::new();
        if row > 0 {
            cells.push((row - 1, col));
        }
        if row + 1 < self.height {
            cells.push((row + 1, col));
        }
        if col > 0 {
            cells.push((row, col - 1));
        }
        if col + 1 < self.width {
            cells.push((row, col + 1));
        }
        cells
            .into_iter()
            .map(|(r, c)| ((r, c), self.costs[r * self.width + c]))
            .collect()
    }
}

// How many edges leave `node`.
fn degree<G: Graph>(graph: &G, node: &G::Node) -> usize {
    graph.neighbors(node).len()
}

// The total weight of walking `path` one edge at a time, or `None` if some
// step has no edge. A path of zero or one node costs nothing.
fn path_cost<G: Graph<Edge = u32>>(graph: &G, path: &[G::Node]) -> Option<u32> {
    path.windows(2)
        .map(|step| graph.edge(&step[0], &step[1]))
        .sum()
}

// The neighbor behind the cheapest edge out of `node` (on a tie, the one
// listed first), or `None` if no edge leaves it.
fn cheapest_step<G>(graph: &G, node: &G::Node) -> Option<G::Node>
where
    G: Graph,
    G::Edge: Ord,
{
    graph
        .neighbors(node)
        .into_iter()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(next, _)| next)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1 -> 2 (4), 1 -> 3 (1), 2 -> 4 (3), 3 -> 2 (1), 3 -> 4 (6).
    // Edges are one-way.
    fn roads() -> AdjList {
        let mut g = AdjList::new();
        g.add_edge(1, 2, 4);
        g.add_edge(1, 3, 1);
        g.add_edge(2, 4, 3);
        g.add_edge(3, 2, 1);
        g.add_edge(3, 4, 6);
        g
    }

    // 1 2 3
    // 4 5 6
    // 7 8 9
    fn board() -> Grid {
        Grid::new(3, vec![1, 2, 3, 4, 5, 6, 7, 8, 9])
    }

    // Generic over ANY graph: it names the node type as `G::Node` and compares
    // nodes with `==`, with no bound of its own. That compiles only if the
    // trait itself promises that nodes are comparable.
    fn leads_to<G: Graph>(graph: &G, from: &G::Node, to: &G::Node) -> bool {
        graph.neighbors(from).iter().any(|(next, _)| next == to)
    }

    #[test]
    fn degree_counts_outgoing_edges() {
        let g = roads();
        assert_eq!(degree(&g, &1), 2);
        assert_eq!(degree(&g, &3), 2);
        // A sink, and a node that is not in the graph at all.
        assert_eq!(degree(&g, &4), 0);
        assert_eq!(degree(&g, &99), 0);
    }

    #[test]
    fn path_cost_adds_the_edge_weights() {
        let g = roads();
        assert_eq!(path_cost(&g, &[1, 2, 4]), Some(7));
        assert_eq!(path_cost(&g, &[1, 3, 2, 4]), Some(5));
    }

    #[test]
    fn path_cost_is_none_when_a_step_has_no_edge() {
        let g = roads();
        // Edges are one-way.
        assert_eq!(path_cost(&g, &[2, 1]), None);
        assert_eq!(path_cost(&g, &[1, 4]), None);
        // Only the last step is missing.
        assert_eq!(path_cost(&g, &[1, 2, 4, 1]), None);
    }

    #[test]
    fn a_path_of_zero_or_one_node_costs_nothing() {
        let g = roads();
        assert_eq!(path_cost(&g, &[1]), Some(0));
        assert_eq!(path_cost(&g, &[]), Some(0));
    }

    #[test]
    fn edge_and_cheapest_step() {
        let g = roads();
        assert_eq!(g.edge(&1, &3), Some(1));
        assert_eq!(g.edge(&3, &1), None);
        assert_eq!(cheapest_step(&g, &1), Some(3));
        assert_eq!(cheapest_step(&g, &3), Some(2));
        assert_eq!(cheapest_step(&g, &4), None);
    }

    #[test]
    fn the_same_functions_work_on_a_grid() {
        let b = board();
        // A corner, a border cell, the middle, and a cell off the board.
        assert_eq!(degree(&b, &(0, 0)), 2);
        assert_eq!(degree(&b, &(0, 1)), 3);
        assert_eq!(degree(&b, &(1, 1)), 4);
        assert_eq!(degree(&b, &(3, 0)), 0);
        // Entering (0, 1), (1, 1) and (2, 1) costs 2 + 5 + 8.
        assert_eq!(path_cost(&b, &[(0, 0), (0, 1), (1, 1), (2, 1)]), Some(15));
        // No diagonal moves.
        assert_eq!(path_cost(&b, &[(0, 0), (1, 1)]), None);
        assert_eq!(cheapest_step(&b, &(1, 1)), Some((0, 1)));
        assert_eq!(cheapest_step(&b, &(2, 2)), Some((1, 2)));
    }

    #[test]
    fn each_graph_type_fixes_its_own_node_and_edge_types() {
        // Fully qualified paths: there is one impl per type, so the type alone
        // determines `Node` and `Edge`.
        let start: <AdjList as Graph>::Node = 1;
        let cell: <Grid as Graph>::Node = (2, 2);
        let cost: Option<<Grid as Graph>::Edge> = board().edge(&(2, 1), &cell);
        assert_eq!(cost, Some(9));

        assert!(leads_to(&roads(), &start, &3));
        assert!(!leads_to(&roads(), &3, &start));
        assert!(leads_to(&board(), &(2, 1), &cell));
        assert!(!leads_to(&board(), &(0, 0), &cell));
    }
}
