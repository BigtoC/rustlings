# Module 2 · Binary Search Trees: Link Cursors, a Lazy In-Order Iterator, No Recursion

> Module 2 (data structures), right after `60_lru_cache`. A binary search
> tree on `Option<Box<Node>>` is the tree question of Rust live-coding rounds:
> insert, search, iterate in order, invert, measure and delete, without `Rc`,
> `RefCell` or recursion. Every exercise has a test on a 200_000-deep tree,
> too deep for a recursive walk on a test thread's stack, so each one makes
> you write the loop. All **std**, **100% safe**, **stable** Rust, edition
> 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Press `h` when you want
> the full answer.

## Core Ideas

- **Ownership already is a tree.** Each node owns its two children through
  `Option<Box<Node>>`, the nullable owning pointer of `27_data_structures`.
  No node has two owners, so the tree needs no `Rc` and no `RefCell`. Parent
  pointers would give every node a second referrer; that is what the arena of
  `59_arena` is for.
- **A cursor points at a link, not at a node.** `cur: &mut Option<Box<Node>>`
  is the place where a node is, or would be. `insert` writes into the empty
  link where the search ends; `remove` takes the node out of its link and
  writes the replacement back. No parent pointer, no "left or right child?"
  flag.
- **Stepping a cursor compiles; returning it from the loop does not.**
  `cur = &mut node.left` is accepted because assigning to `cur` ends the loans
  on the old `*cur` (problem case #4 of the NLL RFC). `return cur` inside
  `while let Some(node) = cur` is problem case #3, the one from
  `37_borrowck_errors/borrowck3`, inside a loop: E0499.
- **A lazy iterator keeps the recursion's stack in a field.**
  `InOrder<'a> { stack: Vec<&'a Node> }` holds the pending left chain: O(h)
  memory and O(1) amortized time per `next`. Its items are `&'a i32` and
  borrow the tree, not the iterator.
- **Several `&mut` into one tree are fine when they are disjoint.** A
  `Vec<&mut Node>` holds nodes that were split off their parents through two
  different fields, `left` and `right`.
- **Depth is the enemy.** Sorted insertion turns a BST into a linked list, so
  h = n. Every recursive walk needs one stack frame per level, and 200_000
  levels overflow the 2 MiB stack of a test thread, which aborts the whole test
  run with "has overflowed its stack". The drop glue the compiler derives is
  recursive too, so every exercise gives `Bst` an iterative `Drop`.

## Recursion vs. Loops

| Operation          | The recursive answer                       | The loop in this module                           | Extra space |
| ------------------ | ------------------------------------------ | ------------------------------------------------- | ----------- |
| `insert`, search   | recurse into one child                     | a cursor walks one path                           | O(1)        |
| in-order iteration | left subtree, node, right subtree          | a stack of `&Node` holding the pending left chain | O(h)        |
| `invert`           | swap the children, recurse into both       | a stack of `&mut Node`                            | O(h)        |
| `height`           | 1 + the larger of the two subtree heights  | breadth-first, one level at a time                | O(width)    |
| `remove`           | recurse to the node, then to its successor | `find_slot`, then `take_min`                      | O(1)        |
| drop               | the derived drop glue                      | an explicit stack of `Box<Node>` (given code)     | O(h)        |

A follow-up question sometimes asks for the drop in O(1) extra space. Rotate
the tree as you go, so that the node you free never has a left child:

```rust
impl Drop for Bst {
    fn drop(&mut self) {
        let mut cur = self.root.take();
        while let Some(mut node) = cur {
            if let Some(mut left) = node.left.take() {
                // Rotate right: `left` moves up, `node` becomes its right
                // child. Nothing is freed in this step.
                node.left = left.right.take();
                left.right = Some(node);
                cur = Some(left);
            } else {
                // No left child: free `node` and go on with its right one.
                cur = node.right.take();
            }
        }
    }
}
```

## The Loop That Does Not Compile

`bst4` starts from the `find_slot` everyone writes first:

```rust
fn find_slot(&mut self, key: i32) -> &mut Option<Box<Node>> {
    let mut cur = &mut self.root;
    while let Some(node) = cur {
        if key == node.key {
            return cur; // E0499
        }
        cur = if key < node.key { &mut node.left } else { &mut node.right };
    }
    cur
}
```

The mutable borrow taken by `while let Some(node) = cur` flows into `cur` on
the path that keeps looping, and `cur` is returned, so the borrow must last
for the caller's lifetime. NLL's constraints are location-insensitive: the
borrow also counts as live on the path that returns early, where returning
`cur` borrows `*cur` again. `borrowck3` has the full story. What stable Rust
accepts is a loop that decides with a **shared** borrow, which ends as soon as
the decision is made, and takes the **mutable** borrow only on the path that
steps the cursor. The rewrites that look equivalent do not help, because
each of them still takes the mutable borrow before the test (all checked on
rustc 1.96):

| Rewrite                                                              | Result          |
| -------------------------------------------------------------------- | --------------- |
| `break` instead of `return cur`, then `cur` after the loop           | E0499           |
| `match cur { Some(node) if node.key != key => .., _ => return cur }` | E0499           |
| a let chain: `while let Some(node) = cur && node.key != key`         | E0499           |
| a `take_min` loop that `break`s and then calls `cur.take()`          | E0499 and E0506 |

The last row shows that returning the cursor is not what matters: once a
mutable borrow taken in the loop flows into `cur` on one path, any other path
that took the same borrow and then uses `*cur` again is rejected. (`bst1`'s
`insert` is fine: its loop ends when the `while let` pattern fails to match,
and a failed match borrows nothing.)

On nightly, the original Polonius (`-Zpolonius`) accepts all of these. The
newer implementation behind `-Zpolonius=next` accepts `borrowck3`'s
straight-line functions but, as of nightly 1.99 (mid-2026), still rejects the
loops.

## Exercise Path

1. **bst1** — Two empty bodies (E0308). Write `insert` with a `&mut` cursor
   that stops at the empty link where the key belongs (a duplicate returns
   `false` and changes nothing), and `contains` as a read-only loop down one
   path (a test hides a key off that path). The tests check the exact shape of
   the tree, use `i32::MIN` and `i32::MAX` as keys (no comparing by
   subtraction), and search and insert on a 200_000-deep zigzag path.
2. **bst2** — `iter` and `InOrder::next` are empty (E0308). Keep the pending
   left chain on the `Vec<&'a Node>` stack. The stack may never hold more nodes
   than the tree is tall (checked on a 1023-node tree), every item must point
   at the key inside its node, and a 200_000-deep chain of left children rules
   out a recursive helper. `IntoIterator for &Bst` is given, so
   `for key in &tree` works.
3. **bst3** — `height` is empty (E0308), and `invert` compiles but does
   nothing, so its tests fail. Invert with a stack of `&mut Node`, relinking the
   same nodes (the tests compare every node's links by address), and measure
   the height breadth-first. Both run on the 200_000-deep tree.
4. **bst4** — `find_slot` returns its cursor from inside the loop (E0499).
   Restructure it, then write `take_min` and `remove` (E0308) for leaves,
   nodes with one child and nodes with two. The successor NODE moves into the
   removed node's place, every other key stays in its own node, and the
   last test finds a successor 200_000 left links down.

Related:

- `27_data_structures/linkedlist3` introduces the `&mut` tail cursor, and
  `linkedlist1` the iterative `Drop`.
- `37_borrowck_errors/borrowck3` is NLL problem case #3 without the loop, and
  `24_ownership_model/ownership3` shows NLL ending a borrow at its last use.
- LeetCode's `Option<Rc<RefCell<TreeNode>>>` signature was left out of this
  module; its run-time borrow traps are in `31_debugging/debugging6` and
  `debugging7`.
- A generic `Bst<K: Ord>` would rely on the `Ord` contract of
  `44_trait_contracts`.
- `62_graphs` turns recursive graph walks into loops with an explicit stack in
  the same way.
- A mutable iterator over the tree (`IterMut`) is the harder counterpart of
  `bst2`; it is an unbuilt proposal under "Additional topics" in
  `deep-dive/ROADMAP.md`.

## Further Reading

- [`Option::as_deref`](https://doc.rust-lang.org/std/option/enum.Option.html#method.as_deref), [`Option::as_deref_mut`](https://doc.rust-lang.org/std/option/enum.Option.html#method.as_deref_mut), [`Option::take`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take) and [`Option::is_some_and`](https://doc.rust-lang.org/std/option/enum.Option.html#method.is_some_and)
- [RFC 2094: non-lexical lifetimes](https://rust-lang.github.io/rfcs/2094-nll.html), especially "Problem case #3" and "Problem case #4: mutating `&mut` references"
- [Polonius, the next-generation borrow checker](https://rust-lang.github.io/polonius/)
- [Iter](https://rust-unofficial.github.io/too-many-lists/second-iter.html) and [IterMut](https://rust-unofficial.github.io/too-many-lists/second-iter-mut.html) in *Learning Rust With Entirely Too Many Linked Lists*
- [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html), whose docs explain why std uses a B-tree rather than a binary search tree, and [Rust Collections Case Study: BTreeMap](https://faultlore.com/blah/rust-btree-case/) (Aria Desires)
- [`std::collections::VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html), the queue for breadth-first walks
- LeetCode [700. Search in a Binary Search Tree](https://leetcode.com/problems/search-in-a-binary-search-tree/), [701. Insert into a Binary Search Tree](https://leetcode.com/problems/insert-into-a-binary-search-tree/), [173. Binary Search Tree Iterator](https://leetcode.com/problems/binary-search-tree-iterator/), [226. Invert Binary Tree](https://leetcode.com/problems/invert-binary-tree/), [104. Maximum Depth of Binary Tree](https://leetcode.com/problems/maximum-depth-of-binary-tree/) and [450. Delete Node in a BST](https://leetcode.com/problems/delete-node-in-a-bst/)
