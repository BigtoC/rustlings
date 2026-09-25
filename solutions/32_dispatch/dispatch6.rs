// Traits & Dispatch - trait objects, part 6: a generic method is not dyn-compatible.
//
// `dispatch2` showed one way to lose `dyn` support: a method that returns
// `Self` by value. A GENERIC method is the other classic one, and it is the
// one that bites the visitor pattern. A vtable is a fixed table of function
// pointers, one slot per method, and rustc emits one for each concrete type it
// erases (here, when a `Box<Leaf>` becomes a `Box<dyn Node>`). But
// `accept<V: Visitor>` is not one function. It is a family of functions, one
// monomorphized copy per visitor type `V`, and that family is open-ended: a
// test module, or another crate, can define a new visitor long after the
// vtable for `Leaf` was compiled. There is no finite table to build, so every
// `dyn Node` is rejected with E0038 "the trait `Node` is not dyn compatible",
// with the note "...because method `accept` has generic type parameters".
// (`fn accept(&self, v: &mut impl Visitor)` is the same generic method, just
// spelled differently.)
//
// dispatch2's fix, `where Self: Sized`, is the wrong tool here. It removes the
// method from the vtable. That is right for a helper you only ever call on a
// concrete type (`boxed` below does exactly that), but `accept` exists to be
// called on a node whose type you DON'T know: `walk` and `Branch` call it
// through `Box<dyn Node>`. Gate it and `dyn Node` compiles again, but every
// one of those calls fails with "the `accept` method cannot be invoked on a
// trait object".
//
// So the type parameter has to go. The fix changes the cost model, not the
// behavior: `Leaf` and `Branch` still pick the right `visit_*` method, but
// each `accept` is compiled once instead of once per `V`, and every
// `visit_*` call inside it becomes a vtable call. If the set of node kinds
// were closed, an enum (`dispatch4`) with a generic visitor would keep static
// dispatch; rustc's own help ("consider defining an enum where each variant
// holds one of these types") points that way. Here both sides stay open: the
// tests add a visitor of their own, and other code could add node types.
//
// Interviewers ask "which trait items break dyn compatibility, and what is
// the fix for each?". The list: returning `Self`, generic methods, associated
// functions with no `self` receiver, `Self` in an argument
// (`fn eq(&self, other: &Self)`), associated consts, generic associated types,
// `-> impl Trait`, `async fn`, and a `Sized` or `Clone` supertrait. The fixes
// differ; the README has a table.

trait Visitor {
    // Default no-ops, so a visitor only overrides what it cares about.
    fn visit_leaf(&mut self, _leaf: &Leaf) {}
    fn visit_branch(&mut self, _branch: &Branch) {}
}

trait Node {
    // Taking the visitor as `&mut dyn Visitor` leaves ONE `accept` per node
    // type, so it gets one vtable slot and `Node` is dyn-compatible. The
    // variation over visitor types moved into the visitor's own vtable: each
    // call now dispatches dynamically on the node AND on the visitor (double
    // dispatch). Any `&mut V` with `V: Visitor` coerces to it at the call site.
    fn accept(&self, visitor: &mut dyn Visitor);

    // Given, complete: dispatch2's `where Self: Sized` used where it belongs.
    // `boxed` takes `self` by value and is only ever called on a concrete node
    // (`Leaf { .. }.boxed()`), so it is kept out of the vtable.
    fn boxed(self) -> Box<dyn Node>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

struct Leaf {
    value: i32,
}

struct Branch {
    label: String,
    children: Vec<Box<dyn Node>>,
}

// The impls must match the trait's new signature.
impl Node for Leaf {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_leaf(self);
    }
}

impl Node for Branch {
    // Pre-order: the branch itself, then each child from left to right.
    // `child.accept(visitor)` implicitly reborrows the `&mut dyn Visitor`, so
    // the same visitor can be handed to every child in turn.
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_branch(self);
        for child in &self.children {
            child.accept(visitor);
        }
    }
}

// Given, complete: visit every root in order.
fn walk<V: Visitor>(roots: &[Box<dyn Node>], visitor: &mut V) {
    for node in roots {
        node.accept(visitor);
    }
}

#[derive(Default)]
struct CountingVisitor {
    leaves: usize,
    branches: usize,
}

impl Visitor for CountingVisitor {
    fn visit_leaf(&mut self, _leaf: &Leaf) {
        self.leaves += 1;
    }

    fn visit_branch(&mut self, _branch: &Branch) {
        self.branches += 1;
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(value: i32) -> Box<dyn Node> {
        Leaf { value }.boxed()
    }

    fn branch(label: &str, children: Vec<Box<dyn Node>>) -> Box<dyn Node> {
        Branch {
            label: label.to_string(),
            children,
        }
        .boxed()
    }

    // root(1, inner(2, 3), 4): 4 leaves, 2 branches.
    fn nested() -> Vec<Box<dyn Node>> {
        vec![branch(
            "root",
            vec![leaf(1), branch("inner", vec![leaf(2), leaf(3)]), leaf(4)],
        )]
    }

    // A visitor type that `Leaf`, `Branch` and `walk` have never heard of.
    #[derive(Default)]
    struct Trace {
        events: Vec<String>,
        leaf_sum: i32,
    }

    impl Visitor for Trace {
        fn visit_leaf(&mut self, leaf: &Leaf) {
            self.events.push(format!("leaf {}", leaf.value));
            self.leaf_sum += leaf.value;
        }

        fn visit_branch(&mut self, branch: &Branch) {
            self.events.push(format!("branch {}", branch.label));
        }
    }

    // Overrides only `visit_leaf`; `visit_branch` keeps the default no-op.
    #[derive(Default)]
    struct MaxLeaf {
        max: Option<i32>,
    }

    impl Visitor for MaxLeaf {
        fn visit_leaf(&mut self, leaf: &Leaf) {
            self.max = self.max.max(Some(leaf.value));
        }
    }

    #[test]
    fn counts_leaves_and_branches() {
        // [Leaf, Branch, Leaf]
        let roots = vec![leaf(1), branch("empty", Vec::new()), leaf(2)];
        let mut counter = CountingVisitor::default();
        walk(&roots, &mut counter);
        assert_eq!((counter.leaves, counter.branches), (2, 1));
    }

    #[test]
    fn recurses_into_nested_branches() {
        let mut counter = CountingVisitor::default();
        walk(&nested(), &mut counter);
        assert_eq!((counter.leaves, counter.branches), (4, 2));
    }

    #[test]
    fn works_with_a_visitor_defined_elsewhere() {
        let mut trace = Trace::default();
        walk(&nested(), &mut trace);
        assert_eq!(
            trace.events,
            [
                "branch root",
                "leaf 1",
                "branch inner",
                "leaf 2",
                "leaf 3",
                "leaf 4"
            ]
        );
        assert_eq!(trace.leaf_sum, 10);
    }

    #[test]
    fn default_visit_methods_are_no_ops() {
        let mut max = MaxLeaf::default();
        walk(&nested(), &mut max);
        assert_eq!(max.max, Some(4));

        let mut none = MaxLeaf::default();
        walk(&[branch("no leaves", Vec::new())], &mut none);
        assert_eq!(none.max, None);
    }

    #[test]
    fn visitor_chosen_at_run_time() {
        // The visitor is a trait object too: which one runs is decided at run
        // time, and each `accept` call dispatches on BOTH vtables.
        let roots = nested();
        let mut counter = CountingVisitor::default();
        let mut trace = Trace::default();
        let visitors: [&mut dyn Visitor; 2] = [&mut counter, &mut trace];
        for visitor in visitors {
            for node in &roots {
                node.accept(visitor);
            }
        }
        assert_eq!((counter.leaves, counter.branches), (4, 2));
        assert_eq!(trace.events.len(), 6);
    }
}
