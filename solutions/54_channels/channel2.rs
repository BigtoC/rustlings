// Module 4 · Channels — part 2: shutting a pipeline down by disconnection, from either end.
//
// A pipeline is a chain of threads joined by channels. Each stage reads its
// input with `for item in input { .. }`, does its bit of work and sends the
// result on. The interview question is how such a thing STOPS, and std's
// answer needs no extra message at all. `for item in rx` ends when `recv()`
// fails, and `recv()` fails only once the queue is empty AND every `Sender`
// of the channel has been dropped. So shutdown is a cascade of drops: the
// source runs out of input and drops its `Sender`; the next stage's loop
// ends, the stage returns and drops ITS `Sender`; and so on, down to the
// consumer. No "stop" flag, no sentinel value, no counting.
//
// That makes every `Sender` a promise that more may come. One stray clone
// keeps its channel open forever, and every stage behind it waits in `recv`
// for an item that never arrives. Nothing panics and no error is reported:
// the program just stops making progress. The usual culprit is a
// `tx.clone()` handed to a thread while the original `tx` stays alive in the
// function that spawned it. That is harmless while the function returns
// right away, and fatal once it goes on to read the results or to join the
// threads. Thread pools follow the same rule: to shut one down, first drop
// the `Sender` of the job queue, THEN join the workers. Join first, and each
// worker waits in `recv` for its next job while you wait for the worker.
//
// Shutdown can also start at the other end. When the consumer has seen
// enough and drops its `Receiver`, every later `send` to it fails with
// `Err(SendError(item))`. For a stage, that failure is not a bug but the
// signal to stop. The stage returns, which drops its own `Receiver`, and the
// failure travels upstream the same way until the source stops producing. A
// stage that `unwrap()`s its sends turns the consumer's "that is enough"
// into a panic in every stage. A stage that ignores the error keeps working
// for nobody.
//
// The links here are bounded (`sync_channel`, as in `channel1`), which helps
// twice. A fast stage cannot race ahead and pile up work that nobody will
// read, and a stage blocked in `send` on a full link is woken with an error
// the moment the receiving side goes away.
//
// Rustlings has no test timeout, so the tests run everything under a
// watchdog: `recv_timeout` on a result channel turns "hangs forever" into a
// failed test after 10 seconds.
//
// Interviewers ask: when does `for msg in rx` end? How does a pipeline shut
// down when the producer finishes, and when the consumer quits early? Why
// does a thread pool drop its `Sender` before it joins its workers?

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

/// Each link between two stages holds at most this many items, so no stage
/// can run far ahead of the next one.
const LINK_CAP: usize = 2;

/// The `batch` stage groups this many squares into one `Vec`.
const BATCH_SIZE: usize = 3;

/// What `run_pipeline` saw.
#[derive(Debug, PartialEq, Eq)]
struct Report {
    /// The batches the consumer kept, in order.
    batches: Vec<Vec<u64>>,
    /// The stages whose thread panicked instead of returning.
    panicked: Vec<&'static str>,
}

/// Stage 1: feeds `numbers` into the pipeline.
fn source(numbers: impl Iterator<Item = u64>, out: SyncSender<u64>) {
    // A failed send hands the number back: the stage downstream has hung up.
    // Return at once, which stops pulling from `numbers` and drops `out`.
    for n in numbers {
        if out.send(n).is_err() {
            return;
        }
    }
}

/// Stage 2: squares every number.
fn square(input: Receiver<u64>, out: SyncSender<u64>) {
    // Returning drops `input`, so the stage upstream sees the hang-up on
    // its next send: the stop signal travels all the way back to the source.
    for n in input {
        if out.send(n * n).is_err() {
            return;
        }
    }
}

/// Stage 3: groups the squares into batches of `BATCH_SIZE`. When its input
/// ends, it sends the last, partial batch.
fn batch(input: Receiver<u64>, out: SyncSender<Vec<u64>>) {
    let mut current = Vec::with_capacity(BATCH_SIZE);
    for square in input {
        current.push(square);
        if current.len() == BATCH_SIZE {
            let full = std::mem::replace(&mut current, Vec::with_capacity(BATCH_SIZE));
            // A failed send means nobody reads the batches any more: stop
            // reading squares. (`break` works too: `current` was just
            // emptied, so the flush below has nothing to send.)
            if out.send(full).is_err() {
                return;
            }
        }
    }
    // The loop ends only once every Sender of `input` is gone: nothing more
    // will arrive, so the partial batch is complete.
    // The flush can fail too, if the consumer hung up just now. There is
    // nothing left to stop, so the error is simply ignored.
    if !current.is_empty() {
        let _ = out.send(current);
    }
}

/// Starts one stage on a thread named after it (panic messages show the
/// name).
fn spawn_stage(
    name: &'static str,
    body: impl FnOnce() + Send + 'static,
) -> (&'static str, JoinHandle<()>) {
    let handle = thread::Builder::new()
        .name(name.to_owned())
        .spawn(body)
        .expect("failed to spawn a stage thread");
    (name, handle)
}

/// Runs `numbers` through `source -> square -> batch`, one thread per stage.
/// The calling thread is the consumer: it keeps the first `limit` batches and
/// then hangs up. Returns once every stage thread has ended.
fn run_pipeline(numbers: impl Iterator<Item = u64> + Send + 'static, limit: usize) -> Report {
    let (numbers_tx, numbers_rx) = mpsc::sync_channel(LINK_CAP);
    let (squares_tx, squares_rx) = mpsc::sync_channel(LINK_CAP);
    let (batches_tx, batches_rx) = mpsc::sync_channel(LINK_CAP);

    // Each stage gets the ORIGINAL Sender of its output, moved into its
    // thread: no clone stays behind in this function. When a stage returns,
    // its Sender is dropped, and that channel's last Sender is gone, so the
    // next stage's `for` loop (and finally the consumer's `collect()`) ends.
    let stages = [
        spawn_stage("source", move || source(numbers, numbers_tx)),
        spawn_stage("square", move || square(numbers_rx, squares_tx)),
        spawn_stage("batch", move || batch(squares_rx, batches_tx)),
    ];

    // The consumer. `into_iter()` moves the Receiver into the iterator, and
    // `take(limit)` stops after `limit` batches. `collect()` takes the
    // iterator by value, so the Receiver is dropped as soon as the batches
    // are collected: from then on, every send to it fails.
    let batches = batches_rx.into_iter().take(limit).collect();

    // `join` waits for a thread, and returns `Err` if the thread panicked.
    let panicked = stages
        .into_iter()
        .filter_map(|(name, stage)| stage.join().is_err().then_some(name))
        .collect();
    Report { batches, panicked }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::RangeInclusive;
    use std::panic;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError};
    use std::time::Duration;

    const WATCHDOG: Duration = Duration::from_secs(10);

    // Runs `f` on a thread of its own and returns its result, or fails the
    // test with `hung` if `f` has not returned within `WATCHDOG`. A pipeline
    // that never shuts down would otherwise hang the test forever.
    fn within<R: Send + 'static>(hung: &str, f: impl FnOnce() -> R + Send + 'static) -> R {
        let (done_tx, done_rx) = mpsc::channel();
        // Named like the test's own thread, so a panic message names the test.
        let name = thread::current().name().unwrap_or("test").to_owned();
        let worker = thread::Builder::new()
            .name(name)
            .spawn(move || {
                let _ = done_tx.send(f());
            })
            .unwrap();
        match done_rx.recv_timeout(WATCHDOG) {
            Ok(result) => result,
            // `f` panicked, and `done_tx` was dropped while it unwound: fail
            // this test with `f`'s own panic.
            Err(RecvTimeoutError::Disconnected) => match worker.join() {
                Err(payload) => panic::resume_unwind(payload),
                Ok(()) => unreachable!("the worker sends its result before it ends"),
            },
            Err(RecvTimeoutError::Timeout) => panic!("{hung} (watchdog: {WATCHDOG:?})"),
        }
    }

    // What the tests learn about the numbers that the source stage reads.
    #[derive(Default)]
    struct Probe {
        pulled: AtomicUsize,
        dropped: AtomicBool,
    }

    // The input `1..=last`. It counts the numbers pulled out of it, and
    // records when it is dropped, which happens when the source stage ends.
    struct Numbers {
        rest: RangeInclusive<u64>,
        probe: Arc<Probe>,
    }

    impl Iterator for Numbers {
        type Item = u64;

        fn next(&mut self) -> Option<u64> {
            let n = self.rest.next()?;
            self.probe.pulled.fetch_add(1, Ordering::SeqCst);
            Some(n)
        }
    }

    impl Drop for Numbers {
        fn drop(&mut self) {
            self.probe.dropped.store(true, Ordering::SeqCst);
        }
    }

    fn numbers(last: u64) -> (Numbers, Arc<Probe>) {
        let probe = Arc::new(Probe::default());
        let numbers = Numbers {
            rest: 1..=last,
            probe: Arc::clone(&probe),
        };
        (numbers, probe)
    }

    const NEVER_SHUT_DOWN: &str = "pipeline never shut down: a Sender is still alive";

    fn assert_all_stages_returned(report: &Report, probe: &Probe) {
        assert!(
            report.panicked.is_empty(),
            "these stages panicked: {:?}",
            report.panicked
        );
        assert!(
            probe.dropped.load(Ordering::SeqCst),
            "`run_pipeline` returned before the source stage ended: join every stage"
        );
    }

    #[test]
    fn squares_come_out_in_batches_and_the_pipeline_shuts_down() {
        let (input, probe) = numbers(7);
        let report = within(NEVER_SHUT_DOWN, move || run_pipeline(input, usize::MAX));
        assert_eq!(
            report.batches,
            [vec![1, 4, 9], vec![16, 25, 36], vec![49]],
            "every square, in order, with the partial last batch flushed at shutdown"
        );
        assert_all_stages_returned(&report, &probe);
        assert_eq!(probe.pulled.load(Ordering::SeqCst), 7);
    }

    #[test]
    fn an_exact_multiple_leaves_no_empty_batch() {
        let (input, probe) = numbers(6);
        let report = within(NEVER_SHUT_DOWN, move || run_pipeline(input, usize::MAX));
        assert_eq!(report.batches, [vec![1, 4, 9], vec![16, 25, 36]]);
        assert_all_stages_returned(&report, &probe);
    }

    #[test]
    fn an_empty_input_shuts_down_at_once() {
        let (input, probe) = numbers(0);
        let report = within(NEVER_SHUT_DOWN, move || run_pipeline(input, usize::MAX));
        assert_eq!(report.batches, Vec::<Vec<u64>>::new());
        assert_all_stages_returned(&report, &probe);
    }

    #[test]
    fn a_consumer_that_stops_early_shuts_every_stage_down() {
        // The consumer wants 2 batches (6 squares) out of 10_000.
        let (input, probe) = numbers(10_000);
        let report = within(
            "pipeline never shut down after the consumer hung up: \
             a Sender is still alive, or a stage ignored a failed send",
            move || run_pipeline(input, 2),
        );
        assert_eq!(report.batches, [vec![1, 4, 9], vec![16, 25, 36]]);
        assert!(
            report.panicked.is_empty(),
            "these stages panicked after the consumer hung up: {:?}. A failed \
             send means \"stop\": return instead of unwrapping",
            report.panicked
        );
        assert_all_stages_returned(&report, &probe);
        // The bounded links hold only a few numbers, so a pipeline that stops
        // at the first failed send pulls a few dozen at most.
        let pulled = probe.pulled.load(Ordering::SeqCst);
        assert!(
            pulled < 100,
            "the source pulled {pulled} numbers for a consumer that wanted 6 \
             squares: a stage that ignores a failed send keeps the pipeline \
             working for nobody"
        );
    }

    #[test]
    fn a_limit_of_zero_hangs_up_before_reading() {
        let (input, probe) = numbers(10_000);
        let report = within(
            "pipeline never shut down after the consumer hung up",
            move || run_pipeline(input, 0),
        );
        assert_eq!(report.batches, Vec::<Vec<u64>>::new());
        assert_all_stages_returned(&report, &probe);
    }

    #[test]
    fn square_returns_when_its_downstream_hangs_up() {
        within(
            "`square` kept waiting for input after its downstream hung up",
            || {
                let (tx, input) = mpsc::sync_channel(LINK_CAP);
                let (out, results) = mpsc::sync_channel(LINK_CAP);
                drop(results);
                tx.send(3).unwrap();
                // `tx` stays alive, so only the failed send can end the stage.
                let (_, stage) = spawn_stage("square", move || square(input, out));
                assert!(
                    stage.join().is_ok(),
                    "`square` panicked when its send failed: return instead"
                );
                // The stage dropped its Receiver on the way out, so the
                // hang-up travels upstream: our next send fails.
                assert_eq!(tx.send(4), Err(SendError(4)));
            },
        );
    }

    #[test]
    fn source_stops_pulling_when_its_downstream_hangs_up() {
        within(
            "`source` never returned after its downstream hung up",
            || {
                let (input, probe) = numbers(1_000);
                let (out, results) = mpsc::sync_channel(LINK_CAP);
                drop(results);
                let (_, stage) = spawn_stage("source", move || source(input, out));
                assert!(
                    stage.join().is_ok(),
                    "`source` panicked when its send failed: return instead"
                );
                assert_eq!(
                    probe.pulled.load(Ordering::SeqCst),
                    1,
                    "`source` kept pulling numbers after its first send failed"
                );
                assert!(probe.dropped.load(Ordering::SeqCst));
            },
        );
    }

    #[test]
    fn batch_does_not_panic_flushing_into_a_closed_link() {
        within("`batch` never returned after its input ended", || {
            let (tx, input) = mpsc::sync_channel(LINK_CAP);
            let (out, results) = mpsc::sync_channel(LINK_CAP);
            drop(results);
            // Two squares make only a partial batch, so the loop ends on the
            // disconnect and the flush is the first send, and it fails.
            tx.send(1).unwrap();
            tx.send(4).unwrap();
            drop(tx);
            let (_, stage) = spawn_stage("batch", move || batch(input, out));
            assert!(
                stage.join().is_ok(),
                "`batch` panicked flushing its last batch into a link that \
                 nobody reads: return instead"
            );
        });
    }

    #[test]
    fn drop_the_sender_first_then_join_the_stage() {
        // How a thread pool shuts down, with `square` as its only worker:
        // close the job queue by dropping its Sender, and only then join.
        within(
            "joining `square` after dropping the only Sender of its input never returned",
            || {
                let (jobs, input) = mpsc::sync_channel(LINK_CAP);
                let (out, results) = mpsc::sync_channel(LINK_CAP);
                let (_, worker) = spawn_stage("square", move || square(input, out));
                jobs.send(5).unwrap();
                assert_eq!(results.recv(), Ok(25));
                // While we hold a Sender, `for n in input` cannot end, so the
                // worker cannot finish: joining it now would wait forever.
                assert!(
                    !worker.is_finished(),
                    "`square` returned while its input was still open"
                );
                drop(jobs);
                assert!(worker.join().is_ok(), "`square` panicked");
                // On its way out, the worker dropped its own Sender, so the
                // disconnect travels on downstream.
                assert_eq!(results.recv(), Err(RecvError));
            },
        );
    }
}
