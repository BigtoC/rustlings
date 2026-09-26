// Module 1 · Interior mutability — part 4: per-thread state with `thread_local!` and `Cell`.
//
// Part 3's statics are ONE value for the whole process. `thread_local!`
// declares something that looks like a static, but every thread gets its OWN
// value: created on that thread's first access, dropped when the thread
// exits.
//
// Since no other thread can ever reach a thread's copy, the value does not
// have to be `Sync`, so part 3's rule does not apply. `Cell`, `RefCell` and
// even `Rc` are fine inside a `thread_local!`, which is why a per-thread
// counter can be a plain `Cell<u32>` instead of an atomic.
//
// You reach the value through a `LocalKey<T>`:
//
//   - `KEY.with(|value| ..)` lends a `&T` for the duration of the closure
//     only. The value dies with its thread, so there is no `&'static T` to
//     hand out. (`try_with` returns an `AccessError` instead of panicking
//     when the value is already destroyed, e.g. inside another thread-local's
//     destructor.)
//   - `LocalKey<Cell<T>>` has shortcuts (Rust 1.73): `get`, `set`, `take` and
//     `replace`, and `LocalKey<RefCell<T>>` has `with_borrow` and
//     `with_borrow_mut`. (`LocalKey::update` is still unstable on 1.96.)
//   - `static KEY: T = const { .. }` initializes from a constant, which lets
//     std avoid lazy initialization, and for a type without drop glue (like
//     `Cell<u32>`) track no extra state at all. Clippy's
//     `missing_const_for_thread_local` asks for it whenever it applies.
//
// Global or per thread? A `static` atomic is right for "how many requests
// has the PROCESS served". It is wrong for anything that describes the
// CURRENT thread: its recursion depth, the span it is inside, a scratch
// buffer, a re-entrancy guard, a per-worker job number. Get that wrong and a
// new thread starts wherever the other threads left off, which is the bug
// below.
//
// Two traps interviewers like: a thread POOL reuses its threads, so
// thread-local state survives from one job to the next on the same worker,
// and a multi-threaded async runtime may move a task to another thread at
// any `.await`, so a thread-local is not a task-local (tokio has
// `task_local!` for that). Also, destructors of thread-locals are
// best-effort: on some platforms they do not run for the main thread.
//
// libtest runs every `#[test]` on a thread of its own, so thread-local state
// never leaks from one test into another. A global static does, and the
// tests run in parallel. That is why only ONE test in this file touches the
// counter.
//
// How interviewers probe this: "`static` or `thread_local!`: when do you use
// each?", "Why may a thread-local hold a `RefCell` when a `static` may not?",
// "Why does `with` take a closure instead of returning `&'static T`?", "Is
// thread-local state safe with a thread pool, or across an `.await`?".

use std::cell::Cell;

// Every worker thread numbers the jobs it runs, 1, 2, 3, ..., and puts the
// number in its log lines. The numbering belongs to the thread: a new worker
// starts at 1, whatever the other workers have done.
//
// One counter PER THREAD: every thread has its own `Cell`, starting at 0,
// and no other thread can ever reach it. That is also why a `!Sync` `Cell` is
// allowed here and no atomic is needed. `const { .. }` makes the initial
// value a constant, so there is no lazy-init step, and a `Cell<u32>` needs no
// destructor either.
thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

// Counts one more job on the CURRENT thread and returns its number.
fn bump() -> u32 {
    // `LocalKey<Cell<u32>>::get` / `set` act on the current thread's cell.
    let n = CALLS.get() + 1;
    CALLS.set(n);
    n
}

// How many jobs the current thread has counted so far.
fn calls() -> u32 {
    CALLS.get()
}

// Starts a job on the current thread and returns its log line.
fn log_line(job: &str) -> String {
    format!("[job {}] {job}", bump())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // This is the ONLY test that touches `CALLS`: in the starter it is a
    // process-wide static, and other tests would run in parallel with this
    // one.
    #[test]
    fn each_thread_numbers_its_own_jobs() {
        assert_eq!(calls(), 0, "a new thread has not counted anything yet");
        assert_eq!(bump(), 1);
        assert_eq!(log_line("warm up the cache"), "[job 2] warm up the cache");
        assert_eq!(calls(), 2);

        let (first, second, count) = thread::spawn(|| (bump(), bump(), calls())).join().unwrap();
        assert_eq!(first, 1, "a new thread must start counting at 1");
        assert_eq!((second, count), (2, 2));
        assert_eq!(
            calls(),
            2,
            "another thread's jobs must not change this thread's count"
        );

        // Four workers at once, three jobs each: every worker numbers its
        // jobs 1, 2, 3, however their threads interleave.
        let numbers: Vec<[u32; 3]> = thread::scope(|s| {
            let workers: Vec<_> = (0..4)
                .map(|_| s.spawn(|| [bump(), bump(), bump()]))
                .collect();
            workers.into_iter().map(|w| w.join().unwrap()).collect()
        });
        assert_eq!(numbers, vec![[1, 2, 3]; 4]);

        let line = thread::spawn(|| log_line("resize images")).join().unwrap();
        assert_eq!(line, "[job 1] resize images");

        assert_eq!(bump(), 3, "this thread carries on where it left off");
        assert_eq!(calls(), 3);
    }
}
