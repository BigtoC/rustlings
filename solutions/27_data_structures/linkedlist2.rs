// Module 2 · Data structures — part 4: reverse a LeetCode `ListNode` list in place.
//
// "Reverse a linked list" is the standard warm-up in Rust live-coding rounds,
// and it is asked precisely because the C answer does not carry over. In C
// you shuffle three raw pointers (`prev`, `cur`, `next`) that alias the same
// nodes. In Rust every node has exactly ONE owner: `head` owns the first node,
// the first node's `next` owns the second, and so on. So reversal is a
// sequence of MOVES, not pointer writes: detach the rest of the list from the
// current node, point the node at the part you have already reversed, and
// that node becomes the new front of the reversed part. Nothing is copied and
// nothing is allocated: O(n) time, O(1) extra space.
//
// `reverse` takes the list BY VALUE, so you own every node and may move out
// of any of them. That is the difference from `linkedlist1`'s `pop`, which
// only had `&mut self` and needed `Option::take` to get the head out of a
// borrow. `take` is still the tidiest way to say "hand me this link's
// contents and leave `None` behind" while you rewire a node, and
// `std::mem::replace` does the same but leaves a value of your choice.
//
// `ListNode` is LeetCode's node (problem 206 uses exactly this shape), with
// two changes:
//
//   - It does not derive `Clone` (LeetCode's does). Copying nodes is never
//     the answer to an ownership puzzle.
//   - `ListNode::new` stamps every node with a serial number, so the tests
//     can check that your result is made of the ORIGINAL nodes. Comparing
//     node addresses alone would not prove it: once your code frees a node,
//     the allocator may hand that same address to the next `Box::new`, and a
//     list rebuilt from copied values can land exactly on the old addresses.
//     The field is private to `mod list`, so no code outside it can copy or
//     forge a serial.
//
// How interviewers probe it: "What is the space complexity?" (O(1) when you
// relink; a `Vec` of values or a recursive helper is O(n)). "Can you do it
// recursively?" (yes, but with one stack frame per node: the last test
// reverses 200_000 nodes, which overflows a 2 MiB test-thread stack under
// recursion). The tests free that long list with an iterative helper, because
// the derived drop glue is recursive too (`linkedlist1` covers why).

mod list {
    use std::sync::atomic::{AtomicU64, Ordering};

    // Where serial numbers come from. `Relaxed` is enough: all we need is
    // that no two calls to `fetch_add` return the same number.
    static NEXT_SERIAL: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug)]
    pub struct ListNode {
        pub val: i32,
        pub next: Option<Box<ListNode>>,
        serial: u64,
    }

    impl ListNode {
        pub fn new(val: i32) -> Self {
            ListNode {
                val,
                next: None,
                serial: NEXT_SERIAL.fetch_add(1, Ordering::Relaxed),
            }
        }

        // Which `ListNode::new` call created this node.
        pub fn serial(&self) -> u64 {
            self.serial
        }
    }
}

use list::ListNode;

// Reverses the list and returns its new head (the old last node).
fn reverse(head: Option<Box<ListNode>>) -> Option<Box<ListNode>> {
    // `prev` is the reversed front part, `cur` the rest of the input. Each
    // round moves one node from `cur` to `prev`: `take` detaches the node's
    // tail into `cur`, the node is pointed at `prev`, and it becomes the new
    // `prev`. Every step moves a `Box` (a pointer), so the nodes never change
    // address and nothing is allocated or freed. The loop keeps two locals,
    // whatever the length: O(1) extra space and no recursion.
    let mut prev = None;
    let mut cur = head;
    while let Some(mut node) = cur {
        cur = node.next.take();
        node.next = prev;
        prev = Some(node);
    }
    prev
}

// Helpers for the tests (and for experimenting in `main`).

// Builds a list holding `vals` in order. It works back to front, so each new
// node can take ownership of the list built so far.
fn from_slice(vals: &[i32]) -> Option<Box<ListNode>> {
    let mut head = None;
    for &val in vals.iter().rev() {
        let mut node = Box::new(ListNode::new(val));
        node.next = head;
        head = Some(node);
    }
    head
}

// The values front to back. It only borrows the list.
fn values(list: &Option<Box<ListNode>>) -> Vec<i32> {
    let mut out = Vec::new();
    let mut cur = list.as_deref();
    while let Some(node) = cur {
        out.push(node.val);
        cur = node.next.as_deref();
    }
    out
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // The (serial, address) of every node, front to back. A node keeps both
    // for its whole life: moving a `Box` moves the pointer, never the node.
    fn identity(list: &Option<Box<ListNode>>) -> Vec<(u64, *const ListNode)> {
        let mut out = Vec::new();
        let mut cur = list.as_deref();
        while let Some(node) = cur {
            out.push((node.serial(), std::ptr::from_ref(node)));
            cur = node.next.as_deref();
        }
        out
    }

    // Frees a list one node at a time, so a long list does not overflow the
    // stack through the recursive drop glue.
    fn drop_iteratively(mut list: Option<Box<ListNode>>) {
        while let Some(mut node) = list {
            list = node.next.take();
        }
    }

    #[test]
    fn reverses_three_values() {
        let reversed = reverse(from_slice(&[1, 2, 3]));
        assert_eq!(values(&reversed), [3, 2, 1]);
    }

    #[test]
    fn relinks_the_original_nodes_instead_of_copying_them() {
        let list = from_slice(&[10, 20, 30, 40, 50]);
        let mut expected = identity(&list);
        expected.reverse();
        let reversed = reverse(list);
        assert_eq!(values(&reversed), [50, 40, 30, 20, 10]);
        // The old tail is the new head, the old head the new tail, and every
        // node in between is the same node as before, only relinked. A list
        // rebuilt from the values would have new serials.
        assert_eq!(identity(&reversed), expected);
    }

    #[test]
    fn empty_list_stays_empty() {
        assert!(reverse(None).is_none());
    }

    #[test]
    fn single_node_comes_back_unchanged() {
        let list = from_slice(&[7]);
        let before = identity(&list);
        let reversed = reverse(list);
        assert_eq!(values(&reversed), [7]);
        assert_eq!(identity(&reversed), before);
    }

    #[test]
    fn two_nodes_swap_places() {
        let list = from_slice(&[1, 2]);
        let before = identity(&list);
        let reversed = reverse(list);
        assert_eq!(values(&reversed), [2, 1]);
        assert_eq!(identity(&reversed), [before[1], before[0]]);
    }

    #[test]
    fn reversing_twice_gives_back_the_same_nodes() {
        // Duplicates and negative values. Swapping VALUES along the list gets
        // the numbers right both times, but after the first reversal its nodes
        // would still be in their old order.
        let list = from_slice(&[3, -1, 3, 0, 8]);
        let before = identity(&list);
        let once = reverse(list);
        assert_eq!(values(&once), [8, 0, 3, -1, 3]);
        let reversed_ids: Vec<_> = before.iter().rev().copied().collect();
        assert_eq!(identity(&once), reversed_ids);
        // The result is a well-formed list that can be reversed again.
        let twice = reverse(once);
        assert_eq!(values(&twice), [3, -1, 3, 0, 8]);
        assert_eq!(identity(&twice), before);
    }

    #[test]
    fn reverses_a_long_list_without_recursion() {
        const LEN: i32 = 200_000;
        let list = from_slice(&(0..LEN).collect::<Vec<_>>());
        let before = identity(&list);
        let reversed = reverse(list);
        let after = identity(&reversed);
        let vals = values(&reversed);
        drop_iteratively(reversed);
        assert!(
            vals.iter().copied().eq((0..LEN).rev()),
            "the values are not in reverse order"
        );
        assert!(
            after.iter().eq(before.iter().rev()),
            "the result is not made of the original nodes"
        );
    }
}
