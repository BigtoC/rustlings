// Iterators - implementing Iterator, custom adapters, laziness, part 3: laziness
// is not an optimization detail, it is the semantics.
//
// Adapters like `map` and `filter` do ZERO work when you call them. They only
// build a description of a computation; nothing runs until an EAGER consumer -
// `for_each`, `collect`, `count`, `sum`, `for` loop, ... - starts calling `next`
// to pull values through. A `map` whose closure has side effects but whose
// result is never consumed is therefore a no-op: a classic silent bug. (Clippy's
// `#[must_use]` on iterator adapters exists precisely to catch this.)
//
// `count_via_map` below wants to count elements by incrementing `seen` once per
// element. It builds a `map` that would do that - but then drops it on the floor.

fn count_via_map(data: &[i32]) -> usize {
    let mut seen = 0;

    // TODO: This line builds a lazy `Map` and immediately discards it, so the
    // closure never runs and `seen` stays 0. Replace it with an EAGER consumer
    // that actually drives the iterator, e.g.
    //   `data.iter().for_each(|_| seen += 1);`
    // (Keep it clippy-clean: `map` used only for side effects is exactly what
    // clippy warns about - use `for_each` instead.)
    data.iter().map(|_| {
        seen += 1;
    });

    seen
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_elements() {
        assert_eq!(count_via_map(&[1, 2, 3, 4]), 4);
    }

    #[test]
    fn counts_empty() {
        assert_eq!(count_via_map(&[]), 0);
    }
}
