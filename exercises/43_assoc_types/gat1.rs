// Traits & Abstraction · Associated types — part 3: a lending iterator with a GAT, `type Item<'a>`.
//
// `slice::windows(n)` yields overlapping windows as `&[T]`, but std has no
// `windows_mut`. That is not an oversight. std's `Iterator` fixes ONE item
// type for the whole iteration:
//
//     trait Iterator {
//         type Item;
//         fn next(&mut self) -> Option<Self::Item>;
//     }
//
// `Item` cannot mention the borrow of `self` that a call to `next` makes, so
// every item must stay valid on its own, however many more calls follow. A
// caller may keep all the items at once, and `collect()` does exactly that.
// `iter_mut()` can live with this: its `&mut T` items never overlap, so the
// iterator hands its borrow of the slice over piece by piece. Overlapping
// mutable windows cannot: over five elements, windows `[0..3]` and `[1..4]`
// share elements 1 and 2, and holding both would mean two live `&mut` to the
// same elements: a compile error in safe code, and undefined behavior even in
// `unsafe` code. Window 1 may only exist once window 0 is gone.
//
// A LENDING iterator promises exactly that. Each item borrows from the
// iterator itself, so the item type needs a lifetime parameter of its own: a
// generic associated type (GAT, stable since Rust 1.65).
//
//     trait LendingIterator {
//         type Item<'a> where Self: 'a;
//         fn next(&mut self) -> Option<Self::Item<'_>>;
//     }
//
// `next` returns `Item<'_>`, the item for THIS borrow of `self`. While that
// item is alive the iterator stays mutably borrowed, so the borrow checker
// rejects a second call to `next` until the item is gone. `where Self: 'a`
// says that `Item<'a>` only exists for lifetimes the iterator outlives, and
// rustc insists on it: delete it from the trait below and you get "missing
// required bound on `Item`".
//
// The cost: a lending iterator is not an `Iterator`. There is no `for` loop
// (that needs `IntoIterator`), no `collect`, and none of the adapters. You
// drive it by hand with `while let Some(item) = it.next() { .. }`.
//
// How interviewers probe this: "What are GATs for?", "Why can't std's
// `Iterator` yield items that borrow from the iterator?", and "Why is there
// `windows` but no `windows_mut`?"

// An iterator whose items borrow from the iterator itself. Provided; don't
// change it.
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

// Overlapping windows of `size` elements, lent out one at a time as
// `&mut [T]`. Over five elements, windows of 3 are `[0..3]`, `[1..4]` and
// `[2..5]`. A window longer than the slice never fits, so there are none.
struct WindowsMut<'s, T> {
    slice: &'s mut [T],
    size: usize,
    start: usize,
}

impl<'s, T> WindowsMut<'s, T> {
    fn new(slice: &'s mut [T], size: usize) -> Self {
        assert!(size > 0, "window size must be non-zero");
        WindowsMut {
            slice,
            size,
            start: 0,
        }
    }
}

// TODO: rustc rejects `next` with "lifetime may not live long enough ...
// method was supposed to return data with lifetime `'s` but it is returning
// data with lifetime `'1`" (no error code), and the tests' `count` helper with
// E0277 "the trait bound `WindowsMut<'_, {integer}>: LendingIterator` is not
// satisfied". `'1` is the borrow of `self` in this call to `next`. The window
// can only borrow the slice THROUGH that borrow, but `Item = &'s mut [T]`
// promises a window that outlives it, while the next call hands out an
// overlapping one. Make `WindowsMut` implement the `LendingIterator` trait
// above INSTEAD of `Iterator`, so that each window borrows the iterator.
// Requirements:
//   - the windows overlap, come in order and are each exactly `size` long;
//     they are views of the original elements (the tests write through them
//     and compare pointers), never copies;
//   - `WindowsMut` stays generic over any `T` (the tests use `String`s);
//   - no `unsafe`, no `Cell` or `RefCell`, and no changes to the trait or
//     the tests.
// Until you make `WindowsMut` lend its windows, this exercise will not
// compile.
impl<'s, T> Iterator for WindowsMut<'s, T> {
    type Item = &'s mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        // `get_mut(..)` returns `None` once the window would run past the end,
        // however large `size` is.
        let window = self.slice.get_mut(self.start..)?.get_mut(..self.size)?;
        self.start += 1;
        Some(window)
    }
}

// Replaces each element with the sum of itself and everything before it:
// `[1, 2, 3, 4]` becomes `[1, 3, 6, 10]`. Each window of two adds its first
// element to its second, and the next window starts at that updated element.
fn prefix_sums_in_place(values: &mut [i64]) {
    // TODO: Once `WindowsMut` implements `LendingIterator` and not `Iterator`,
    // this loop is E0277 "`WindowsMut<'_, i64>` is not an iterator", because a
    // `for` loop needs `IntoIterator`. Drive the lending iterator by hand
    // instead, with the same windows of two and the same update. Until you
    // drive the windows without `Iterator`, this exercise will not compile.
    for window in WindowsMut::new(values, 2) {
        window[1] += window[0];
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Works for ANY lending iterator: it never holds more than one item.
    fn count<L: LendingIterator>(mut it: L) -> usize {
        let mut n = 0;
        while it.next().is_some() {
            n += 1;
        }
        n
    }

    // An owned copy of every window's contents. Collecting the windows
    // themselves is impossible: each one must be dropped before `next` can
    // lend the following one. Collecting data MADE from each window is fine.
    fn contents<T: Clone>(slice: &mut [T], size: usize) -> Vec<Vec<T>> {
        let mut out = Vec::new();
        let mut windows = WindowsMut::new(slice, size);
        while let Some(window) = windows.next() {
            out.push(window.to_vec());
        }
        out
    }

    #[test]
    fn prefix_sums_of_one_to_four() {
        let mut v = [1, 2, 3, 4];
        prefix_sums_in_place(&mut v);
        assert_eq!(v, [1, 3, 6, 10]);
    }

    #[test]
    fn prefix_sums_of_short_and_signed_slices() {
        let mut empty: [i64; 0] = [];
        prefix_sums_in_place(&mut empty);
        assert_eq!(empty, []);

        let mut one = [5];
        prefix_sums_in_place(&mut one);
        assert_eq!(one, [5]);

        let mut signed = [-1, 1, -1, 1, 10];
        prefix_sums_in_place(&mut signed);
        assert_eq!(signed, [-1, 0, -1, 0, 10]);
    }

    #[test]
    fn three_overlapping_windows_of_three_over_five() {
        let mut v = [1, 2, 3, 4, 5];
        assert_eq!(count(WindowsMut::new(&mut v, 3)), 3);
        assert_eq!(
            contents(&mut v, 3),
            [[1, 2, 3], [2, 3, 4], [3, 4, 5]].map(Vec::from)
        );
        // Windows of one are the elements, and one window covers all five.
        assert_eq!(count(WindowsMut::new(&mut v, 1)), 5);
        assert_eq!(contents(&mut v, 5), [vec![1, 2, 3, 4, 5]]);
    }

    #[test]
    fn empty_or_oversized_yields_no_windows() {
        let mut empty: [u8; 0] = [];
        assert!(WindowsMut::new(&mut empty, 1).next().is_none());

        let mut v = [1, 2, 3, 4, 5];
        assert!(WindowsMut::new(&mut v, 6).next().is_none());
        assert!(WindowsMut::new(&mut v, usize::MAX).next().is_none());
        assert_eq!(count(WindowsMut::new(&mut v, 6)), 0);
    }

    #[test]
    #[should_panic(expected = "window size must be non-zero")]
    fn a_window_of_zero_is_rejected() {
        let mut v = [1, 2, 3];
        WindowsMut::new(&mut v, 0);
    }

    #[test]
    fn windows_are_views_into_the_slice() {
        let mut v = [10, 20, 30, 40];
        let start = v.as_ptr();
        let mut windows = WindowsMut::new(&mut v, 2);
        let mut i = 0;
        while let Some(window) = windows.next() {
            // Window `i` starts at element `i` of the original array.
            assert_eq!(window.as_ptr(), start.wrapping_add(i));
            assert_eq!(window.len(), 2);
            i += 1;
        }
        assert_eq!(i, 3);
    }

    #[test]
    fn a_write_through_one_window_is_seen_by_the_next() {
        // One bubble-sort pass: the largest element travels to the end only
        // because each window starts with the element the previous window
        // just moved.
        let mut v = [5, 1, 4, 2, 3];
        let mut windows = WindowsMut::new(&mut v, 2);
        while let Some(pair) = windows.next() {
            if pair[0] > pair[1] {
                pair.swap(0, 1);
            }
        }
        assert_eq!(v, [1, 4, 2, 3, 5]);
    }

    #[test]
    fn works_for_elements_that_are_not_copy() {
        let mut words = ["a", "b", "c"].map(String::from);
        let mut windows = WindowsMut::new(&mut words, 2);
        while let Some(pair) = windows.next() {
            let (left, right) = pair.split_at_mut(1);
            right[0].insert_str(0, &left[0]);
        }
        // Each `String` was edited where it lives, in the array.
        assert_eq!(words, ["a", "ab", "abc"]);
    }
}
