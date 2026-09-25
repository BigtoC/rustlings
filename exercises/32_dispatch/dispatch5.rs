// Traits & Dispatch - trait objects, part 5: downcasting with `Any` and trait upcasting.
//
// Erasing a type is easy (`dispatch1`). Getting it back is the follow-up
// question. Once a `Circle` sits inside a `Box<dyn Shape>`, the vtable only
// knows `Shape`'s methods. To recover a `&Circle` ("downcasting") you need a
// run-time type tag, and in std that tag is `std::any::TypeId`. The trait `Any`
// has one method, `type_id(&self) -> TypeId`, and a blanket impl for EVERY
// `'static` type. On `dyn Any`, std builds `is::<T>()`, `downcast_ref::<T>()`
// and `downcast_mut::<T>()` on top of it: compare the erased value's `TypeId`
// with `TypeId::of::<T>()`, and only on a match hand the data pointer back as a
// `&T`.
//
// Why `'static`? A `TypeId` is computed with lifetimes erased, so `&'a str`
// would get the same one as `&'static str`. If `Any` accepted borrowed types,
// a downcast could turn a short-lived `&'a str` into a `&'static str`. So `Any`
// requires `'static`, and making `Any` a supertrait of `Shape` means every
// shape must be `'static` too (`impl<'a> Shape for Label<'a>` would be E0478
// "lifetime bound not satisfied").
//
// How do you get from a `&dyn Shape` to a `&dyn Any`? Since Rust 1.86 a trait
// object coerces to any of its SUPERTRAITS ("trait upcasting"). rustc lays out
// the vtable of `dyn Shape` so that the vtable of each supertrait can be found
// from it, and the coercion keeps the data pointer and, where needed, swaps in
// the supertrait's vtable pointer. `as_debug` below already does this with
// `Debug`. If `Any` is not a supertrait, `dyn Shape` to `dyn Any` is not an
// upcast at all, and rustc rejects it with E0308. (Before 1.86 the usual
// workaround was an `as_any` method on the trait; the README shows that
// pattern and its own version of the trap.)
//
// The trap. `Any` is implemented for every `'static` type, and that includes
// the smart pointers that hold your trait objects: a `Box<dyn Shape>` is an
// `Any` in its own right, with its own `TypeId`. A coercion to `&dyn Any`
// erases exactly the type of the value you hand it, and the compiler is happy
// to erase the wrong one. Nothing fails to compile and nothing panics: every
// downcast just returns `None`. Interviewers ask "how do you get a `&Circle`
// back out of a `Box<dyn Shape>`?" and then "why does your downcast return
// `None`?". Two wrong answers they hear a lot: an `unsafe` pointer cast (no
// type check at all, so a `Rectangle` would be read as a `Circle`), and a
// `fn as_circle(&self)` hook on the trait (a closed list that every
// implementor, including other crates, has to extend).

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::Debug;

// TODO: A trait object only upcasts to its supertraits, and `Debug` is the
// only one `Shape` has. So once `all_of` erases the right value, the coercion
// to `&dyn Any` is rejected with E0308 mismatched types, "expected trait
// `Any`, found trait `Shape`". (If rustc's help says "try removing the method
// call", don't: that leads straight back to the bug.) Requirements:
//   - `&dyn Shape` must coerce to `&dyn Any`, and still to `&dyn Debug`
//     (`as_debug` stays as it is);
//   - don't add methods to `Shape`: the tests implement it for a type of
//     their own, and a new required method would break that impl, just as it
//     would break every downstream implementor.
// Until `Shape` can be upcast to `dyn Any` and `all_of` uses that, the tests
// will fail.
trait Shape: Debug {
    fn area(&self) -> f64;
}

#[derive(Debug)]
struct Circle {
    radius: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        PI * self.radius * self.radius
    }
}

#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

// Given, complete: trait upcasting in action. `Debug` is a supertrait of
// `Shape`, so a `&dyn Shape` coerces to a `&dyn Debug` that points at the same
// value and uses the `Debug` part of its vtable.
fn as_debug(shape: &dyn Shape) -> &dyn Debug {
    shape
}

// Returns every shape whose concrete type is `T`, in their original order, as
// references into the boxes.
fn all_of<T: Any>(shapes: &[Box<dyn Shape>]) -> Vec<&T> {
    // TODO: This compiles and runs, but it never finds anything. Five tests
    // fail: `finds_every_circle_in_order` gets `left: []` instead of the radii
    // `[1.0, 3.0]`, three more find 0 shapes where they expect 1 or 2, and
    // `never_returns_the_box_itself` fails because `all_of::<Box<dyn Shape>>`
    // returns all 3 elements. Work out whose `TypeId` `any` really carries:
    // what is the type of `s`? Requirements:
    //   - keep the signature; return references INTO the boxes (the tests
    //     compare addresses), in the original order;
    //   - no `unsafe` (the crate forbids it), no `TypeId` comparison followed
    //     by a cast, and no per-type hook such as `fn as_circle`;
    //   - it must work for shape types this module has never seen (the tests
    //     define one).
    // Until `any` erases the shape rather than the pointer that owns it, the
    // tests will fail.
    shapes
        .iter()
        .filter_map(|s| {
            let any: &dyn Any = s;
            any.downcast_ref::<T>()
        })
        .collect()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Circle, Rectangle, Circle: two circles, at indices 0 and 2.
    fn mixed() -> Vec<Box<dyn Shape>> {
        vec![
            Box::new(Circle { radius: 1.0 }),
            Box::new(Rectangle {
                width: 2.0,
                height: 3.0,
            }),
            Box::new(Circle { radius: 3.0 }),
        ]
    }

    // A shape type that `all_of` has never heard of. It only has to implement
    // `Shape` as the trait is declared.
    #[derive(Debug)]
    struct Hexagon {
        side: f64,
    }

    impl Shape for Hexagon {
        fn area(&self) -> f64 {
            1.5 * 3.0_f64.sqrt() * self.side * self.side
        }
    }

    #[test]
    fn finds_every_circle_in_order() {
        let shapes = mixed();
        let radii: Vec<f64> = all_of::<Circle>(&shapes).iter().map(|c| c.radius).collect();
        assert_eq!(radii, [1.0, 3.0]);
    }

    #[test]
    fn returns_references_into_the_boxes() {
        let shapes = mixed();
        let found = all_of::<Circle>(&shapes);
        assert_eq!(found.len(), 2);
        // A downcast copies nothing: the `&Circle` has the same address as the
        // `dyn Shape` it came from.
        assert!(std::ptr::addr_eq(found[0], &*shapes[0]));
        assert!(std::ptr::addr_eq(found[1], &*shapes[2]));
    }

    #[test]
    fn tells_each_concrete_type_apart() {
        let shapes = mixed();
        let rectangles = all_of::<Rectangle>(&shapes);
        assert_eq!(rectangles.len(), 1);
        assert_eq!(rectangles[0].area(), 6.0);
        assert!(all_of::<String>(&shapes).is_empty());
        assert!(all_of::<Circle>(&[]).is_empty());
    }

    #[test]
    fn works_for_shapes_defined_elsewhere() {
        let mut shapes = mixed();
        shapes.push(Box::new(Hexagon { side: 2.0 }));
        let hexagons = all_of::<Hexagon>(&shapes);
        assert_eq!(hexagons.len(), 1);
        assert_eq!(hexagons[0].side, 2.0);
        assert_eq!(all_of::<Circle>(&shapes).len(), 2);
    }

    #[test]
    fn never_returns_the_box_itself() {
        // The elements are `Box<dyn Shape>`, but the values INSIDE them are a
        // `Circle`, a `Rectangle` and a `Circle`. None of those is a box.
        let shapes = mixed();
        assert!(all_of::<Box<dyn Shape>>(&shapes).is_empty());
    }

    #[test]
    fn upcasts_to_debug() {
        let shapes = mixed();
        assert_eq!(
            format!("{:?}", as_debug(&*shapes[0])),
            "Circle { radius: 1.0 }"
        );
    }
}
