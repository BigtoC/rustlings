// Module 2 · Binary search trees — part 1: insert with a `&mut` cursor, search with a loop (E0308).
//
// "Implement insert and search for a binary search tree" is the tree question
// of live-coding rounds, and in Rust it is really a question about ownership.
// A BST node owns its two subtrees, so the tree needs no `Rc` and no
// `RefCell`: `Option<Box<Node>>` is a nullable owning pointer, exactly as in
// the linked lists of `27_data_structures`, only with two links per node.
// Every node is reachable along exactly one path from the root, so the
// ownership graph already is a tree. (Parent pointers would break that; see
// `59_arena`.)
//
// Search is a read-only walk: a cursor of type `Option<&Node>` that steps
// left or right until it finds the key or falls off the tree.
// `Option::as_deref` turns a `&Option<Box<Node>>` into that `Option<&Node>`.
//
// Insert needs a pointer you can WRITE through, and the trick is the tail
// cursor of `27_data_structures/linkedlist3`: point at the LINK, not at a
// node. A cursor of type `&mut Option<Box<Node>>` starts at the root link and
// steps to a child link of the node it passes. When it reaches an empty link,
// that link is exactly where the new node belongs, and one assignment through
// the cursor hangs it there. There is no parent pointer and no "was it the
// left or the right child?" bookkeeping, which the textbook C version needs
// (C's pointer-to-pointer, `Node **link`, is the same trick).
//
// Why does the borrow checker let the cursor step? The node you match on is a
// reborrow of `*cur`, and stepping makes `cur` point further down THROUGH that
// reborrow. Assigning to `cur` ends ("kills") every loan on the old `*cur`,
// so there are never two live `&mut` into the tree. This is "problem case #4"
// of the NLL RFC (RFC 2094), and NLL accepts it. An early `return false`
// inside such a loop is fine too, because `false` borrows nothing. Part 4
// returns the CURSOR itself from inside the loop, and that is another story.
//
// Why a loop and not the three-line recursion? A BST is only as shallow as
// its insertion order allows. Insert keys in sorted order and every node gets
// one child: the tree is a linked list, and a recursive walk needs one stack
// frame per level. A test thread has a 2 MiB stack, so the tests'
// 200_000-deep tree kills a recursive insert or search with "thread ... has
// overflowed its stack", which aborts the whole test run. The drop glue the
// compiler derives for `Option<Box<Node>>` recurses in the same way, which is
// why `Bst` comes with a hand-written, iterative `Drop` (given code).
//
// Interviewers follow up with the costs: O(h) time for both operations, where
// the height h is O(log n) on average for a random insertion order but n for
// a sorted one, and O(1) extra space for the loops. Self-balancing trees
// (AVL, red-black) keep h in O(log n) always; std's `BTreeMap` is a B-tree
// instead, with many keys per node.

use std::cmp::Ordering;

struct Node {
    key: i32,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn new(key: i32) -> Self {
        Node {
            key,
            left: None,
            right: None,
        }
    }
}

// A set of `i32` keys. Every key in a node's left subtree is smaller than the
// node's key, and every key in its right subtree is larger.
struct Bst {
    root: Option<Box<Node>>,
    len: usize,
}

impl Bst {
    fn new() -> Self {
        Bst { root: None, len: 0 }
    }

    fn len(&self) -> usize {
        self.len
    }

    // Inserts `key` and returns `true`, or returns `false` and changes nothing
    // if `key` is already in the tree. A new key always becomes a LEAF: it
    // hangs from the empty link where a search for it ends.
    fn insert(&mut self, key: i32) -> bool {
        // `cur` points at a LINK: first the root link, then a child link of
        // the node it just passed. Each step reborrows THROUGH the old cursor
        // and overwrites it, which ends the old loan (the NLL RFC's problem
        // case #4), so there is only ever one live `&mut` into the tree.
        // `return false` borrows nothing, so returning early is fine.
        let mut cur = &mut self.root;
        while let Some(node) = cur {
            match key.cmp(&node.key) {
                Ordering::Less => cur = &mut node.left,
                Ordering::Greater => cur = &mut node.right,
                Ordering::Equal => return false,
            }
        }
        // The loop only ends at an empty link: the one where the search for
        // `key` fell off the tree, which is where the new leaf belongs.
        *cur = Some(Box::new(Node::new(key)));
        self.len += 1;
        true
    }

    // Returns whether `key` is in the tree.
    fn contains(&self, key: i32) -> bool {
        // A read-only walk. `as_deref` turns `&Option<Box<Node>>` into
        // `Option<&Node>`, and a shared reference is `Copy`, so the cursor
        // simply moves down one level per step. `cmp` compares without any
        // arithmetic, so `i32::MIN` and `i32::MAX` are ordinary keys.
        let mut cur = self.root.as_deref();
        while let Some(node) = cur {
            match key.cmp(&node.key) {
                Ordering::Less => cur = node.left.as_deref(),
                Ordering::Greater => cur = node.right.as_deref(),
                Ordering::Equal => return true,
            }
        }
        false
    }
}

// The drop glue the compiler derives for `Option<Box<Node>>` is recursive:
// dropping a node first drops its children, one stack frame per level, so a
// 200_000-deep tree would overflow the stack while being freed. This `Drop`
// moves the nodes onto an explicit stack instead. Every node has both
// children taken out before it is dropped, so no drop ever recurses. The
// `Vec` holds at most about one waiting subtree per level: O(h). (The README
// shows an O(1)-space version that rotates the tree as it goes.)
impl Drop for Bst {
    fn drop(&mut self) {
        let mut pending: Vec<Box<Node>> = self.root.take().into_iter().collect();
        while let Some(mut node) = pending.pop() {
            pending.extend(node.left.take());
            pending.extend(node.right.take());
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // The keys in pre-order: a node, then its left subtree, then its right
    // subtree. Together with the BST order this pins down the exact shape of
    // the tree. A loop with an explicit stack, so it works on deep trees too.
    fn preorder(tree: &Bst) -> Vec<i32> {
        let mut out = Vec::new();
        let mut stack: Vec<&Node> = tree.root.as_deref().into_iter().collect();
        while let Some(node) = stack.pop() {
            out.push(node.key);
            // Right first, so that the left subtree comes off the stack first.
            stack.extend(node.right.as_deref());
            stack.extend(node.left.as_deref());
        }
        out
    }

    // Builds a tree that is a single path: `keys[0]` is the root, and every
    // other key hangs below the one before it, on the left when it is
    // smaller. The keys must form a valid BST path. It is built bottom-up in
    // O(n), without `insert`, which would walk the whole path again for every
    // key: O(n^2) for a deep path.
    fn path(keys: &[i32]) -> Bst {
        let mut below: Option<Box<Node>> = None;
        for &key in keys.iter().rev() {
            let mut node = Box::new(Node::new(key));
            if let Some(child) = below.take() {
                if child.key < key {
                    node.left = Some(child);
                } else {
                    node.right = Some(child);
                }
            }
            below = Some(node);
        }
        Bst {
            root: below,
            len: keys.len(),
        }
    }

    // The keys of an `n`-node path that turns at every step, right, left,
    // right, ...: node `i` holds `i` when `i` is even and `2n - i` when `i` is
    // odd. The small keys climb from 0 and the large ones fall from 2n - 1, so
    // each key lies between the two before it.
    fn zigzag_keys(n: i32) -> Vec<i32> {
        (0..n)
            .map(|i| if i % 2 == 0 { i } else { 2 * n - i })
            .collect()
    }

    #[test]
    fn an_empty_tree_contains_nothing() {
        let tree = Bst::new();
        assert_eq!(tree.len(), 0);
        assert!(!tree.contains(0));
        assert!(!tree.contains(i32::MIN));
        assert!(tree.root.is_none());
    }

    #[test]
    fn inserts_build_the_standard_shape() {
        let mut tree = Bst::new();
        for key in [50, 30, 70, 20, 40, 60, 80, 35, 65] {
            assert!(tree.insert(key), "insert({key}) into the tree");
        }
        assert_eq!(tree.len(), 9);
        //         50
        //       /    \
        //     30      70
        //    /  \    /  \
        //   20  40  60  80
        //       /     \
        //      35     65
        assert_eq!(preorder(&tree), [50, 30, 20, 40, 35, 70, 60, 65, 80]);
        for key in [50, 30, 70, 20, 40, 60, 80, 35, 65] {
            assert!(tree.contains(key), "contains({key})");
        }
        for key in [0, 10, 25, 36, 45, 55, 62, 66, 75, 90] {
            assert!(!tree.contains(key), "contains({key}) on a missing key");
        }
    }

    #[test]
    fn contains_looks_only_along_the_search_path() {
        // A deliberately BROKEN tree: 99 hangs in the LEFT subtree of 50,
        // where no search for 99 ever goes. A search that follows one path,
        // as a BST search must, never sees it; a scan of the whole tree does.
        //       50
        //      /  \
        //    30    70
        //   /  \
        //  20   99
        let leaf = |key| Some(Box::new(Node::new(key)));
        let tree = Bst {
            root: Some(Box::new(Node {
                key: 50,
                left: Some(Box::new(Node {
                    key: 30,
                    left: leaf(20),
                    right: leaf(99),
                })),
                right: leaf(70),
            })),
            len: 5,
        };
        for key in [50, 30, 70, 20] {
            assert!(tree.contains(key), "contains({key})");
        }
        assert!(
            !tree.contains(99),
            "contains(99) looked off its search path (50, 70, then an empty \
             link): search one path, don't scan the tree"
        );
    }

    #[test]
    fn duplicates_return_false_and_change_nothing() {
        let mut tree = Bst::new();
        for key in [8, 3, 10, 1, 6, 14] {
            assert!(tree.insert(key));
        }
        let before = preorder(&tree);
        // The root, an inner node and a leaf, on both sides.
        for key in [8, 3, 10, 1, 6, 14, 8] {
            assert!(!tree.insert(key), "insert({key}) a second time");
        }
        assert_eq!(tree.len(), 6);
        assert_eq!(preorder(&tree), before);
    }

    #[test]
    fn shuffled_inserts_are_all_found() {
        let mut tree = Bst::new();
        // 7919 is prime, so `i * 7919 % 1000` visits every number in 0..1000
        // exactly once, in a scrambled order. Only even keys go in, so every
        // odd key is a miss that ends between two stored keys.
        for i in 0..1000 {
            assert!(tree.insert(2 * (i * 7919 % 1000)));
        }
        assert_eq!(tree.len(), 1000);
        for key in 0..2000 {
            assert_eq!(tree.contains(key), key % 2 == 0, "contains({key})");
        }
        assert!(!tree.contains(-1));
        assert!(!tree.contains(2000));
    }

    #[test]
    fn extreme_and_negative_keys() {
        let mut tree = Bst::new();
        // With `i32::MIN` at the root, comparing by subtraction overflows at
        // the very next insert, and debug builds panic on that.
        let keys = [i32::MIN, i32::MAX, 0, -1, 1, i32::MIN + 1, i32::MAX - 1];
        for key in keys {
            assert!(tree.insert(key), "insert({key})");
        }
        assert_eq!(tree.len(), keys.len());
        // MIN -> right MAX -> left 0, whose children are -1 (with MIN + 1
        // below it) and 1 (with MAX - 1 below it).
        assert_eq!(
            preorder(&tree),
            [i32::MIN, i32::MAX, 0, -1, i32::MIN + 1, 1, i32::MAX - 1]
        );
        for key in keys {
            assert!(tree.contains(key), "contains({key})");
            assert!(!tree.insert(key), "insert({key}) a second time");
        }
        for key in [-2, 2, i32::MIN + 2, i32::MAX - 2] {
            assert!(!tree.contains(key), "contains({key}) on a missing key");
        }
    }

    #[test]
    fn sorted_inserts_degenerate_into_a_linked_list() {
        let mut tree = Bst::new();
        for key in 0..2000 {
            assert!(tree.insert(key));
        }
        // Every node has only a right child: the height is n, not log2(n).
        assert!(preorder(&tree).into_iter().eq(0..2000));
        let mut cur = tree.root.as_deref();
        while let Some(node) = cur {
            assert!(node.left.is_none(), "{} has a left child", node.key);
            cur = node.right.as_deref();
        }
    }

    #[test]
    fn a_200_000_deep_tree_needs_loops_not_recursion() {
        const N: i32 = 200_000;
        let keys = zigzag_keys(N);
        let mut tree = path(&keys);
        // The root, the root's right child (the largest key) and the deepest
        // node, which holds N + 1 below N - 2.
        assert!(tree.contains(0));
        assert!(tree.contains(2 * N - 1));
        assert!(tree.contains(N + 1));
        // N - 1 and N lie between N - 2 and N + 1: these misses fall off at
        // the very bottom of the path.
        assert!(!tree.contains(N - 1));
        assert!(!tree.contains(N));
        assert!(!tree.contains(2 * N));

        // Two new leaves at the bottom and one at the top.
        assert!(tree.insert(N));
        assert!(tree.insert(N - 1));
        assert!(tree.insert(-5));
        // Duplicates deep down and at the root.
        assert!(!tree.insert(N));
        assert!(!tree.insert(N + 1));
        assert!(!tree.insert(0));
        assert_eq!(tree.len(), keys.len() + 3);
        assert!(tree.contains(N) && tree.contains(N - 1) && tree.contains(-5));

        // -5 is the root's left child; N hangs below N + 1, and N - 1 below
        // N. For a path, pre-order is just the order along the path.
        let mut expected = vec![0, -5];
        expected.extend_from_slice(&keys[1..]);
        expected.extend([N, N - 1]);
        assert!(preorder(&tree) == expected, "the new keys are misplaced");
        // `tree` is dropped here by the iterative `Drop`.
    }
}
