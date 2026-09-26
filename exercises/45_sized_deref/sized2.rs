// Traits & Abstraction · `?Sized` and unsized types — part 6: forwarding impls for `Box<S>` and `&S`, so pointers to a trait's implementors satisfy its bound (E0277, E0599).
//
// `total_area<S: Shape>(shapes: &[S])` happily takes a `&[Circle]`. Hand it
// a `&[Box<dyn Shape>]` and rustc says E0277 "the trait bound
// `Box<dyn Shape>: Shape` is not satisfied", although `boxed.area()`
// compiles. A method call auto-derefs through the box; a trait bound is
// checked against the exact type (the lesson of `deref2`, where `Deref` did
// not make an `Admin` a `Greet`). The compiler does implement `Shape` for
// `dyn Shape` itself, for every dyn-compatible trait, so a `&dyn Shape` can
// call `area` through the vtable. But `Box<dyn Shape>`, `&dyn Shape` and
// `&Circle` are other types, and nobody implemented `Shape` for them.
//
// The standard answer is a FORWARDING IMPL: a generic impl for a pointer
// type whose every method calls the same method on the value behind the
// pointer. std writes them for its own traits all the time:
// `impl<T: Display + ?Sized> Display for Box<T>` (and `Display` for `&T`),
// `impl<I: Iterator + ?Sized> Iterator for &mut I` and for `Box<I>`,
// `io::Write` for `&mut W`, the `Fn` traits for `&F` and `Box<F>`. That is
// why a `Box<dyn Display>` prints and a `&mut iter` can be passed to a
// function that takes `impl Iterator`. It is never automatic, because
// forwarding is not always the right meaning: `Clone for Box<T>` clones the
// value into a new box, and `impl<E: Error> Error for Box<E>` deliberately
// leaves out `?Sized` (see `35_error_design`).
//
// The `?Sized` is the point of the impl. The pointers you most want covered
// are `Box<dyn Shape>` and `&dyn Shape`, and their pointee is unsized. With
// only `impl<S: Shape> Shape for Box<S>`, a `Box<Circle>` is a `Shape` but a
// `Box<dyn Shape>` still is not: E0277 "the size for values of type
// `dyn Shape` cannot be known at compilation time". Nor is one concrete
// `impl Shape for Box<dyn Shape>` enough: `dyn Shape + Send` is a different
// unsized type (think `Box<dyn Error + Send + Sync>`), and only a `?Sized`
// type parameter covers every trait object at once.
//
// Two classic bugs in a forwarding impl. First, calling the method on `self`
// inside the impl: method lookup finds the wrapper's own impl before it
// looks behind the pointer, so the method calls itself forever (rustc only
// warns, "function cannot return without recursing", and the program
// overflows its stack). Second, forgetting a PROVIDED method: a method with
// a default body that you don't forward runs that default on the wrapper
// and silently ignores the override of the value inside. std's
// `Iterator for &mut I` forwards `size_hint` and `nth` for that reason.
//
// Part B is the same trick for test doubles. `SignupService<M: Mailer>`
// owns its mailer (as in `50_testing_seams/seams1`, which read the mock back
// through `service.mailer()`). Once a shared reference to a mailer is itself
// a mailer, a test can keep the mock and lend the service `&mock`, or a
// `&dyn Mailer`, and read the mock whenever it likes.
// The Rust API Guidelines recommend the same shape for I/O (C-RW-VALUE):
// take `R: Read` by value, and callers who want to keep their reader pass
// `&mut reader`, which works because std forwards `Read` through `&mut R`.
//
// How interviewers probe it: "Why doesn't `Box<dyn Shape>` implement `Shape`
// automatically?", "Write the impl that makes it work. Why `?Sized`?",
// "What is wrong with `fn area(&self) -> f64 { self.area() }` in it?", and
// "How can a function that takes `R: Read` by value leave the caller its
// reader?"

use std::cell::RefCell;
use std::f64::consts::PI;

// ---- Part A — shapes behind pointers --------------------------------------

trait Shape {
    fn area(&self) -> f64;

    // A provided method. `Circle` and `Square` override it.
    fn name(&self) -> &'static str {
        "shape"
    }
}

struct Circle {
    radius: f64,
}

struct Square {
    side: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        PI * self.radius * self.radius
    }

    fn name(&self) -> &'static str {
        "circle"
    }
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }

    fn name(&self) -> &'static str {
        "square"
    }
}

// TODO: `total_area` and `names` take a slice of any `S: Shape`, and the
// tests pass slices of `Box<dyn Shape>`, `Box<dyn Shape + Send>`,
// `Box<Square>`, `&Circle`, `&dyn Shape` and `&Box<dyn Shape>`: E0277 "the
// trait bound `Box<dyn Shape>: Shape` is not satisfied", and the same for
// the others, although `boxed.area()` compiles by auto-deref.
// Requirement: make every `Box` of a shape and every shared reference to a
// shape a `Shape` itself, every trait object type included, with one
// generic impl per pointer type (not one impl per shape or per trait object
// type). Each method forwards to the shape behind the pointer, the provided
// `name` too: a boxed `Circle` is still called "circle".
// Constraints: don't change `Shape`, `Circle`, `Square`, `total_area`,
// `names` or the tests; no `unsafe`. Read rustc's warnings: a forwarding
// method that calls itself compiles, then overflows the stack. Until you
// write both impls, this exercise will not compile.

// Generic code: one copy per `S`, statically dispatched (see `32_dispatch`).
fn total_area<S: Shape>(shapes: &[S]) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}

fn names<S: Shape>(shapes: &[S]) -> Vec<&'static str> {
    shapes.iter().map(|shape| shape.name()).collect()
}

// ---- Part B — lending a test double ---------------------------------------

trait Mailer {
    fn send(&self, to: &str, subject: &str);
}

// TODO: the tests lend the service a mock that they keep,
// `SignupService::new(&mock)` (and a `&dyn Mailer`), so that they can read
// the mock while and after the service uses it: E0277 "the trait bound
// `&MockMailer: Mailer` is not satisfied" (and `&dyn Mailer: Mailer`), then
// E0599 "the method `register` exists for struct
// `SignupService<&MockMailer>`, but its trait bounds were not satisfied".
// Requirement: one generic impl that makes a shared reference to any mailer,
// `dyn Mailer` included, a `Mailer` that forwards to the mailer behind it.
// Constraints: don't change `Mailer`, `SignupService`, `MockMailer` or the
// tests (the service keeps taking its mailer by value, so an owned mock must
// still work); no `unsafe`. Until you write the impl, this exercise will not
// compile.

const WELCOME: &str = "Welcome aboard!";

// The code under test (finished; don't change it).
struct SignupService<M: Mailer> {
    mailer: M,
    users: Vec<String>,
}

impl<M: Mailer> SignupService<M> {
    fn new(mailer: M) -> Self {
        SignupService {
            mailer,
            users: Vec::new(),
        }
    }

    // Registers `email` and sends it a welcome mail. A repeated address is
    // refused, and nothing is sent for it.
    fn register(&mut self, email: &str) -> bool {
        if self.users.iter().any(|user| user == email) {
            return false;
        }
        self.mailer.send(email, WELCOME);
        self.users.push(email.to_string());
        true
    }

    fn mailer(&self) -> &M {
        &self.mailer
    }
}

// A recording double, finished (writing one is `50_testing_seams/seams1`).
// Deliberately not `Clone`: the test must look at the very mock it lent out.
#[derive(Default)]
struct MockMailer {
    sent: RefCell<Vec<String>>,
}

impl MockMailer {
    fn sent(&self) -> Vec<String> {
        self.sent.borrow().clone()
    }
}

impl Mailer for MockMailer {
    fn send(&self, to: &str, subject: &str) {
        self.sent.borrow_mut().push(format!("{to}: {subject}"));
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected an area of {expected}, got {actual}"
        );
    }

    fn unit_circle() -> Circle {
        Circle { radius: 1.0 }
    }

    fn mixed() -> Vec<Box<dyn Shape>> {
        vec![Box::new(unit_circle()), Box::new(Square { side: 2.0 })]
    }

    // ----- Part A -----

    #[test]
    fn plain_shapes_still_work() {
        let squares = [Square { side: 1.0 }, Square { side: 2.0 }];
        assert_close(total_area(&squares), 5.0);
        assert_eq!(names(&squares), ["square", "square"]);
    }

    #[test]
    fn boxed_trait_objects_are_shapes() {
        let shapes = mixed();
        assert_close(total_area(&shapes), PI + 4.0);
        assert_eq!(names(&shapes), ["circle", "square"]);
    }

    #[test]
    fn trait_objects_with_auto_traits_are_shapes() {
        // `dyn Shape + Send` and `dyn Shape + Sync` are trait object types of
        // their own, not `dyn Shape`: an impl for `Box<dyn Shape>` or
        // `&dyn Shape` alone does not cover them.
        let sendable: Vec<Box<dyn Shape + Send>> =
            vec![Box::new(Square { side: 3.0 }), Box::new(unit_circle())];
        assert_close(total_area(&sendable), 9.0 + PI);
        assert_eq!(names(&sendable), ["square", "circle"]);
        let c = unit_circle();
        let shared: Vec<&(dyn Shape + Sync)> = vec![&c];
        assert_close(total_area(&shared), PI);
        assert_eq!(names(&shared), ["circle"]);
    }

    #[test]
    fn boxed_concrete_shapes_are_shapes() {
        let squares: Vec<Box<Square>> = vec![
            Box::new(Square { side: 1.0 }),
            Box::new(Square { side: 3.0 }),
        ];
        assert_close(total_area(&squares), 10.0);
        assert_eq!(names(&squares), ["square", "square"]);
    }

    #[test]
    fn references_to_shapes_are_shapes() {
        let c = unit_circle();
        assert_close(total_area(&[&c, &c]), 2.0 * PI);
        assert_eq!(names(&[&c]), ["circle"]);
    }

    #[test]
    fn dyn_references_are_shapes() {
        let c = unit_circle();
        let s = Square { side: 2.0 };
        let shapes: Vec<&dyn Shape> = vec![&s, &c, &s];
        assert_close(total_area(&shapes), PI + 8.0);
        assert_eq!(names(&shapes), ["square", "circle", "square"]);
    }

    #[test]
    fn forwarding_impls_nest() {
        let shapes = mixed();
        // `&Box<dyn Shape>`: the `&S` impl around the `Box<S>` impl.
        let borrowed: Vec<&Box<dyn Shape>> = shapes.iter().collect();
        assert_close(total_area(&borrowed), PI + 4.0);
        assert_eq!(names(&borrowed), ["circle", "square"]);
        // `Box<&dyn Shape>`: the other way around.
        let c = unit_circle();
        let boxed_refs: Vec<Box<&dyn Shape>> = vec![Box::new(&c)];
        assert_close(total_area(&boxed_refs), PI);
        assert_eq!(names(&boxed_refs), ["circle"]);
    }

    #[test]
    fn an_empty_slice_has_no_area() {
        let none: Vec<Box<dyn Shape>> = Vec::new();
        assert_close(total_area(&none), 0.0);
        assert!(names(&none).is_empty());
    }

    // ----- Part B -----

    #[test]
    fn the_service_can_borrow_a_mock_that_the_test_keeps() {
        let mock = MockMailer::default();
        let mut service = SignupService::new(&mock);
        assert!(service.register("ann@example.com"));
        assert!(!service.register("ann@example.com"));
        // The test reads its own mock while the service still borrows it.
        assert_eq!(mock.sent(), ["ann@example.com: Welcome aboard!"]);
        assert!(service.register("bob@example.com"));
        drop(service);
        assert_eq!(
            mock.sent(),
            [
                "ann@example.com: Welcome aboard!",
                "bob@example.com: Welcome aboard!"
            ]
        );
    }

    #[test]
    fn a_dyn_mailer_reference_works() {
        let mock = MockMailer::default();
        let mailer: &dyn Mailer = &mock;
        let mut service = SignupService::new(mailer);
        assert!(service.register("cy@example.com"));
        assert_eq!(mock.sent(), ["cy@example.com: Welcome aboard!"]);
    }

    #[test]
    fn two_services_can_share_one_mock() {
        let mock = MockMailer::default();
        let mut shop = SignupService::new(&mock);
        let mut forum = SignupService::new(&mock);
        assert!(shop.register("x@example.com"));
        assert!(forum.register("y@example.com"));
        assert!(forum.register("x@example.com"));
        assert_eq!(
            mock.sent(),
            [
                "x@example.com: Welcome aboard!",
                "y@example.com: Welcome aboard!",
                "x@example.com: Welcome aboard!"
            ]
        );
    }

    #[test]
    fn an_owned_mock_still_works() {
        let mut service = SignupService::new(MockMailer::default());
        assert!(service.register("dee@example.com"));
        assert_eq!(
            service.mailer().sent(),
            ["dee@example.com: Welcome aboard!"]
        );
    }
}
