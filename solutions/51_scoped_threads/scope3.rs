// Module 4 · Scoped threads — part 3: a `Barrier` between the two phases of a parallel prefix sum.
//
// Parts 1 and 2 split the work into chunks that never need each other. Some
// algorithms are not that polite. The classic one is the prefix sum (a
// "scan"): `out[i] = in[0] + in[1] + ... + in[i]`. Every output depends on
// everything before it, so it looks hopelessly sequential. The standard
// two-phase trick parallelizes it anyway:
//
//   1. Each thread scans its own chunk as if it were the whole input, and
//      publishes the chunk's total (its last value).
//   2. Each thread adds an offset to every element of its chunk: the sum of
//      the totals of all EARLIER chunks.
//
//     input      [ 1  2  3 |  4  5  6 |  7  8 ]
//     phase 1    [ 1  3  6 |  4  9 15 |  7 15 ]    totals: 6, 15, 15
//     offset        +0         +6         +21
//     output     [ 1  3  6 | 10 15 21 | 28 36 ]
//
// Phase 2 of chunk 2 reads the totals of chunks 0 and 1, so it must not
// start before THEY have finished phase 1. The borrow checker cannot help
// with that. The totals are atomics, so reading one early is not a data race
// (undefined behavior, which safe Rust rules out); it is a race condition
// (a wrong answer, which it does not). Getting the order right is
// synchronization, and "nobody continues until everybody got here" is
// exactly what `std::sync::Barrier` does:
//
//   - `Barrier::new(k)` is a barrier for `k` threads. `wait()` blocks until
//     `k` threads have called it, then releases all of them at once. Exactly
//     one of them gets a `BarrierWaitResult` whose `is_leader()` is `true`,
//     which is handy for one-off work between two phases.
//   - It is reusable: the same `k` threads can `wait` again at the next phase
//     boundary.
//   - `k` must be the number of threads that will really call `wait`. With
//     one too many, every thread blocks forever: there is no timeout and no
//     error.
//   - std builds it from a `Mutex` and a `Condvar`, so everything a thread
//     wrote before its `wait` is visible to every thread after its `wait`
//     returns. That is why the totals may be read with `Ordering::Relaxed`
//     (orderings are the subject of `36_atomics`, later in the course).
//
// Why not two `thread::scope` calls, one per phase? The end of a scope IS a
// barrier: every thread is joined there. For a single phase boundary that is
// a perfectly good answer. A `Barrier` keeps the SAME threads, with their
// chunks and their local state, running across phases, which matters when
// there are many phases (iterative solvers, simulations, a game-of-life
// step) and spawning threads over and over is not free. That is the version
// to write here.
//
// A missing barrier shows up only when some chunk happens to publish late,
// so hoping the tests catch it by chance would be useless. `prefix_sum_with`
// has a test seam instead: worker `i` calls `before_publish(i)` between its
// phase-1 scan and publishing its total, and the tests use it to make one
// chunk slow. With a barrier, a slow chunk only delays the others. Without
// one, they read a total that has not been published yet.
//
// How interviewers probe this: "Parallelize a prefix sum.", "What does a
// barrier give you that a mutex doesn't?", "What happens if the count is
// wrong?", "Barrier or join-and-respawn?".

use std::sync::Barrier;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

// Replaces every element of `data` with the sum of itself and all elements
// before it, using up to `n` threads.
fn prefix_sum(data: &mut [u64], n: usize) {
    prefix_sum_with(data, n, &|_| {});
}

// `prefix_sum` with a test seam: worker `i` calls `before_publish(i)` after
// scanning chunk `i` and before publishing its total. Production callers pass
// a no-op.
fn prefix_sum_with(data: &mut [u64], n: usize, before_publish: &(dyn Fn(usize) + Sync)) {
    // The split from part 2: at most `n` chunks (one if `n` is 0).
    let size = data.len().div_ceil(n.max(1)).max(1);
    let chunk_count = data.len().div_ceil(size);
    // `totals[i]` is the sum of chunk `i`, published by worker `i` at the end
    // of phase 1. Every worker writes its own slot through a shared `&`,
    // which is why they are atomics.
    let totals: Vec<AtomicU64> = (0..chunk_count).map(|_| AtomicU64::new(0)).collect();
    let totals = &totals;
    // One barrier for exactly the workers that `chunks_mut` will produce:
    // `chunk_count`, not `n`, which can be larger (9 elements on 4 threads
    // make only 3 chunks). A count that is too high leaves every worker
    // blocked in `wait` forever. The `&` lets each `move` closure take a
    // copy of the reference, so they all share this one barrier.
    let barrier = Barrier::new(chunk_count);
    let barrier = &barrier;

    thread::scope(|s| {
        for (i, chunk) in data.chunks_mut(size).enumerate() {
            s.spawn(move || {
                // Phase 1: scan this chunk on its own, then publish its total.
                let mut running = 0;
                for x in chunk.iter_mut() {
                    running += *x;
                    *x = running;
                }
                before_publish(i);
                totals[i].store(running, Ordering::Relaxed);

                // Phase 2. `wait` returns only after all `chunk_count` workers
                // have called it, i.e. after every total has been stored, and
                // the barrier's internal mutex makes those stores visible
                // here, so `Relaxed` loads are enough. Chunk `i`'s offset is
                // the sum of the totals of chunks `0..i` (none for chunk 0).
                barrier.wait();
                let offset: u64 = totals[..i].iter().map(|t| t.load(Ordering::Relaxed)).sum();
                for x in chunk.iter_mut() {
                    *x += offset;
                }
            });
        }
    });
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    // The sequential reference: an inclusive running sum.
    fn scan(input: &[u64]) -> Vec<u64> {
        input
            .iter()
            .scan(0, |sum, x| {
                *sum += x;
                Some(*sum)
            })
            .collect()
    }

    fn one_to(len: u64) -> Vec<u64> {
        (1..=len).collect()
    }

    // How many chunks (and so workers) the given split makes.
    fn chunks_for(len: usize, n: usize) -> usize {
        len.div_ceil(len.div_ceil(n.max(1)).max(1))
    }

    // Runs `prefix_sum_with(input, n)` on a helper thread and waits at most
    // 20 s for the result. If `slow` names a chunk, that worker's
    // `before_publish` first waits (up to 5 s) until every worker has
    // finished its scan, and then takes another 100 ms before it publishes.
    // A correct solution just waits for it at the barrier; one without a
    // barrier reads a total that is still 0.
    //
    // The helper is a plain `thread::spawn`, the one place in this module
    // that needs it: a watchdog must be able to walk away from a stuck
    // thread, and a scoped thread cannot be abandoned (`scope` would wait for
    // it forever). That is also why everything it uses is owned or in an
    // `Arc`.
    fn run(input: &[u64], n: usize, slow: Option<usize>) -> Vec<u64> {
        let mut data = input.to_vec();
        let workers = chunks_for(data.len(), n);
        let scanned = Arc::new((Mutex::new(0), Condvar::new()));
        let (done, result) = mpsc::channel();
        thread::spawn(move || {
            let hook = |i: usize| {
                let (count, changed) = &*scanned;
                let mut count = count.lock().unwrap();
                *count += 1;
                changed.notify_all();
                if slow == Some(i) {
                    let everybody_scanned = changed
                        .wait_timeout_while(count, Duration::from_secs(5), |c| *c < workers)
                        .unwrap();
                    drop(everybody_scanned);
                    thread::sleep(Duration::from_millis(100));
                }
            };
            prefix_sum_with(&mut data, n, &hook);
            let _ = done.send(data);
        });
        match result.recv_timeout(Duration::from_secs(20)) {
            Ok(out) => out,
            Err(RecvTimeoutError::Timeout) => panic!(
                "prefix_sum(len = {}, n = {n}) did not finish within 20 s: \
                 some worker is stuck in `wait`. Is the barrier sized for the \
                 {workers} worker(s) that were actually spawned, and do they \
                 all share the same one?",
                input.len()
            ),
            Err(RecvTimeoutError::Disconnected) => {
                panic!("prefix_sum panicked (see the message above)")
            }
        }
    }

    // What the output looks like when phase 2 never happens: every chunk
    // scanned on its own, starting from 0.
    fn without_offsets(input: &[u64], n: usize) -> Vec<u64> {
        let size = input.len().div_ceil(n.max(1)).max(1);
        input.chunks(size).flat_map(scan).collect()
    }

    // Runs the prefix sum and compares it with `scan`, element by element.
    // It reports the first wrong value and a likely cause, instead of
    // printing two vectors of 1000 numbers.
    fn check(input: &[u64], n: usize, slow: Option<usize>) {
        let out = run(input, n, slow);
        let expected = scan(input);
        assert_eq!(out.len(), expected.len(), "the length must not change");
        let Some(i) = (0..out.len()).find(|&i| out[i] != expected[i]) else {
            return;
        };
        let cause = if out == without_offsets(input, n) {
            String::from(
                "Every chunk after the first starts from 0: phase 2 is \
                 missing, or it read the totals before they were published.",
            )
        } else if let Some(k) = slow {
            format!(
                "This test makes chunk {k} slow to publish. If the tests \
                 without a slow chunk pass, a worker read a total before it \
                 was published: wait until every worker has published before \
                 reading any total."
            )
        } else {
            String::from(
                "Check the offsets: each one is the sum of the totals of the \
                 EARLIER chunks.",
            )
        };
        assert_eq!(
            out[i],
            expected[i],
            "len = {}, n = {n}, slow chunk = {slow:?}: first wrong value at \
             index {i}. {cause}",
            input.len()
        );
    }

    #[test]
    fn two_chunks_of_two() {
        check(&[1, 2, 3, 4], 2, None);
    }

    #[test]
    fn matches_a_sequential_scan() {
        for len in [1, 2, 7, 100, 1000] {
            for n in [1, 2, 3, 4, 8] {
                check(&one_to(len), n, None);
            }
        }
    }

    #[test]
    fn zeros_and_uneven_values() {
        // Empty and zero-sum chunks still pass their offsets along.
        check(&[5, 0, 0, 0, 0, 0, 0, 7], 4, None);
        check(&[0, 0, 0, 1_000_000, 1, 0, 0, 3], 3, None);
    }

    #[test]
    fn an_empty_slice_and_zero_threads_are_fine() {
        for n in [0, 1, 4] {
            assert_eq!(run(&[], n, None), Vec::<u64>::new());
        }
        check(&one_to(10), 0, None);
    }

    #[test]
    fn more_threads_requested_than_chunks_exist() {
        // 9 elements on 4 threads: chunks of 3, so only 3 workers. 5 elements
        // on 64 threads: 5 workers. 1000 elements on 64: chunks of 16, so 63.
        check(&one_to(9), 4, None);
        check(&one_to(5), 64, None);
        check(&one_to(1000), 64, None);
    }

    #[test]
    fn a_slow_first_chunk_only_delays_the_others() {
        check(&one_to(1000), 4, Some(0));
    }

    #[test]
    fn a_slow_middle_chunk_only_delays_the_others() {
        // 10 elements on 3 threads: chunks of 4, 4 and 2. Chunk 2 needs the
        // total of the slow chunk 1.
        check(&one_to(10), 3, Some(1));
    }
}
