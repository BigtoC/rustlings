// Module 1 · Drop and RAII — part 3: a `defer` guard that owns an `FnOnce` closure (E0507).
//
// Go and Swift have a `defer` statement, C++ has scope guards. Rust needs no
// keyword for it: a value whose `Drop` runs a closure IS a defer statement.
// The closure runs when the guard goes out of scope, and a scope ends on every
// path out of it: falling off the end of the block, an early `return`, a `?`
// that propagates an error, and a panic that unwinds through the frame. Parts
// 1 and 2 said exactly when that happens; this part puts it to work. It is
// also how `MutexGuard` unlocks, and crates such as `scopeguard` package this
// very pattern.
//
// The closure's bound is `F: FnOnce()`, the most permissive one: a cleanup
// step may CONSUME what it captured (hand a value on, push an owned record
// into a log), so it can only be called once. Calling an `FnOnce` therefore
// takes the closure BY VALUE. But `Drop::drop` only gets `&mut self` — the
// value is still alive, and its fields are dropped right after `drop`
// returns — so `(self.f)()` would move `self.f` out from behind a `&mut`. That
// is E0507, the error of `24_ownership_model/ownership4`, and rustc adds a
// note: "this value implements `FnOnce`, which causes it to be moved when
// called". Requiring `FnMut` or `Fn` instead would make that line compile,
// but the guard would then reject every cleanup closure that consumes its
// captures (a test uses one). Calling a clone of the closure needs
// `F: Clone`, and a closure that owns a non-`Clone` value is not `Clone`
// (that test's closure owns one).
//
// `cancel` is what makes it a real guard and not just a destructor: a success
// path disarms the cleanup ("delete the half-written file unless every step
// succeeded"). It takes the guard BY VALUE, so the guard is gone afterwards.
// It must not run the closure, and it must not LEAK it either: the closure
// owns its captures, and they have to be dropped as usual.
// `mem::forget(self)` skips `drop` and leaks everything the guard owns (and
// this course denies `clippy::mem_forget`).
//
// Is a deferred closure guaranteed to run? No, and interviewers like that
// question. A destructor does not run when its value is leaked (`mem::forget`
// and `Box::leak` are SAFE functions, and part 6 leaks a whole tree through an
// `Rc` cycle), when the process ends without unwinding (`std::process::exit`,
// `panic = "abort"`, a panic inside a `drop` that runs during unwinding), or
// when `main` returns while another thread still owns the value: that ends
// the process without unwinding the other thread. So safe code may use RAII
// for cleanup, but unsafe code must never depend on a destructor running to
// stay sound. That lesson cost Rust its original `thread::scoped` API just
// before 1.0 (the "Leakpocalypse"); today's `thread::scope` joins the threads
// before it returns, whether or not any guard is dropped.
//
// How interviewers probe it: "Implement `defer`, with a way to cancel it",
// "Why can't `drop` just call the closure?", "Does it run on `?`? On a panic?",
// "Is it guaranteed to run?", and "What does `let _ = Defer::new(..);` do?"
// (see `drop1`: the guard is dropped on that very line).

// Runs its closure when it is dropped, unless it was canceled first.
// `#[must_use]` makes a bare `Defer::new(..);` statement a warning, because
// the unnamed guard would be dropped, and its closure run, on that line.
#[must_use = "the closure runs as soon as the guard is dropped; bind it to a named variable"]
struct Defer<F: FnOnce()> {
    // `Some` while the guard is armed. The `Option` gives `drop` (which only
    // has `&mut self`) a valid value to leave behind when it takes the closure
    // out: `None`. Unlike `mem::take`, `Option::take` needs no `F: Default`.
    f: Option<F>,
}

impl<F: FnOnce()> Defer<F> {
    fn new(f: F) -> Self {
        // A new guard is armed.
        Defer { f: Some(f) }
    }

    // Disarms the guard: the closure will never run.
    fn cancel(mut self) {
        // `drop` still runs on this guard when `cancel` returns; a value with
        // a `Drop` impl is always dropped unless it is leaked. So `cancel`
        // cannot skip `drop`, it can only disarm it: assigning `None` drops the
        // closure (and its captures) right here, and `drop` then finds nothing
        // to call. `mut self` only makes the local binding mutable; callers
        // still just write `guard.cancel()`.
        self.f = None;
    }
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        // `take()` moves the closure out and leaves `None` in the field, so
        // the value stays valid behind `&mut self`, and the closure, now owned
        // by this local `f`, can be called by value. `None` means the guard
        // was canceled, and then there is nothing to call.
        if let Some(f) = self.f.take() {
            f();
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
    use std::num::ParseIntError;
    use std::panic::{self, AssertUnwindSafe};
    use std::rc::Rc;

    #[test]
    fn runs_when_the_guard_goes_out_of_scope() {
        let log = RefCell::new(Vec::new());
        {
            let _guard = Defer::new(|| log.borrow_mut().push("cleanup"));
            log.borrow_mut().push("work");
        }
        log.borrow_mut().push("after the scope");
        assert_eq!(*log.borrow(), ["work", "cleanup", "after the scope"]);
    }

    #[test]
    fn runs_exactly_once() {
        let mut calls = 0;
        {
            let _guard = Defer::new(|| calls += 1);
        }
        assert_eq!(calls, 1);
    }

    // Uses a guard on every path out of the function.
    fn parse_with_cleanup(
        input: &str,
        log: &RefCell<Vec<&'static str>>,
    ) -> Result<i32, ParseIntError> {
        let _guard = Defer::new(|| log.borrow_mut().push("cleanup"));
        if input.is_empty() {
            return Ok(0);
        }
        let n: i32 = input.trim().parse()?;
        log.borrow_mut().push("parsed");
        Ok(n)
    }

    #[test]
    fn runs_on_every_way_out_of_a_function() {
        let log = RefCell::new(Vec::new());
        assert_eq!(parse_with_cleanup("42", &log), Ok(42));
        assert_eq!(*log.borrow(), ["parsed", "cleanup"]);

        // An early `return`.
        log.borrow_mut().clear();
        assert_eq!(parse_with_cleanup("", &log), Ok(0));
        assert_eq!(*log.borrow(), ["cleanup"]);

        // A `?` that propagates the error.
        log.borrow_mut().clear();
        assert!(parse_with_cleanup("forty-two", &log).is_err());
        assert_eq!(*log.borrow(), ["cleanup"]);
    }

    #[test]
    fn runs_while_a_panic_unwinds() {
        // Test binaries unwind even though the course's profiles set
        // `panic = "abort"`: Cargo ignores that setting for test targets.
        let log = RefCell::new(Vec::new());
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = Defer::new(|| log.borrow_mut().push("cleanup"));
            log.borrow_mut().push("work");
            panic!("simulated failure");
        }));
        assert!(result.is_err());
        assert_eq!(*log.borrow(), ["work", "cleanup"]);
    }

    #[test]
    fn two_guards_run_in_reverse_order() {
        let log = RefCell::new(Vec::new());
        {
            let _close_file = Defer::new(|| log.borrow_mut().push("close file"));
            let _unlock = Defer::new(|| log.borrow_mut().push("unlock"));
        }
        // Last in, first out, like any two locals (`drop1`).
        assert_eq!(*log.borrow(), ["unlock", "close file"]);
    }

    // Deliberately NOT `Clone`: a closure that consumes one is `FnOnce` and
    // nothing more, and it is not `Clone` either.
    #[derive(Debug, PartialEq)]
    struct Receipt(&'static str);

    #[test]
    fn the_closure_may_consume_what_it_captured() {
        let filed = RefCell::new(Vec::new());
        let receipt = Receipt("refund #7");
        {
            // `push(receipt)` moves `receipt` out of the closure, so this
            // closure can only be called once.
            let _guard = Defer::new(|| filed.borrow_mut().push(receipt));
            assert!(filed.borrow().is_empty());
        }
        assert_eq!(*filed.borrow(), [Receipt("refund #7")]);
    }

    #[test]
    fn cancel_disarms_the_guard() {
        let ran = Cell::new(false);
        let guard = Defer::new(|| ran.set(true));
        guard.cancel();
        assert!(!ran.get(), "a canceled guard must not run its closure");
    }

    #[test]
    fn cancel_drops_the_closure_instead_of_leaking_it() {
        let ran = Cell::new(false);
        let token = Rc::new(());
        let captured = Rc::clone(&token);
        let guard = Defer::new(|| {
            drop(captured);
            ran.set(true);
        });
        // The closure owns `captured`, so there are two handles now.
        assert_eq!(Rc::strong_count(&token), 2);
        guard.cancel();
        assert!(!ran.get(), "a canceled guard must not run its closure");
        // Canceling dropped the closure, and `captured` with it, right away.
        // Forgetting the guard would leak it and leave the count at 2.
        assert_eq!(Rc::strong_count(&token), 1);
    }

    #[test]
    fn let_underscore_runs_it_immediately() {
        // Not a bug in `Defer`, but in the caller: `_` binds nothing, so the
        // new guard is a temporary, dropped at the end of this statement.
        let log = RefCell::new(Vec::new());
        let _ = Defer::new(|| log.borrow_mut().push("cleanup"));
        log.borrow_mut().push("work");
        assert_eq!(*log.borrow(), ["cleanup", "work"]);
    }
}
