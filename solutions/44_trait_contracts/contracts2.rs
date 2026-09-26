// Traits & Abstraction · Trait contracts — part 2: an `Ord` that agrees with `Eq` and `PartialOrd` (clippy::derive_ord_xor_partial_ord).
//
// `Ord` promises a TOTAL order: for any `a` and `b`, exactly one of `a < b`,
// `a == b` and `a > b` holds, and `<` is transitive. Two more rules tie it to
// the traits next to it:
//
//   - `a.partial_cmp(b) == Some(a.cmp(b))`: `PartialOrd` (the `<`, `<=`, `>`
//     and `>=` operators) must give the same answers as `Ord`;
//   - `a.cmp(b) == Equal` exactly when `a == b`: the order must agree with
//     `PartialEq` (this follows from the first rule and `PartialOrd`'s own
//     contract).
//
// Why both matter: the standard library does not promise WHICH method a
// collection calls. Today `slice::sort` and `BinaryHeap` compare with the `<`
// and `<=` operators (so `PartialOrd`), while `BTreeSet`, `BTreeMap` and
// `Iterator::max` call `Ord::cmp`. If the two disagree, one type sorts one
// way in a heap and another way in a set. Breaking the contract is a LOGIC
// error, not undefined behavior: the collection that sees it may return wrong
// answers, panic or loop, but it will not corrupt memory.
//
// The `Job` below breaks both rules at once:
//
//   - `PartialOrd` is DERIVED. A derived ordering on a struct is lexicographic
//     in field DECLARATION order: compare the first field, and only on a tie
//     look at the next one. `name` is declared first, so the scheduler's heap
//     runs jobs in reverse alphabetical order, whatever their priority. The
//     order of the fields in a struct is behavior once you derive `Ord`:
//     reordering them "for readability" changes how the type sorts.
//   - `Ord` is written by hand and compares `priority` only, so it calls two
//     DIFFERENT jobs with the same priority `Equal`. A `BTreeSet` knows nothing
//     but `cmp`: to it, `Equal` means "already present". `insert` drops the
//     second job, and `contains` finds a job that was never inserted.
//
// Clippy flags the first mistake with its deny-by-default lint
// `derive_ord_xor_partial_ord`. Nothing flags the second: `cmp` is ordinary
// code, and only a test finds that it is coarser than `==`.
//
// Interviewers ask: "How does `#[derive(Ord)]` compare two structs?", "What
// happens in a `BTreeMap` if `Ord` disagrees with `Eq`?", "Why should
// `partial_cmp` return `Some(self.cmp(other))`?" and "Build a priority queue
// that is FIFO among equal priorities."

use std::cmp::Ordering;
use std::collections::BinaryHeap;

// A job waiting to run. Higher `priority` runs sooner. Among jobs of equal
// priority the one submitted first (the lower `submitted` number) runs
// sooner. `name` is a label, but it is part of a job's identity: two jobs are
// equal only when ALL three fields are (that is what the derived `PartialEq`
// says, and it stays).
//
// `BinaryHeap` is a MAX-heap: `pop` returns the GREATEST element. So "greater"
// has to mean "runs sooner": `a > b` means `a` runs before `b`. A `BTreeSet`
// iterates from least to greatest, so it lists jobs in reverse run order and
// `last()` is the job that runs next.
//
// `Job` deliberately does not implement `Clone`.
//
// `PartialOrd` is no longer derived. It is written by hand below and simply
// asks `Ord`, so the operators, `sort` and the heap all see the one order
// `cmp` defines. That is the canonical shape clippy's
// `non_canonical_partial_ord_impl` lint checks for.
#[derive(Debug, PartialEq, Eq)]
struct Job {
    name: String,
    priority: u8,
    submitted: u64,
}

impl Job {
    fn new(name: &str, priority: u8, submitted: u64) -> Self {
        Job {
            name: name.to_string(),
            priority,
            submitted,
        }
    }
}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // A total order has an answer for every pair, so this is always
        // `Some`, and it can never disagree with `cmp`.
        Some(self.cmp(other))
    }
}

impl Ord for Job {
    // Each `then_with` only runs on a tie, so this is the lexicographic order
    // "priority, then FIFO, then name". `submitted` is compared the other way
    // round (`other` first) because an EARLIER job must be GREATER to pop
    // first from the max-heap. Comparing all three fields, the same fields the
    // derived `PartialEq` compares, is what makes `cmp` return `Equal` exactly
    // when `==` is true, so the `BTreeSet` keeps every distinct job.
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.submitted.cmp(&self.submitted))
            .then_with(|| self.name.cmp(&other.name))
    }
}

// The scheduler. Every job gets the next submission number, so jobs
// submitted earlier have lower numbers.
struct Scheduler {
    queue: BinaryHeap<Job>,
    clock: u64,
}

impl Scheduler {
    fn new() -> Self {
        Scheduler {
            queue: BinaryHeap::new(),
            clock: 0,
        }
    }

    fn submit(&mut self, name: &str, priority: u8) {
        self.queue.push(Job::new(name, priority, self.clock));
        self.clock += 1;
    }

    // Removes and returns the job that runs next.
    fn next_job(&mut self) -> Option<Job> {
        self.queue.pop()
    }

    // Runs the queue dry and returns the names in the order the jobs ran.
    fn drain_names(&mut self) -> Vec<String> {
        let mut names = Vec::new();
        while let Some(job) = self.next_job() {
            names.push(job.name);
        }
        names
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn names<'a>(jobs: impl IntoIterator<Item = &'a Job>) -> Vec<&'a str> {
        jobs.into_iter().map(|job| job.name.as_str()).collect()
    }

    // A table of jobs that differ in one field at a time.
    fn table() -> Vec<Job> {
        vec![
            Job::new("backup", 1, 0),
            Job::new("backup", 1, 7),
            Job::new("backup", 9, 0),
            Job::new("alert", 1, 0),
            Job::new("alert", 9, 3),
            Job::new("zebra", 9, 3),
            Job::new("email", 5, 2),
            Job::new("", 0, 0),
            Job::new("max", u8::MAX, u64::MAX),
        ]
    }

    #[test]
    fn scheduler_runs_the_highest_priority_first() {
        let mut s = Scheduler::new();
        s.submit("backup", 1);
        s.submit("deploy", 9);
        s.submit("email", 5);
        s.submit("alert", 7);
        assert_eq!(s.drain_names(), ["deploy", "alert", "email", "backup"]);
    }

    #[test]
    fn scheduler_is_first_in_first_out_within_a_priority() {
        let mut s = Scheduler::new();
        for name in ["zulu", "alpha", "mike", "bravo", "yankee"] {
            s.submit(name, 3);
        }
        assert_eq!(
            s.drain_names(),
            ["zulu", "alpha", "mike", "bravo", "yankee"]
        );
    }

    #[test]
    fn an_urgent_job_submitted_later_jumps_the_queue() {
        let mut s = Scheduler::new();
        s.submit("report", 2);
        s.submit("cleanup", 2);
        assert_eq!(s.next_job().map(|job| job.name), Some("report".to_string()));
        s.submit("page-oncall", 8);
        s.submit("rotate-logs", 2);
        assert_eq!(s.drain_names(), ["page-oncall", "cleanup", "rotate-logs"]);
        assert!(s.next_job().is_none());
    }

    #[test]
    fn btree_set_keeps_every_distinct_job() {
        let mut set = BTreeSet::new();
        assert!(set.insert(Job::new("resize", 4, 10)));
        assert!(set.insert(Job::new("thumbnail", 4, 11)));
        assert!(set.insert(Job::new("notify", 6, 12)));
        assert_eq!(set.len(), 3);
        // Jobs equal in priority and submission number are still different
        // jobs when their names differ.
        assert!(set.insert(Job::new("audit", 4, 10)));
        assert_eq!(set.len(), 4);
        // Inserting an equal job is refused, as it should be.
        assert!(!set.insert(Job::new("resize", 4, 10)));
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn btree_set_contains_only_what_was_inserted() {
        let mut set = BTreeSet::new();
        set.insert(Job::new("resize", 4, 10));
        set.insert(Job::new("notify", 6, 12));
        assert!(
            !set.contains(&Job::new("ghost", 4, 99)),
            "`contains` found a job that was never inserted"
        );
        assert!(!set.contains(&Job::new("resize", 4, 11)));
        assert!(!set.contains(&Job::new("notify", 5, 12)));
        assert!(set.contains(&Job::new("resize", 4, 10)));
        assert!(set.contains(&Job::new("notify", 6, 12)));
    }

    #[test]
    fn a_collected_btree_set_finds_its_own_jobs() {
        // `collect` sorts the jobs with `<` (`PartialOrd`), then the set
        // searches with `cmp` (`Ord`). If the two disagree, the set can miss
        // a job that is in it.
        let set: BTreeSet<Job> = [Job::new("resize", 4, 10), Job::new("notify", 6, 12)]
            .into_iter()
            .collect();
        assert_eq!(set.len(), 2);
        for job in [Job::new("resize", 4, 10), Job::new("notify", 6, 12)] {
            assert!(set.contains(&job), "the set lost {job:?}");
        }
    }

    #[test]
    fn btree_set_order_is_the_reverse_run_order() {
        let set: BTreeSet<Job> = [
            Job::new("backup", 1, 0),
            Job::new("deploy", 9, 1),
            Job::new("alert", 9, 2),
            Job::new("email", 5, 3),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            names(set.iter().rev()),
            ["deploy", "alert", "email", "backup"]
        );
        assert_eq!(set.last().map(|job| job.name.as_str()), Some("deploy"));
        assert_eq!(set.first().map(|job| job.name.as_str()), Some("backup"));
    }

    #[test]
    fn sort_and_max_agree_with_the_heap() {
        // `sort` compares with `<`, `max` with `cmp`, the heap with `<=`: all
        // three must see the same order.
        let mut sorted = table();
        sorted.sort();
        let mut heap: BinaryHeap<Job> = table().into_iter().collect();
        let mut popped = Vec::new();
        while let Some(job) = heap.pop() {
            popped.push(job);
        }
        popped.reverse();
        assert_eq!(sorted, popped);
        let max = table().into_iter().max().unwrap();
        assert_eq!(Some(&max), sorted.last());
        assert_eq!(max, Job::new("max", u8::MAX, u64::MAX));
    }

    #[test]
    fn cmp_agrees_with_eq_and_partial_cmp() {
        let jobs = table();
        for a in &jobs {
            for b in &jobs {
                let ord = a.cmp(b);
                assert_eq!(ord == Ordering::Equal, a == b, "{a:?} vs {b:?}");
                assert_eq!(a.partial_cmp(b), Some(ord), "{a:?} vs {b:?}");
                assert_eq!(b.cmp(a), ord.reverse(), "{a:?} vs {b:?}");
                assert_eq!(a < b, ord == Ordering::Less, "{a:?} < {b:?}");
                assert_eq!(a > b, ord == Ordering::Greater, "{a:?} > {b:?}");
                // A fresh, equal job compares `Equal` to the original.
                let copy = Job::new(&a.name, a.priority, a.submitted);
                assert_eq!(a.cmp(&copy), Ordering::Equal);
            }
        }
    }

    #[test]
    fn cmp_is_transitive() {
        let jobs = table();
        for a in &jobs {
            for b in &jobs {
                for c in &jobs {
                    if a <= b && b <= c {
                        assert!(a <= c, "{a:?} <= {b:?} <= {c:?} but not {a:?} <= {c:?}");
                    }
                }
            }
        }
    }
}
