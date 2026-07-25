// Iterators - implementing Iterator, custom adapters, laziness, part 2: a custom
// lazy adapter.
//
// The adapters in the standard library (`map`, `filter`, `zip`, ...) are not
// magic: each is a plain struct that OWNS the iterator it wraps and computes its
// own `next` by pulling from that inner iterator. Chaining adapters just nests
// these structs - `v.iter().map(..).filter(..)` is a `Filter<Map<slice::Iter>>`.
// Crucially it is all LAZY: constructing the struct does no work; elements are
// produced only when a consumer calls `next`.
//
// Here we build `Pairwise`, an adapter that yields overlapping neighbour pairs:
// `[1, 2, 3, 4]` -> `(1, 2), (2, 3), (3, 4)`. It remembers the previous element
// in `prev` so each new element can be paired with the one before it.

struct Pairwise<I: Iterator> {
    inner: I,
    prev: Option<I::Item>,
}

impl<I> Iterator for Pairwise<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        // Left element: the remembered one, or the next from `inner` on the very
        // first call. Right element: the following item. `?` reports "done".
        let prev = match self.prev.take() {
            Some(p) => p,
            None => self.inner.next()?,
        };
        let cur = self.inner.next()?;
        self.prev = Some(cur.clone());
        Some((prev, cur))
    }
}

fn pairwise<I: IntoIterator>(iter: I) -> Pairwise<I::IntoIter>
where
    I::Item: Clone,
{
    Pairwise {
        inner: iter.into_iter(),
        prev: None,
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_pairs() {
        let got: Vec<(i32, i32)> = pairwise(vec![1, 2, 3, 4]).collect();
        assert_eq!(got, vec![(1, 2), (2, 3), (3, 4)]);
    }

    #[test]
    fn too_short_yields_nothing() {
        // A single element has no neighbour, so the very first `next` is `None`.
        assert_eq!(pairwise(vec![1]).next(), None);
    }
}
