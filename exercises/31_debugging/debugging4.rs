// Module 5 · Debugging — part 4: RefCell's runtime borrow panic (BorrowMutError).
//
// `RefCell<T>` gives you interior mutability: it lets you mutate through a shared
// `&RefCell<T>`. The catch is that the "aliasing XOR mutability" rule still
// holds — RefCell just moves the check from COMPILE time to RUN time. Every
// `borrow()` and `borrow_mut()` consults a hidden counter, and an overlapping
// pair PANICS with `BorrowMutError` instead of being rejected by the compiler.
//
// This is the runtime face of the exact rule module 1's borrow checker enforced
// statically: a live shared borrow (`borrow()`) and an exclusive one
// (`borrow_mut()`) may not coexist. The Ref guard returned by `borrow()` holds
// the shared borrow open until it is dropped — usually the end of its block.
//
// The fix is to scope the shared borrow tightly: copy the value out so the Ref
// guard is dropped IMMEDIATELY, then take `borrow_mut()` on the now-free cell.

use std::cell::RefCell;

// TODO: This compiles but panics with `BorrowMutError`. `current` is a `Ref`
// guard, so the shared borrow stays alive across the whole `if` and overlaps the
// `borrow_mut()` inside it. End the shared borrow before mutating.
fn bump_if_below(cell: &RefCell<i32>, limit: i32) {
    let current = cell.borrow();
    if *current < limit {
        *cell.borrow_mut() += 1;
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumps_while_below_limit() {
        let c = RefCell::new(0);
        bump_if_below(&c, 5);
        assert_eq!(*c.borrow(), 1);
    }

    #[test]
    fn does_not_bump_at_limit() {
        let c = RefCell::new(1);
        bump_if_below(&c, 1);
        assert_eq!(*c.borrow(), 1);
    }
}
