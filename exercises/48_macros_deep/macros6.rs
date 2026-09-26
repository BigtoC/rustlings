// Traits & Abstraction · Declarative macros — part 2: evaluate each argument once: a recursive `max!` that binds before it compares.
//
// A macro argument is not a value. `$x:expr` captures an EXPRESSION, and every
// `$x` in the transcriber pastes that expression again, so it runs again. The
// classic C bug is `#define MAX(a, b) ((a) > (b) ? (a) : (b))`: `MAX(i++, j)`
// increments `i` twice when it wins. `macro_rules!` fixes two of C's
// problems, but not this one:
//
//   - An `expr` fragment keeps its grouping. A C macro pastes text, so
//     `#define DOUBLE(x) x * 2` turns `DOUBLE(1 + 1)` into `1 + 1 * 2`, 3.
//     Rust pastes the parsed expression, so the same macro gives 4.
//   - Local variables the macro declares are hygienic. A `let first = ..`
//     inside the expansion is a different variable from the caller's
//     `first`, even though they have the same name, so the macro can
//     neither read, overwrite nor shadow the caller's variables.
//   - What `macro_rules!` does NOT fix: an expression pasted twice is
//     evaluated twice. A function call, a closure call, an `i += 1` in a
//     block, a lock, an allocation: all of it happens once per paste.
//
// The `max!` below is recursive: it compares its first argument with the
// `max!` of the rest. It pastes `$x` twice and the recursive call twice, and
// the second problem is far worse than the first. Every level can expand the
// rest of the list twice, so with arguments in increasing order,
// `max!(a1, .., an)` makes 2^n - 1 calls: 7 calls for three arguments, 1023
// for ten. `std::cmp::max` is a function, so its arguments are evaluated
// once, before the call. A macro earns its keep here only by taking any
// number of arguments, and it must evaluate each of them once, left to
// right, like a function would.
//
// The fix is to bind. Evaluate each argument into a local exactly once, then
// work with the locals. std does this everywhere: `assert_eq!` expands to
// `match (&$left, &$right) { (left_val, right_val) => .. }`, and `dbg!` to
// `match $val { tmp => .. }` (its source notes that the `match` is
// intentional, because of how long temporaries live). A transcriber written
// with double braces, `=> {{ .. }}`, expands to a block expression: the outer
// pair delimits the transcriber, the inner pair is a block, so the macro can
// declare locals and still be used wherever an expression goes.
//
// Interviewers ask: "What is wrong with a `max!` that pastes `$a` twice?",
// "Are `macro_rules!` macros hygienic, and which names does that cover?",
// "How does `assert_eq!` avoid evaluating its arguments twice?" and "When
// would you write a macro instead of a generic function?".

// TODO: Five tests fail. `each_argument_is_evaluated_exactly_once` reports
// `left: 7, right: 3` (three arguments, seven calls),
// `arguments_are_evaluated_left_to_right` logs `[3, 9, 4, 9, 9, 4, 9]`,
// `the_work_grows_linearly_not_exponentially` counts 1023 calls for ten
// arguments, and the side-effect and nested-call tests fail the same way.
// Make `max!` evaluate every argument exactly once, left to right.
// Requirements: keep the result the same (the maximum by `>`, so it keeps
// working for any `PartialOrd` type, floats and strings included); keep
// accepting one or more arguments with an optional trailing comma; the
// arguments may be values that are neither `Copy` nor `Clone`; no helper
// function, the fix goes in the macro. Until you evaluate every argument
// exactly once, the tests will fail.
macro_rules! max {
    ($x:expr $(,)?) => {
        $x
    };
    ($x:expr, $($rest:expr),+ $(,)?) => {
        if $x > max!($($rest),+) {
            $x
        } else {
            max!($($rest),+)
        }
    };
}

// Given, complete: a caller. An auction closes at the highest of the opening
// price and the two bids.
fn closing_price(opening: u32, first_bid: u32, second_bid: u32) -> u32 {
    max!(opening, first_bid, second_bid)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    // Hands back the value it is given, and counts and logs every call.
    #[derive(Default)]
    struct Probe {
        calls: Cell<usize>,
        log: RefCell<Vec<i32>>,
    }

    impl Probe {
        fn next(&self, value: i32) -> i32 {
            self.calls.set(self.calls.get() + 1);
            self.log.borrow_mut().push(value);
            value
        }
    }

    #[test]
    fn each_argument_is_evaluated_exactly_once() {
        let probe = Probe::default();
        let biggest = max!(probe.next(3), probe.next(9), probe.next(4));
        assert_eq!(biggest, 9);
        assert_eq!(probe.calls.get(), 3);
    }

    #[test]
    fn arguments_are_evaluated_left_to_right() {
        let probe = Probe::default();
        let biggest = max!(probe.next(3), probe.next(9), probe.next(4));
        assert_eq!(biggest, 9);
        assert_eq!(*probe.log.borrow(), [3, 9, 4]);
    }

    #[test]
    fn the_work_grows_linearly_not_exponentially() {
        let probe = Probe::default();
        let biggest = max!(
            probe.next(1),
            probe.next(2),
            probe.next(3),
            probe.next(4),
            probe.next(5),
            probe.next(6),
            probe.next(7),
            probe.next(8),
            probe.next(9),
            probe.next(10),
        );
        assert_eq!(biggest, 10);
        assert_eq!(probe.calls.get(), 10);
    }

    #[test]
    fn a_side_effect_in_an_argument_happens_once() {
        let mut ticks = 0;
        let biggest = max!(
            {
                ticks += 5;
                ticks
            },
            3
        );
        assert_eq!(biggest, 5);
        assert_eq!(ticks, 5);
    }

    #[test]
    fn finds_the_maximum_wherever_it_is() {
        assert_eq!(max!(9, 1, 1), 9);
        assert_eq!(max!(1, 9, 1), 9);
        assert_eq!(max!(1, 1, 9), 9);
        assert_eq!(max!(-4, -2, -8, -3), -2);
        assert_eq!(max!(5, 5, 5), 5);
    }

    #[test]
    fn the_auction_closes_at_the_highest_price() {
        assert_eq!(closing_price(100, 120, 110), 120);
        assert_eq!(closing_price(100, 90, 95), 100);
        assert_eq!(closing_price(0, 0, 1), 1);
    }

    #[test]
    fn one_argument_and_trailing_commas() {
        assert_eq!(max!(7), 7);
        assert_eq!(max!(7,), 7);
        assert_eq!(max!(2, 8, 5,), 8);
    }

    #[test]
    fn works_for_any_partial_ord_type() {
        // `f64` is `PartialOrd` but not `Ord`.
        assert_eq!(max!(1.5, -2.0, 0.25), 1.5);
        assert_eq!(max!("pear", "apple", "zebra", "mango"), "zebra");
        assert_eq!(max!('a', 'z', 'm'), 'z');
    }

    #[test]
    fn moves_arguments_that_are_not_copy() {
        let biggest = max!(
            String::from("pear"),
            String::from("zebra"),
            String::from("apple"),
        );
        assert_eq!(biggest, "zebra");
        let longest = max!(vec![1, 2], vec![1, 2, 3], vec![0; 2]);
        assert_eq!(longest, [1, 2, 3]);

        // Neither `Copy` nor `Clone`: the macro can only move it.
        #[derive(Debug, PartialEq, PartialOrd)]
        struct Bid(u32);
        let best = max!(Bid(30), Bid(95), Bid(40));
        assert_eq!(best, Bid(95));
    }

    #[test]
    fn the_macros_locals_do_not_capture_the_callers_variables() {
        // Hygiene: whatever the macro names its locals, they are not these.
        let first = 1;
        let rest = 10;
        let a = 4;
        let b = 7;
        assert_eq!(max!(rest, first, 3), 10);
        assert_eq!(max!(first, a, b), 7);
        assert_eq!(max!(b, a), 7);
        assert_eq!((first, rest, a, b), (1, 10, 4, 7));
    }

    #[test]
    fn nested_calls_work() {
        assert_eq!(max!(max!(1, 2), 3, max!(0, -1)), 3);
        let probe = Probe::default();
        let biggest = max!(max!(probe.next(1), probe.next(6)), probe.next(2));
        assert_eq!(biggest, 6);
        assert_eq!(*probe.log.borrow(), [1, 6, 2]);
    }
}
