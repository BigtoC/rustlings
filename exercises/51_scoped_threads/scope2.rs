// Module 4 · Scoped threads — part 2: one disjoint `&mut` chunk per thread (E0499).
//
// Part 1's threads only READ the caller's slice. Scoped threads may also
// borrow it mutably, and then the borrow checker applies the rule it applies
// everywhere (`37_borrowck_errors/borrowck1`): at most ONE live `&mut` to a
// place. A closure that writes through `data` captures `*data` by mutable
// borrow, and a closure handed to `Scope::spawn` keeps its captures alive for
// as long as its thread may run, which is the rest of the scope. So a second
// closure that also writes through `data` is E0499 "cannot borrow `*data` as
// mutable more than once at a time", even though the two closures touch
// different halves. The checker never compares `..mid` with `mid..`: both
// closures borrow the whole place `*data`.
//
// That refusal is not pedantry. It is the very rule that makes the threads
// free of data races: if two threads can never hold `&mut` to overlapping
// memory, no element can ever be written by two threads at once, and no lock
// is needed. So the fix is not a lock (a `Mutex` around the slice compiles,
// and makes the threads take turns). The fix is to cut the slice FIRST into
// pieces the checker knows are disjoint, and to give each thread one piece
// of its own. Slice methods such as `split_at_mut`, `chunks_mut` and
// `chunks_exact_mut` do the cutting: safe functions built on a little
// `unsafe` code inside std, which has proven that the pieces never overlap.
// (A hand-rolled `split_at_mut`, and what Miri says when it gets the proof
// wrong, is part of the `deep-dive/src/ub_zoo.rs` lab.)
// A `&mut [u64]` is `Send`, because `[u64]` is, so each piece can be moved
// into its own thread.
//
// Once the borrows are right, the arithmetic is where interview answers
// break. `n` threads over `len` elements means chunks of `len / n` elements
// rounded UP, or `n` chunks cannot cover everything; the last chunk may be
// shorter. And two inputs crash the obvious formulas: `n == 0` (an integer
// division by zero panics, in release builds too) and `len == 0` (a chunk
// size of 0 makes the chunking itself panic). Decide what they mean before
// writing the formula; here, zero threads means one.
//
// Every worker thread reports back through its handle, or through a panic.
// If a scoped thread panics, `thread::scope` waits for all the others and
// then panics itself ("a scoped thread panicked"), unless you `join` that
// handle and deal with the `Err`. A failure inside a worker cannot get lost
// by accident: you would have to `join` it and throw the `Err` away.
//
// How interviewers probe this: "Two threads write to different halves of one
// array: why does Rust reject it, and how do you convince it?", "What goes in
// the `Mutex`?" (nothing: disjoint chunks need no lock), "What happens when
// `n` is 0, or larger than the input?", "What if one worker panics?".

use std::thread;

// Multiplies every element of `data` by `factor`, in place, on up to `n`
// threads.
fn scale_in_place(data: &mut [u64], factor: u64, n: usize) {
    scale_in_place_with(data, factor, n, &|_| {});
}

// `scale_in_place` with a test seam: `on_chunk` is called once per chunk, on
// the thread that scales that chunk. Production callers pass a no-op.
fn scale_in_place_with(
    data: &mut [u64],
    factor: u64,
    n: usize,
    on_chunk: &(dyn Fn(&[u64]) + Sync),
) {
    // TODO: rustc rejects the second `s.spawn` with E0499 "cannot borrow
    // `*data` as mutable more than once at a time": both closures capture all
    // of `*data` mutably, for as long as their threads may run. The starter
    // also always uses two threads and ignores `n` (hence rustc's warning
    // that `n` is unused). Requirements:
    //   - split `data` into at most `n` contiguous chunks (treat `n == 0` as
    //     1) and scale each chunk on a thread of its own, all at the same
    //     time; with enough elements, use all `n` threads (a test runs 4
    //     threads over 1000 elements);
    //   - call `on_chunk` once per chunk, on the thread that scales it;
    //   - every element is multiplied by `factor` exactly once, in place;
    //     overflow still panics in debug builds, as `*x *= factor` does, and
    //     that panic must reach the caller;
    //   - no input may crash the split, including `len == 0` and `n == 0`.
    // Constraints: work on the caller's memory, so no copy of the data (the
    // tests check that every chunk is part of `data`), no `Mutex` or other
    // lock around the slice, no `unsafe`, and don't change the tests.
    // Until you give every thread a `&mut` of its own, this exercise will not
    // compile.
    let mid = data.len() / 2;
    thread::scope(|s| {
        s.spawn(|| {
            on_chunk(&data[..mid]);
            for x in &mut data[..mid] {
                *x *= factor;
            }
        });
        s.spawn(|| {
            on_chunk(&data[mid..]);
            for x in &mut data[mid..] {
                *x *= factor;
            }
        });
    });
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::ops::Range;
    use std::sync::{Condvar, Mutex};
    use std::thread::ThreadId;
    use std::time::Duration;

    // What one `on_chunk` call saw: the ADDRESSES of its chunk (plain
    // numbers, so the record is `Send`) and the thread that ran it.
    type Seen = (Range<usize>, ThreadId);

    fn addresses(chunk: &[u64]) -> Range<usize> {
        let range = chunk.as_ptr_range();
        range.start.addr()..range.end.addr()
    }

    fn one_to(len: u64) -> Vec<u64> {
        (1..=len).collect()
    }

    fn times(data: &[u64], factor: u64) -> Vec<u64> {
        data.iter().map(|x| x * factor).collect()
    }

    // Runs `scale_in_place_with`, then checks the chunks it reported: their
    // non-empty ones, in address order, must tile `data` exactly (no gap, no
    // overlap, no copy), there are at most `max(n, 1)` of them, and each ran
    // on a thread of its own. Returns them, sorted by address.
    fn scale_and_check_chunks(data: &mut [u64], factor: u64, n: usize) -> Vec<Seen> {
        let whole = addresses(data);
        let seen = Mutex::new(Vec::new());
        scale_in_place_with(data, factor, n, &|chunk| {
            let me = thread::current().id();
            seen.lock().unwrap().push((addresses(chunk), me));
        });
        let mut seen: Vec<Seen> = seen.into_inner().unwrap();
        seen.sort_by_key(|(range, _)| range.start);

        let mut next = whole.start;
        for (range, _) in seen.iter().filter(|(range, _)| !range.is_empty()) {
            assert_eq!(
                range.start, next,
                "chunks must be adjacent pieces of `data` itself (found a gap, \
                 an overlap or a copy)"
            );
            next = range.end;
        }
        assert_eq!(next, whole.end, "the chunks must cover all of `data`");
        assert!(
            seen.len() <= n.max(1),
            "{} chunks for n = {n}: use at most n threads (one if n is 0)",
            seen.len()
        );
        let threads: HashSet<ThreadId> = seen.iter().map(|(_, thread)| *thread).collect();
        assert_eq!(
            threads.len(),
            seen.len(),
            "every chunk needs a thread of its own"
        );
        seen
    }

    #[test]
    fn matches_the_sequential_result_for_every_size_and_thread_count() {
        for len in [1, 2, 7, 1000] {
            for n in [1, 3, 4, 64] {
                let mut data = one_to(len);
                let expected = times(&data, 3);
                scale_in_place(&mut data, 3, n);
                assert_eq!(data, expected, "len = {len}, n = {n}");
            }
        }
    }

    #[test]
    fn an_empty_slice_does_not_crash_the_split() {
        for n in [0, 1, 3, 4, 64] {
            let mut data: Vec<u64> = Vec::new();
            scale_in_place(&mut data, 3, n);
            assert!(data.is_empty());
        }
    }

    #[test]
    fn zero_threads_means_one_thread() {
        for len in [1, 7, 1000] {
            let mut data = one_to(len);
            let expected = times(&data, 5);
            let seen = scale_and_check_chunks(&mut data, 5, 0);
            assert_eq!(data, expected, "len = {len}, n = 0");
            assert_eq!(seen.len(), 1, "n = 0 means one chunk: all of `data`");
        }
    }

    #[test]
    fn more_threads_than_elements() {
        let mut data = one_to(7);
        scale_and_check_chunks(&mut data, 2, 64);
        assert_eq!(data, [2, 4, 6, 8, 10, 12, 14]);
    }

    #[test]
    fn chunks_are_disjoint_pieces_of_data_each_on_its_own_thread() {
        for (len, n) in [(10, 3), (7, 4), (2, 2), (1000, 64)] {
            let mut data = one_to(len);
            let expected = times(&data, 7);
            scale_and_check_chunks(&mut data, 7, n);
            assert_eq!(data, expected, "len = {len}, n = {n}");
        }
    }

    #[test]
    fn uses_all_n_threads_when_there_is_enough_work() {
        let mut data = one_to(1000);
        let seen = scale_and_check_chunks(&mut data, 2, 4);
        let busy = seen.iter().filter(|(range, _)| !range.is_empty()).count();
        assert_eq!(busy, 4, "1000 elements are plenty of work for 4 threads");
        assert_eq!(data, times(&one_to(1000), 2));
    }

    // A meeting point with a deadline. Every chunk checks in and then waits
    // until `expected` chunks have arrived. If it waits 20 s in vain, the
    // chunks did not run at the same time, and everybody stops waiting. Only
    // a solution that runs the chunks one after another ever waits that long.
    struct Meeting {
        state: Mutex<(usize, bool)>, // (arrived, somebody gave up)
        changed: Condvar,
    }

    impl Meeting {
        fn new() -> Self {
            Meeting {
                state: Mutex::new((0, false)),
                changed: Condvar::new(),
            }
        }

        fn arrive_and_wait(&self, expected: usize) -> bool {
            let mut state = self.state.lock().unwrap();
            state.0 += 1;
            self.changed.notify_all();
            let (mut state, wait) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(20), |(arrived, gave_up)| {
                    *arrived < expected && !*gave_up
                })
                .unwrap();
            if wait.timed_out() {
                state.1 = true;
                self.changed.notify_all();
            }
            state.0 >= expected
        }
    }

    #[test]
    fn all_chunks_are_scaled_at_the_same_time() {
        let mut data = one_to(1000);
        let meeting = Meeting::new();
        let met = Mutex::new(Vec::new());
        scale_in_place_with(&mut data, 2, 4, &|_| {
            let ok = meeting.arrive_and_wait(4);
            met.lock().unwrap().push(ok);
        });
        assert_eq!(
            met.into_inner().unwrap(),
            [true; 4],
            "a chunk waited 20 s for the others: start every thread before \
             joining any, and don't make them take turns on a lock"
        );
        assert_eq!(data, times(&one_to(1000), 2));
    }

    #[test]
    #[should_panic]
    fn a_panic_in_a_worker_reaches_the_caller() {
        // `u64::MAX * 2` overflows, and tests run in debug mode, so the
        // thread that scales it panics. The caller must not return normally
        // as if every element had been scaled.
        let mut data = vec![1, 2, 3, u64::MAX];
        scale_in_place(&mut data, 2, 2);
    }
}
