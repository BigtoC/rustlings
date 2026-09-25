// Module 5 · Debugging — part 7: `while let` and `if let` keep their scrutinee guard in the body.
//
// `debugging6` fixed a `match` by moving the `borrow_mut()` into a `let`
// statement of its own. A loop condition gives you no such "before":
// `while let PAT = EXPR { BODY }` evaluates EXPR again on every iteration, and
// the temporaries EXPR creates are dropped only at the end of that iteration's
// BODY. So in
//
//     while let Some(task) = self.queue.borrow_mut().pop_front() {
//         self.attempt(task); // the RefMut is still alive in here
//     }
//
// every task runs with the queue mutably borrowed, and the first task that
// asks for a retry (`self.queue.borrow_mut().push_back(..)`) panics with
// "RefCell already borrowed". The Rust Book shows the `Mutex` version of this
// bug in its thread pool (chapter 21.2, Listing 21-21):
// `while let Ok(job) = receiver.lock().unwrap().recv() { job(); }` holds the
// lock while the job runs, so the workers take turns instead of running in
// parallel. Nothing fails; the pool is just slow.
//
// `if let` is the case edition 2024 changed, and only halfway. The rule since
// Rust 2024: temporaries of an `if let` scrutinee are dropped BEFORE the `else`
// block runs (in 2021 they lived through `else` as well). The then-block did
// not change: the temporaries stay alive in it, because the pattern's
// bindings may borrow from them. `match` and `while let` did not change at all.
// Part B is the then-block case, with a shared `Ref` from `borrow()`. It is
// the same bug as the LeetCode classic
// `if let Some(left) = node.borrow().left.clone() { node.borrow_mut()... }`:
// `.clone()` makes the binding own its value, but the `Ref` is still alive.
//
// The positions that DO end their temporaries early are worth memorizing:
// a `let` statement (at its `;`), a `let ... else` (at its `;`, or before
// its `else` block runs when the pattern does not match; the same in both
// editions), and a plain `if COND` / `while COND` condition without a
// pattern (before the body). That is why
// `if self.queue.borrow().is_empty() { self.queue.borrow_mut()... }` is fine.
//
// Interviewers ask "why does `while let` hold the lock for the whole body?"
// and "what did edition 2024 change for `if let`, and what did it leave
// alone?". A strong answer names the temporary scopes above and shows the
// rewrite that ends each borrow before the work starts.

use std::cell::RefCell;
use std::collections::VecDeque;

// A task that fails its first `failures_left` runs, then succeeds. `Task`
// deliberately derives nothing: it is not `Clone`.
struct Task {
    name: String,
    failures_left: u32,
}

impl Task {
    fn new(name: &str, failures_left: u32) -> Self {
        Task {
            name: name.to_string(),
            failures_left,
        }
    }
}

// A retrying worker. Producers elsewhere in the program hold an `Rc<Worker>`
// and add tasks through `&self`, which is why the queue lives in a `RefCell`.
struct Worker {
    queue: RefCell<VecDeque<Task>>,
    log: RefCell<Vec<String>>,
}

impl Worker {
    fn new(tasks: Vec<Task>) -> Self {
        Worker {
            queue: RefCell::new(tasks.into()),
            log: RefCell::new(Vec::new()),
        }
    }

    // Puts a failed task at the BACK of the queue, so the tasks that are
    // already waiting run before its retry.
    fn retry_later(&self, task: Task) {
        self.queue.borrow_mut().push_back(task);
    }

    // Runs `task` once. A failing run logs "<name> failed" and schedules a
    // retry. A successful run logs "<name> ok".
    fn attempt(&self, mut task: Task) {
        if task.failures_left == 0 {
            self.log.borrow_mut().push(format!("{} ok", task.name));
        } else {
            task.failures_left -= 1;
            self.log.borrow_mut().push(format!("{} failed", task.name));
            self.retry_later(task);
        }
    }

    // Part A: runs tasks until the queue is empty, retries included. Returns
    // how many runs there were.
    fn run_all(&self) -> usize {
        // `let ... else` is a statement, so the `RefMut` created by
        // `borrow_mut()` is dropped at its end (before the `else` block, too).
        // Each task then runs with the queue unborrowed, and `retry_later` can
        // borrow it again.
        let mut runs = 0;
        loop {
            let Some(task) = self.queue.borrow_mut().pop_front() else {
                break;
            };
            runs += 1;
            self.attempt(task);
        }
        runs
    }

    // Part B: moves the task called `name` to the front of the queue (someone
    // clicked "run this now"), keeping the other tasks in order. If several
    // queued tasks have that name, only the first of them (the one that would
    // run soonest) moves. Returns `false` if no queued task has that name.
    fn promote(&self, name: &str) -> bool {
        // Search in a statement of its own: the `Ref` from `borrow()` is
        // dropped at the `;`, and only the plain `usize` survives. The
        // then-block can then take the one `borrow_mut()` it needs.
        let found = self.queue.borrow().iter().position(|t| t.name == name);
        if let Some(pos) = found {
            let mut queue = self.queue.borrow_mut();
            let task = queue.remove(pos).expect("`position` just found it");
            queue.push_front(task);
            true
        } else {
            false
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(w: &Worker) -> Vec<String> {
        w.queue.borrow().iter().map(|t| t.name.clone()).collect()
    }

    // ---- Part A: `run_all` ----

    #[test]
    fn run_all_retries_failed_tasks_after_the_others() {
        let w = Worker::new(vec![
            Task::new("a", 0),
            Task::new("b", 2),
            Task::new("c", 1),
        ]);
        assert_eq!(w.run_all(), 6);
        // A retry goes to the back of the queue, so `c` gets its first run
        // before `b` gets its second.
        assert_eq!(
            *w.log.borrow(),
            ["a ok", "b failed", "c failed", "b failed", "c ok", "b ok"]
        );
        assert!(w.queue.borrow().is_empty());
    }

    #[test]
    fn run_all_without_failures_runs_each_task_once() {
        let w = Worker::new(vec![Task::new("x", 0), Task::new("y", 0)]);
        assert_eq!(w.run_all(), 2);
        assert_eq!(*w.log.borrow(), ["x ok", "y ok"]);
        assert!(w.queue.borrow().is_empty());
    }

    #[test]
    fn run_all_on_an_empty_queue_runs_nothing() {
        let w = Worker::new(Vec::new());
        assert_eq!(w.run_all(), 0);
        assert!(w.log.borrow().is_empty());
    }

    // ---- Part B: `promote` ----

    #[test]
    fn promote_moves_the_task_to_the_front_and_keeps_the_others_in_order() {
        let w = Worker::new(vec![
            Task::new("a", 0),
            Task::new("b", 0),
            Task::new("c", 0),
            Task::new("d", 0),
        ]);
        let c_name = w.queue.borrow()[2].name.as_ptr();
        assert!(w.promote("c"));
        // Only `c` moved; `a`, `b` and `d` keep their relative order (a swap
        // with the front would not).
        assert_eq!(names(&w), ["c", "a", "b", "d"]);
        // It is the same task, not a rebuilt one: same heap buffer.
        assert_eq!(w.queue.borrow()[0].name.as_ptr(), c_name);
    }

    #[test]
    fn promote_the_last_task() {
        let w = Worker::new(vec![
            Task::new("a", 0),
            Task::new("b", 0),
            Task::new("c", 0),
        ]);
        assert!(w.promote("c"));
        assert_eq!(names(&w), ["c", "a", "b"]);
    }

    #[test]
    fn promote_the_front_task_changes_nothing_else() {
        let w = Worker::new(vec![Task::new("a", 0), Task::new("b", 0)]);
        assert!(w.promote("a"));
        assert_eq!(names(&w), ["a", "b"]);
    }

    #[test]
    fn promote_moves_only_the_first_task_with_that_name() {
        let w = Worker::new(vec![
            Task::new("a", 0),
            Task::new("x", 0),
            Task::new("b", 0),
            Task::new("x", 1),
        ]);
        let first_x = w.queue.borrow()[1].name.as_ptr();
        let second_x = w.queue.borrow()[3].name.as_ptr();
        assert!(w.promote("x"));
        assert_eq!(names(&w), ["x", "a", "b", "x"]);
        // The first `x` moved to the front; the second one stayed at the back.
        assert_eq!(w.queue.borrow()[0].name.as_ptr(), first_x);
        assert_eq!(w.queue.borrow()[3].name.as_ptr(), second_x);
    }

    #[test]
    fn promote_an_unknown_task_returns_false() {
        let w = Worker::new(vec![Task::new("a", 0), Task::new("b", 0)]);
        assert!(!w.promote("z"));
        assert_eq!(names(&w), ["a", "b"]);
        assert!(!Worker::new(Vec::new()).promote("a"));
    }

    #[test]
    fn a_promoted_task_runs_first() {
        let w = Worker::new(vec![Task::new("a", 0), Task::new("b", 1)]);
        assert!(w.promote("b"));
        assert_eq!(w.run_all(), 3);
        assert_eq!(*w.log.borrow(), ["b failed", "a ok", "b ok"]);
    }
}
