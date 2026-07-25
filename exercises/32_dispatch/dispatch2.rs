// Traits & Dispatch - trait objects, part 2: object safety (dyn compatibility).
//
// Not every trait can become a `dyn Trait`. To build a vtable, the compiler
// needs every method to have a fixed, known calling convention behind a pointer.
// A method that returns `Self` BY VALUE breaks this: different implementors
// return values of different sizes, so there is no single slot the vtable can
// describe. Such a trait is not "dyn-compatible" (older name: not "object
// safe"), and `Box<dyn Widget>` is rejected with E0038.
//
// The fix is not to delete the method — it is to tell the compiler that method
// only exists on concrete, `Sized` types by adding `where Self: Sized`. That
// removes it from the vtable (so `dyn Widget` works again) while keeping it
// callable on a real `Button` or `Slider`.

trait Widget {
    fn width(&self) -> u32;

    // TODO: This method returns `Self` by value, which makes `Widget` not
    // dyn-compatible, so `Box<dyn Widget>` below fails to compile with E0038.
    // Gate it out of the vtable by adding a `where Self: Sized` bound here:
    //     fn duplicate(&self) -> Self where Self: Sized;
    // and add the SAME `where Self: Sized` bound to each `duplicate` impl below.
    // Until you do, this exercise will not compile.
    fn duplicate(&self) -> Self;
}

struct Button {
    w: u32,
}

impl Widget for Button {
    fn width(&self) -> u32 {
        self.w
    }

    fn duplicate(&self) -> Self {
        Button { w: self.w }
    }
}

struct Slider {
    w: u32,
}

impl Widget for Slider {
    fn width(&self) -> u32 {
        self.w
    }

    fn duplicate(&self) -> Self {
        Slider { w: self.w }
    }
}

// This must work once `Widget` is dyn-compatible: `width` stays in the vtable.
fn total_width(ws: &[Box<dyn Widget>]) -> u32 {
    ws.iter().map(|w| w.width()).sum()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyn_widget_sums_widths() {
        let widgets: Vec<Box<dyn Widget>> =
            vec![Box::new(Button { w: 10 }), Box::new(Slider { w: 5 })];
        assert_eq!(total_width(&widgets), 15);
    }

    #[test]
    fn duplicate_still_works_on_concrete_types() {
        let b = Button { w: 42 };
        assert_eq!(b.duplicate().width(), 42);
    }
}
