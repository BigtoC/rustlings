// Module 2 · Binary search trees — part 2: a lazy in-order iterator that borrows the tree (E0308).
//
// An in-order walk (left subtree, node, right subtree) visits the keys of a
// BST in ascending order. The recursive walk is four lines, but it hands every
// key to a `Vec` or a callback in one go. An ITERATOR has to be lazy: each
// `next()` produces one key and returns. So the state that the recursion kept
// on the call stack, "which ancestors still have to be visited", has to live
// in the iterator itself. Here it is `stack: Vec<&'a Node>`, the nodes whose
// keys are still to come, with the NEXT key on top. That is LeetCode 173,
// "Binary Search Tree Iterator".
//
// Two facts make it work. The smallest key of any subtree sits at the end of
// its chain of LEFT children. And the key that comes right after a node's key
// is the smallest key of that node's right subtree. Done right, every node is
// pushed and popped exactly once, so a full walk is O(n) and each `next()` is
// O(1) amortized, and the stack never holds more than one node per level:
// O(h) memory. The eager version, a `Vec` of all the keys, costs O(n) memory
// and walks the whole tree even when the caller only wants `take(3)`.
// Interviewers ask exactly that: "what does your iterator cost in memory?"
//
// The lifetime is the other half of the question. `InOrder<'a>` holds
// `&'a Node` references and yields `&'a i32`: the items borrow the TREE, not
// the iterator. So `let smallest = tree.iter().next();` stays valid after the
// temporary iterator is gone, and two iterators can walk the same tree at the
// same time. And while any iterator or item is alive, the tree is
// SHARED-borrowed, so calling `insert` (which takes `&mut self`) in a loop over
// the tree is E0502: the bug class that other languages meet as iterator
// invalidation or a `ConcurrentModificationException` at run time is ruled
// out by the types. This is the first borrowing iterator over a container of
// your own in the course. The std collections have the same shape: an
// `iter(&self)` that returns an `Iter<'_, T>`, plus an `IntoIterator` impl for
// `&Collection` (given below for `&Bst`).
//
// `Option::as_deref` does the conversion you need at every step: it turns a
// `&'a Option<Box<Node>>` into an `Option<&'a Node>`, keeping the lifetime.

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

// The in-order iterator. It borrows the tree for `'a`.
struct InOrder<'a> {
    // The nodes whose keys are still to come. The top of the stack holds the
    // next key.
    stack: Vec<&'a Node>,
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

    // Returns an iterator over the keys in ascending order.
    fn iter(&self) -> InOrder<'_> {
        // Start with the root and its whole chain of left children: the
        // smallest key is at the bottom of that chain, so it ends up on top
        // of the stack. That is one node per level at most, and nothing to
        // the right of the chain has been touched yet.
        let mut iter = InOrder { stack: Vec::new() };
        iter.push_left_chain(self.root.as_deref());
        iter
    }
}

impl<'a> InOrder<'a> {
    // Pushes `node` and its chain of left children, a loop rather than
    // recursion. The parameter must be `Option<&'a Node>`, not
    // `Option<&Node>`: the nodes go into a `Vec<&'a Node>`, so they must be
    // borrowed for `'a`, not just for this call.
    fn push_left_chain(&mut self, mut node: Option<&'a Node>) {
        while let Some(n) = node {
            self.stack.push(n);
            node = n.left.as_deref();
        }
    }
}

impl<'a> Iterator for InOrder<'a> {
    type Item = &'a i32;

    fn next(&mut self) -> Option<Self::Item> {
        // The top node holds the smallest key not yet visited: everything in
        // its left subtree came out before it. The keys right after it are in
        // its right subtree, starting with that subtree's leftmost node, so
        // push the right child's left chain before returning. `?` ends the
        // walk (every later call too) once the stack is empty.
        let node = self.stack.pop()?;
        self.push_left_chain(node.right.as_deref());
        // `node` is a `&'a Node`, so `&node.key` is a `&'a i32`: it borrows
        // the tree, not `self`.
        Some(&node.key)
    }
}

// `for key in &tree` works like `for x in &vec`: a `for` loop calls
// `IntoIterator::into_iter` on what it is given, and this impl makes a shared
// borrow of a `Bst` iterable.
impl<'a> IntoIterator for &'a Bst {
    type Item = &'a i32;
    type IntoIter = InOrder<'a>;

    fn into_iter(self) -> InOrder<'a> {
        self.iter()
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
    use std::collections::HashSet;
    use std::ptr;

    fn from_keys(keys: impl IntoIterator<Item = i32>) -> Bst {
        let mut tree = Bst::new();
        for key in keys {
            assert!(tree.insert(key), "the test inserts {key} twice");
        }
        tree
    }

    // A perfect tree of height `h`: 2^h - 1 nodes holding 1..2^h. Inserting
    // level by level, each level's keys halfway between the keys above them,
    // fills every level completely.
    fn perfect(h: u32) -> Bst {
        from_keys((0..h).flat_map(|level| {
            let step = 1 << (h - level);
            (0..1 << level).map(move |i| (2 * i + 1) * step / 2)
        }))
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

    // The address of every key stored in the tree (a pre-order walk with an
    // explicit stack, so it works on deep trees too).
    fn key_addresses(tree: &Bst) -> HashSet<*const i32> {
        let mut out = HashSet::new();
        let mut stack: Vec<&Node> = tree.root.as_deref().into_iter().collect();
        while let Some(node) = stack.pop() {
            out.insert(ptr::from_ref(&node.key));
            stack.extend(node.left.as_deref());
            stack.extend(node.right.as_deref());
        }
        out
    }

    #[test]
    fn an_empty_tree_yields_nothing_ever() {
        let tree = Bst::new();
        let mut it = tree.iter();
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None);
        assert_eq!((&tree).into_iter().count(), 0);
    }

    #[test]
    fn keys_come_out_sorted() {
        // 7919 is prime, so `i * 7919 % 500` visits 0..500 in a scrambled
        // order.
        let tree = from_keys((0..500).map(|i| 3 * (i * 7919 % 500) - 700));
        let keys: Vec<i32> = tree.iter().copied().collect();
        let expected: Vec<i32> = (0..500).map(|i| 3 * i - 700).collect();
        assert_eq!(keys, expected);

        let small = from_keys([4, 2, 6, 1, 3, 5, 7, i32::MIN, i32::MAX]);
        let keys: Vec<i32> = small.iter().copied().collect();
        assert_eq!(keys, [i32::MIN, 1, 2, 3, 4, 5, 6, 7, i32::MAX]);
    }

    #[test]
    fn every_key_once_then_none_forever() {
        let tree = from_keys([2, 1, 3]);
        let mut it = tree.iter();
        assert_eq!(it.next(), Some(&1));
        assert_eq!(it.next(), Some(&2));
        assert_eq!(it.next(), Some(&3));
        for _ in 0..3 {
            assert_eq!(it.next(), None);
        }
    }

    #[test]
    fn take_3_gives_the_three_smallest_keys() {
        let tree = from_keys([50, 30, 70, 20, 40, 60, 80, 10, 25, 5]);
        let first: Vec<i32> = tree.iter().take(3).copied().collect();
        assert_eq!(first, [5, 10, 20]);
        let one = from_keys([9]);
        let first: Vec<i32> = one.iter().take(3).copied().collect();
        assert_eq!(first, [9]);
    }

    #[test]
    fn the_stack_never_holds_more_nodes_than_the_height() {
        // 1023 nodes, height 10.
        let tree = perfect(10);
        assert_eq!(tree.len(), 1023);
        let mut it = tree.iter();
        assert!(
            it.stack.len() <= 10,
            "a new iterator holds {} nodes: it walked the tree up front",
            it.stack.len()
        );
        let mut expected = 1;
        while let Some(&key) = it.next() {
            assert_eq!(key, expected);
            assert!(
                it.stack.len() <= 10,
                "after yielding {key} the stack holds {} nodes",
                it.stack.len()
            );
            expected += 1;
        }
        assert_eq!(expected, 1024);

        // `take` stops pulling: the rest of the walk never happens.
        let mut it = tree.iter();
        let first: Vec<i32> = it.by_ref().take(3).copied().collect();
        assert_eq!(first, [1, 2, 3]);
        assert!(it.stack.len() <= 10);
        assert_eq!(it.next(), Some(&4));
    }

    #[test]
    fn items_point_at_the_keys_inside_the_tree() {
        let tree = from_keys([8, 4, 12, 2, 6, 10, 14, 1]);
        let stored = key_addresses(&tree);
        let mut seen = HashSet::new();
        for key in tree.iter() {
            let address = ptr::from_ref(key);
            assert!(
                stored.contains(&address),
                "{key} is not a reference to a key inside the tree"
            );
            assert!(seen.insert(address), "{key} was yielded twice");
        }
        assert_eq!(seen.len(), tree.len());
    }

    #[test]
    fn items_borrow_the_tree_not_the_iterator() {
        let tree = from_keys([20, 10, 30]);
        // The iterator is a temporary that is gone at the end of each
        // statement; the items it returned are still valid.
        let smallest = tree.iter().next();
        let largest = tree.iter().last();
        assert_eq!(smallest, Some(&10));
        assert_eq!(largest, Some(&30));
        // `smallest` points at the key in the root's left child.
        let left = tree.root.as_ref().and_then(|root| root.left.as_deref());
        assert!(left.is_some_and(|node| smallest.is_some_and(|key| ptr::eq(key, &node.key))));
    }

    #[test]
    fn two_iterators_and_reads_while_iterating() {
        let tree = from_keys([5, 3, 8, 1, 4, 7, 9, 2, 6]);
        // Two shared borrows of the same tree, walking it side by side.
        assert!(tree.iter().zip(tree.iter().skip(1)).all(|(a, b)| a < b));
        // A `for` loop over `&tree`, calling a `&self` method in the body.
        let mut count = 0;
        for &key in &tree {
            assert!(tree.contains(key));
            count += 1;
        }
        assert_eq!(count, tree.len());
    }

    #[test]
    fn a_200_000_deep_left_chain() {
        // The root holds the largest key and every node has only a left
        // child, so the first key is 200_000 levels down.
        const N: i32 = 200_000;
        let keys: Vec<i32> = (0..N).rev().collect();
        let tree = path(&keys);
        let first: Vec<i32> = tree.iter().take(3).copied().collect();
        assert_eq!(first, [0, 1, 2]);
        assert!(tree.iter().copied().eq(0..N), "wrong keys or order");
    }

    #[test]
    fn a_200_000_deep_zigzag() {
        // A path that turns at every step: node `i` holds `i` when `i` is even
        // and `2n - i` when `i` is odd, so the small keys climb from 0 and the
        // large ones fall from 2n - 1.
        const N: i32 = 200_000;
        let keys: Vec<i32> = (0..N)
            .map(|i| if i % 2 == 0 { i } else { 2 * N - i })
            .collect();
        let tree = path(&keys);
        let expected = (0..N).step_by(2).chain((N + 1..2 * N).step_by(2));
        assert!(tree.iter().copied().eq(expected), "wrong keys or order");
    }
}
