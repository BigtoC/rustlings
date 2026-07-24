//! Lab · a raw-pointer doubly linked list (compare with the standard library's `LinkedList`)
//!
//! In `27_data_structures/linkedlist1` we wrote a **safe** singly linked list
//! using `Option<Box<Node>>` — but that approach cannot produce an efficient
//! **doubly** linked list: a node is pointed at simultaneously by its
//! predecessor's `next` and its successor's `prev`, which violates the
//! borrowing rule that "there may be only one `&mut` at a time".
//!
//! The standard library's `std::collections::LinkedList` therefore switches to
//! **raw pointers** (`NonNull<Node<T>>`) and maintains these links by hand,
//! outside the boundary of safe Rust. Here we do the same: we expose a fully
//! safe API on the outside, while internally using `unsafe` plus `// SAFETY:`
//! comments to maintain the invariants. This is a textbook example of "a safe
//! abstraction wrapping an unsafe implementation".
//!
//! Invariants:
//! - `head`/`tail` are either both `None` (empty list) or both point at valid
//!   nodes owned by this list;
//! - every node is "owned" exactly once (either as `head`, or as some node's `next`);
//! - `len` equals the actual number of nodes in the list.

use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    // Tell the compiler this struct logically "owns" these `Node<T>`s (affects drop-check and variance).
    _marker: PhantomData<Box<Node<T>>>,
}

struct Node<T> {
    value: T,
    prev: Option<NonNull<Node<T>>>,
    next: Option<NonNull<Node<T>>>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        LinkedList {
            head: None,
            tail: None,
            len: 0,
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push_back(&mut self, value: T) {
        // Allocate the node on the heap and "leak" the raw pointer — ownership is now managed by the list by hand.
        let node = Box::new(Node {
            value,
            prev: self.tail,
            next: None,
        });
        let node_ptr = NonNull::from(Box::leak(node));

        match self.tail {
            // SAFETY: `tail` is a valid node owned by this list, so writing its `next` is safe.
            Some(tail) => unsafe { (*tail.as_ptr()).next = Some(node_ptr) },
            None => self.head = Some(node_ptr),
        }
        self.tail = Some(node_ptr);
        self.len += 1;
    }

    pub fn push_front(&mut self, value: T) {
        let node = Box::new(Node {
            value,
            prev: None,
            next: self.head,
        });
        let node_ptr = NonNull::from(Box::leak(node));

        match self.head {
            // SAFETY: `head` is a valid node owned by this list, so writing its `prev` is safe.
            Some(head) => unsafe { (*head.as_ptr()).prev = Some(node_ptr) },
            None => self.tail = Some(node_ptr),
        }
        self.head = Some(node_ptr);
        self.len += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.map(|head| {
            // SAFETY: `head` came from `Box::leak` and is still alive; here we reclaim its
            // ownership so it is properly freed when this closure ends.
            let boxed = unsafe { Box::from_raw(head.as_ptr()) };
            self.head = boxed.next;
            match self.head {
                // SAFETY: the new head node is likewise a valid node owned by this list.
                Some(new_head) => unsafe { (*new_head.as_ptr()).prev = None },
                None => self.tail = None,
            }
            self.len -= 1;
            boxed.value
        })
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.map(|tail| {
            // SAFETY: same as `pop_front`, reclaim ownership of the tail node.
            let boxed = unsafe { Box::from_raw(tail.as_ptr()) };
            self.tail = boxed.prev;
            match self.tail {
                // SAFETY: the new tail node is a valid node owned by this list.
                Some(new_tail) => unsafe { (*new_tail.as_ptr()).next = None },
                None => self.head = None,
            }
            self.len -= 1;
            boxed.value
        })
    }

    pub fn front(&self) -> Option<&T> {
        // SAFETY: `head` points at a valid node owned by this list, and the borrow does not outlive `&self`.
        self.head.map(|head| unsafe { &(*head.as_ptr()).value })
    }

    pub fn back(&self) -> Option<&T> {
        // SAFETY: same as above.
        self.tail.map(|tail| unsafe { &(*tail.as_ptr()).value })
    }
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        // Pop nodes one by one, triggering each node's `Box` deallocation to avoid leaks.
        while self.pop_front().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_both_ends() {
        let mut list = LinkedList::new();
        list.push_back(2);
        list.push_back(3);
        list.push_front(1);
        // Logical order: 1, 2, 3
        assert_eq!(list.len(), 3);
        assert_eq!(list.front(), Some(&1));
        assert_eq!(list.back(), Some(&3));

        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_back(), Some(3));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), None);
        assert!(list.is_empty());
    }

    #[test]
    fn drop_frees_remaining_nodes() {
        // Drop without explicitly clearing; `Drop` should free all nodes (verifiable as leak-free under Miri).
        let mut list = LinkedList::new();
        for i in 0..1000 {
            list.push_back(i);
        }
        assert_eq!(list.len(), 1000);
        // `list` goes out of scope here, and Drop frees the nodes one by one.
    }

    #[test]
    fn works_with_owned_values() {
        let mut list = LinkedList::new();
        list.push_back(String::from("hello"));
        list.push_front(String::from("world"));
        assert_eq!(list.pop_front().as_deref(), Some("world"));
        assert_eq!(list.pop_front().as_deref(), Some("hello"));
    }
}
