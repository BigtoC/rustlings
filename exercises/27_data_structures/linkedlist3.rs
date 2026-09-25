// Module 2 · Data structures — part 5: merge two sorted lists with a `&mut` tail cursor.
//
// "Merge two sorted lists without allocating" (LeetCode 21) is the usual
// follow-up to reversal. The C answer keeps two independent pointers into the
// result, `head` and `tail`, and appends with `tail->next = node; tail =
// node;`. In Rust the result list OWNS its nodes, so a cursor into it has to
// be a `&mut` borrowed through the chain that starts at `head`, and while that
// cursor is in use you cannot touch `head` at all. That is fine: you only need
// `head` back at the end, after the cursor's last use. The natural cursor
// points at the LINK rather than at a node: `tail: &mut Option<Box<ListNode>>`
// is the empty `next` field at the very end of the result, the place where
// the next node goes. Appending writes a node into that place; advancing
// re-points the cursor at the new node's own, still empty, `next`.
//
// Why the borrow checker accepts the advance: the new cursor is borrowed
// THROUGH the old one (a reborrow of `*tail`), and the old cursor is
// overwritten in the same statement, so there is never a second, independent
// `&mut` to the result. `Option::insert` (stable since Rust 1.53) helps: it
// stores a value in an `Option` and returns `&mut` to the stored value, so
// "append, then step onto the new node" is one expression, with no dummy head
// node and no `unwrap`. When one input runs out, the rest of the other is
// already sorted, and a single assignment through the cursor attaches all of
// it. That is O(n + m) time and O(1) extra space.
//
// Two details interviewers listen for:
//
//   - Stability. On equal values, take the node from the FIRST list. This is
//     what makes merge sort stable. The tests check it by node identity,
//     since equal values are otherwise indistinguishable.
//   - Recursion. The recursive merge is short, but it needs a stack frame for
//     each node it places before one input runs out. The last test merges two
//     100_000-node lists that alternate all the way to the end.
//
// `ListNode` is the same as in `linkedlist2`: LeetCode's node without
// `Clone`, plus a private serial number from `ListNode::new`. The tests
// compare (serial, address) pairs, so a merged list rebuilt from copied values
// fails even when the allocator happens to reuse the old addresses.

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

// Merges two lists, each sorted in ascending order, into one sorted list and
// returns its head. On equal values the node from `a` comes first.
fn merge(a: Option<Box<ListNode>>, b: Option<Box<ListNode>>) -> Option<Box<ListNode>> {
    // TODO: The empty body is rejected with E0308 "mismatched types":
    // expected `Option<Box<ListNode>>`, found `()`. (rustc notes that you
    // could return `a` or `b`. Either compiles, and both fail the tests.)
    // Splice the input nodes into one sorted list, building it front to back.
    // Requirements:
    //   - the result is made of the input nodes, relinked: no copies made
    //     with `ListNode::new`, no `Vec` of values or nodes, no sorting (the
    //     tests compare serials and addresses);
    //   - stable: on equal values the node from `a` goes first;
    //   - iterative, O(1) extra space, no recursion (one test merges 200_000
    //     nodes). Once one input is empty, the rest of the other can be
    //     attached in one step: there is no need to walk it;
    //   - no `unsafe`, and don't change the tests.
    // Until you return the merged list, this exercise will not compile.
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
    fn merges_two_sorted_lists() {
        let merged = merge(from_slice(&[1, 4, 5]), from_slice(&[1, 2, 6, 7]));
        assert_eq!(values(&merged), [1, 1, 2, 4, 5, 6, 7]);
    }

    #[test]
    fn merged_list_is_made_of_the_input_nodes() {
        let a = from_slice(&[1, 4, 5]);
        let b = from_slice(&[1, 2, 6, 7]);
        let (ia, ib) = (identity(&a), identity(&b));
        let merged = merge(a, b);
        // Every node is an input node, and the 1 from `a` comes before the 1
        // from `b`. A list rebuilt from the values would have new serials.
        assert_eq!(
            identity(&merged),
            [ia[0], ib[0], ib[1], ia[1], ia[2], ib[2], ib[3]]
        );
    }

    #[test]
    fn runs_of_equal_values_take_from_a_first() {
        let a = from_slice(&[2, 2, 5]);
        let b = from_slice(&[2, 5, 5]);
        let (ia, ib) = (identity(&a), identity(&b));
        let merged = merge(a, b);
        assert_eq!(values(&merged), [2, 2, 2, 5, 5, 5]);
        // Both 2s from `a` before the 2 from `b`; the 5 from `a` before the
        // 5s from `b`.
        assert_eq!(
            identity(&merged),
            [ia[0], ia[1], ib[0], ia[2], ib[1], ib[2]]
        );
    }

    #[test]
    fn empty_inputs() {
        assert!(merge(None, None).is_none());

        let a = from_slice(&[3, 8]);
        let ia = identity(&a);
        assert_eq!(identity(&merge(a, None)), ia);

        let b = from_slice(&[-4, 0, 9]);
        let ib = identity(&b);
        assert_eq!(identity(&merge(None, b)), ib);
    }

    #[test]
    fn one_list_ends_before_the_other_starts() {
        let high = from_slice(&[10, 20]);
        let low = from_slice(&[1, 2, 3]);
        let (ih, il) = (identity(&high), identity(&low));
        let merged = merge(high, low);
        assert_eq!(values(&merged), [1, 2, 3, 10, 20]);
        let expected: Vec<_> = il.iter().chain(&ih).copied().collect();
        assert_eq!(identity(&merged), expected);
    }

    #[test]
    fn merges_long_lists_without_recursion() {
        const HALF: i32 = 100_000;
        // `a` holds the even numbers and `b` the odd ones, so the merge
        // alternates between the lists for all 200_000 nodes.
        let a = from_slice(&(0..HALF).map(|i| 2 * i).collect::<Vec<_>>());
        let b = from_slice(&(0..HALF).map(|i| 2 * i + 1).collect::<Vec<_>>());
        let (ia, ib) = (identity(&a), identity(&b));
        let merged = merge(a, b);
        let after = identity(&merged);
        let vals = values(&merged);
        drop_iteratively(merged);
        assert!(
            vals.iter().copied().eq(0..2 * HALF),
            "the values are not merged in order"
        );
        let expected = ia.iter().zip(&ib).flat_map(|(x, y)| [x, y]);
        assert!(
            after.iter().eq(expected),
            "the result is not made of the original nodes"
        );
    }
}
