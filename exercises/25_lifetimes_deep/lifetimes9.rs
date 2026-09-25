// Module 1 · Lifetimes (deep) — part 9: leaking on purpose to get a `&'static str`.
//
// Part 7 showed that a `T: 'static` bound is usually what an API should ask
// for. Sometimes, though, you really want a `&'static str` built at run time:
// a config value read once at startup, which many threads and structs will
// read until the process exits, and which you would rather pass around as a
// plain `&str` than as an `Arc<str>`. A `&'static` reference needs data that
// is NEVER freed, and you can arrange that by giving up ownership on purpose.
// `Box::leak` turns a `Box<T>` into a `&'a mut T` for any lifetime `'a` that
// `T` itself outlives, `'static` included, and `String::leak` (Rust 1.72)
// does the same for a `String`.
//
// Leaking is memory-SAFE: nothing dangles, the memory is simply never given
// back, and the value's `Drop` never runs. (Rust does not promise that memory
// gets freed: `std::mem::forget` is a safe function, and an `Rc` cycle leaks
// without any `unsafe`.) Whether it is a good idea is a design question:
//   - Fine: a bounded amount, done once (config, command-line arguments,
//     interned names). The OS reclaims it at exit, which is when you would
//     have freed it anyway.
//   - Wrong: once per request or per call. Memory then grows without limit,
//     which is why `37_borrowck_errors/borrowck2`, in the next module,
//     rejects `Box::leak` as a way out of E0515.
//   - Wrong: a type whose `Drop` does real work (flushing a `BufWriter`,
//     deleting a temporary file). A leaked value is never dropped.
// For ONE global value, a `static` holding a `std::sync::OnceLock` (set once
// at run time) or a `LazyLock` (computed on first use) is usually nicer: see
// this module's README.
//
// How interviewers probe this: "How do you get a `&'static str` from data
// you only have at run time?", "Is `Box::leak` unsafe? Is it ever the right
// answer?".

// Called once per configuration value, at startup. Every call hands back its
// own `&'static str`, which stays valid until the process exits.
fn leak_config(s: String) -> &'static str {
    // TODO: This is rejected with E0515 "cannot return reference to function
    // parameter `s`": `s` belongs to this function and is freed when it
    // returns, and no lifetime annotation can change that. Keep the
    // signature, and make the text live until the process exits by giving up
    // ownership of the String's heap buffer ON PURPOSE.
    // Constraints: don't copy the text (a test compares heap addresses); no
    // shared global slot (every call returns its own string); no `unsafe`, no
    // `mem::forget`; don't change the tests.
    // Until you return a reference that really is `'static`, this exercise
    // will not compile.
    &s
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn returns_the_same_text() {
        assert_eq!(leak_config(String::from("mode=fast")), "mode=fast");
        assert_eq!(leak_config(String::new()), "");
    }

    #[test]
    fn every_call_gets_its_own_string() {
        // Two configs, two strings: not one global slot that is set once.
        let eu = leak_config(String::from("region=eu"));
        let us = leak_config(String::from("region=us"));
        assert_eq!((eu, us), ("region=eu", "region=us"));
    }

    #[test]
    fn the_heap_buffer_is_handed_over_not_copied() {
        // `String::from` allocates exactly `len` bytes, so there is no spare
        // capacity to trim, and leaking keeps the very same buffer. A copy
        // (`clone`, `to_owned`, `format!`, ...) would live somewhere else.
        let config = String::from("log=debug");
        let heap = config.as_ptr();
        assert_eq!(leak_config(config).as_ptr(), heap);
    }

    #[test]
    fn outlives_its_owner_and_crosses_threads() {
        let config: &'static str;
        {
            let owner = format!("threads={}", 4);
            config = leak_config(owner);
            // `owner` was moved into `leak_config`: nothing here owns the
            // text any more, and this scope ends right now.
        }
        // No `Arc` and no `thread::scope`: a plain `&'static str` is `Copy`,
        // and it may be moved into as many threads as you like.
        let handles: Vec<_> = (0..2)
            .map(|_| thread::spawn(move || config.to_uppercase()))
            .collect();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), "THREADS=4");
        }
    }
}
