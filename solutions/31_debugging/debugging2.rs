// Module 5 · Debugging — part 2: hunting a logic bug that COMPILES.
//
// The nastiest bugs are the ones the compiler is happy with. This file builds
// fine, but the tests fail. Your job is to *locate* the defect, not just guess.
//
// Two everyday tools:
//   - `assert_eq!(a, b)` — when it fails it prints BOTH sides, so you instantly
//     see "expected 15, got 10" and can reason backwards.
//   - `{:?}` / the `dbg!` macro — sprinkle `dbg!(sum);` inside a loop to watch a
//     value evolve, then delete it once you understand the bug.
//
// `sum_to(n)` is supposed to return 1 + 2 + ... + n (the sum of every integer
// from 1 up to AND INCLUDING n). For n = 5 that is 1+2+3+4+5 = 15.

fn sum_to(n: u32) -> u32 {
    let mut sum = 0;
    // The bug was `1..n`, which stops BEFORE `n`. The inclusive range `1..=n`
    // adds `n` itself.
    for i in 1..=n {
        sum += i;
    }
    sum
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_to_five() {
        // 1 + 2 + 3 + 4 + 5 = 15
        assert_eq!(sum_to(5), 15);
    }

    #[test]
    fn sum_to_one() {
        // Just the single value 1.
        assert_eq!(sum_to(1), 1);
    }

    #[test]
    fn sum_to_ten() {
        // 1 + 2 + ... + 10 = 55
        assert_eq!(sum_to(10), 55);
    }
}
