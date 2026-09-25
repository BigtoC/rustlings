// Closures - bounds beyond the basics, part 6: `for<'a>` outside the `Fn`
// traits, iterating a container you own by reference.
//
// Elision writes `for<'a>` for you only inside `Fn(..)` sugar and `fn` pointer
// types (`closure5`). Any other bound about a borrow that the function creates
// INSIDE its body has to spell the binder out, and the one you meet most often
// is "a shared borrow of `C` can be iterated":
//
//     where for<'a> &'a C: IntoIterator<Item = &'a i32>
//
// That is the bound behind `for x in &c`. std implements `IntoIterator` for
// `&Vec<T>`, `&[T; N]`, `&VecDeque<T>`, `&BTreeSet<T>`, `&HashMap<K, V>` and so
// on, each yielding references that live as long as the borrow. Iterating by
// reference is what lets one function make several passes over a container
// without consuming it. `C: IntoIterator<Item = i32>` gives you ONE pass
// (`into_iter` takes `self`); a second pass would need `C: Clone` (copy the
// whole container) or a `collect` into a fresh `Vec`, and plenty of types
// support neither.
//
// `stats` takes its container BY VALUE, so the container is a local of
// `stats`, dropped at the end of the body. A lifetime parameter on the function
// is chosen by the caller and outlives the call, so `where &'a C: IntoIterator`
// only promises that borrows lasting that long can be iterated, and no borrow of
// a local lasts that long: every `&c` is E0597 "`c` does not live long enough
// ... argument requires that `c` is borrowed for `'a`". With the binder the
// impl is promised for EVERY lifetime, so each `&c` may be as short as the
// statement that uses it. (The binder does not force `C: 'static` either: a
// container that itself holds borrows is accepted too.)
//
// How interviewers probe this: "Write a generic function that iterates its
// argument twice", "Why does `where &'a C: IntoIterator` not work for a
// container the function owns?", and "Name a bound that needs an explicit
// `for<'a>`".

// Returns `(count, sum, max)` of the numbers in `c`, making three passes over it.
fn stats<'a, C: 'a>(c: C) -> (usize, i32, Option<i32>)
where
    &'a C: IntoIterator<Item = &'a i32>,
{
    // TODO: rustc rejects each `&c` below with E0597 "`c` does not live long
    // enough ... argument requires that `c` is borrowed for `'a`" (three
    // times, once per pass). `'a` belongs to the caller, while `c` is a local
    // of `stats` that is dropped when it returns. Change the bound so it
    // promises that a borrow of ANY lifetime can be iterated. Keep the
    // signature otherwise as it is: `stats` takes `c` by value, and it must
    // work for every container the tests use, including one that is neither
    // `Clone` nor iterable by value. No cloning or collecting the container,
    // and no leaking it. Until the bound covers borrows that end inside
    // `stats`, this exercise will not compile.
    let count = (&c).into_iter().count();
    let sum = (&c).into_iter().sum();
    let max = (&c).into_iter().max().copied();
    (count, sum, max)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::{BTreeSet, VecDeque};
    use std::rc::Rc;

    #[test]
    fn works_for_a_vec_and_an_array() {
        assert_eq!(stats(vec![1, 2, 3]), (3, 6, Some(3)));
        assert_eq!(stats([3, 1, 2]), (3, 6, Some(3)));
    }

    #[test]
    fn works_for_a_vecdeque_and_a_btreeset() {
        assert_eq!(stats(VecDeque::from([2, 3, 1])), (3, 6, Some(3)));
        assert_eq!(stats(BTreeSet::from([3, 2, 1])), (3, 6, Some(3)));
    }

    #[test]
    fn an_empty_container_has_no_max() {
        assert_eq!(stats(Vec::new()), (0, 0, None));
    }

    #[test]
    fn negative_numbers() {
        // The max of all-negative numbers is negative: it is not "at least 0".
        assert_eq!(stats(vec![-5, -2, -9]), (3, -16, Some(-2)));
    }

    // A collection that can only be iterated BY REFERENCE and cannot be
    // cloned, like many domain types. It also records when it is dropped.
    struct Readings {
        values: Vec<i32>,
        dropped: Rc<Cell<bool>>,
    }

    impl<'a> IntoIterator for &'a Readings {
        type Item = &'a i32;
        type IntoIter = std::slice::Iter<'a, i32>;

        fn into_iter(self) -> Self::IntoIter {
            self.values.iter()
        }
    }

    impl Drop for Readings {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    #[test]
    fn works_for_a_collection_only_iterable_by_reference() {
        let dropped = Rc::new(Cell::new(false));
        let readings = Readings {
            values: vec![4, 8, 15],
            dropped: Rc::clone(&dropped),
        };
        assert_eq!(stats(readings), (3, 27, Some(15)));
        // `stats` owned the readings, so they are gone once it returns: a
        // leaked container would never be dropped.
        assert!(dropped.get());
    }

    // A container that borrows its numbers from somewhere else, so it is not
    // `'static`.
    struct Window<'s> {
        values: &'s [i32],
    }

    impl<'a> IntoIterator for &'a Window<'_> {
        type Item = &'a i32;
        type IntoIter = std::slice::Iter<'a, i32>;

        fn into_iter(self) -> Self::IntoIter {
            self.values.iter()
        }
    }

    #[test]
    fn works_for_a_container_that_borrows_its_numbers() {
        // `for<'a>` does not demand `C: 'static`: this `Window` borrows from
        // a local array.
        let data = [1, -4, 9, 2];
        assert_eq!(stats(Window { values: &data[1..] }), (3, 7, Some(9)));
    }
}
