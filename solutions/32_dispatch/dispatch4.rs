// Traits & Dispatch - part 4: enum dispatch (a closed set of variants).
//
// When the set of "kinds" is CLOSED and known up front, you often don't need a
// trait object at all. An `enum` holds any one of its variants inline (no heap
// `Box`, no vtable), and a `match` in the method picks the arm — each arm can
// inline like ordinary code. This is the third dispatch strategy:
//
//   - dyn:  open set (anyone can add a type), heap + vtable indirection.
//   - generic: static, monomorphized per concrete type, homogeneous.
//   - enum: CLOSED set you enumerate here, zero indirection, but outside code
//     cannot add a new variant without editing this enum.
//
// The price of enum dispatch is exactly that closedness: every new shape means a
// new arm in the `match`, and third-party crates can't extend the set.

use std::f64::consts::PI;

enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
}

impl Shape {
    // Each `match` arm inlines — no heap `Box`, no vtable hop.
    fn area(&self) -> f64 {
        match self {
            Shape::Circle { radius } => PI * radius * radius,
            Shape::Rectangle { width, height } => width * height,
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_dispatch_sums_areas() {
        // Circle r=1.0 -> PI, Rectangle 2.0 x 3.0 -> 6.0.
        let shapes = [
            Shape::Circle { radius: 1.0 },
            Shape::Rectangle {
                width: 2.0,
                height: 3.0,
            },
        ];
        let total: f64 = shapes.iter().map(|s| s.area()).sum();
        assert!((total - (PI + 6.0)).abs() < 1e-9);
    }
}
