// Closures - the Fn / FnMut / FnOnce hierarchy, part 3: `FnOnce` for a consuming
// closure.
//
// `FnOnce` sits at the bottom of the hierarchy, and the hierarchy nests:
// every `Fn` is also an `FnMut`, and every `FnMut` is also an `FnOnce`
// (`Fn` implies `FnMut` implies `FnOnce`). A closure that MOVES a captured
// value out of itself — for example, one that captures an owned `String` and
// then returns it — can only be called ONCE, because after the first call the
// captured value is gone. Such a closure implements `FnOnce` but NOT `Fn` or
// `FnMut`, so a bound of `F: Fn` is too strict to accept it.

fn run<F: Fn() -> String>(f: F) -> String {
    // TODO: The bound `F: Fn() -> String` is too strict. The closure passed in
    // by the test moves its captured `String` out of itself, so it can only be
    // called once — it implements `FnOnce`, not `Fn`. The compiler reports that
    // the closure "implements `FnOnce`, not `Fn`". Relax the bound to
    // `F: FnOnce() -> String`.
    // Until you relax the bound, this exercise will not compile.
    f()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_consuming_closure() {
        let owned = String::from("owned value");
        // `move` captures `owned` by value; returning it moves it OUT of the
        // closure, which makes the closure `FnOnce`.
        let closure = move || owned;
        assert_eq!(run(closure), "owned value");
    }
}
