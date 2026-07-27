// Iterators - implementing Iterator, custom adapters, laziness, part 3: laziness
// is not an optimization detail, it is the semantics.
//
// Adapters like `map` and `filter` do ZERO work when you call them. They only
// build a description of a computation; nothing runs until an EAGER consumer -
// `for_each`, `collect`, `count`, `sum`, `for` loop, ... - starts calling `next`
// to pull values through. A `map` whose closure has side effects but whose
// result is never consumed is therefore a no-op: a classic silent bug. (That is
// why std marks the adapters `#[must_use]`, so rustc's `unused_must_use` lint
// notices the dropped one.)
//
// `count_elements` below counts by incrementing `seen` once per element. Getting
// that to actually HAPPEN is the whole exercise: an adapter built only for its side
// effect and then dropped runs nothing at all, and the count stays 0. Only an eager
// consumer drives `next`.

fn count_elements(data: &[i32]) -> usize {
    let mut seen = 0;

    // TODO: This line builds a lazy `Map` and immediately discards it, so the
    // closure never runs and `seen` stays 0 — which is why the test fails.
    // Replace it with an EAGER consumer that really drives the iterator. Not
    // `map`: a `map` kept only for its side effect is the very bug shown here.
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
        assert_eq!(count_elements(&[1, 2, 3, 4]), 4);
    }

    #[test]
    fn counts_empty() {
        assert_eq!(count_elements(&[]), 0);
    }
}
