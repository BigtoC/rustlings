// Module 4 · Send / Sync — part 1: `Rc<T>` is `!Send`, `Arc<T>` is `Send`.
//
// `Rc<T>` is a reference-counted pointer whose count is a plain, non-atomic
// integer. That makes cloning cheap, but it also means two threads bumping the
// count at once would race. To stop that at compile time, `Rc<T>` is `!Send`:
// it may not be moved into another thread. `thread::spawn` requires its closure
// (and everything it captures) to be `Send`, so capturing an `Rc` is rejected.
//
// The fix is `Arc<T>` ("atomic Rc"): same API, but the count is an atomic, so
// `Arc<T>` is `Send + Sync` and can be shared across threads freely.

use std::sync::Arc;
use std::thread;

// Spawn 3 threads that each read the shared number, then sum the values they
// return. All threads observe the same `42`, so the total is `3 * 42 = 126`.
fn sum_shared() -> i32 {
    let shared = Arc::new(42);

    let mut handles = Vec::new();
    for _ in 0..3 {
        let clone = Arc::clone(&shared);
        handles.push(thread::spawn(move || *clone));
    }

    handles.into_iter().map(|h| h.join().unwrap()).sum()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_value_crosses_threads() {
        assert_eq!(sum_shared(), 126);
    }
}
