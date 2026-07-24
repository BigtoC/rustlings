// Module 4 · Send / Sync — part 3: reasoning about the marker traits.
//
// `Send` and `Sync` are auto traits: a type is `Send`/`Sync` automatically if
// all of its fields are. So a struct is only as thread-safe as its least
// thread-safe field. One `!Send` field "poisons" the whole struct.
//
// We can turn that compile-time fact into a checkable assertion. A function
// bounded by `T: Send` can only be instantiated with a `Send` type, so
// `assert_send::<SomeType>()` compiles if and only if `SomeType: Send`. Same
// idea for `Sync`. These functions never run any interesting code — the *type
// check itself* is the test.

use std::rc::Rc;

// Compile-time assertions: instantiating these is only allowed when the type
// argument satisfies the bound.
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

// `Shared` is meant to be handed to other threads, so it must be `Send + Sync`.
// TODO: The `value` field is an `Rc<i32>`, which is `!Send` and `!Sync`, so
// `Shared` is neither — and `assert_send::<Shared>()` / `assert_sync::<Shared>()`
// below fail to compile. Change the field type to the atomic pointer:
//   value: Arc<i32>,
// (and update the `use` at the top to `std::sync::Arc`, plus `Arc::new` below).
struct Shared {
    value: Rc<i32>,
}

impl Shared {
    fn new(value: i32) -> Self {
        Shared {
            value: Rc::new(value),
        }
    }

    fn get(&self) -> i32 {
        *self.value
    }
}

fn main() {
    // If `Shared` were not `Send + Sync`, these two lines would fail to compile.
    assert_send::<Shared>();
    assert_sync::<Shared>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_is_send_and_sync() {
        // The real assertion is that these instantiations type-check at all.
        assert_send::<Shared>();
        assert_sync::<Shared>();

        let shared = Shared::new(7);
        assert_eq!(shared.get(), 7);
    }
}
