// Module 2 · Data structures — part 1: a singly-linked stack, in SAFE Rust.
//
// The textbook linked list is where every Rust learner first fights the borrow
// checker. The trick is ownership: each node OWNS the next one through a
// `Box`, and "no next node" is simply `None`. That gives us
// `Option<Box<Node>>` — a nullable, owning pointer with zero `unsafe`.
//
// The one puzzle worth savouring is `pop`: to move the old head out of
// `&mut self` you cannot just read `self.head` (that would move out of a
// borrow). `Option::take` swaps the field with `None` and hands you the old
// value — the canonical safe way to "steal" an owned value out of a borrow.
//
// One catch that bites in production: the drop glue the compiler derives for
// `Option<Box<Node>>` is RECURSIVE. Dropping the head drops its `next`, which
// drops its `next`, ... — one stack frame per node, so a long list overflows the
// stack instead of being freed. The fix is a hand-written `Drop` that unlinks the
// nodes in a loop, which is what `Stack` does below.
//
// (The raw-pointer, `unsafe` doubly-linked version lives in `deep-dive/`.)

struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

struct Stack {
    head: Option<Box<Node>>,
}

impl Stack {
    fn new() -> Self {
        Stack { head: None }
    }

    fn push(&mut self, value: i32) {
        // TODO: Create a new `Node` whose `next` is the CURRENT head, then make
        // it the new head. Use `self.head.take()` to move the old head out
        // (leaving `None` behind) so you can store it in the new node's `next`.
    }

    fn pop(&mut self) -> Option<i32> {
        // TODO: Take the current head out with `self.head.take()`. If there was
        // a node, set `self.head` to that node's `next` and return
        // `Some(node.value)`; otherwise return `None`. Use `.map(...)` on the
        // taken `Option<Box<Node>>` to do both in one expression.
        //
        // Until you return an `Option<i32>` here, this exercise will not
        // compile (the body's type `()` does not match the return type).
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // Unlink node by node instead of letting the derived, RECURSIVE drop glue
        // walk the chain: a long list would otherwise need one stack frame per node
        // and overflow. Every node has its `next` taken out before it is dropped,
        // so nothing recurses. Note this does not go through `pop` — teardown must
        // not depend on it.
        let mut cur = self.head.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_on_empty_is_none() {
        let mut stack = Stack::new();
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn push_then_pop_is_lifo() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn interleaved_push_and_pop() {
        let mut stack = Stack::new();
        stack.push(10);
        assert_eq!(stack.pop(), Some(10));
        stack.push(20);
        stack.push(30);
        assert_eq!(stack.pop(), Some(30));
        stack.push(40);
        assert_eq!(stack.pop(), Some(40));
        assert_eq!(stack.pop(), Some(20));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn dropping_a_long_stack_does_not_overflow_the_stack() {
        let mut stack = Stack::new();
        for i in 0..200_000 {
            stack.push(i);
        }
        // `stack` is dropped here. With the compiler's recursive drop glue this
        // would need 200_000 stack frames and abort; the iterative `Drop` above
        // needs none. Reaching the end of this test IS the assertion.
    }
}
