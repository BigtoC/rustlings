// Iterators - implementing Iterator, custom adapters, laziness, part 1: writing
// only `next`.
//
// The `Iterator` trait has ONE required method: `fn next(&mut self) ->
// Option<Self::Item>`. Everything else - `take`, `map`, `filter`, `sum`,
// `collect`, and dozens more - are DEFAULT methods the trait provides for free,
// all built on top of your `next`. So to make any type iterable you implement a
// single method that answers "what is the next element, and are there more?"
// (`Some(x)` = here's one, `None` = done). Here we build an INFINITE iterator:
// `next` never returns `None`, and the caller decides when to stop with `take`.

struct Fibonacci {
    curr: u64,
    next: u64,
}

impl Iterator for Fibonacci {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        // Remember the current value, advance the pair, then yield the old one.
        let current = self.curr;
        self.curr = self.next;
        self.next += current;
        Some(current)
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
        // ten `next` calls - our `next` never stops on its own.
        let got: Vec<u64> = fibonacci().take(10).collect();
        assert_eq!(got, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
    }

    #[test]
    fn map_then_sum() {
        // `map` and `sum` are also default methods driven by our `next`.
        let doubled: u64 = fibonacci().take(6).map(|x| x * 2).sum();
        assert_eq!(doubled, 24);
    }
}
