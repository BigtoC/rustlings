// Module 4 · Scoped threads — part 1: threads that borrow the caller's slice (E0521).
//
// `thread::spawn` asks for a lot:
//
//     pub fn spawn<F, T>(f: F) -> JoinHandle<T>
//     where
//         F: FnOnce() -> T + Send + 'static,
//         T: Send + 'static,
//
// `Send`, because the closure and everything it captures move to another
// thread (`30_send_sync`). `'static`, because nothing ties the new thread to
// the function that started it: drop the `JoinHandle` and the thread keeps
// running, detached, long after that function has returned and its stack
// frame is gone. `25_lifetimes_deep/lifetimes7` showed the owned side of this:
// move an owned value in and the bound is met. This exercise hits the other
// side. A function that only BORROWS its input cannot hand that borrow to
// `thread::spawn` at all, however soon it joins.
//
// Even `thread::spawn(..)` followed by `.join()` on the very next line is
// rejected. `join` is an ordinary method call, and the signature of `spawn`
// cannot know that it will run: if code between the spawn and the join
// panicked, the function would unwind, its borrows would end, and the caller
// could free the data while the thread was still reading it. The error codes
// are the ones that `56_async_bounds/async_send2` sorted out for spawned
// futures: E0521 "borrowed data escapes outside of function" when the
// closure captures a reference PARAMETER (here `data`, the halves split from
// it, and `on_chunk`), and E0373 when it borrows a LOCAL. `move` does not
// help with E0521: moving a reference moves the borrow, and the borrow is the
// problem.
//
// Copying the data into an owned `Vec` (or an `Arc<Vec<u64>>`) would satisfy
// `'static` for the data, and it is the wrong answer here: it costs a full
// copy of the input just to read it. What you want is a thread that is
// GUARANTEED to be joined before the borrow ends. That is
// `std::thread::scope`, stable since Rust 1.63:
//
//     pub fn scope<'env, F, T>(f: F) -> T
//     where
//         F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T;
//
//     impl<'scope, 'env> Scope<'scope, 'env> {
//         pub fn spawn<F, T>(&'scope self, f: F) -> ScopedJoinHandle<'scope, T>
//         where
//             F: FnOnce() -> T + Send + 'scope,
//             T: Send + 'scope;
//     }
//
// A closure spawned on the `Scope` only has to outlive `'scope`, and `scope`
// does not return until every such thread has finished: it runs your closure,
// catches a panic if there is one, joins every thread that is still running,
// and only then returns (or resumes the panic). So anything that outlives the
// call to `scope` (a parameter, or a local declared before it) may be
// borrowed by the threads, and `&mut` borrows work too (part 2). `Send` is
// still required: sharing `&[u64]` with a thread needs `[u64]: Sync`.
//
// The join happens inside `scope`'s own code, not in a destructor. That is
// the whole trick, and the reason the guard-based `thread::scoped` of 2015 had
// to go: the module README tells that story. One more detail interviewers
// like: if a scoped thread panics and you never `join` its handle, `scope`
// itself panics ("a scoped thread panicked") once all threads are joined. If
// you `join` it yourself, you get the panic back as an `Err` instead.
//
// How interviewers probe this: "Why does `spawn` need `'static`?", "Does
// joining right away make it compile?", "How does `thread::scope` make
// borrowing sound?", "Why not just `Arc` the data?".

use std::thread;

// Sums `data` on two threads, one per half.
fn parallel_sum(data: &[u64]) -> u64 {
    parallel_sum_with(data, &|_| {})
}

// `parallel_sum` with a test seam: `on_chunk` is called with each half, on the
// thread that sums that half. Production callers pass a no-op; the tests use
// it to check which memory each half is and which thread summed it.
fn parallel_sum_with(data: &[u64], on_chunk: &(dyn Fn(&[u64]) + Sync)) -> u64 {
    let (left, right) = data.split_at(data.len() / 2);

    // `thread::scope` joins every thread spawned on `s` before it returns
    // (or before it resumes a panic), so the threads only have to outlive the
    // scope, not the program: `Scope::spawn` asks for `'scope`, not
    // `'static`. `left`, `right` and `on_chunk` all outlive the call to
    // `scope`, so the closures may borrow them. Both threads are spawned
    // before either is joined, so the halves are summed at the same time, and
    // `join().unwrap()` passes a worker's panic on to the caller.
    thread::scope(|s| {
        let a = s.spawn(|| {
            on_chunk(left);
            left.iter().sum::<u64>()
        });
        let b = s.spawn(|| {
            on_chunk(right);
            right.iter().sum::<u64>()
        });
        a.join().unwrap() + b.join().unwrap()
    })
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;
    use std::sync::{Condvar, Mutex};
    use std::thread::ThreadId;
    use std::time::Duration;

    // What one `on_chunk` call saw: the ADDRESSES of the half it was given
    // (plain numbers, so the record is `Send`) and the thread that ran it.
    type Seen = (Range<usize>, ThreadId);

    fn addresses(half: &[u64]) -> Range<usize> {
        let range = half.as_ptr_range();
        range.start.addr()..range.end.addr()
    }

    #[test]
    fn sums_one_to_ten_thousand() {
        let data: Vec<u64> = (1..=10_000).collect();
        assert_eq!(parallel_sum(&data), 50_005_000);
    }

    #[test]
    fn an_empty_slice_sums_to_zero() {
        assert_eq!(parallel_sum(&[]), 0);
    }

    #[test]
    fn odd_lengths_lose_nothing() {
        // 1 element: the left half is empty. 7 elements: 3 + 4.
        assert_eq!(parallel_sum(&[42]), 42);
        assert_eq!(parallel_sum(&[1, 2, 3, 4, 5, 6, 7]), 28);
    }

    #[test]
    fn sums_an_array_on_the_stack() {
        // No heap and no `'static` anywhere: the threads borrow this test's
        // own stack frame.
        let readings = [7u64, 11, 13, 17, 19];
        assert_eq!(parallel_sum(&readings), 67);
    }

    #[test]
    fn the_borrow_ends_when_the_call_returns() {
        // Every thread has been joined when `parallel_sum` returns, so the
        // shared borrow of `v` is over and `v` may be mutated right away.
        let mut v = vec![1, 2, 3];
        assert_eq!(parallel_sum(&v), 6);
        v.push(4);
        assert_eq!(parallel_sum(&v), 10);
        v.clear();
        assert_eq!(parallel_sum(&v), 0);
    }

    #[test]
    fn each_half_is_the_callers_memory_on_its_own_thread() {
        let data: Vec<u64> = (1..=101).collect();
        let seen: Mutex<Vec<Seen>> = Mutex::new(Vec::new());
        let total = parallel_sum_with(&data, &|half| {
            let me = thread::current().id();
            seen.lock().unwrap().push((addresses(half), me));
        });
        assert_eq!(total, 5151);

        let mut seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 2, "call `on_chunk` exactly once per half");
        seen.sort_by_key(|(range, _)| range.start);
        let whole = addresses(&data);
        let (left, right) = (&seen[0].0, &seen[1].0);
        // A copy (`to_vec`, `Arc<Vec<_>>`, a leaked box) lives at another
        // address, so its halves are not part of `data`.
        assert_eq!(
            left.start, whole.start,
            "the left half must start at `data` itself, not at a copy"
        );
        assert_eq!(left.end, right.start, "the halves must be adjacent");
        assert_eq!(right.end, whole.end, "the right half must end with `data`");
        let words = |r: &Range<usize>| (r.end - r.start) / size_of::<u64>();
        assert_eq!((words(left), words(right)), (50, 51), "split in the middle");
        assert_ne!(
            seen[0].1, seen[1].1,
            "the two halves must be summed on two different threads"
        );
    }

    // A meeting point with a deadline. Each half checks in and then waits for
    // the other one; if it waits 20 s in vain, the halves did not run at the
    // same time. Only a solution that finishes one half before starting the
    // other ever waits that long.
    struct Meeting {
        arrived: Mutex<usize>,
        all_here: Condvar,
    }

    impl Meeting {
        fn arrive_and_wait(&self, expected: usize) -> bool {
            let mut arrived = self.arrived.lock().unwrap();
            *arrived += 1;
            self.all_here.notify_all();
            let (_arrived, wait) = self
                .all_here
                .wait_timeout_while(arrived, Duration::from_secs(20), |n| *n < expected)
                .unwrap();
            !wait.timed_out()
        }
    }

    #[test]
    fn both_halves_are_summed_at_the_same_time() {
        let data: Vec<u64> = (1..=1_000).collect();
        let meeting = Meeting {
            arrived: Mutex::new(0),
            all_here: Condvar::new(),
        };
        let met = Mutex::new(Vec::new());
        let total = parallel_sum_with(&data, &|_| {
            let ok = meeting.arrive_and_wait(2);
            met.lock().unwrap().push(ok);
        });
        assert_eq!(total, 500_500);
        assert_eq!(
            met.into_inner().unwrap(),
            [true, true],
            "one half waited 20 s for the other: start both threads before \
             joining either"
        );
    }
}
