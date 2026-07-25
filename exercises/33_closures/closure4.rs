// Closures - the Fn / FnMut / FnOnce hierarchy, part 4: returning closures,
// `impl Fn` vs `Box<dyn Fn>`.
//
// Every closure that CAPTURES something has a UNIQUE, anonymous,
// compiler-generated type — even two closures with identical signatures are
// distinct types. A return type of `-> impl Fn(..)` means "I return exactly ONE
// concrete type; I just won't name it", so it works only when every code path
// returns the *same* closure. When two `if`/`else` arms build DIFFERENT
// capturing closures they have different types, and `impl Trait` cannot unify
// them (E0308). The fix is a trait object: `Box<dyn Fn(..)>` erases each
// concrete closure type behind a pointer plus a vtable (dynamic dispatch — the
// same mechanism as the trait-objects module), giving both arms one shared
// return type.

// `adder` returns the same closure type on every call, so `impl Fn` fits.
fn adder(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y
}

fn make_op(add: bool) -> impl Fn(i32) -> i32 {
    let step = 1;
    // TODO: The two arms build DIFFERENT closures (`move |x| x + step` and
    // `move |x| x - step`); because each captures `step`, they are distinct
    // anonymous types. With `-> impl Fn(i32) -> i32` the compiler needs a single
    // concrete return type and reports E0308: "`if` and `else` have incompatible
    // types". Change the return type to `Box<dyn Fn(i32) -> i32>` and wrap each
    // arm in `Box::new(...)`:
    //     if add { Box::new(move |x| x + step) } else { Box::new(move |x| x - step) }
    // Until you box both arms behind a trait object, this exercise will not
    // compile.
    if add {
        move |x| x + step
    } else {
        move |x| x - step
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impl_fn_adder() {
        assert_eq!(adder(3)(4), 7);
    }

    #[test]
    fn boxed_dyn_fn_selects_op() {
        let f = make_op(true);
        assert_eq!(f(10), 11);
        let g = make_op(false);
        assert_eq!(g(10), 9);
    }
}
