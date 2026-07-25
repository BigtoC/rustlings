// Closures - the Fn / FnMut / FnOnce hierarchy, part 1: `move` capture for an
// escaping closure.
//
// A closure captures the variables it uses from the enclosing scope, and the
// compiler picks the *least-privileged* capture mode the body actually needs:
// by shared reference (`&T`) if reading is enough, by exclusive reference
// (`&mut T`) if it mutates, and by value only when forced. That inferred
// default is fine while the closure stays put — but a closure RETURNED from a
// function outlives the function's own local variables. A borrow of a local
// would dangle the instant the function returns, so the borrow checker refuses
// it. The `move` keyword forces the closure to capture its variables BY VALUE,
// so it owns them and can carry them out of the function.

fn make_greeter(name: String) -> impl Fn() -> String {
    // `move` makes the closure own `name`, so it can outlive `make_greeter`.
    move || format!("Hello, {name}!")
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeter_owns_its_name() {
        let g = make_greeter("Sam".to_string());
        // An `Fn` closure takes `&self`, so it can be called any number of times.
        assert_eq!(g(), "Hello, Sam!");
        assert_eq!(g(), "Hello, Sam!");
    }
}
