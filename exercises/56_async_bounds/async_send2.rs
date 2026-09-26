// Module 4 · Async bounds — part 2: a spawned future must own its data (E0521, E0373).
//
// Part 1 was about `Send`. This part is about the other half of the bound,
// `'static`. `spawn` returns at once, and the task it started keeps running on
// its own: the caller may return, drop the `JoinHandle` (the task goes on
// without it), or forget it. Even a caller that awaits the handle at once can
// be canceled at that `.await` (dropping a future cancels it, as in
// `57_async_combinators`), and the task runs on. The runtime cannot know when
// the task will finish, so the future may not borrow anything from the
// caller's stack: `F: 'static`, the "contains no borrow that could expire"
// bound from `25_lifetimes_deep/lifetimes7`.
//
// An async block is lazy like a closure: it captures its environment when it
// is created and runs later. Without `move` it borrows every variable it only
// reads, as a closure does, and both kinds of borrow are rejected:
//
//   - A borrow of a LOCAL is E0373 "async block may outlive the current
//     function, but it borrows `prefix`, which is owned by the current
//     function" (the closure version is `33_closures/closure1`).
//   - A borrow through a reference PARAMETER is E0521 "borrowed data escapes
//     outside of function ... argument requires that `'1` must outlive
//     `'static`": the caller owns the data, and the task would outlive this
//     function's borrow of it.
//
// rustc's help for E0373 is "use the `move` keyword", and it is only half an
// answer. `async move` moves whatever the block uses, but `name` is a
// `&String`: moving a reference moves the BORROW, so E0521 stays. And a
// `String` moved into the first task of a loop is gone for the second one:
// E0382 "use of moved value ... value moved here, in previous iteration of
// loop". What satisfies `'static` is owned data: give every task its own
// copy, or put one read-only copy behind an `Arc` (`Arc<str>`,
// `Arc<[String]>`) and move an `Arc` clone (a reference-count bump) into each
// task.
//
// Why is there no "scoped" spawn that could borrow, the way `thread::scope`
// lets threads borrow (coming up in `51_scoped_threads`)? `thread::scope`
// works because the call itself blocks until every thread has been joined, so
// the borrows provably end before it returns. An async scope would be a
// future, and it would have to rely on its destructor to wait for the tasks.
// But safe code may `mem::forget` a future: the borrow ends without any
// destructor running, while the tasks still use the data (and a destructor
// that blocks would stall the executor's thread anyway). If you only need
// concurrency, not parallelism, don't spawn at all: `join` the futures inside
// the current task (`57_async_combinators/join1`), where borrowing is fine.
//
// How interviewers probe this: "Why does `tokio::spawn` need `'static` when I
// `.await` the handle right away?", "Why doesn't `async move` fix it?", "Clone
// or `Arc`?", "Why can't there be a safe scoped `spawn`?".

use std::cell::Cell;
use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};

// ---- A tiny thread-per-task runtime (given) --------------------------------

thread_local! {
    // How many tasks this thread has spawned. The tests read it to check that
    // `greet_all` still starts one task per name.
    static SPAWNED: Cell<usize> = const { Cell::new(0) };
}

// Runs `future` on a new OS thread and returns a handle to join it. The bounds
// are exactly `tokio::spawn`'s: the future moves to another thread (`Send`)
// and may outlive its caller (`'static`), and so does its output.
fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    SPAWNED.set(SPAWNED.get() + 1);
    thread::spawn(move || block_on(future))
}

// Polls `future` on the current thread until it is ready. The leaf future in
// this file wakes itself and is `Pending` for one poll only, so a plain poll
// loop is enough (a real executor sleeps until the waker fires, as in
// `29_async_runtime/runtime2`). The bound turns a future that never finishes
// into a panic instead of a hang.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..1_000 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
    panic!("block_on: the future was still pending after 1000 polls");
}

// Stands in for I/O: `Pending` on the first poll, `Ready` on the second (the
// `YieldOnce` future of `28_futures/futures2`).
struct Io {
    done: bool,
}

impl Future for Io {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            Poll::Ready(())
        } else {
            self.done = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn io() -> Io {
    Io { done: false }
}

// ---- The greeter ------------------------------------------------------------

// Renders one greeting after a (simulated) template lookup.
async fn render(prefix: &str, name: &str) -> String {
    io().await;
    format!("{prefix}{name}!")
}

// Greets every name on a task of its own and returns the greetings in the
// order of `names`. `greeting` is trimmed once, and the resulting prefix is
// shared by all tasks.
fn greet_all(names: &[String], greeting: &str) -> Vec<String> {
    // TODO: rustc rejects the `spawn` below twice: E0521 "borrowed data
    // escapes outside of function ... `names` escapes the function body here
    // ... argument requires that `'1` must outlive `'static`" for the name,
    // and E0373 "async block may outlive the current function, but it borrows
    // `prefix`, which is owned by the current function" for the prefix. A
    // spawned task may outlive `greet_all`, so it must own everything it
    // uses. Adding `move` alone is not enough (try it: E0521 stays, and E0382
    // appears). Keep the signature (the caller keeps its `names`), keep one
    // `spawn` per name, and keep the greetings in order. Don't leak memory to
    // get a `&'static`, and don't copy the whole slice for every task. Until
    // you give every task data it owns, this exercise will not compile.
    let prefix = format!("{}, ", greeting.trim());
    let mut handles = Vec::new();
    for name in names {
        handles.push(spawn(async { render(&prefix, name).await }));
    }
    handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn greets_every_name_in_order() {
        let team = names(&["Ada", "Grace", "Barbara"]);
        assert_eq!(
            greet_all(&team, "Hello"),
            ["Hello, Ada!", "Hello, Grace!", "Hello, Barbara!"]
        );
    }

    #[test]
    fn the_caller_keeps_its_names() {
        let team = names(&["Ada", "Grace"]);
        let greetings = greet_all(&team, "Hi");
        // `greet_all` only borrowed the names: they are still here, unchanged.
        assert_eq!(team, ["Ada", "Grace"]);
        assert_eq!(greetings, ["Hi, Ada!", "Hi, Grace!"]);
    }

    #[test]
    fn the_greeting_is_trimmed_once() {
        let team = names(&["Linus"]);
        assert_eq!(greet_all(&team, "  Welcome \n"), ["Welcome, Linus!"]);
    }

    #[test]
    fn works_on_part_of_a_local_vec() {
        // A slice of a local `Vec` is not `'static`, so `names` cannot be
        // `&'static [String]`.
        let team = names(&["Ada", "Grace", "Barbara", "Margaret"]);
        assert_eq!(
            greet_all(&team[1..3], "Hey"),
            ["Hey, Grace!", "Hey, Barbara!"]
        );
    }

    #[test]
    fn spawns_one_task_per_name() {
        let team = names(&["a", "b", "c", "d", "e"]);
        let before = SPAWNED.get();
        let greetings = greet_all(&team, "Yo");
        assert_eq!(greetings.len(), 5);
        assert_eq!(
            SPAWNED.get() - before,
            5,
            "every name must be greeted on a task started with `spawn`"
        );
    }

    #[test]
    fn no_names_no_tasks() {
        let before = SPAWNED.get();
        assert!(greet_all(&[], "Hello").is_empty());
        assert_eq!(SPAWNED.get(), before);
    }

    #[test]
    fn many_names_stay_in_order() {
        let team: Vec<String> = (0..64).map(|i| format!("user{i}")).collect();
        let expected: Vec<String> = team.iter().map(|name| format!("Hi, {name}!")).collect();
        assert_eq!(greet_all(&team, "Hi"), expected);
    }
}
