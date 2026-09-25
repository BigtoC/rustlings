// Module 1 · Lifetimes (deep) — part 8: `Box<dyn Trait>` means `Box<dyn Trait + 'static>`.
//
// A trait object hides the concrete type, but not the question "how long may
// the hidden value borrow things?". So every trait object type carries a
// lifetime bound, `dyn Trait + 'x`, and when you don't write one, rustc fills
// in a DEFAULT that depends on the type the object sits in:
//
//     Box<dyn Trait>       means  Box<dyn Trait + 'static>
//     Rc<dyn Trait>, Arc<dyn Trait>, Vec<Box<dyn Trait>>: also `+ 'static`
//     &'a dyn Trait        means  &'a (dyn Trait + 'a)
//     &'a mut dyn Trait    means  &'a mut (dyn Trait + 'a)
//
// That default applies wherever you write a type in a signature, a struct
// field or a type alias, even inside a struct that has lifetime parameters of
// its own. (Only a type written INSIDE a function body, such as a `let`
// annotation, gets an inferred lifetime instead.) So `Box<dyn Fn(&str)>`
// quietly means "a boxed closure that borrows nothing short-lived", and the
// `Handler` alias below hides even that. An `impl Fn(&str)` PARAMETER, on the
// other hand, has no hidden bound at all: it is an anonymous generic type
// that may borrow anything. Boxing such a value into a `Box<dyn Fn(&str)>` is
// E0310 "the parameter type `impl Fn(&str)` may not live long enough".
//
// rustc's `help:` then suggests adding `+ 'static` to the parameter. That
// silences E0310 and moves the failure to every caller: a closure that
// borrows a local now fails with E0373 "closure may outlive the current
// function, but it borrows `log`, which is owned by the current function",
// because a `'static` closure may not borrow anything local. (Its own `help:`
// then says "use the `move` keyword", which would move the log into the
// closure, so the test could no longer read it.) `+ 'static` is the right
// bound for a callback handed to a thread or stored in a global. For an event
// bus that lives inside one function and reports into that function's
// locals, it is the wrong design: the bus should say "my handlers borrow data
// that lives at least `'a`", the lifetime-parameter-on-a-struct idea from
// part 4, applied to the object type.
//
// Declaration order matters once a struct borrows through a trait object. As
// far as the compiler knows, the destructor of a `dyn Fn` may use whatever
// the closure borrowed, so the borrowed data must still be alive when the bus
// is dropped: declare it BEFORE the bus (locals are dropped in reverse
// order). Swap the two `let`s in a test and you get E0597 "`log` does not live
// long enough ... borrow might be used here, when `bus` is dropped and runs
// the destructor for type `EventBus<'_>`".
//
// How interviewers probe this: "What is the lifetime of the `dyn Error` in
// `Box<dyn Error>`?", "Why does pushing a closure into a `Vec<Box<dyn Fn()>>`
// demand `'static`?", "Is rustc's `+ 'static` suggestion right here?". You
// will meet the default again in `35_error_design/err2`, where `source()`
// returns `Option<&(dyn Error + 'static)>`.

// A handler is called with the name of every event emitted after it was
// registered. Handlers are different closure types, so they are boxed as
// trait objects to live in one `Vec`.
type Handler = Box<dyn Fn(&str)>;

struct EventBus {
    handlers: Vec<Handler>,
}

impl EventBus {
    fn new() -> Self {
        EventBus {
            handlers: Vec::new(),
        }
    }

    // TODO: This is rejected with E0310 "the parameter type `impl Fn(&str)`
    // may not live long enough": `Handler` is a `Box<dyn Fn(&str)>`, which
    // means `+ 'static`, so only closures that borrow nothing local may go
    // into the bus. The tests register closures that borrow a `RefCell` log
    // and a `Cell` counter owned by the test function. Following rustc's
    // `+ 'static` help moves the failure into the tests (E0373 "closure may
    // outlive the current function, but it borrows `log`"), and the tests are
    // right. Change the bus so that its handlers may borrow data that
    // outlives the bus: the alias, the struct, the `impl` block and `on` all
    // have to agree on how long that data lives.
    // Constraints: keep the handlers as boxed trait objects in one `Vec`; no
    // `'static` bound, no `Rc`, no `unsafe`; don't change the tests.
    // Until the bus can hold handlers that borrow the caller's locals, this
    // exercise will not compile.
    fn on(&mut self, f: impl Fn(&str)) {
        self.handlers.push(Box::new(f));
    }

    fn emit(&self, event: &str) {
        for handler in &self.handlers {
            handler(event);
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[test]
    fn a_handler_can_record_into_a_borrowed_log() {
        // `log` is declared BEFORE `bus`, so it is dropped after it.
        let log = RefCell::new(Vec::new());
        let mut bus = EventBus::new();
        // No handlers yet: nobody hears this one.
        bus.emit("ignored");
        bus.on(|event| log.borrow_mut().push(event.to_string()));
        bus.emit("a");
        bus.emit("b");
        // Once the bus is gone, nothing borrows `log` any more, so it may be
        // moved out of. The borrow lasted exactly as long as the bus.
        drop(bus);
        assert_eq!(log.into_inner(), ["a", "b"]);
    }

    #[test]
    fn a_counting_handler_sees_every_event() {
        let count = Cell::new(0);
        let mut bus = EventBus::new();
        bus.on(|_| count.set(count.get() + 1));
        bus.emit("a");
        bus.emit("b");
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn handlers_run_in_registration_order() {
        let log = RefCell::new(Vec::new());
        let mut bus = EventBus::new();
        bus.on(|event| log.borrow_mut().push(format!("first {event}")));
        bus.on(|event| log.borrow_mut().push(format!("second {event}")));
        bus.emit("a");
        bus.emit("b");
        assert_eq!(
            *log.borrow(),
            ["first a", "second a", "first b", "second b"]
        );
    }

    #[test]
    fn borrowing_and_owning_handlers_share_one_bus() {
        let seen = Cell::new(0);
        let hits = Rc::new(Cell::new(0));
        let mut bus = EventBus::new();
        // This one borrows `seen` from the test...
        bus.on(|_| seen.set(seen.get() + 1));
        // ...and this one owns its state: a `'static` closure is still
        // welcome, it simply outlives any `'a` the bus asks for.
        let counter = Rc::clone(&hits);
        bus.on(move |event| {
            if event == "hit" {
                counter.set(counter.get() + 1);
            }
        });
        bus.emit("hit");
        bus.emit("miss");
        bus.emit("hit");
        assert_eq!((seen.get(), hits.get()), (3, 2));
        // The bus owns its handlers: dropping it drops the moved-in `Rc`.
        assert_eq!(Rc::strong_count(&hits), 2);
        drop(bus);
        assert_eq!(Rc::strong_count(&hits), 1);
    }
}
