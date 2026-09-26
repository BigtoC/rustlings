// Module 2 · Arenas — part 1: a tree with parent links in one `Vec`, addressed by typed `NodeId`s (E0308).
//
// "Build a tree in which every node knows its parent" (or a graph, or a
// doubly linked list) is where a C or Java answer stops porting to Rust.
// There, a child points to its parent and the parent points to its children.
// In Rust every value has exactly ONE owner, so ownership has to form a tree
// with no back edges. A parent can own its children through `Box`, but then
// the children cannot own their parent as well. A `&Node` back link does not
// work either: a reference into the structure keeps the structure borrowed,
// so nothing in it can change while the link exists. Even the first child,
// which borrows its parent, cannot be pushed onto that parent's `children`:
// E0502, aliasing XOR mutability.
//
// The textbook escape is `Rc<RefCell<Node>>` with `Weak` parent links
// (`26_smart_pointers_deep/smartptr2` and `smartptr3`). It works, but look at
// the bill: a heap allocation and two reference counts per node, a run-time
// borrow check on every access that panics when a `borrow_mut` overlaps any
// other borrow (`31_debugging/debugging4`), a tree that is neither `Send` nor
// `Sync` (`Rc`'s counts are not atomic), a `clone()` that copies one handle
// instead of the tree, and a leak as soon as someone builds a cycle of strong
// links.
//
// The answer interviewers are after is an ARENA. The tree owns all of its
// nodes in a single `Vec`, and every link is an INDEX into that `Vec`. The
// tree is the only owner, so ownership is a plain tree again. A link is a
// `Copy` number that borrows nothing, so the borrow checker has nothing to
// object to: you can hold the ids of a hundred nodes and still add a child
// through `&mut self`. The newtype `NodeId(usize)` makes the index TYPED: it
// cannot be mixed up with a length, an offset or another kind of id, and
// (with its field private, in a real crate) only the tree can mint one. The
// payoff is concrete: one growable allocation with the nodes side by side
// (cache friendly), `Send` and `Sync` whenever `T` is, a `#[derive(Clone)]`
// that copies the whole tree and in which every old id still works, and a
// drop that is one flat loop over the `Vec` instead of recursive drop glue
// (compare `27_data_structures/linkedlist1`). The Rust compiler itself is
// built this way (`IndexVec` plus `newtype_index!` ids), and so are
// `petgraph`'s graphs.
//
// The price: the borrow checker no longer checks the links. An id is just a
// number. One from another tree, or a made-up one, reaches the wrong node or
// fails the bounds check. That is a logic bug, never undefined behavior,
// because indexing a `Vec` is always bounds-checked. And to rustc the whole
// tree is ONE borrow, so a `&mut` to two nodes at once needs
// `get_disjoint_mut` (`37_borrowck_errors/borrowck1`). This tree never
// removes a node, so every id it hands out stays valid for the tree's whole
// life. Removal is what makes ids go stale: that is part 2.
//
// The algorithms become loops over indices. `path_to_root` follows the
// `parent` links up to the root. The lowest common ancestor (LCA) of `a` and
// `b` is the deepest node that has both of them in its subtree, where a node
// counts as its own ancestor. With parent links this is LeetCode 1650
// ("Lowest Common Ancestor of a Binary Tree III"), and `path_to_root` does
// most of the work: look at what the two paths have in common.
//
// How interviewers probe it: "Represent a tree with parent pointers, or a
// graph, in safe Rust without `Rc<RefCell>`. What do you give up compared
// with `Rc` and `Weak`? What stops a `NodeId` from dangling? Now write
// `lca`, in O(depth) time and ideally O(1) extra space."

// A typed index: the position of a node in `Tree::nodes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NodeId(usize);

#[derive(Debug, Clone)]
struct NodeData<T> {
    value: T,
    // `None` only for the root.
    parent: Option<NodeId>,
    // In the order the children were added.
    children: Vec<NodeId>,
}

// A tree always has a root: node 0, created by `new`.
#[derive(Debug, Clone)]
struct Tree<T> {
    nodes: Vec<NodeData<T>>,
}

impl<T> Tree<T> {
    fn new(root: T) -> Self {
        Tree {
            nodes: vec![NodeData {
                value: root,
                parent: None,
                children: Vec::new(),
            }],
        }
    }

    fn root(&self) -> NodeId {
        NodeId(0)
    }

    // The number of nodes, the root included.
    fn len(&self) -> usize {
        self.nodes.len()
    }

    // These accessors panic on an id that does not belong to this tree, like
    // indexing a `Vec` out of bounds does.
    fn value(&self, id: NodeId) -> &T {
        &self.nodes[id.0].value
    }

    fn value_mut(&mut self, id: NodeId) -> &mut T {
        &mut self.nodes[id.0].value
    }

    fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id.0].parent
    }

    fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id.0].children
    }

    // Adds `value` as the last child of `parent` and returns the new node's
    // id.
    fn add_child(&mut self, parent: NodeId, value: T) -> NodeId {
        // The new node's id is the position it is about to take. Indexing the
        // parent FIRST makes an unknown parent (including `id` itself) panic
        // while the tree is still untouched; only then is the node pushed.
        // Both links are plain numbers, so neither one borrows anything.
        let id = NodeId(self.nodes.len());
        self.nodes[parent.0].children.push(id);
        self.nodes.push(NodeData {
            value,
            parent: Some(parent),
            children: Vec::new(),
        });
        id
    }

    // The ids from `id` up to the root, both included: `[id, .., root]`.
    fn path_to_root(&self, id: NodeId) -> Vec<NodeId> {
        // `successors` yields `id`, then keeps applying `parent` until it
        // returns `None` (after the root). It is a loop, so a deep chain costs
        // heap space for the path, not stack frames.
        std::iter::successors(Some(id), |&node| self.parent(node)).collect()
    }

    // The lowest common ancestor of `a` and `b`.
    fn lca(&self, a: NodeId, b: NodeId) -> NodeId {
        // Both paths end at the root. Read from that end, they agree on every
        // common ancestor and split right below the lowest one, so the last
        // matching pair is the answer. O(depth) time and space; there is
        // always at least one match, the root itself.
        let path_a = self.path_to_root(a);
        let path_b = self.path_to_root(b);
        path_a
            .iter()
            .rev()
            .zip(path_b.iter().rev())
            .take_while(|(x, y)| x == y)
            .last()
            .map(|(&common, _)| common)
            .expect("both paths end at the root")
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::panic::{self, AssertUnwindSafe};
    use std::thread;

    //          r
    //        /   \
    //       a     b
    //      / \     \
    //     c   d     e
    //         |
    //         f
    fn sample() -> (Tree<&'static str>, [NodeId; 7]) {
        let mut tree = Tree::new("r");
        let r = tree.root();
        let a = tree.add_child(r, "a");
        let b = tree.add_child(r, "b");
        let c = tree.add_child(a, "c");
        let d = tree.add_child(a, "d");
        let e = tree.add_child(b, "e");
        let f = tree.add_child(d, "f");
        (tree, [r, a, b, c, d, e, f])
    }

    #[test]
    fn add_child_hands_out_sequential_ids_and_sets_both_links() {
        let (tree, [r, a, b, c, d, e, f]) = sample();
        // The id is the node's position in the `Vec`, in insertion order.
        assert_eq!(
            [r, a, b, c, d, e, f],
            [0, 1, 2, 3, 4, 5, 6].map(NodeId),
            "ids should be handed out in order"
        );
        assert_eq!(tree.len(), 7);
        // Child -> parent links.
        assert_eq!(tree.parent(r), None);
        assert_eq!(tree.parent(a), Some(r));
        assert_eq!(tree.parent(b), Some(r));
        assert_eq!(tree.parent(c), Some(a));
        assert_eq!(tree.parent(d), Some(a));
        assert_eq!(tree.parent(e), Some(b));
        assert_eq!(tree.parent(f), Some(d));
        // Parent -> child links, in the order the children were added.
        assert_eq!(tree.children(r), [a, b]);
        assert_eq!(tree.children(a), [c, d]);
        assert_eq!(tree.children(b), [e]);
        assert_eq!(tree.children(d), [f]);
        for leaf in [c, e, f] {
            assert!(tree.children(leaf).is_empty(), "{leaf:?} is a leaf");
        }
        // Every value landed in its own node.
        let values: Vec<&str> = [r, a, b, c, d, e, f].map(|id| *tree.value(id)).into();
        assert_eq!(values, ["r", "a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn an_unknown_parent_panics_and_changes_nothing() {
        let mut tree = Tree::new(0);
        let a = tree.add_child(tree.root(), 1);
        // Ids 0 and 1 exist. `NodeId(2)` is the id the NEW node would get, so
        // a version that pushes the node before it looks at the parent would
        // make node 2 its own parent. `NodeId(99)` is far out of range.
        for unknown in [NodeId(2), NodeId(99)] {
            let result = panic::catch_unwind(AssertUnwindSafe(|| tree.add_child(unknown, 7)));
            assert!(result.is_err(), "add_child({unknown:?}, ..) should panic");
            assert_eq!(tree.len(), 2, "no node may be added for {unknown:?}");
            assert_eq!(tree.children(tree.root()), [a]);
            assert!(tree.children(a).is_empty());
        }
        // The tree still works normally afterwards.
        assert_eq!(tree.add_child(a, 2), NodeId(2));
        assert_eq!(tree.path_to_root(NodeId(2)), [NodeId(2), a, tree.root()]);
    }

    #[test]
    fn path_to_root_follows_the_parent_links() {
        let (tree, [r, a, b, c, d, e, f]) = sample();
        assert_eq!(tree.path_to_root(f), [f, d, a, r]);
        assert_eq!(tree.path_to_root(c), [c, a, r]);
        assert_eq!(tree.path_to_root(e), [e, b, r]);
        assert_eq!(tree.path_to_root(a), [a, r]);
        // The root's path is the root alone.
        assert_eq!(tree.path_to_root(r), [r]);
    }

    #[test]
    fn lca_of_siblings_is_their_parent() {
        let (tree, [r, a, b, c, d, ..]) = sample();
        assert_eq!(tree.lca(c, d), a);
        assert_eq!(tree.lca(d, c), a);
        assert_eq!(tree.lca(a, b), r);
    }

    #[test]
    fn lca_of_cousins_and_uneven_depths() {
        let (tree, [r, a, _b, c, _d, e, f]) = sample();
        // Cousins: children of two siblings.
        assert_eq!(tree.lca(c, e), r);
        // Different depths: `f` is three levels down, `c` two and `e` two.
        assert_eq!(tree.lca(f, c), a);
        assert_eq!(tree.lca(c, f), a);
        assert_eq!(tree.lca(f, e), r);
        assert_eq!(tree.lca(e, f), r);
    }

    #[test]
    fn lca_with_an_ancestor_is_the_ancestor() {
        let (tree, [r, a, b, _c, d, e, f]) = sample();
        assert_eq!(tree.lca(a, f), a);
        assert_eq!(tree.lca(f, a), a);
        assert_eq!(tree.lca(d, f), d);
        assert_eq!(tree.lca(b, e), b);
        assert_eq!(tree.lca(f, r), r);
        assert_eq!(tree.lca(r, e), r);
    }

    #[test]
    fn lca_of_a_node_with_itself_is_the_node() {
        let (tree, ids) = sample();
        for id in ids {
            assert_eq!(tree.lca(id, id), id);
        }
    }

    #[test]
    fn lca_matches_a_brute_force_answer_on_every_pair() {
        // A 60-node tree with an irregular shape: each new node picks a
        // pseudo-random parent among the nodes already there (a fixed LCG, so
        // the tree is the same on every run).
        let mut tree = Tree::new(0_u32);
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        for value in 1..60 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let parent = NodeId((seed >> 33) as usize % tree.len());
            tree.add_child(parent, value);
        }

        // The brute-force answer, built from the given `parent` accessor only:
        // collect every ancestor of `a` (itself included), then climb from `b`
        // until you reach one of them.
        let brute_force_lca = |a: NodeId, b: NodeId| {
            let mut ancestors_of_a = HashSet::new();
            let mut cur = Some(a);
            while let Some(id) = cur {
                ancestors_of_a.insert(id);
                cur = tree.parent(id);
            }
            let mut cur = b;
            while !ancestors_of_a.contains(&cur) {
                cur = tree
                    .parent(cur)
                    .expect("the root is an ancestor of every node");
            }
            cur
        };

        for i in 0..tree.len() {
            for j in 0..tree.len() {
                let (a, b) = (NodeId(i), NodeId(j));
                assert_eq!(tree.lca(a, b), brute_force_lca(a, b), "lca({a:?}, {b:?})");
            }
        }
    }

    #[test]
    fn a_deep_chain_needs_no_recursion() {
        const DEPTH: usize = 200_000;
        let mut tree = Tree::new(0);
        let mut tip = tree.root();
        for value in 1..=DEPTH {
            tip = tree.add_child(tip, value);
        }
        assert_eq!(tree.len(), DEPTH + 1);

        let path = tree.path_to_root(tip);
        assert_eq!(path.len(), DEPTH + 1);
        assert_eq!(path.first(), Some(&tip));
        assert_eq!(path.last(), Some(&tree.root()));
        assert!(
            path.iter().map(|&id| *tree.value(id)).eq((0..=DEPTH).rev()),
            "the path should visit every level once, bottom to top"
        );

        // Dropping the tree drops a `Vec`: one flat loop over the nodes, not
        // one stack frame per level like the drop glue of a `Box` chain.
        drop(tree);
    }

    #[test]
    fn a_tree_of_strings_can_move_to_another_thread() {
        // An `Rc<RefCell<Node>>` tree is neither `Send` nor `Sync`: `Rc`'s
        // counts are not atomic, so rustc refuses to let one cross a thread.
        // A tree of indices is as thread-safe as the values in it.
        fn assert_send_sync<X: Send + Sync>() {}
        assert_send_sync::<Tree<String>>();
        assert_send_sync::<NodeId>();

        let mut tree = Tree::new(String::from("root"));
        let child = tree.add_child(tree.root(), String::from("child"));
        // Move the tree to another thread, grow it there, and get it back.
        // `child` is a plain number, so it crosses the thread boundary too.
        let (tree, grandchild) = thread::spawn(move || {
            let grandchild = tree.add_child(child, String::from("grandchild"));
            (tree, grandchild)
        })
        .join()
        .expect("the thread should not panic");
        assert_eq!(tree.len(), 3);
        assert_eq!(tree.children(child), [grandchild]);
        assert_eq!(tree.value(grandchild), "grandchild");
        assert_eq!(
            tree.path_to_root(grandchild),
            [grandchild, child, tree.root()]
        );
    }

    #[test]
    fn clone_is_a_deep_copy_and_ids_work_in_both() {
        let mut tree = Tree::new(String::from("root"));
        let a = tree.add_child(tree.root(), String::from("a"));
        let mut copy = tree.clone();

        // An id is a position, so it names the matching node in the copy.
        assert_eq!(copy.value(a), "a");
        assert_eq!(copy.parent(a), Some(copy.root()));
        // The copy owns its own strings (other heap buffers), not handles to
        // the original's nodes.
        assert_ne!(copy.value(a).as_ptr(), tree.value(a).as_ptr());

        // Changing the copy leaves the original alone, and vice versa.
        copy.value_mut(a).push_str(" (copy)");
        let b = copy.add_child(a, String::from("b"));
        tree.value_mut(a).push_str(" (original)");
        assert_eq!(copy.value(a), "a (copy)");
        assert_eq!(tree.value(a), "a (original)");
        assert_eq!(copy.children(a), [b]);
        assert!(tree.children(a).is_empty());
        assert_eq!((tree.len(), copy.len()), (2, 3));
    }
}
