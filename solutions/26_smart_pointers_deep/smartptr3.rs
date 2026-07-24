// Module 1 · Smart pointers (deep) — part 3: `RefCell<T>` interior mutability.
//
// Rust's borrowing rules normally require `&mut` to mutate. `RefCell<T>` moves
// that check from COMPILE time to RUN time: you can mutate its contents through
// a plain shared `&RefCell<T>` by calling `.borrow_mut()`, which hands you a
// mutable guard (and panics if the aliasing-XOR-mutability rule is broken at
// runtime). This is "interior mutability".
//
// Pairing it as `Rc<RefCell<T>>` is the standard recipe for shared, mutable
// state: `Rc` gives many owners, `RefCell` lets any of them mutate the value.
// A change made through one handle is visible through every other handle,
// because they all point at the SAME cell.

use std::cell::RefCell;
use std::rc::Rc;

fn push_and_len(handle: &Rc<RefCell<Vec<i32>>>, value: i32) -> usize {
    let mut data = handle.borrow_mut();
    data.push(value);
    data.len()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_handles_share_one_vec() {
        let a = Rc::new(RefCell::new(vec![1, 2]));
        let b = Rc::clone(&a);

        // Mutating through `b`...
        let len = push_and_len(&b, 3);
        assert_eq!(len, 3);

        // ...is visible through `a`: they point at the same underlying `Vec`.
        assert_eq!(*a.borrow(), vec![1, 2, 3]);
        assert_eq!(Rc::strong_count(&a), 2);
    }

    #[test]
    fn mutation_through_a_shared_ref() {
        // `cell` is NOT declared `mut`, yet we can still change its contents —
        // that is exactly what interior mutability buys us.
        let cell = RefCell::new(String::from("a"));
        let shared: &RefCell<String> = &cell;
        shared.borrow_mut().push('b');
        assert_eq!(*cell.borrow(), "ab");
    }
}
