// Iterators - implementing Iterator, custom adapters, laziness, part 1: writing
// only `next`.
//
// The `Iterator` trait has ONE required method: `fn next(&mut self) ->
// Option<Self::Item>`. Everything else - `take`, `map`, `filter`, `sum`,
// `collect`, and dozens more - are DEFAULT methods the trait provides for free,
// all built on top of your `next`. So to make any type iterable you implement a
// single method that answers "what is the next element, and are there more?"
// (`Some(x)` = here's one, `None` = done). Here we build an UNBOUNDED iterator:
// it keeps producing values for as long as the caller asks, so the caller is the
// one that stops it, with `take`.
//
// "Unbounded" is not quite "infinite", though: F(94) is the first Fibonacci
// number too big for a `u64`. Because each step works out the number two ahead
// before it yields the current one, this iterator ends after 92 values. The honest
// way to stop is `checked_add` plus `None` — a plain `+` would panic with "attempt
// to add with overflow" in debug builds and silently wrap in release (the exact
// trap `31_debugging/debugging2` is about).

struct Fibonacci {
    curr: u64,
    next: u64,
}

impl Iterator for Fibonacci {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: Yield the current Fibonacci number and advance the pair — the new
        // `curr` is the old `next`, the new `next` is their sum. Add with
        // `checked_add` and end the iteration with `None` once `u64` can no longer
        // hold the next value, and make sure you bail BEFORE mutating the state so
        // that repeated calls keep reporting exhaustion. An empty body evaluates
        // to `()`, not `Option<u64>`, so this will not compile yet.
    }
}

fn fibonacci() -> Fibonacci {
    Fibonacci { curr: 0, next: 1 }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_ten() {
        // `take(10)` is a default method that wraps our iterator and stops after
        // ten `next` calls - our `next` would happily keep going.
        let got: Vec<u64> = fibonacci().take(10).collect();
        assert_eq!(got, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
    }

    #[test]
    fn ends_instead_of_overflowing_u64() {
        // `checked_add` turns "`u64` ran out" into an ordinary end-of-iteration
        // rather than an "attempt to add with overflow" panic.
        let mut it = fibonacci();
        let all: Vec<u64> = it.by_ref().collect();
        assert_eq!(all.len(), 92);
        assert_eq!(*all.last().unwrap(), 4_660_046_610_375_530_309);
        // And exhaustion sticks, because we bailed before mutating the state.
        assert_eq!(it.next(), None);
    }

    #[test]
    fn map_then_sum() {
        // `map` and `sum` are also default methods driven by our `next`.
        let doubled: u64 = fibonacci().take(6).map(|x| x * 2).sum();
        assert_eq!(doubled, 24);
    }
}
