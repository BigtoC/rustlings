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
        let node = Box::new(Node {
            value,
            next: self.head.take(),
        });
        self.head = Some(node);
    }

    fn pop(&mut self) -> Option<i32> {
        self.head.take().map(|node| {
            self.head = node.next;
            node.value
        })
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
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
}
