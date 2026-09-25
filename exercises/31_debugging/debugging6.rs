// Module 5 · Debugging — part 6: a guard in a `match` scrutinee lives until the match ends.
//
// In `debugging4` the `Ref` guard had a name, so you could see how long it
// lived. Most guards never get a name. `self.lanes.borrow_mut()` returns a
// `RefMut` that exists only as a TEMPORARY inside a bigger expression, and a
// temporary is not dropped after its last use. It is dropped at the end of its
// TEMPORARY SCOPE, which is usually the whole enclosing statement (up to the
// `;`). A `match` scrutinee is not a temporary scope of its own, so its
// temporaries outlive the entire `match`, arms included:
//
//     match self.lanes.borrow_mut().now.pop_front() {
//         Some(job) => self.execute(job), // the RefMut is still alive here
//         None => ...,
//     }
//
// The lanes stay mutably borrowed while the job runs. When the job submits a
// follow-up (a second `borrow_mut()` on the same cell), `RefCell` panics with
// "RefCell already borrowed".
//
// Why doesn't the borrow checker end the guard early? Two different lifetimes
// are involved. `pop_front()` returns an owned `Job`, so no REFERENCE into the
// guard survives the scrutinee, and non-lexical lifetimes (NLL) end that
// borrow at once: the borrow checker has nothing to complain about. The guard
// itself, though, is a VALUE. When a value is dropped is fixed by the syntax
// (its drop scope), not by its last use, and only the `RefMut`'s `Drop` gives
// the borrow back to the `RefCell`. So the cure is syntactic too: end the
// statement that created the guard before you run the job.
//
// Part B is the same rule inside one statement. Two `borrow_mut()` calls in
// one expression create two temporary guards, and both live until the `;`.
// The obvious rewrite, one named guard, then runs into a COMPILE error: a
// guard is a smart pointer, and `guard.now` really means
// `(*DerefMut::deref_mut(&mut guard)).now`. That call borrows the WHOLE guard,
// so borrowing `guard.now` and `guard.next` mutably at the same time is E0499,
// even though the two fields are disjoint. Through a plain `&mut Lanes` the
// borrow checker CAN see disjoint fields (a split borrow, as in
// `37_borrowck_errors/borrowck1`), so the trick is to go through
// `DerefMut` only once.
//
// Interviewers like this bug because the code compiles, reads naturally, and
// fails only when a job happens to call back into the scheduler. Expect the
// follow-ups: "why doesn't `pop_front()` returning an owned value end the
// borrow?" and "where else does a temporary outlive its expression?" (a
// `while let` body and an `if let` then-branch: see `debugging7`). With a
// `Mutex` instead of a `RefCell` the same code deadlocks: see `debugging8`.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;

// A unit of work. Running a job logs its name and submits its follow-up jobs.
// `Job` deliberately derives nothing: it is not `Clone`, so "clone the job"
// is not a way out.
struct Job {
    name: String,
    then: Vec<Job>,
}

impl Job {
    fn new(name: &str, then: Vec<Job>) -> Self {
        Job {
            name: name.to_string(),
            then,
        }
    }
}

// `now` holds the jobs of the current tick. Jobs submitted while a tick runs
// wait in `next`, so a job that keeps submitting work cannot starve the jobs
// that are already waiting.
struct Lanes {
    now: VecDeque<Job>,
    next: VecDeque<Job>,
}

// The scheduler is shared: in a real program jobs, timers and I/O callbacks
// all hold an `Rc<Scheduler>` and submit work through `&self`. That is why its
// state lives in `RefCell`s.
struct Scheduler {
    lanes: RefCell<Lanes>,
    log: RefCell<Vec<String>>,
}

impl Scheduler {
    fn new() -> Self {
        Scheduler {
            lanes: RefCell::new(Lanes {
                now: VecDeque::new(),
                next: VecDeque::new(),
            }),
            log: RefCell::new(Vec::new()),
        }
    }

    // Queues `job` for the next tick.
    fn submit(&self, job: Job) {
        self.lanes.borrow_mut().next.push_back(job);
    }

    // Runs one job: logs it, then submits its follow-ups (in order). This is
    // the "callback into the scheduler" that real jobs do.
    fn execute(&self, job: Job) {
        self.log.borrow_mut().push(job.name);
        for follow_up in job.then {
            self.submit(follow_up);
        }
    }

    // Part A: runs the next job of the current tick. Returns `false` when the
    // current lane is empty.
    fn run_next(&self) -> bool {
        // TODO: The `run_next_*` tests panic with "RefCell already borrowed"
        // inside `submit`. The `RefMut` created in the scrutinee is a
        // temporary, so it is still alive in the `Some` arm while `execute`
        // runs the job and the job submits its follow-ups. Make the guard die
        // BEFORE the job runs. Requirements:
        //   - keep `execute` and `submit` as they are: a job must run with the
        //     lanes unborrowed, because real jobs call back into the scheduler;
        //   - one job per call, follow-ups go to the next tick in order;
        //   - no copying (`Job` is not `Clone` and the tests compare heap
        //     pointers), no `try_borrow_mut` that silently skips work, and no
        //     `unsafe`.
        // Until the guard is gone before `execute` runs, the tests will fail.
        match self.lanes.borrow_mut().now.pop_front() {
            Some(job) => {
                self.execute(job);
                true
            }
            None => false,
        }
    }

    // Part B: runs every job of the current tick, then starts the next tick:
    // the jobs submitted meanwhile become the current lane. SWAPPING the two
    // lanes (instead of building a new `VecDeque` or moving the jobs over)
    // takes O(1) and keeps both buffers, so the lanes do not reallocate every
    // tick. Returns how many jobs ran.
    fn run_tick(&self) -> usize {
        let mut ran = 0;
        while self.run_next() {
            ran += 1;
        }
        // TODO: The `run_tick_*` tests (once Part A works) panic with "RefCell
        // already borrowed" in the `mem::swap` below. Each `borrow_mut()`
        // creates a temporary `RefMut`, both temporaries live until the `;`,
        // and the second call finds the cell already borrowed. Swap the lanes
        // through ONE guard. Expect the obvious version to be rejected with
        // E0499 "cannot borrow `lanes` as mutable more than once at a time"
        // (if you name the guard `lanes`): every field access through a guard
        // borrows the whole guard. Requirements:
        //   - one `borrow_mut()` for the whole swap;
        //   - swap the two lanes, don't rebuild or refill them: no lane buffer
        //     may be freed, and no job may be copied or moved into the other
        //     buffer (the tests check capacities and addresses);
        //   - no `unsafe`, don't change `Lanes` or the tests.
        // Until the swap goes through a single guard, the tests will fail.
        mem::swap(
            &mut self.lanes.borrow_mut().now,
            &mut self.lanes.borrow_mut().next,
        );
        ran
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(name: &str) -> Job {
        Job::new(name, Vec::new())
    }

    // A scheduler whose current lane holds `now` and whose next lane holds
    // `next`.
    fn scheduler(now: Vec<Job>, next: Vec<Job>) -> Scheduler {
        let s = Scheduler::new();
        {
            let mut lanes = s.lanes.borrow_mut();
            lanes.now.extend(now);
            lanes.next.extend(next);
        }
        s
    }

    fn now_names(s: &Scheduler) -> Vec<String> {
        s.lanes
            .borrow()
            .now
            .iter()
            .map(|j| j.name.clone())
            .collect()
    }

    fn next_names(s: &Scheduler) -> Vec<String> {
        s.lanes
            .borrow()
            .next
            .iter()
            .map(|j| j.name.clone())
            .collect()
    }

    // ---- Part A: `run_next` ----

    #[test]
    fn run_next_runs_one_job_and_queues_its_follow_ups_in_order() {
        let test = job("test");
        let test_name = test.name.as_ptr();
        let build = Job::new("build", vec![test, job("package")]);
        let s = scheduler(vec![build, job("lint")], vec![]);

        assert!(s.run_next());
        // Exactly one job ran...
        assert_eq!(*s.log.borrow(), ["build"]);
        // ...the rest of the current tick is untouched...
        assert_eq!(now_names(&s), ["lint"]);
        // ...and the follow-ups wait for the next tick, in the order the job
        // listed them.
        assert_eq!(next_names(&s), ["test", "package"]);
        // The follow-up was moved into the lane, not rebuilt: its name still
        // uses the same heap buffer.
        assert_eq!(s.lanes.borrow().next[0].name.as_ptr(), test_name);
    }

    #[test]
    fn run_next_leaves_follow_ups_for_the_next_tick() {
        let s = scheduler(
            vec![
                Job::new("a", vec![job("a2")]),
                Job::new("b", vec![job("b2")]),
            ],
            vec![],
        );
        assert!(s.run_next());
        assert!(s.run_next());
        // Jobs submitted during this tick do not run in this tick.
        assert!(!s.run_next());
        assert_eq!(*s.log.borrow(), ["a", "b"]);
        assert!(now_names(&s).is_empty());
        assert_eq!(next_names(&s), ["a2", "b2"]);
    }

    #[test]
    fn run_next_on_an_empty_lane_runs_nothing() {
        let s = scheduler(vec![], vec![job("later")]);
        assert!(!s.run_next());
        assert!(s.log.borrow().is_empty());
        assert_eq!(next_names(&s), ["later"]);
    }

    // ---- Part B: `run_tick` ----

    #[test]
    fn run_tick_runs_the_tick_then_promotes_the_next_lane() {
        let build = Job::new("build", vec![job("test"), job("package")]);
        let s = scheduler(vec![build], vec![job("docs")]);

        assert_eq!(s.run_tick(), 1);
        // `docs` was already waiting; the follow-ups of `build` queue behind it.
        assert_eq!(now_names(&s), ["docs", "test", "package"]);
        assert!(next_names(&s).is_empty());

        assert_eq!(s.run_tick(), 3);
        assert_eq!(*s.log.borrow(), ["build", "docs", "test", "package"]);
        assert!(now_names(&s).is_empty());

        assert_eq!(s.run_tick(), 0);
    }

    #[test]
    fn run_tick_swaps_the_lanes_and_keeps_both_buffers() {
        let docs = job("docs");
        let docs_name = docs.name.as_ptr();
        // A busy tick: the current lane grows a large buffer.
        let busy: Vec<Job> = (0..32).map(|i| job(&format!("job{i}"))).collect();
        let s = scheduler(busy, vec![docs]);
        let (capacity_before, docs_slot) = {
            let lanes = s.lanes.borrow();
            (
                lanes.now.capacity() + lanes.next.capacity(),
                std::ptr::from_ref(&lanes.next[0]),
            )
        };

        assert_eq!(s.run_tick(), 32);

        let lanes = s.lanes.borrow();
        // The waiting job is now current, and it is the same job (same heap
        // buffer), not a copy.
        assert_eq!(lanes.now.len(), 1);
        assert_eq!(lanes.now[0].name.as_ptr(), docs_name);
        assert!(lanes.next.is_empty());
        // It did not even move. Swapping two `VecDeque`s exchanges their
        // headers (buffer pointer, capacity, head, length) in O(1), so `docs`
        // is still in the same slot of the same buffer. Moving the jobs over
        // one by one (`append`, `extend`, a `pop_front` / `push_back` loop) is
        // O(n) and puts `docs` into the other buffer.
        assert!(
            std::ptr::eq(&lanes.now[0], docs_slot),
            "the waiting job was moved into the other buffer: swap the lanes, not the jobs"
        );
        // Neither buffer was freed: the drained lane's buffer is kept for the
        // next tick. Replacing a lane with a fresh `VecDeque` loses capacity.
        assert_eq!(
            lanes.now.capacity() + lanes.next.capacity(),
            capacity_before
        );
    }
}
