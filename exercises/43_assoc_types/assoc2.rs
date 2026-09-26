// Traits & Abstraction · Associated types — part 2: `ConvertTo<T>` and `Add<Rhs>` (E0119, E0283, E0369, E0308).
//
// Part 1 was the case where a type has ONE natural answer: a graph has one
// node type. Now the opposite. A temperature in Celsius converts to Fahrenheit
// AND to Kelvin. With `type Target` in the trait, `Celsius` can implement it
// only once, and the second impl is E0119 "conflicting implementations".
// Coherence allows at most one impl of a given trait for a given type (the
// rest of the rules are in `42_coherence`), and `ConvertTo` is the same trait
// whatever its `Target`. A type PARAMETER changes that: `ConvertTo<Fahrenheit>`
// and `ConvertTo<Kelvin>` count as two different traits, so both impls can
// exist. That is the design of `From<T>`, `AsRef<T>` and `PartialEq<Rhs>`.
//
// The price is paid at the call site. With an associated type, `c.convert()`
// has exactly one possible type. With a parameter, rustc must know WHICH impl
// you mean, and when nothing says so it stops with E0283 "type annotations
// needed", noting "multiple `impl`s satisfying `Celsius: ConvertTo<_>` found".
// You name the target with a type annotation (`let f: Fahrenheit =
// c.convert();`), a turbofish on the trait (`ConvertTo::<Kelvin>::convert(&c)`)
// or a fully qualified path (`<Celsius as ConvertTo<Kelvin>>::convert(&c)`).
// Often the context names it for you, for example a `collect()` into a
// `Vec<Kelvin>`.
//
// Operator traits use BOTH kinds, and `std::ops::Add` shows why:
//
//     trait Add<Rhs = Self> {
//         type Output;
//         fn add(self, rhs: Rhs) -> Self::Output;
//     }
//
// `Rhs` is a parameter because one left-hand type may accept several
// right-hand types (`i32 + i32` and `i32 + &i32`), or one that is not `Self`
// (`String + &str`, `Instant + Duration`). It defaults to `Self`, so
// `impl Add for T` means `T + T`. `Output` is an associated type: once both
// operand types are known, the result type is fixed, so `a + b + c` type-checks
// without annotations. Two consequences come up in live coding all the time:
//
//   - `add` takes `self` BY VALUE, so `a + b` MOVES non-`Copy` operands. That
//     is deliberate: an owned left operand can hand its buffer to the result.
//     To add borrowed values, implement the trait for references as well
//     (std does this for its numbers, which is why `&1 + &2` compiles).
//   - In generic code, `T: Add` only promises that `T + T` is SOME type,
//     `<T as Add>::Output`. Nothing says it is `T` until the bound does.
//
// How interviewers probe this: "Why is `Add::Output` an associated type while
// `Rhs` is a parameter?", "Why does `a + b` move `a`, and how do you make
// `&a + &b` work?", and "Write `fn dot<T>(a: &[T], b: &[T]) -> T` for any
// numeric `T`."

use std::fmt;
use std::ops::{Add, Mul};

// ---- Part A: one source type, several targets ----

#[derive(Debug, Clone, Copy, PartialEq)]
struct Celsius(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fahrenheit(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Kelvin(f64);

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.1} °C", self.0)
    }
}

impl fmt::Display for Fahrenheit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.1} °F", self.0)
    }
}

impl fmt::Display for Kelvin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.2} K", self.0)
    }
}

// TODO: The second impl is E0119 "conflicting implementations of trait
// `ConvertTo` for type `Celsius`", and each `ConvertTo<Kelvin>` or
// `ConvertTo::<Fahrenheit>` in the tests is E0107 "trait takes 0 generic
// arguments but 1 generic argument was supplied". rustc's help there, "replace
// the generic bound with the associated type", points the wrong way:
// `ConvertTo<Target = Kelvin>` would still allow only one impl per type.
// Redesign the trait so that one type can convert to several targets, named by
// the trait's single type parameter the way the tests write it, and rewrite
// both impls to match. Keep the method name and the two formulas. Until you
// let `Celsius` implement the trait once per target, this exercise will not
// compile.
trait ConvertTo {
    type Target;

    fn convert(&self) -> Self::Target;
}

impl ConvertTo for Celsius {
    type Target = Fahrenheit;

    fn convert(&self) -> Fahrenheit {
        Fahrenheit(self.0 * 9.0 / 5.0 + 32.0)
    }
}

impl ConvertTo for Celsius {
    type Target = Kelvin;

    fn convert(&self) -> Kelvin {
        Kelvin(self.0 + 273.15)
    }
}

// One line of a weather report: "20.0 °C = 68.0 °F = 293.15 K".
fn report(c: Celsius) -> String {
    // TODO: Once the trait takes a parameter, both `let`s below are E0283
    // "type annotations needed", with the note "multiple `impl`s satisfying
    // `Celsius: ConvertTo<_>` found": nothing in these lines says which target
    // each call wants. Say it at each call site, without changing the output
    // or the trait. Until you name the target of both calls, this exercise
    // will not compile.
    let f = c.convert();
    let k = c.convert();
    format!("{c} = {f} = {k}")
}

// ---- Part B: operators on a type that owns a buffer ----

// A vector of any dimension. It owns a heap buffer, so it is not `Copy`, and
// it is deliberately not `Clone` either. Adding vectors of different
// dimensions is a bug, so every `+` panics with "dimension mismatch".
#[derive(Debug, PartialEq)]
struct Vector(Vec<f64>);

impl Vector {
    fn zeros(dim: usize) -> Self {
        Vector(vec![0.0; dim])
    }
}

// `Vector + Vector`: both operands move in, and the result reuses the left
// operand's buffer, so no allocation happens.
impl Add for Vector {
    type Output = Vector;

    fn add(mut self, rhs: Vector) -> Vector {
        assert_eq!(self.0.len(), rhs.0.len(), "dimension mismatch");
        for (x, y) in self.0.iter_mut().zip(&rhs.0) {
            *x += y;
        }
        self
    }
}

// TODO: The tests add vectors that they still use afterwards. `&a + &b` is
// E0369 "cannot add `&Vector` to `&Vector`", and folding borrowed vectors into
// an owned total, `total + v` with `v: &Vector`, is E0308 "mismatched types:
// expected `Vector`, found `&Vector`". Make both of them work. Requirements:
//   - `&a + &b` leaves `a` and `b` as they were and returns a new `Vector`;
//   - `total + v` consumes `total` and reuses its buffer, like the impl above
//     (a test compares heap pointers);
//   - operands of different dimensions panic with "dimension mismatch" and are
//     never silently truncated;
//   - `Vector` stays neither `Clone` nor `Copy`, and the tests stay as they
//     are.
// Until you implement both borrowed forms of `+`, this exercise will not
// compile.

// ---- Part C: generic numeric code ----

// The dot product of two slices of the same length, for any numeric type.
// TODO: Both operators in the `fold` are E0308 "mismatched types", with the
// notes "expected type parameter `T`, found associated type `<T as
// Mul>::Output`" and "... `<T as Add>::Output`". `T: Mul` says that `T * T`
// has SOME type, not that it is `T`, and the running total must stay a `T`.
// The body is fine; the bounds are what is missing. The function must stay
// generic (the tests use integers, floats and `Wrapping<u8>`) and keep
// panicking on slices of different lengths. rustc's help shows the syntax;
// make sure you can explain why it is needed. Until you make the bounds say
// what the operators produce, this exercise will not compile.
fn dot<T>(a: &[T], b: &[T]) -> T
where
    T: Add + Mul + Copy + Default,
{
    assert_eq!(a.len(), b.len(), "dimension mismatch");
    a.iter()
        .zip(b)
        .fold(T::default(), |acc, (&x, &y)| acc + x * y)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::Wrapping;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // ---- Part A ----

    #[test]
    fn boiling_point_in_both_scales() {
        let boiling = Celsius(100.0);
        // A type annotation picks the impl...
        let f: Fahrenheit = boiling.convert();
        assert_eq!(f, Fahrenheit(212.0));
        // ...and so does a fully qualified path.
        let k = <Celsius as ConvertTo<Kelvin>>::convert(&boiling);
        assert!(close(k.0, 373.15), "got {k:?}");
    }

    #[test]
    fn minus_forty_is_the_same_in_both_scales() {
        let f = ConvertTo::<Fahrenheit>::convert(&Celsius(-40.0));
        assert_eq!(f, Fahrenheit(-40.0));
    }

    #[test]
    fn the_context_can_pick_the_target() {
        // `collect` into `Vec<Kelvin>` is enough for rustc to know which
        // `convert` each call means.
        let readings: Vec<Kelvin> = [Celsius(0.0), Celsius(-273.15)]
            .iter()
            .map(|c| c.convert())
            .collect();
        assert!(close(readings[0].0, 273.15), "got {readings:?}");
        assert!(close(readings[1].0, 0.0), "got {readings:?}");
    }

    #[test]
    fn report_shows_every_scale() {
        assert_eq!(report(Celsius(20.0)), "20.0 °C = 68.0 °F = 293.15 K");
        assert_eq!(report(Celsius(-40.0)), "-40.0 °C = -40.0 °F = 233.15 K");
    }

    // ---- Part B ----

    #[test]
    fn adding_borrowed_vectors_keeps_both() {
        let a = Vector(vec![1.0, 2.0, 3.0]);
        let b = Vector(vec![10.0, 20.0, 30.0]);
        let c = &a + &b;
        assert_eq!(c, Vector(vec![11.0, 22.0, 33.0]));
        // `a` and `b` were only borrowed, so both are still here, unchanged,
        // and can be added again.
        assert_eq!(a, Vector(vec![1.0, 2.0, 3.0]));
        assert_eq!(b, Vector(vec![10.0, 20.0, 30.0]));
        assert_eq!(&c + &a, Vector(vec![12.0, 24.0, 36.0]));
    }

    #[test]
    fn owned_plus_owned_still_works() {
        let a = Vector(vec![1.0, 2.0]);
        let buffer = a.0.as_ptr();
        let sum = a + Vector(vec![3.0, 4.0]);
        assert_eq!(sum, Vector(vec![4.0, 6.0]));
        assert_eq!(sum.0.as_ptr(), buffer);
    }

    #[test]
    fn folding_borrowed_vectors_reuses_one_buffer() {
        let parts = [
            Vector(vec![1.0, 2.0]),
            Vector(vec![3.0, 4.0]),
            Vector(vec![5.0, 6.0]),
        ];
        let total = parts.iter().fold(Vector::zeros(2), |total, v| {
            let buffer = total.0.as_ptr();
            let total = total + v;
            // The owned left operand handed its buffer to the result. Had
            // `+` allocated instead, the new buffer would have been made while
            // `total` was still alive, so it could not have the same address.
            assert_eq!(total.0.as_ptr(), buffer, "`total + v` allocated");
            total
        });
        assert_eq!(total, Vector(vec![9.0, 12.0]));
        // The parts were only borrowed.
        assert_eq!(parts[2], Vector(vec![5.0, 6.0]));
    }

    #[test]
    fn zero_dimensional_vectors_add_up_to_nothing() {
        let empty = Vector::zeros(0);
        assert_eq!(&empty + &empty, Vector(Vec::new()));
        assert_eq!(Vector::zeros(0) + &empty, Vector(Vec::new()));
    }

    #[test]
    #[should_panic(expected = "dimension mismatch")]
    fn borrowed_plus_borrowed_rejects_different_dimensions() {
        let _ = &Vector(vec![1.0, 2.0]) + &Vector(vec![1.0, 2.0, 3.0]);
    }

    #[test]
    #[should_panic(expected = "dimension mismatch")]
    fn owned_plus_borrowed_rejects_different_dimensions() {
        let _ = Vector(vec![1.0, 2.0, 3.0]) + &Vector(vec![1.0, 2.0]);
    }

    // ---- Part C ----

    #[test]
    fn dot_works_for_integers_and_floats() {
        assert_eq!(dot(&[1, 2, 3], &[4, 5, 6]), 32);
        assert_eq!(dot(&[-2i64, 7], &[3, 1]), 1);
        assert_eq!(dot(&[0.5, 1.5], &[2.0, 4.0]), 7.0);
    }

    #[test]
    fn dot_of_empty_slices_is_zero() {
        assert_eq!(dot::<u64>(&[], &[]), 0);
    }

    #[test]
    fn dot_works_for_any_type_with_the_operators() {
        // `Wrapping<u8>` is not a primitive, but it has `+`, `*`, `Copy` and a
        // zero `Default`. 200 * 2 wraps around to 144; plus 1 * 3 is 147.
        let a = [Wrapping(200u8), Wrapping(1)];
        let b = [Wrapping(2u8), Wrapping(3)];
        assert_eq!(dot(&a, &b), Wrapping(147));
    }

    #[test]
    #[should_panic(expected = "dimension mismatch")]
    fn dot_rejects_different_lengths() {
        dot(&[1, 2, 3], &[1, 2]);
    }
}
