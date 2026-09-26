// Module 1 · Drop and RAII — part 6: an `Rc` cycle leaks, and no `Drop` ever runs.
//
// `26_smart_pointers_deep/smartptr2` said it: two `Rc`s that point at each
// other keep each other's strong count above zero, so neither value is ever
// freed. Here you watch it happen. Every `Node` bumps a shared counter in its
// `Drop`, and a tree whose child holds a STRONG pointer back to its parent
// never runs a single one of those drops: when the test's handles go away,
// the parent is still owned by the child and the child by the parent. Nothing
// panics, nothing crashes, and neither rustc nor clippy warns you. The memory
// (and anything else the nodes own, such as files or sockets) is simply gone
// for the rest of the program.
//
// This is SAFE Rust, and that is the point interviewers probe with "Can safe
// Rust leak memory?". Yes. Rust's safety promise is about undefined behavior:
// no use-after-free, no double free, no data races. A leak is none of those,
// so `mem::forget`, `Box::leak` and `Rc` cycles are all safe (part 3 has the
// consequences: a destructor is never guaranteed to run). The borrow checker
// cannot see a cycle either, because every `Rc` in it is a perfectly valid,
// owning handle.
//
// The fix is a design decision, not a trick: decide which direction OWNS.
// In a tree, a parent owns its children, and a child only needs to FIND its
// parent, which may have been freed in the meantime. So the link back must be
// a pointer that does not own. A child that asks for its parent then gets an
// `Option`: `Some` while the parent is alive, `None` once it is gone.
//
// How interviewers probe it: "Can safe Rust leak? Is that unsafe?", "How do
// you model a tree (or a doubly linked list) with parent pointers?", "How
// would you notice a leak like this?" (a drop counter as here, `strong_count`
// in tests, a leak checker such as LeakSanitizer or Valgrind in CI), and "What
// else could you use?" (an arena of nodes addressed by index, see
// `59_arena`, where nothing is reference counted at all).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct Node {
    name: &'static str,
    // A child only needs to FIND its parent, so it holds a `Weak`: it keeps
    // the parent's allocation around but not the parent itself, so the
    // parent's strong count is 1 (the caller's handle), not 2, and dropping
    // that handle frees it. Ownership now flows one way, from parent to
    // children, so there is no cycle. `RefCell` stays because `adopt` sets the
    // link through a shared `Rc<Node>`.
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
    // Shared with the test: how many nodes have been dropped so far. A
    // `Cell<u32>` is a counter that can be bumped through a shared reference
    // (`40_interior_mutability` covers `Cell`).
    drops: Rc<Cell<u32>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

impl Node {
    fn new(name: &'static str, drops: &Rc<Cell<u32>>) -> Rc<Node> {
        Rc::new(Node {
            name,
            // No parent yet: an empty `Weak`, which never upgrades.
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(Vec::new()),
            drops: Rc::clone(drops),
        })
    }

    // The node's parent, if it has one and it is still alive.
    fn parent(&self) -> Option<Rc<Node>> {
        // `upgrade` hands out a new strong `Rc` if the parent is still alive,
        // and `None` once its last strong handle has been dropped.
        self.parent.borrow().upgrade()
    }

    fn child_names(&self) -> Vec<&'static str> {
        self.children
            .borrow()
            .iter()
            .map(|child| child.name)
            .collect()
    }
}

// Makes `child` a child of `parent`.
fn adopt(parent: &Rc<Node>, child: &Rc<Node>) {
    // `Rc::downgrade` makes a non-owning link: the weak count goes up, the
    // strong count does not. The parent still owns the child (a strong `Rc`
    // in `children`).
    *child.parent.borrow_mut() = Rc::downgrade(parent);
    parent.children.borrow_mut().push(Rc::clone(child));
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter() -> Rc<Cell<u32>> {
        Rc::new(Cell::new(0))
    }

    #[test]
    fn a_parent_and_its_child_are_freed_together() {
        let drops = counter();
        {
            let root = Node::new("root", &drops);
            let leaf = Node::new("leaf", &drops);
            adopt(&root, &leaf);
        }
        assert_eq!(
            drops.get(),
            2,
            "the nodes leaked: their `Drop` never ran after the last handle went away"
        );
    }

    #[test]
    fn a_weak_probe_sees_the_tree_go_away() {
        let drops = counter();
        let probe = {
            let root = Node::new("root", &drops);
            let leaf = Node::new("leaf", &drops);
            adopt(&root, &leaf);
            // A `Weak` does not keep the root alive; it only lets the test ask
            // later whether the root still exists.
            Rc::downgrade(&root)
        };
        assert!(
            probe.upgrade().is_none(),
            "the root outlived its last handle"
        );
    }

    #[test]
    fn a_child_can_reach_its_living_parent() {
        let drops = counter();
        let root = Node::new("root", &drops);
        let leaf = Node::new("leaf", &drops);
        assert!(leaf.parent().is_none());
        adopt(&root, &leaf);

        let parent = leaf.parent().expect("the root is alive");
        // The very same node, not a copy of it.
        assert!(Rc::ptr_eq(&parent, &root));
        assert_eq!(parent.name, "root");
        assert_eq!(root.child_names(), ["leaf"]);
        assert!(root.parent().is_none());
    }

    #[test]
    fn the_link_to_the_parent_does_not_own_it() {
        let drops = counter();
        let root = Node::new("root", &drops);
        let leaf = Node::new("leaf", &drops);
        adopt(&root, &leaf);
        // Only the test's handle owns the root; the child's link does not.
        assert_eq!(Rc::strong_count(&root), 1);
        // The leaf is owned twice: by the test and by the root's `children`.
        assert_eq!(Rc::strong_count(&leaf), 2);

        drop(root);
        assert_eq!(
            drops.get(),
            1,
            "dropping the last handle must free the root"
        );
        assert!(leaf.parent().is_none(), "the root is gone");
        // Freeing the root released its `children`, so the test's handle is
        // the leaf's only owner now.
        assert_eq!(Rc::strong_count(&leaf), 1);
        drop(leaf);
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn a_parent_owns_its_children() {
        let drops = counter();
        let root = Node::new("root", &drops);
        {
            let leaf = Node::new("leaf", &drops);
            adopt(&root, &leaf);
        }
        // The test's handle to the leaf is gone, but the root still owns it.
        assert_eq!(drops.get(), 0);
        assert_eq!(root.child_names(), ["leaf"]);
        let leaf = Rc::clone(&root.children.borrow()[0]);
        assert!(Rc::ptr_eq(
            &leaf.parent().expect("the root is alive"),
            &root
        ));
        drop(leaf);

        drop(root);
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn a_deeper_tree_is_freed_completely() {
        let drops = counter();
        {
            let root = Node::new("root", &drops);
            let left = Node::new("left", &drops);
            let right = Node::new("right", &drops);
            let grandchild = Node::new("grandchild", &drops);
            adopt(&root, &left);
            adopt(&root, &right);
            adopt(&left, &grandchild);
            assert_eq!(root.child_names(), ["left", "right"]);
            let up = grandchild.parent().and_then(|p| p.parent());
            assert!(Rc::ptr_eq(&up.expect("the root is alive"), &root));
        }
        assert_eq!(drops.get(), 4, "some nodes leaked");
    }
}
