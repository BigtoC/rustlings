// Module 2 · Data structures — part 6: split a list in half, remove the nth node from the end.
//
// Finding the middle of a linked list with "fast and slow pointers" (the
// tortoise and the hare) is a one-pass idiom every interviewer knows: `slow`
// steps one node, `fast` steps two, and when `fast` falls off the end, `slow`
// is at the middle. In C both are plain pointers into the same list. Ported
// to `Option<Box<ListNode>>` it stops compiling, and explaining why is the
// real question.
//
// To CUT the list after the middle, `slow` must be a `&mut` to a link: only a
// mutable borrow lets you `take()` the rest of the list out of it. But a
// `&mut` to a link is exclusive access to everything reachable from it, that
// is, the whole remainder of the list, including every node `fast` still has
// to read. `fast` is a shared borrow of those same nodes, and the loop uses
// both borrows in every round. That breaks aliasing XOR mutability, so rustc
// rejects it with E0502. It is not a borrow-checker false positive (unlike
// `37_borrowck_errors/borrowck3`): nothing in the types stops `slow` from
// cutting off and freeing the very nodes `fast` points into, so accepting the
// code would mean trusting the loop body instead of the types.
//
// The fix is to separate reading and writing IN TIME. Count the nodes with a
// shared cursor (that borrow ends with the loop), then walk ONE `&mut` cursor
// to the link you want and cut there. That is two passes instead of one, but
// still O(n) time and O(1) extra space, and it is the answer interviewers want
// to hear, together with the reason. (Keeping both cursors in one pass means
// two live pointers into the list, one of them writing. That is what raw
// pointers and `unsafe` are for, as in the `deep-dive/` list.)
//
// `remove_nth_from_end` (LeetCode 19) hides the same trap in another shape:
// the textbook answer runs a lead pointer `n` nodes ahead of a trailing one,
// which is again a `&` in front of a `&mut` over the same nodes. Count first
// instead. The node to remove then sits at index `len - n` from the front,
// but only when `n <= len`: a `usize` subtraction that underflows panics in a
// debug build ("attempt to subtract with overflow") and wraps around in a
// release build. `usize::checked_sub` returns `None` in that case, which turns
// the bad input into an ordinary branch.
//
// `ListNode` is the same as in `linkedlist2`: LeetCode's node without `Clone`,
// plus a private serial number from `ListNode::new`, so the tests can check
// that both functions move the original nodes around instead of rebuilding
// them.

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

// Cuts the list after its middle node and returns `(front, back)`. With an odd
// length the extra node stays in front: [1, 2, 3, 4, 5] -> ([1, 2, 3], [4, 5]).
fn split_half(mut head: Option<Box<ListNode>>) -> (Option<Box<ListNode>>, Option<Box<ListNode>>) {
    // TODO: rustc rejects this fast/slow walk with E0502 "cannot borrow
    // `head` as mutable because it is also borrowed as immutable". `fast` is
    // a shared borrow into the list, `slow` a mutable one, and the loop uses
    // both. A `&mut` to a link is exclusive access to every node after it,
    // which includes every node `fast` still has to read. Rewrite the body so
    // that reading the list and cutting it no longer overlap. Requirements:
    //   - keep the signature and the behavior described above (empty and
    //     one-node lists work too);
    //   - both halves are the original nodes, in order (the tests compare
    //     serials and addresses): no `ListNode::new`, no `Vec` of values or
    //     nodes, O(1) extra space;
    //   - no `unsafe`, no `Rc`/`RefCell`, and don't change the tests.
    // Until you stop using a shared borrow of the list while the `&mut`
    // cursor is alive, this exercise will not compile.
    let mut fast = head.as_deref();
    let mut slow = &mut head;
    while let Some(node) = fast {
        fast = node.next.as_deref().and_then(|next| next.next.as_deref());
        if let Some(slow_node) = slow {
            slow = &mut slow_node.next;
        }
    }
    let back = slow.take();
    (head, back)
}

// Removes the `n`th node counted from the END (`n == 1` is the last node) and
// returns the head. If `n` is 0 or larger than the length, nothing is removed.
fn remove_nth_from_end(head: Option<Box<ListNode>>, n: usize) -> Option<Box<ListNode>> {
    // TODO: The empty body is rejected with E0308 "mismatched types":
    // expected `Option<Box<ListNode>>`, found `()`. (rustc suggests returning
    // `head`. That compiles, but removes nothing.) rustc reports this error
    // together with the E0502 above. Requirements:
    //   - `n` counts from the end, starting at 1; `n == 0` or `n` larger than
    //     the length (even `usize::MAX`) returns the list unchanged, without
    //     panicking;
    //   - the remaining nodes are the original nodes, in order (the tests
    //     compare serials and addresses); O(1) extra space, no recursion (one
    //     test uses a 200_000-node list);
    //   - no `unsafe`, and don't change the tests. A lead cursor running `n`
    //     nodes ahead of a `&mut` one hits the same E0502 as `split_half`.
    // Until you return the list from every path, this exercise will not
    // compile.
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

    // ---- split_half ----

    #[test]
    fn odd_length_keeps_the_extra_node_in_front() {
        let list = from_slice(&[1, 2, 3, 4, 5]);
        let ids = identity(&list);
        let (front, back) = split_half(list);
        assert_eq!(values(&front), [1, 2, 3]);
        assert_eq!(values(&back), [4, 5]);
        // The halves are the original nodes: the list was cut, not copied.
        assert_eq!(identity(&front), ids[..3]);
        assert_eq!(identity(&back), ids[3..]);
    }

    #[test]
    fn even_length_splits_evenly() {
        let list = from_slice(&[10, 20, 30, 40, 50, 60]);
        let ids = identity(&list);
        let (front, back) = split_half(list);
        assert_eq!(values(&front), [10, 20, 30]);
        assert_eq!(values(&back), [40, 50, 60]);
        assert_eq!(identity(&front), ids[..3]);
        assert_eq!(identity(&back), ids[3..]);
    }

    #[test]
    fn splits_tiny_lists() {
        let (front, back) = split_half(None);
        assert!(front.is_none());
        assert!(back.is_none());

        let (front, back) = split_half(from_slice(&[1]));
        assert_eq!(values(&front), [1]);
        assert!(back.is_none());

        let (front, back) = split_half(from_slice(&[1, 2]));
        assert_eq!(values(&front), [1]);
        assert_eq!(values(&back), [2]);

        let (front, back) = split_half(from_slice(&[1, 2, 3]));
        assert_eq!(values(&front), [1, 2]);
        assert_eq!(values(&back), [3]);
    }

    // ---- remove_nth_from_end ----

    #[test]
    fn removes_the_second_node_from_the_end() {
        let list = from_slice(&[1, 2, 3, 4, 5]);
        let ids = identity(&list);
        let list = remove_nth_from_end(list, 2);
        assert_eq!(values(&list), [1, 2, 3, 5]);
        // The other nodes are untouched, only relinked around the gap.
        assert_eq!(identity(&list), [ids[0], ids[1], ids[2], ids[4]]);
    }

    #[test]
    fn removes_the_head_and_the_tail() {
        let list = from_slice(&[1, 2, 3, 4, 5]);
        let ids = identity(&list);
        // `n == len` is the first node.
        let list = remove_nth_from_end(list, 5);
        assert_eq!(values(&list), [2, 3, 4, 5]);
        assert_eq!(identity(&list), ids[1..]);
        // `n == 1` is the last node.
        let list = remove_nth_from_end(list, 1);
        assert_eq!(values(&list), [2, 3, 4]);
        assert_eq!(identity(&list), ids[1..4]);
    }

    #[test]
    fn removes_each_position() {
        // (n, the value of the `n`th node from the end)
        for (n, gone) in [(1, 5), (2, 4), (3, 3), (4, 2), (5, 1)] {
            let list = remove_nth_from_end(from_slice(&[1, 2, 3, 4, 5]), n);
            let expected: Vec<i32> = (1..=5).filter(|&v| v != gone).collect();
            assert_eq!(values(&list), expected, "n = {n}");
        }
    }

    #[test]
    fn out_of_range_n_leaves_the_list_unchanged() {
        let mut list = from_slice(&[1, 2, 3, 4, 5]);
        let ids = identity(&list);
        for n in [0, 6, 100, usize::MAX] {
            list = remove_nth_from_end(list, n);
            assert_eq!(identity(&list), ids, "n = {n}");
        }
    }

    #[test]
    fn removes_from_a_long_list_without_recursion() {
        const LEN: i32 = 200_000;
        let list = from_slice(&(0..LEN).collect::<Vec<_>>());
        let ids = identity(&list);
        // The last node: the walk goes through all 200_000 links.
        let list = remove_nth_from_end(list, 1);
        let after = identity(&list);
        drop_iteratively(list);
        assert!(
            after.iter().eq(&ids[..ids.len() - 1]),
            "only the last node should be gone"
        );
    }

    #[test]
    fn single_node_and_empty_lists() {
        assert!(remove_nth_from_end(from_slice(&[9]), 1).is_none());

        let one = from_slice(&[9]);
        let ids = identity(&one);
        assert_eq!(identity(&remove_nth_from_end(one, 2)), ids);

        assert!(remove_nth_from_end(None, 0).is_none());
        assert!(remove_nth_from_end(None, 1).is_none());
    }
}
