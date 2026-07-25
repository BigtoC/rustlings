// Closures - the Fn / FnMut / FnOnce hierarchy, part 2: `FnMut` for a mutating
// closure.
//
// The three closure traits form a hierarchy defined by how much access CALLING
// the closure needs:
//   - `Fn`     — callable through `&self`     (the body only reads its captures)
//   - `FnMut`  — callable through `&mut self` (the body mutates its captures)
//   - `FnOnce` — callable through `self`      (the body consumes its captures)
// A closure that MUTATES a captured variable can only offer `FnMut` (and
// `FnOnce`), never the stricter `Fn`, because each call needs an exclusive
// `&mut` to the closure itself. So a function that demands `F: Fn` will reject
// it — it must ask for `F: FnMut` instead.

fn apply_twice<F: Fn()>(mut f: F) {
    // TODO: The bound `F: Fn()` is too strict. The closure passed in by the test
    // mutates its captured `count`, so it implements `FnMut`, not `Fn`, and the
    // compiler reports E0525: "expected a closure that implements the `Fn`
    // trait, but this closure only implements `FnMut`". Relax the bound to
    // `F: FnMut()`. The `mut f` binding is already here, so `f()` can borrow
    // `f` as `&mut`.
    // Until you relax the bound, this exercise will not compile.
    f();
    f();
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_two_calls() {
        let mut count = 0;
        // This closure mutates `count`, so it is `FnMut`, not `Fn`.
        let inc = || {
            count += 1;
        };
        apply_twice(inc);
        // `inc` (and its `&mut count` borrow) is dropped inside `apply_twice`,
        // so we can read `count` again here.
        assert_eq!(count, 2);
    }
}
