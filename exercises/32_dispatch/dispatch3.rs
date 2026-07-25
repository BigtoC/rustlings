// Traits & Dispatch - part 3: static (monomorphized) vs dynamic dispatch.
//
// The same "call `area` on each element and sum" can be spelled two ways, and
// the difference is entirely in the cost model, not the answer.
//
//   - DYNAMIC: `&[&dyn Shape]` erases the type. There is ONE copy of the code;
//     each `s.area()` is an indirect call through the vtable that cannot inline.
//     The slice can mix `Circle` and `Rectangle` because every element is a fat
//     pointer of the same shape.
//   - STATIC: `<S: Shape>(&[S])` is generic over ONE concrete type. The compiler
//     MONOMORPHIZES it — stamps out a specialized copy per `S` with the real
//     `area` inlined (fast, but larger binary, and the slice is homogeneous).
//
// Same sum, different trade-off: indirection-and-code-sharing vs inlining-and-
// code-duplication.

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

// Given, complete: dynamic dispatch over a heterogeneous slice of trait objects.
fn total_dynamic(shapes: &[&dyn Shape]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

// TODO: Write the STATIC-dispatch twin. It is generic over one concrete `S: Shape`
// and takes a homogeneous slice `&[S]`. Produce the SAME sum as `total_dynamic`:
//     shapes.iter().map(|s| s.area()).sum()
// An empty body returns `()` instead of `f64`, so until you return the sum this
// exercise will not compile.
fn total_static<S: Shape>(shapes: &[S]) -> f64 {}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_dispatch_over_homogeneous_slice() {
        // Three unit circles: 3 * PI.
        let circles = [
            Circle { radius: 1.0 },
            Circle { radius: 1.0 },
            Circle { radius: 1.0 },
        ];
        assert!((total_static(&circles) - 3.0 * PI).abs() < 1e-9);
    }

    #[test]
    fn dynamic_dispatch_over_heterogeneous_slice() {
        // Circle r=1.0 -> PI, Rectangle 2.0 x 3.0 -> 6.0.
        let circle = Circle { radius: 1.0 };
        let rectangle = Rectangle {
            width: 2.0,
            height: 3.0,
        };
        let shapes: [&dyn Shape; 2] = [&circle, &rectangle];
        assert!((total_dynamic(&shapes) - (PI + 6.0)).abs() < 1e-9);
    }
}
