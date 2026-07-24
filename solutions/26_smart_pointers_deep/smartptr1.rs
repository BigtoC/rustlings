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

enum List {
    Cons(i32, Box<List>),
    Nil,
}

use List::{Cons, Nil};

fn sum(list: &List) -> i32 {
    match list {
        Cons(value, rest) => *value + sum(rest),
        Nil => 0,
    }
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
