// Module 1 · Smart pointers (deep) — part 1: `Box<T>` for recursive types.
//
// The compiler must know the size of every type at compile time. A directly
// recursive type would be infinitely large: a `List` that contains a `List`
// that contains a `List`... To break the cycle we store the tail BEHIND a
// pointer. `Box<T>` is the simplest owning heap pointer: it has a fixed size (a
// single machine word) regardless of how big `T` is, so each node becomes
// "one `i32` + one pointer" — a finite, known size.
//
// This is the classic "cons list": every node is either `Cons(value, rest)` or
// the terminator `Nil`.
//
// Two things `Box` does NOT fix, worth knowing before reaching for a cons list in
// anger: any RECURSIVE walk over the chain costs one stack frame per node (`sum`
// below is therefore written as a loop), and so does the drop glue the compiler
// derives for the type — a long enough list overflows the stack when it is freed.
//
// An iterative `Drop` is possible here, but it takes care: dropping any `List`
// value re-enters `List::drop`, so the loop must never hand itself a value that
// still has a long tail. Empty each `Box` in place first, so that every value
// actually dropped is a single node whose tail is already `Nil` and the re-entry
// bottoms out at constant depth. The `Option<Box<Node>>` shape in
// `27_data_structures/linkedlist1` makes the same job a clean four-liner, which is
// a real argument for preferring it.

// TODO: As written, `Cons` stores a `List` inline, so `List` has no finite size
// and won't compile ("recursive type has infinite size"). Put the tail behind
// an indirection: change the second field to `Box<List>`.
enum List {
    Cons(i32, List),
    Nil,
}

use List::{Cons, Nil};

fn sum(list: &List) -> i32 {
    // Walk the chain with a loop, not recursion: one stack frame instead of one
    // per node.
    let mut total = 0;
    let mut cur = list;
    while let Cons(value, rest) = cur {
        total += value;
        cur = rest;
    }
    total
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_a_cons_list() {
        // 1 -> 2 -> 3 -> Nil
        let list = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));
        assert_eq!(sum(&list), 6);
    }

    #[test]
    fn empty_list_sums_to_zero() {
        assert_eq!(sum(&Nil), 0);
    }
}
