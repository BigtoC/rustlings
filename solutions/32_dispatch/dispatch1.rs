// Traits & Dispatch - trait objects, part 1: a heterogeneous collection.
//
// A `Vec<T>` stores many values of ONE concrete type `T`, laid out end to end.
// So how do you keep a `Circle` and a `Rectangle` — two different types — in the
// same list? You erase the concrete type behind a trait object: `Box<dyn Shape>`.
// `dyn Shape` is a "fat pointer" — a data pointer plus a pointer to a vtable of
// the trait's methods — so every element has the same size and the same shape,
// while the real type it points to can differ from slot to slot. Each method
// call then takes an indirect hop through the vtable.

use std::f64::consts::PI;

trait Shape {
    fn area(&self) -> f64;
}

struct Circle {
    radius: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        PI * self.radius * self.radius
    }
}

struct Rectangle {
    width: f64,
    height: f64,
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

// Given, complete: sum the area of every shape without knowing its real type.
fn total_area(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

// Each `Box::new(..)` coerces to `Box<dyn Shape>`, erasing the concrete type so
// two different types share one `Vec`.
fn mixed_shapes() -> Vec<Box<dyn Shape>> {
    vec![
        Box::new(Circle { radius: 1.0 }),
        Box::new(Rectangle {
            width: 2.0,
            height: 3.0,
        }),
    ]
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_heterogeneous_shapes() {
        // Circle r=1.0 -> PI, Rectangle 2.0 x 3.0 -> 6.0.
        assert!((total_area(&mixed_shapes()) - (PI + 6.0)).abs() < 1e-9);
    }
}
