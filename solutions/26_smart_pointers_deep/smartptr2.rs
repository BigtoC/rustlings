// Module 1 · Smart pointers (deep) — part 2: `Rc<T>`, `Weak<T>` and cycles.
//
// `Rc<T>` ("reference counted") gives a value MULTIPLE owners: cloning an `Rc`
// doesn't copy the data, it just bumps a strong counter. The value is dropped
// when the last strong owner goes away. `Rc::strong_count` reports how many
// strong owners currently exist.
//
// But strong references can form CYCLES (A owns B, B owns A) that never reach
// zero and leak. The fix is `Weak<T>`: a non-owning reference that does NOT
// keep the value alive. A child pointing back at its parent should use a `Weak`
// so it doesn't co-own the parent. You turn an `Rc` into a `Weak` with
// `Rc::downgrade`, and turn a `Weak` back into an `Option<Rc>` with `.upgrade()`
// (it yields `None` if the value is already gone).

use std::rc::{Rc, Weak};

struct Node {
    value: i32,
    parent: Weak<Node>,
}

fn link_to_parent(parent: &Rc<Node>) -> Weak<Node> {
    Rc::downgrade(parent)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloning_an_rc_bumps_the_strong_count() {
        let a = Rc::new(Node {
            value: 1,
            parent: Weak::new(),
        });
        assert_eq!(Rc::strong_count(&a), 1);

        let b = Rc::clone(&a);
        assert_eq!(Rc::strong_count(&a), 2);

        drop(b);
        assert_eq!(Rc::strong_count(&a), 1);
    }

    #[test]
    fn a_weak_child_link_does_not_own_the_parent() {
        let parent = Rc::new(Node {
            value: 10,
            parent: Weak::new(),
        });

        let child = Rc::new(Node {
            value: 5,
            parent: link_to_parent(&parent),
        });

        // The child's link is weak, so the parent's STRONG count is unchanged,
        // while its WEAK count went up by one.
        assert_eq!(Rc::strong_count(&parent), 1);
        assert_eq!(Rc::weak_count(&parent), 1);

        // Upgrading the weak link gives back a real `Rc` while the parent lives.
        let recovered = child.parent.upgrade().expect("parent is still alive");
        assert_eq!(recovered.value, 10);
        // The temporary upgrade is itself a strong owner.
        assert_eq!(Rc::strong_count(&parent), 2);
    }
}
