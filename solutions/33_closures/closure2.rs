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

fn apply_twice<F: FnMut()>(mut f: F) {
    // Each `f()` call borrows `f` as `&mut`, which `FnMut` allows.
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
