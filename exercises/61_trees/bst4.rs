// Module 2 · Binary search trees — part 4: `find_slot` returns its cursor from a loop, then `remove` with `take_min` (E0499, E0308).
//
// Deleting from a BST is the operation interviewers save for last, because it
// has three cases. A leaf just goes. A node with one child is replaced by that
// child. A node with two children is replaced by its in-order SUCCESSOR, the
// smallest key of its right subtree: that node is unlinked from where it is
// (it has no left child, so its right subtree takes its place) and then takes
// over both subtrees of the removed node. The textbook C version tracks a
// parent pointer and a "left or right child?" flag for each of those relinks.
// The Rust version uses the link cursor of part 1 again: `find_slot(key)`
// returns a `&mut Option<Box<Node>>`, the link that holds `key` (or the empty
// link where `key` would go), and `Option::take` lifts the node out of it.
// `insert` is a few lines on top of `find_slot` (given below), and so is most
// of `remove`.
//
// The natural `find_slot` does not compile. In its loop, the pattern match
// `while let Some(node) = cur` takes a MUTABLE borrow of `*cur`. On the path
// that keeps looping, that borrow flows into `cur` itself, through
// `cur = &mut node.left`, and `cur` is what the function returns, so the
// borrow must last for the caller's lifetime `'1`. NLL's constraints are
// location-insensitive, so that requirement also holds on the path that stops
// and returns `cur` early, where returning it reborrows `*cur` a second time:
// E0499. That is NLL problem case #3 from `37_borrowck_errors/borrowck3`
// (which explains it in full), this time inside a loop, where there is no
// entry API and no index to fall back on.
//
// What works is deciding with a SHARED borrow that ends as soon as the
// decision is made, and taking the mutable borrow only on the path that steps
// the cursor. The rewrites that look equivalent do not help: a `match` guard,
// a let chain (`while let Some(node) = cur && node.key != key`) and a `break`
// in place of the `return` all take the mutable borrow BEFORE the test, and
// all fail with the same E0499. Returning the cursor is not even necessary:
// once a mutable borrow taken in the loop flows into `cur` on one path, any
// other path that took the same borrow and then uses `*cur` again is
// rejected, as you may find out in `take_min`. (Part 1's `insert` is fine:
// its loop ends when the `while let` pattern fails to match, and a failed
// match borrows nothing.)
//
// On nightly, the original Polonius (`-Zpolonius`) accepts this `find_slot`
// unchanged. The newer implementation behind `-Zpolonius=next` accepts
// `borrowck3`'s straight-line version but, as of nightly 1.99 (mid-2026),
// still rejects this loop. On stable you restructure, and an interviewer who
// asks about it wants to hear why the loop is rejected and how the rewrite
// avoids it.
//
// One more requirement: `remove` relinks NODES instead of moving keys. The C
// textbook shortcut for two children copies the successor's key into the
// doomed node and frees the successor's node instead. In a map every node
// also holds a value, and relinking moves a few pointers however big that
// value is; with generic keys you could not simply copy one anyway. So here
// the node that held `key` is the one freed, and every other key stays in its
// own node. The tests compare node addresses.

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

    // From part 1: a read-only walk down one path.
    fn contains(&self, key: i32) -> bool {
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

    // Returns the link that holds `key`, or, if `key` is absent, the empty
    // link where it would be inserted.
    fn find_slot(&mut self, key: i32) -> &mut Option<Box<Node>> {
        // TODO: rustc rejects this loop with E0499 "cannot borrow `*cur` as
        // mutable more than once at a time" at `return cur`, with the note
        // "returning this value requires that `cur.0` is borrowed for `'1`".
        // The mutable borrow that `while let Some(node) = cur` takes flows
        // into `cur` when the loop steps, and `cur` is returned, so it must
        // last for the caller's `'1`: it is still live on the early-return
        // path. Restructure the loop so that no mutable borrow of `*cur` has
        // to survive the early return. Requirements:
        //   - keep the signature and the behavior: on a hit, the link that
        //     holds `key`; on a miss, the empty link where `key` belongs (the
        //     tests compare addresses);
        //   - iterative, O(1) extra space: no recursion (one test searches a
        //     200_000-deep tree);
        //   - no `unsafe`, no `Rc`/`RefCell`, and don't change the tests.
        // Until you restructure the loop so that no mutable borrow of `*cur`
        // spans the early return, this exercise will not compile.
        let mut cur = &mut self.root;
        while let Some(node) = cur {
            if key == node.key {
                return cur;
            }
            cur = if key < node.key {
                &mut node.left
            } else {
                &mut node.right
            };
        }
        cur
    }

    // Inserts `key` and returns `true`, or returns `false` and changes nothing
    // if `key` is already there. The slot for a new key is the empty link
    // where it belongs.
    fn insert(&mut self, key: i32) -> bool {
        let slot = self.find_slot(key);
        if slot.is_some() {
            return false;
        }
        *slot = Some(Box::new(Node::new(key)));
        self.len += 1;
        true
    }

    // Removes `key` and returns `true`, or returns `false` and changes nothing
    // if `key` is absent.
    fn remove(&mut self, key: i32) -> bool {
        // TODO: E0308 "mismatched types": expected `bool`, found `()` (rustc
        // reports it together with the E0499 above). Take the node out of
        // its slot and put the right replacement in its place. Requirements:
        //   - a leaf just goes; a node with one child is replaced by that
        //     child; a node with two children is replaced by its in-order
        //     SUCCESSOR (the smallest key of its right subtree, detached with
        //     `take_min`), which takes over both of the removed node's
        //     subtrees. The tests check the exact shape;
        //   - relink nodes, don't move keys: the node that held `key` is the
        //     one freed, and every other key stays in its own node (the tests
        //     compare node addresses);
        //   - `len` goes down by one; a missing key returns `false` without
        //     panicking;
        //   - iterative, O(h) time, O(1) extra space; no `unsafe`, and don't
        //     change the tests.
        // Until you return a `bool`, this exercise will not compile.
    }
}

// Detaches the node with the smallest key from the subtree in `slot` and
// returns it with both links empty. Its right subtree (it has no left one)
// takes its place. An empty subtree gives `None`.
fn take_min(slot: &mut Option<Box<Node>>) -> Option<Box<Node>> {
    // TODO: E0308 "mismatched types": expected `Option<Box<Node>>`, found
    // `()`. Requirements:
    //   - follow the LEFT links down to the smallest node, unlink it, and put
    //     its right subtree in its place;
    //   - return that node itself, not a copy (the tests compare addresses),
    //     with `left` and `right` both `None`;
    //   - iterative: in one test the smallest node is 200_000 levels down.
    //     The walk has the shape of `find_slot`, and it hits the same E0499
    //     (or an E0506 "cannot assign to `*cur` because it is borrowed") if
    //     the loop's test takes a mutable borrow.
    // Until you return an `Option<Box<Node>>`, this exercise will not
    // compile.
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
    use std::collections::{BTreeSet, HashMap};
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

    // The keys in ascending order (the iterator of part 2, as a loop).
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

    // The address of the node that holds each key.
    fn addresses(tree: &Bst) -> HashMap<i32, *const Node> {
        let mut out = HashMap::new();
        let mut stack: Vec<&Node> = tree.root.as_deref().into_iter().collect();
        while let Some(node) = stack.pop() {
            out.insert(node.key, ptr::from_ref(node));
            stack.extend(node.left.as_deref());
            stack.extend(node.right.as_deref());
        }
        out
    }

    // Every key still in `tree` must be in the node that held it before.
    fn assert_same_nodes(tree: &Bst, before: &HashMap<i32, *const Node>) {
        for (key, node) in addresses(tree) {
            assert_eq!(
                before.get(&key),
                Some(&node),
                "{key} is not in the node that held it before: relink the \
                 nodes instead of moving keys"
            );
        }
    }

    // The address of the node that holds `key`, found with a loop.
    fn node_of(tree: &Bst, key: i32) -> Option<*const Node> {
        let mut cur = tree.root.as_deref();
        while let Some(node) = cur {
            if key == node.key {
                return Some(ptr::from_ref(node));
            }
            cur = if key < node.key {
                node.left.as_deref()
            } else {
                node.right.as_deref()
            };
        }
        None
    }

    //          50
    //        /    \
    //      30      70
    //     /  \    /  \
    //    20  40  60  80
    //        /     \   \
    //       35     65  85
    fn sample() -> Bst {
        from_keys([50, 30, 70, 20, 40, 60, 80, 35, 65, 85])
    }

    // ---- find_slot and take_min ----

    #[test]
    fn find_slot_on_a_hit_returns_the_link_that_holds_the_key() {
        let mut tree = sample();
        let before = addresses(&tree);
        for key in [50, 30, 70, 20, 40, 60, 80, 35, 65, 85] {
            let node = tree.find_slot(key).as_deref().map(ptr::from_ref);
            assert_eq!(node, Some(before[&key]), "find_slot({key})");
        }
        // The root's slot is the root link itself.
        let root_link = ptr::from_ref(&tree.root);
        assert!(ptr::eq(tree.find_slot(50), root_link));
    }

    #[test]
    fn find_slot_on_a_miss_returns_the_empty_link_where_the_key_belongs() {
        let mut tree = from_keys([50, 30, 70, 20, 40]);
        for key in [35, 10, 45, 99, 60] {
            let slot = tree.find_slot(key);
            assert!(slot.is_none(), "find_slot({key}) on a missing key");
            *slot = Some(Box::new(Node::new(key)));
        }
        // Each new node hangs exactly where a search for it ends.
        assert_eq!(preorder(&tree), [50, 30, 20, 10, 40, 35, 45, 70, 60, 99]);

        let mut empty = Bst::new();
        let root_link = ptr::from_ref(&empty.root);
        assert!(ptr::eq(empty.find_slot(7), root_link));
    }

    #[test]
    fn insert_on_top_of_find_slot() {
        let mut tree = Bst::new();
        for key in [8, 3, 10, 1, 6, 14, 4, 7, 13, i32::MIN, i32::MAX] {
            assert!(tree.insert(key), "insert({key})");
        }
        for key in [8, 6, 13, i32::MAX] {
            assert!(!tree.insert(key), "insert({key}) a second time");
        }
        assert_eq!(tree.len(), 11);
        assert_eq!(
            preorder(&tree),
            [8, 3, 1, i32::MIN, 6, 4, 7, 10, 14, 13, i32::MAX]
        );
    }

    #[test]
    fn take_min_detaches_the_smallest_node() {
        //        50
        //      /    \
        //    30      70
        //   /  \
        //  20   40
        //    \
        //     25
        let mut tree = from_keys([50, 30, 70, 20, 40, 25]);
        let before = addresses(&tree);
        let min = take_min(&mut tree.root).expect("the tree is not empty");
        assert_eq!(min.key, 20);
        assert!(
            ptr::eq(&*min, before[&20]),
            "take_min returned a new node, not the one that held 20"
        );
        assert!(min.left.is_none() && min.right.is_none());
        // 25 took 20's place.
        assert_eq!(preorder(&tree), [50, 30, 25, 40, 70]);
        assert_same_nodes(&tree, &before);
    }

    #[test]
    fn take_min_when_the_subtree_root_is_the_smallest() {
        let mut tree = from_keys([10, 20, 15, 30]);
        let min = take_min(&mut tree.root).expect("the tree is not empty");
        assert_eq!(min.key, 10);
        assert!(min.left.is_none() && min.right.is_none());
        assert_eq!(preorder(&tree), [20, 15, 30]);

        // A subtree slot works the same way: 30 is a leaf.
        let root = tree.root.as_mut().expect("20 is the root now");
        let min = take_min(&mut root.right).expect("20 has a right child");
        assert_eq!(min.key, 30);
        assert_eq!(preorder(&tree), [20, 15]);

        assert!(take_min(&mut None).is_none());
    }

    // ---- remove ----

    #[test]
    fn remove_a_leaf() {
        let mut tree = sample();
        let before = addresses(&tree);
        assert!(tree.remove(20));
        assert!(tree.remove(35));
        assert!(tree.remove(85));
        assert_eq!(tree.len(), 7);
        assert!(!tree.contains(20) && !tree.contains(35) && !tree.contains(85));
        assert_eq!(preorder(&tree), [50, 30, 40, 70, 60, 65, 80]);
        assert_same_nodes(&tree, &before);
    }

    #[test]
    fn remove_a_node_with_one_child() {
        // 40 has only a left child, 35, which takes its place.
        let mut tree = sample();
        let before = addresses(&tree);
        assert!(tree.remove(40));
        assert_eq!(preorder(&tree), [50, 30, 20, 35, 70, 60, 65, 80, 85]);
        assert_same_nodes(&tree, &before);

        // 60 and 80 have only a right child each.
        let mut tree = sample();
        let before = addresses(&tree);
        assert!(tree.remove(60));
        assert!(tree.remove(80));
        assert_eq!(preorder(&tree), [50, 30, 20, 40, 35, 70, 65, 85]);
        assert_eq!(tree.len(), 8);
        assert_same_nodes(&tree, &before);

        // The only child has children of its own: its whole subtree moves
        // up unchanged. Moving 20's successor (25) up in the first tree, or
        // 40's predecessor (35) in the second, would keep the keys sorted
        // too, but that is the two-children recipe, not this one.
        let mut tree = from_keys([10, 20, 30, 25, 35]);
        let before = addresses(&tree);
        assert!(tree.remove(20));
        assert_eq!(preorder(&tree), [10, 30, 25, 35]);
        assert_same_nodes(&tree, &before);

        let mut tree = from_keys([50, 40, 30, 20, 35]);
        let before = addresses(&tree);
        assert!(tree.remove(40));
        assert_eq!(preorder(&tree), [50, 30, 20, 35]);
        assert_same_nodes(&tree, &before);
    }

    #[test]
    fn remove_a_node_with_two_children() {
        // 30's successor is 35, the smallest key of its right subtree, found
        // one step down the left links from 40.
        let mut tree = sample();
        let before = addresses(&tree);
        assert!(tree.remove(30));
        assert_eq!(preorder(&tree), [50, 35, 20, 40, 70, 60, 65, 80, 85]);
        assert_same_nodes(&tree, &before);

        // 70's successor is its right child, 80, which has no left child: it
        // moves up, takes over 60, and keeps 85.
        let mut tree = sample();
        let before = addresses(&tree);
        assert!(tree.remove(70));
        assert_eq!(preorder(&tree), [50, 30, 20, 40, 35, 80, 60, 65, 85]);
        assert_eq!(tree.len(), 9);
        assert_same_nodes(&tree, &before);
    }

    #[test]
    fn remove_the_root() {
        // Two children, and a successor with a right child of its own: 55
        // becomes the root and 57 takes 55's old place.
        let mut tree = from_keys([50, 30, 70, 60, 80, 55, 65, 57]);
        let before = addresses(&tree);
        assert!(tree.remove(50));
        assert_eq!(preorder(&tree), [55, 30, 70, 60, 57, 65, 80]);
        assert_same_nodes(&tree, &before);

        // One child.
        let mut tree = from_keys([10, 5, 2]);
        assert!(tree.remove(10));
        assert_eq!(preorder(&tree), [5, 2]);

        // A single node.
        let mut tree = from_keys([7]);
        assert!(tree.remove(7));
        assert!(tree.root.is_none());
        assert_eq!(tree.len(), 0);
        assert!(!tree.remove(7));
    }

    #[test]
    fn removing_a_missing_key_changes_nothing() {
        let mut tree = sample();
        let before = addresses(&tree);
        let shape = preorder(&tree);
        for key in [33, 100, -1, 36, 64, i32::MIN, i32::MAX] {
            assert!(!tree.remove(key), "remove({key}) on a missing key");
        }
        assert_eq!(tree.len(), 10);
        assert_eq!(preorder(&tree), shape);
        assert_eq!(addresses(&tree), before);
        assert!(!Bst::new().remove(0));
    }

    #[test]
    fn removing_everything_empties_the_tree() {
        // 7919 and 101 do not divide 300, so both formulas visit every key in
        // 0..300 once, in two different scrambled orders.
        let mut tree = from_keys((0..300).map(|i| i * 7919 % 300));
        let mut model: BTreeSet<i32> = (0..300).collect();
        for i in 0..300 {
            let key = i * 101 % 300;
            let before = addresses(&tree);
            assert!(tree.remove(key), "remove({key})");
            model.remove(&key);
            assert!(!tree.contains(key));
            assert_eq!(tree.len(), model.len());
            assert!(inorder(&tree).iter().eq(&model), "after remove({key})");
            assert_same_nodes(&tree, &before);
        }
        assert!(tree.root.is_none());
        assert!(tree.insert(42));
        assert_eq!(preorder(&tree), [42]);
    }

    // A tiny deterministic pseudo-random generator (Knuth's MMIX LCG), so
    // every run replays exactly the same operations.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, bound: u32) -> i32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) % u64::from(bound)) as i32
        }
    }

    #[test]
    fn random_inserts_and_removes_match_a_btreeset() {
        let mut rng = Lcg(2024);
        let mut tree = Bst::new();
        let mut model = BTreeSet::new();
        for step in 0..3000 {
            // Only 48 distinct keys, so hits and misses both happen often.
            let key = rng.below(48);
            if rng.below(2) == 0 {
                assert_eq!(
                    tree.insert(key),
                    model.insert(key),
                    "step {step}: insert({key})"
                );
            } else {
                let before = addresses(&tree);
                assert_eq!(
                    tree.remove(key),
                    model.remove(&key),
                    "step {step}: remove({key})"
                );
                assert_same_nodes(&tree, &before);
            }
            assert_eq!(tree.len(), model.len(), "step {step}");
            assert!(inorder(&tree).iter().eq(&model), "step {step}");
        }
    }

    #[test]
    fn a_200_000_deep_tree_needs_loops_not_recursion() {
        const N: i32 = 200_000;
        // A path that turns at every step: node `i` holds `i` when `i` is even
        // and `2n - i` when `i` is odd. Its deepest node holds N + 1.
        let keys: Vec<i32> = (0..N)
            .map(|i| if i % 2 == 0 { i } else { 2 * N - i })
            .collect();
        let mut tree = path(&keys);
        assert!(tree.find_slot(N + 1).is_some());
        assert!(tree.find_slot(N).is_none());
        // A leaf at the very bottom, a miss just above it, and a node in the
        // middle of the path whose only child moves up.
        assert!(tree.remove(N + 1));
        assert!(!tree.remove(N - 1));
        assert!(tree.remove(100_000));
        assert_eq!(tree.len(), keys.len() - 2);
        let expected: Vec<i32> = keys
            .iter()
            .copied()
            .filter(|&key| key != N + 1 && key != 100_000)
            .collect();
        assert!(
            preorder(&tree) == expected,
            "wrong shape after the removals"
        );

        // The root has two children, and its successor, 1, is at the bottom
        // of a chain of 200_000 left links.
        let mut keys = vec![0];
        keys.extend((1..=N).rev());
        let mut tree = path(&keys);
        assert!(tree.insert(-1));
        let successor = node_of(&tree, 1);
        assert!(tree.remove(0));
        assert_eq!(tree.root.as_deref().map(ptr::from_ref), successor);
        assert_eq!(tree.len(), keys.len());
        assert!(
            inorder(&tree).into_iter().eq([-1].into_iter().chain(1..=N)),
            "wrong keys after removing the root"
        );
    }
}
