// Module 2 · Binary search trees — part 3: invert with a stack of `&mut Node`, height with BFS, no recursion (E0308).
//
// "Invert a binary tree" (LeetCode 226) became famous through a 2015 tweet by
// the author of Homebrew about failing it in a Google interview, and "maximum
// depth of a binary tree" (LeetCode 104) is its usual warm-up. Both have
// three-line recursive answers: swap the children and recurse into both, or
// return 1 + the larger of the two subtree heights. And both recursions go as
// deep as the tree is tall, so on the tests' 200_000-deep tree they overflow
// the 2 MiB stack of a test thread. The follow-up interviewers like is "now
// do it without recursion", which means: keep the pending work on an explicit
// stack or queue in heap memory instead of on the call stack.
//
// For `invert`, the pending work is "nodes whose children still have to be
// swapped", so the stack holds `&mut Node`s: several mutable references into
// ONE tree, alive at the same time, in one `Vec`. That compiles, and the
// reason is worth saying out loud in an interview. Each `&mut Node` on the
// stack was split off its parent through the parent's `left` or `right`
// field, and those two fields are disjoint places, so borrowing both at once
// is fine (like `split_at_mut` on a slice). The parent's own `&mut` was
// popped off the stack before that and is never used again. So no two
// references on the stack can reach the same node. Compare
// `37_borrowck_errors/borrowck1`, where `&mut v[i]` and `&mut v[j]` conflict:
// the borrow checker does not evaluate indices, but it does see field names.
//
// Swapping two `Option<Box<Node>>` fields moves two pointers, not two
// subtrees, so the whole inversion is O(n) time with O(h) extra space. It
// relinks the nodes the tree already has: no node is created, copied or
// freed, and every key stays in its own node (the tests compare addresses).
// Note that after one inversion the in-order walk is DESCENDING: the tree is
// the mirror image of a BST, so `insert` (or any search) would go down the
// wrong side until you invert it back.
//
// For `height`, a breadth-first walk (BFS) is the natural iterative version:
// a queue holds one level of the tree, and the height is the number of levels.
// It uses O(w) extra space for the widest level w, which is about n/2 in a
// perfect tree; a depth-first stack of `(node, depth)` pairs uses O(h)
// instead. Knowing which one is cheaper for which shape of tree is a standard
// follow-up question. Here the height counts NODES: an empty tree has height
// 0 and a single node height 1, as in LeetCode 104.

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
// node's key, and every key in its right subtree is larger (until `invert`
// mirrors it).
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

    // From part 1: a `&mut` cursor walks down to the empty link where `key`
    // belongs.
    fn insert(&mut self, key: i32) -> bool {
        let mut cur = &mut self.root;
        while let Some(node) = cur {
            match key.cmp(&node.key) {
                Ordering::Less => cur = &mut node.left,
                Ordering::Greater => cur = &mut node.right,
                Ordering::Equal => return false,
            }
        }
        *cur = Some(Box::new(Node::new(key)));
        self.len += 1;
        true
    }

    // Mirrors the tree: at every node, the left and right subtrees trade
    // places.
    fn invert(&mut self) {
        // TODO: This empty body compiles (it returns `()`), so once `height`
        // compiles, the invert tests fail, `inverts_the_leetcode_226_example`
        // among them. Swap the children of EVERY node, at every level.
        // Requirements:
        //   - relink the nodes the tree already has: no new nodes, and no
        //     moving keys from one node to another (the tests compare node
        //     addresses);
        //   - iterative, with an explicit stack or queue: no recursion (one
        //     test inverts a 200_000-deep tree); O(n) time;
        //   - no `unsafe`, no `Rc`/`RefCell`, and don't change the tests.
        // Until you mirror every level, the invert tests will fail.
    }

    // The number of nodes on the longest path from the root down to a leaf.
    fn height(&self) -> usize {
        // TODO: The empty body is rejected with E0308 "mismatched types":
        // expected `usize`, found `()`. Requirements:
        //   - an empty tree has height 0 and a single node height 1;
        //   - measure the LONGEST path, wherever it is, not just the leftmost
        //     or the rightmost one;
        //   - iterative: no recursion (one test measures a 200_000-deep
        //     tree); O(n) time; `height` only reads, so it takes `&self`;
        //   - no `unsafe`, and don't change the signatures or the tests.
        // Until you return a `usize`, this exercise will not compile.
    }
}

// From part 1: frees the nodes from an explicit stack, because the derived
// drop glue would recurse once per level and overflow on a deep tree.
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
    use std::collections::HashMap;
    use std::ptr;

    fn from_keys(keys: impl IntoIterator<Item = i32>) -> Bst {
        let mut tree = Bst::new();
        for key in keys {
            assert!(tree.insert(key), "the test inserts {key} twice");
        }
        tree
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

    // The keys of an `n`-node path that turns at every step: node `i` holds
    // `i` when `i` is even and `2n - i` when `i` is odd. The small keys climb
    // from 0 and the large ones fall from 2n - 1.
    fn zigzag_keys(n: i32) -> Vec<i32> {
        (0..n)
            .map(|i| if i % 2 == 0 { i } else { 2 * n - i })
            .collect()
    }

    // The keys in pre-order (a node, then its left subtree, then its right
    // subtree), which pins down the exact shape of the tree. A loop with an
    // explicit stack, so it works on deep trees too.
    fn preorder(tree: &Bst) -> Vec<i32> {
        let mut out = Vec::new();
        let mut stack: Vec<&Node> = tree.root.as_deref().into_iter().collect();
        while let Some(node) = stack.pop() {
            out.push(node.key);
            stack.extend(node.right.as_deref());
            stack.extend(node.left.as_deref());
        }
        out
    }

    // The keys in in-order (left subtree, node, right subtree): ascending
    // for a BST, descending for an inverted one. The iterator of part 2,
    // written as a loop.
    fn inorder(tree: &Bst) -> Vec<i32> {
        let mut out = Vec::new();
        let mut stack: Vec<&Node> = Vec::new();
        let mut cur = tree.root.as_deref();
        loop {
            while let Some(node) = cur {
                stack.push(node);
                cur = node.left.as_deref();
            }
            let Some(node) = stack.pop() else {
                return out;
            };
            out.push(node.key);
            cur = node.right.as_deref();
        }
    }

    // For every node, identified by its address: its key and the addresses
    // of its left and right children.
    type Links = HashMap<*const Node, (i32, Option<*const Node>, Option<*const Node>)>;

    fn links(tree: &Bst) -> Links {
        let mut out = HashMap::new();
        let mut stack: Vec<&Node> = tree.root.as_deref().into_iter().collect();
        while let Some(node) = stack.pop() {
            let left = node.left.as_deref().map(ptr::from_ref);
            let right = node.right.as_deref().map(ptr::from_ref);
            out.insert(ptr::from_ref(node), (node.key, left, right));
            stack.extend(node.left.as_deref());
            stack.extend(node.right.as_deref());
        }
        out
    }

    // ---- height ----

    #[test]
    fn height_of_empty_and_single_node_trees() {
        assert_eq!(Bst::new().height(), 0);
        assert_eq!(from_keys([7]).height(), 1);
        assert_eq!(from_keys([7, 3]).height(), 2);
        assert_eq!(from_keys([7, 9]).height(), 2);
        assert_eq!(from_keys([2, 1, 3]).height(), 2);
    }

    #[test]
    fn height_is_the_longest_path_wherever_it_is() {
        // The leftmost and the rightmost paths have 2 nodes each; the longest
        // one runs down the middle: 50 -> 20 -> 30 -> 25 -> 27.
        let middle = from_keys([50, 20, 80, 30, 25, 27]);
        assert_eq!(middle.height(), 5);
        // Longest on the left: 5 -> 3 -> 1 -> 2.
        assert_eq!(from_keys([5, 3, 8, 1, 4, 2]).height(), 4);
        // Longest on the right: 5 -> 8 -> 9 -> 10.
        assert_eq!(from_keys([5, 3, 8, 7, 9, 10]).height(), 4);
    }

    #[test]
    fn height_of_a_perfect_tree() {
        // Level by level, each level's keys halfway between the keys above
        // them: 2^10 - 1 = 1023 nodes in 10 full levels.
        let keys = (0..10).flat_map(|level| {
            let step = 1 << (10 - level);
            (0..1 << level).map(move |i| (2 * i + 1) * step / 2)
        });
        let tree = from_keys(keys);
        assert_eq!(tree.len(), 1023);
        assert_eq!(tree.height(), 10);
    }

    // ---- invert ----

    #[test]
    fn inverts_the_leetcode_226_example() {
        //      4               4
        //    /   \           /   \
        //   2     7   ==>   7     2
        //  / \   / \       / \   / \
        // 1   3 6   9     9   6 3   1
        let mut tree = from_keys([4, 2, 7, 1, 3, 6, 9]);
        tree.invert();
        assert_eq!(preorder(&tree), [4, 7, 9, 6, 2, 3, 1]);
        assert_eq!(inorder(&tree), [9, 7, 6, 4, 3, 2, 1]);
        assert_eq!(tree.len(), 7);
    }

    #[test]
    fn inverts_every_level_of_a_lopsided_tree() {
        let mut tree = from_keys([50, 20, 80, 30, 25, 27, 10, 90, 85]);
        tree.invert();
        // The mirror image, level by level: 50's children are 80 (with 90,
        // which has 85 on its right now) and 20 (30 on the left, 10 on the
        // right), and so on down to 27.
        assert_eq!(preorder(&tree), [50, 80, 90, 85, 20, 30, 25, 27, 10]);
        assert_eq!(inorder(&tree), [90, 85, 80, 50, 30, 27, 25, 20, 10]);
    }

    #[test]
    fn invert_relinks_the_same_nodes() {
        let mut tree = from_keys([8, 4, 12, 2, 6, 10, 14, 1, 7, 13]);
        let before = links(&tree);
        tree.invert();
        let after = links(&tree);
        // Exactly the same nodes, each keeping its key, with its two children
        // swapped. Rebuilding the tree, or moving keys between nodes, fails
        // this.
        assert_eq!(after.len(), before.len());
        for (node, &(key, left, right)) in &before {
            assert_eq!(
                after.get(node),
                Some(&(key, right, left)),
                "the node holding {key} was not relinked in place"
            );
        }
    }

    #[test]
    fn invert_on_empty_and_single_node_trees() {
        let mut empty = Bst::new();
        empty.invert();
        assert!(empty.root.is_none());

        let mut one = from_keys([5]);
        let node = one.root.as_deref().map(ptr::from_ref);
        one.invert();
        assert_eq!(one.root.as_deref().map(ptr::from_ref), node);
        assert_eq!(preorder(&one), [5]);
    }

    #[test]
    fn inverting_twice_restores_the_tree() {
        let mut tree = from_keys([40, 20, 60, 10, 30, 50, 70, 5, 35, 65]);
        let before = links(&tree);
        let shape = preorder(&tree);
        tree.invert();
        assert_ne!(preorder(&tree), shape);
        tree.invert();
        assert_eq!(preorder(&tree), shape);
        assert_eq!(links(&tree), before);
        // A BST again: `insert` finds the right places.
        assert!(tree.insert(45));
        assert!(!tree.insert(65));
        assert_eq!(inorder(&tree), [5, 10, 20, 30, 35, 40, 45, 50, 60, 65, 70]);
    }

    #[test]
    fn invert_keeps_the_height() {
        let mut tree = from_keys([50, 20, 80, 30, 25, 27, 10]);
        tree.invert();
        assert_eq!(tree.height(), 5);
    }

    // ---- both, on a 200_000-deep tree ----

    #[test]
    fn a_200_000_deep_tree_needs_an_explicit_stack() {
        const N: i32 = 200_000;
        let keys = zigzag_keys(N);
        let mut tree = path(&keys);
        assert_eq!(tree.height(), 200_000);

        tree.invert();
        // Mirrored: a zigzag that turns the other way, with descending
        // in-order keys.
        let mut ascending: Vec<i32> = (0..N).step_by(2).chain((N + 1..2 * N).step_by(2)).collect();
        assert!(
            preorder(&tree) == keys,
            "a path keeps its pre-order when it is mirrored"
        );
        ascending.reverse();
        assert!(
            inorder(&tree) == ascending,
            "the in-order walk is not descending"
        );
        assert_eq!(tree.height(), 200_000);

        tree.invert();
        ascending.reverse();
        assert!(
            inorder(&tree) == ascending,
            "inverting twice is not the identity"
        );
    }
}
