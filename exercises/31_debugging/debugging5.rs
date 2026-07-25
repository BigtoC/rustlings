// Module 5 · Debugging — part 5: iterator invalidation is a COMPILE error.
//
// In many languages, mutating a container while you iterate over it is a classic
// runtime footgun: elements get skipped, indices slide out from under you, or
// the program crashes deep inside the iterator. Rust turns that whole class of
// bug into a COMPILE error.
//
// `v.iter()` takes a shared borrow of `v` that lasts for the whole loop. Calling
// `v.remove(i)` needs an exclusive `&mut` borrow at the same time — "aliasing
// XOR mutability" forbids exactly that, so the borrow checker rejects it with
// E0502. (Even if it compiled, index-removing while walking forward would SKIP
// the element after each removal, because `remove` shifts everything left.)
//
// The idiomatic fix does the in-place filter in one safe pass: `Vec::retain`
// keeps only the elements for which the closure returns `true`.

// TODO: This does not compile: `v.remove(i)` tries to mutate `v` while
// `v.iter()` still borrows it (E0502). Don't patch the loop — replace the whole
// body with a single `v.retain(|&x| x % 2 != 0);`, which keeps the odd numbers
// (drops the evens) in one pass. Until you remove the borrow conflict, this
// exercise will not compile.
fn remove_evens(v: &mut Vec<i32>) {
    for (i, &x) in v.iter().enumerate() {
        if x % 2 == 0 {
            v.remove(i);
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_every_even() {
        let mut v = vec![1, 2, 3, 4, 5, 6];
        remove_evens(&mut v);
        assert_eq!(v, vec![1, 3, 5]);
    }

    #[test]
    fn empty_stays_empty() {
        let mut e: Vec<i32> = vec![];
        remove_evens(&mut e);
        assert_eq!(e, Vec::<i32>::new());
    }
}
