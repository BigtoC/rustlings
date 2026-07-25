// Iterators - implementing Iterator, custom adapters, laziness, part 4: making a
// type work in a `for` loop with `IntoIterator`.
//
// A `for` loop does not require `Iterator` directly - it requires
// `IntoIterator`. The loop
//
//     for x in coll { .. }
//
// desugars to
//
//     let mut it = IntoIterator::into_iter(coll);
//     while let Some(x) = it.next() { .. }
//
// So implementing `IntoIterator` is exactly what makes your own type usable in a
// `for` loop. Here we do the CONSUMING form (by value, `self`): iterating a
// `Grid` hands out owned `i32`s and uses up the grid. We can lean on `Vec`'s own
// iterator as our `IntoIter` type rather than writing one from scratch.
//
// Note: the standard library also provides `IntoIterator` for `&Coll` and
// `&mut Coll`, yielding `&T` and `&mut T` - which is why `for x in &v` borrows
// instead of consuming. Here we implement only the owning `impl` on `Grid`.

struct Grid {
    cells: Vec<i32>,
}

impl IntoIterator for Grid {
    type Item = i32;
    type IntoIter = std::vec::IntoIter<i32>;

    fn into_iter(self) -> Self::IntoIter {
        // TODO: Hand back the iterator that owns the cells. `Vec<i32>` already
        // has a consuming iterator of exactly type `std::vec::IntoIter<i32>`, so
        // return `self.cells.into_iter()`.
        // An empty body returns `()`, not `Self::IntoIter`, so until you return
        // the vector's iterator this exercise will not compile.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_loop_over_grid() {
        let g = Grid {
            cells: vec![1, 2, 3],
        };
        let mut sum = 0;
        // This `for` loop is what `IntoIterator` unlocks.
        for x in g {
            sum += x;
        }
        assert_eq!(sum, 6);
    }

    #[test]
    fn explicit_into_iter() {
        let got: Vec<i32> = Grid {
            cells: vec![10, 20],
        }
        .into_iter()
        .collect();
        assert_eq!(got, vec![10, 20]);
    }
}
