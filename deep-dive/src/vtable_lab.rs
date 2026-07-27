//! Lab · building a `&dyn Trait` fat pointer by hand
//!
//! A trait object reference like `&dyn Shape` is NOT a plain pointer. It is a
//! **fat pointer** — a pair of two machine words:
//!
//! - a `*const ()` **data pointer** to the concrete value (the `Circle` or
//!   `Square` itself), plus
//! - a `&'static` pointer to a **vtable**: a per-type, static table of function
//!   pointers (plus size / alignment / `drop_in_place`) that the compiler emits
//!   once for each `impl Trait for Type`.
//!
//! When you call `shape.area()` on a `&dyn Shape`, the compiler does exactly
//! what `DynShape::area` does below: load the function pointer out of the
//! vtable, then call it with the data pointer as the receiver. There is no magic
//! — dynamic dispatch is one pointer load followed by one indirect call.
//!
//! We reconstruct this pair by hand. The compiler's real vtable also stores the
//! value's size, alignment, and a `drop_in_place` slot (so `Box<dyn Shape>` can
//! be dropped); we keep just `area` and `name` to show the shape of the thing
//! without the bookkeeping. The one honest bit we must reproduce is lifetimes:
//! `PhantomData<&'a ()>` re-attaches the borrow that the `*const ()` cast threw
//! away, so a `DynShape<'a>` still cannot outlive the value it was erased from.

use std::f64::consts::PI;
use std::marker::PhantomData;

/// One of the two concrete "shape" types. `Circle` and `Square` share no trait and
/// no common field — the only thing tying them together at runtime is the vtable we
/// hand each of them.
pub struct Circle {
    radius: f64,
}

impl Circle {
    pub fn new(radius: f64) -> Self {
        Circle { radius }
    }
}

/// The other concrete "shape" type; see [`Circle`].
pub struct Square {
    side: f64,
}

impl Square {
    pub fn new(side: f64) -> Self {
        Square { side }
    }
}

/// One vtable = one row of function pointers per concrete type, plus a bit of
/// static metadata. The real compiler-emitted vtable also carries size,
/// alignment, and `drop_in_place`; `name` here stands in for that extra data.
///
/// Note the erased receiver type: `unsafe fn(*const ()) -> f64`. Every entry
/// takes a type-erased data pointer, exactly like the real thing — that is what
/// lets one `DynShape` type hold either a `Circle` or a `Square`.
struct ShapeVtable {
    area: unsafe fn(*const ()) -> f64,
    name: &'static str,
}

/// `area` implementation for `Circle`. Takes an erased pointer and un-erases it.
unsafe fn circle_area(p: *const ()) -> f64 {
    // SAFETY: `p` was produced in `erase_circle` from a live `&Circle`, so it
    // points at a valid, aligned, initialized `Circle`. The `PhantomData<&'a ()>`
    // in `DynShape` keeps that borrow alive, so the `Circle` outlives this call.
    let c = unsafe { &*(p as *const Circle) };
    PI * c.radius * c.radius
}

/// `area` implementation for `Square`.
unsafe fn square_area(p: *const ()) -> f64 {
    // SAFETY: `p` was produced in `erase_square` from a live `&Square` that
    // outlives this call, so casting back to `&Square` is valid.
    let s = unsafe { &*(p as *const Square) };
    s.side * s.side
}

/// The single static vtable for `Circle`. There is exactly one of these per
/// type in the whole program — every erased `Circle` points at this same table,
/// just as every `&dyn Shape` over `Circle` shares one compiler-emitted vtable.
static CIRCLE_VTABLE: ShapeVtable = ShapeVtable {
    area: circle_area,
    name: "Circle",
};

/// The single static vtable for `Square`.
static SQUARE_VTABLE: ShapeVtable = ShapeVtable {
    area: square_area,
    name: "Square",
};

/// Our hand-built stand-in for `&'a dyn Shape`: a data pointer plus a vtable
/// pointer. Compare its two machine-word layout to a real `&dyn` fat pointer —
/// they are the same idea.
///
/// The `_marker: PhantomData<&'a ()>` makes `DynShape<'a>` behave, for the
/// borrow checker, as if it held a `&'a` reference to the erased value. Without
/// it, the raw `*const ()` would be lifetime-free and could dangle; with it, the
/// compiler refuses to let a `DynShape<'a>` outlive its source value.
pub struct DynShape<'a> {
    data: *const (),
    vtable: &'static ShapeVtable,
    _marker: PhantomData<&'a ()>,
}

impl DynShape<'_> {
    /// Dynamic dispatch, by hand: load the function pointer out of the vtable,
    /// then call it with the data pointer. This is precisely what `shape.area()`
    /// on a real `&dyn Shape` compiles down to.
    pub fn area(&self) -> f64 {
        // SAFETY: `self.data` and `self.vtable` were paired in `erase_circle` /
        // `erase_square` so the function pointer matches the concrete type the
        // data pointer refers to. The `'a` borrow (via `PhantomData`) guarantees
        // that value is still alive.
        unsafe { (self.vtable.area)(self.data) }
    }

    /// Read a plain data field out of the vtable — no dispatch, just a load.
    pub fn name(&self) -> &'static str {
        self.vtable.name
    }
}

/// Erase a `&Circle` into a `DynShape`: forget the static type, remember the
/// address and the matching vtable. The lifetime `'a` flows from the input
/// borrow into `DynShape<'a>`, so the result cannot outlive the `Circle`.
pub fn erase_circle(c: &Circle) -> DynShape<'_> {
    DynShape {
        data: c as *const Circle as *const (),
        vtable: &CIRCLE_VTABLE,
        _marker: PhantomData,
    }
}

/// Erase a `&Square` into a `DynShape`, pairing it with the `Square` vtable.
pub fn erase_square(s: &Square) -> DynShape<'_> {
    DynShape {
        data: s as *const Square as *const (),
        vtable: &SQUARE_VTABLE,
        _marker: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_goes_through_the_vtable() {
        let circle = Circle::new(2.0);
        let square = Square::new(3.0);

        // A heterogeneous collection: two different concrete types behind one
        // erased `DynShape` type — only possible because each carries its own
        // vtable pointer.
        let shapes: Vec<DynShape<'_>> = vec![erase_circle(&circle), erase_square(&square)];

        // Names come from the vtable, proving each element is paired with the
        // correct table.
        assert_eq!(shapes[0].name(), "Circle");
        assert_eq!(shapes[1].name(), "Square");

        // Areas are computed by the function pointer pulled from each vtable.
        assert!((shapes[0].area() - PI * 4.0).abs() < 1e-9);
        assert!((shapes[1].area() - 9.0).abs() < 1e-9);
    }

    #[test]
    fn same_type_shares_one_static_vtable() {
        let a = Circle::new(1.0);
        let b = Circle::new(5.0);
        let da = erase_circle(&a);
        let db = erase_circle(&b);

        // Distinct values, distinct data pointers...
        assert!(!std::ptr::eq(da.data, db.data));
        // ...but the *same* vtable: one table per type, shared by every value.
        assert!(std::ptr::eq(da.vtable, db.vtable));
    }

    #[test]
    fn dyn_shape_is_a_two_word_fat_pointer() {
        // The whole point: a hand-built trait object is exactly two pointers
        // wide, matching the size of a real `&dyn Shape` fat pointer.
        assert_eq!(
            std::mem::size_of::<DynShape<'_>>(),
            2 * std::mem::size_of::<usize>()
        );
    }
}
