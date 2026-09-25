// Module 1 · Lifetimes (deep) — part 7: `T: 'static` is not `&'static T`.
//
// Two different things are spelled with `'static`, and interviewers love to
// check that you can tell them apart:
//
//   - `&'static T` is a REFERENCE that stays valid until the program exits,
//     so the data behind it must never be freed: a string literal (it is
//     baked into the binary), a `static` item, or memory leaked on purpose
//     (part 9).
//   - `T: 'static` is a BOUND on a type. It says "`T` contains no borrow that
//     could expire": the type owns all of its data, or borrows only `'static`
//     data. `String`, `Vec<u8>`, `Arc<str>`, `u32` and `&'static str` are all
//     `'static`. A `&'a str` with a shorter `'a` is not, and neither is a
//     struct holding one (like `Excerpt<'a>` from part 4).
//
// The classic trap is "`T: 'static` means the value lives for the whole
// program". It does not. A `String` meets the bound, and it is freed the
// moment its owner drops it. The bound only promises that WHOEVER owns the
// value may keep it for as long as they like, because nothing inside it can
// dangle. That is exactly what `thread::spawn` asks for:
//
//     pub fn spawn<F, T>(f: F) -> JoinHandle<T>
//     where
//         F: FnOnce() -> T,
//         F: Send + 'static,
//         T: Send + 'static,
//
// A spawned thread may outlive the function that started it (drop the
// `JoinHandle` and the thread keeps running), so the compiler cannot prove
// that a borrow of the caller's local would still be valid inside it. Moving
// an owned `String` in is fine. Passing `&local` is E0597 "`local` does not
// live long enough ... argument requires that `local` is borrowed for
// `'static`". (When the threads only need to borrow locals and are joined
// before the function returns, `std::thread::scope`, stable since Rust 1.63,
// lifts the `'static` requirement.)
//
// So a parameter typed `&'static str` is almost always too strict. It takes
// string literals and leaked strings, but not a message built at run time
// with `format!`, and not a shared `Arc<str>`. The flexible spelling is a
// generic parameter with a `'static` BOUND. Async runtimes ask for
// `Send + 'static` on the futures they spawn for the same reason (you will
// see it again in `29_async_runtime/runtime3`).
//
// How interviewers probe this: "Does `T: 'static` mean the value lives
// forever?", "Why does `thread::spawn` accept a `String` but reject a `&str`
// borrowed from a local?", "Is `&'static str` the only way to hand text to a
// thread?".

use std::thread::{self, JoinHandle};

// A background logger. The caller hands over a message and carries on, while
// a separate thread formats the line (formatting can be slow, so it stays off
// the caller's hot path) and hands it back through the `JoinHandle`. A real
// logger would write the line to a file instead.
//
// TODO: Every call in the tests except the one with a literal is rejected
// with E0308 "mismatched types": expected `&str`, found `String` (then
// `Arc<str>`, then the tests' own `Counted` and `Traced` types). Make
// `spawn_logger` accept a message of ANY type that can be displayed, as long
// as the logger thread can take ownership of it, and give that type exactly
// the bounds `thread::spawn` demands, no more (rustc names the missing ones
// one at a time, and a test passes a message that is `Send` but not `Sync`).
// Constraints: the message itself must be moved to the logger thread and be
// formatted and dropped THERE (a test checks which thread did both), so don't
// turn it into a `String` on the caller's side; keep the return type; no
// `Box::leak`/`String::leak`, no `unsafe`; don't change the tests.
// Until you make `spawn_logger` accept owned messages of any displayable
// type, this exercise will not compile.
fn spawn_logger(msg: &'static str) -> JoinHandle<String> {
    thread::spawn(move || format!("[log] {msg}"))
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fmt;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle, ThreadId};

    // A message that records which thread formatted it and which thread
    // dropped it. It owns everything it holds (a literal and an `Arc`), so the
    // type is `'static`, and yet the last test watches a value of it being
    // dropped: `T: 'static` does not mean "lives forever".
    struct Traced {
        text: &'static str,
        seen: Arc<Mutex<Vec<(&'static str, ThreadId)>>>,
    }

    impl fmt::Display for Traced {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let me = thread::current().id();
            self.seen.lock().unwrap().push(("formatted", me));
            f.write_str(self.text)
        }
    }

    impl Drop for Traced {
        fn drop(&mut self) {
            let me = thread::current().id();
            self.seen.lock().unwrap().push(("dropped", me));
        }
    }

    #[test]
    fn logs_a_string_literal() {
        // A literal is a `&'static str`, and `&'static str: 'static`, so the
        // new signature must still accept it.
        let line = spawn_logger("service started").join().unwrap();
        assert_eq!(line, "[log] service started");
    }

    #[test]
    fn logs_a_string_built_at_run_time() {
        let user = 42;
        let handle = spawn_logger(format!("user {user} logged in"));
        assert_eq!(handle.join().unwrap(), "[log] user 42 logged in");
    }

    #[test]
    fn logs_messages_built_in_a_loop() {
        // Each `String` is created in one iteration and moved into its own
        // thread. None of them lives until the end of the program.
        let handles: Vec<JoinHandle<String>> =
            (1..=3).map(|n| spawn_logger(format!("job {n}"))).collect();
        let lines: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(lines, ["[log] job 1", "[log] job 2", "[log] job 3"]);
    }

    #[test]
    fn logs_a_shared_arc_str() {
        let shared: Arc<str> = Arc::from("disk almost full");
        let handle = spawn_logger(Arc::clone(&shared));
        assert_eq!(handle.join().unwrap(), "[log] disk almost full");
        // The logger thread owned one of the two handles and dropped it when
        // it finished: `Arc<str>` is `'static`, but that handle lived exactly
        // as long as the thread.
        assert_eq!(Arc::strong_count(&shared), 1);
    }

    // A message that counts how often it has been rendered. `Display::fmt`
    // only gets `&self`, so the counter is a `Cell`, and that makes the type
    // `Send` but not `Sync`: a value may be MOVED to another thread, but not
    // SHARED between threads. Moving it is all the logger does, so a `Sync`
    // bound would be one bound too many (and would reject this message).
    struct Counted {
        text: &'static str,
        renders: Cell<u32>,
    }

    impl fmt::Display for Counted {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.renders.set(self.renders.get() + 1);
            write!(f, "{} (render #{})", self.text, self.renders.get())
        }
    }

    #[test]
    fn logs_a_message_that_is_send_but_not_sync() {
        let msg = Counted {
            text: "retrying upload",
            renders: Cell::new(0),
        };
        let line = spawn_logger(msg).join().unwrap();
        assert_eq!(line, "[log] retrying upload (render #1)");
    }

    #[test]
    fn the_message_is_formatted_and_dropped_on_the_logger_thread() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let handle = spawn_logger(Traced {
            text: "cache warmed",
            seen: Arc::clone(&seen),
        });
        let logger = handle.thread().id();
        assert_ne!(logger, thread::current().id());
        assert_eq!(handle.join().unwrap(), "[log] cache warmed");
        // Formatted once, then dropped, both on the logger thread. A version
        // that calls `to_string()` or `format!` on the caller's side (which
        // needs no `'static` bound at all) records the caller's thread here.
        assert_eq!(
            *seen.lock().unwrap(),
            [("formatted", logger), ("dropped", logger)],
            "move the message itself to the logger thread; format and drop it there"
        );
    }
}
