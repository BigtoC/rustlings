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

use std::sync::Arc;

// Compile-time assertions: instantiating these is only allowed when the type
// argument satisfies the bound.
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

// `Shared` is meant to be handed to other threads, so it must be `Send + Sync`.
// Because `Arc<i32>` is `Send + Sync`, so is `Shared`.
struct Shared {
    value: Arc<i32>,
}

impl Shared {
    fn new(value: i32) -> Self {
        Shared {
            value: Arc::new(value),
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
