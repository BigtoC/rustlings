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
use std::rc::Rc;

struct Node {
    name: &'static str,
    // TODO: `a_parent_and_its_child_are_freed_together` fails: the drop
    // counter is 0, expected 2. `adopt` makes the parent own the child (in
    // `children`) AND the child own the parent (here), so when the test's
    // handles go away, each node is still kept alive by the other: neither
    // strong count reaches zero, no `Drop` runs, and both nodes leak.
    // Requirements: `parent()` returns the child's parent while the parent is
    // alive and `None` once it has been freed, and a parent keeps owning its
    // children (dropping the test's handle to a child must not free it).
    // Change what a child holds here, and the code that sets and reads it.
    // Keep `children` as it is, no `unsafe`, no extra "unlink" step that
    // callers would have to remember, and don't change the tests. Until the
    // link to the parent stops owning the parent, the tests will fail.
    parent: RefCell<Option<Rc<Node>>>,
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
            // TODO: see the TODO on the `parent` field of `Node`.
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            drops: Rc::clone(drops),
        })
    }

    // The node's parent, if it has one and it is still alive.
    fn parent(&self) -> Option<Rc<Node>> {
        // TODO: see the TODO on the `parent` field of `Node`.
        self.parent.borrow().clone()
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
    // TODO: see the TODO on the `parent` field of `Node`.
    *child.parent.borrow_mut() = Some(Rc::clone(parent));
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
