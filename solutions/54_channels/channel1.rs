// Module 4 · Channels — part 1: backpressure with a bounded `sync_channel` and `try_send`.
//
// `mpsc::channel()` is UNBOUNDED: `send` never blocks, and it fails only when
// the receiver is gone. When the producers are faster than the consumer, the
// queue just grows. Memory climbs until the process is killed, and every new
// item waits behind all the old ones, so latency climbs with it. Nothing ever
// tells the producers to slow down. An unbounded queue does not remove
// overload, it hides it.
//
// The fix is BACKPRESSURE, a bounded queue that pushes back when it is full.
// `52_condvar/condvar2` built one by hand from a `Mutex` and two `Condvar`s.
// std ships one as a channel: `mpsc::sync_channel(n)` buffers at most `n`
// items and offers both ways to push back:
//
//   - `SyncSender::send` BLOCKS while the buffer is full, so the producer
//     slows down to the consumer's pace. Right for a batch job that can
//     afford to wait.
//   - `SyncSender::try_send` NEVER blocks. It returns
//     `Err(TrySendError::Full(item))` when the buffer is full and
//     `Err(TrySendError::Disconnected(item))` when the receiver is gone. Both
//     hand the item back, and the caller decides: retry later, spill it
//     somewhere, or drop it and count the loss ("load shedding"). Right for
//     a request handler, which should answer "busy, try again" (HTTP 503 or
//     429) at once instead of piling up requests it cannot serve.
//
// `sync_channel(0)` has no buffer at all: it is a RENDEZVOUS. `send` waits
// until a receiver takes the item, and `try_send` succeeds only if a receiver
// is ALREADY waiting in `recv`.
//
// Two rules about hanging up hold for every kind of channel. When all the
// senders are gone, the receiver still gets the items already queued, and
// sees the disconnect only after the last one (so `for item in rx` ends once
// the queue is drained). When the receiver is gone, every send fails at once,
// even into a full buffer: a closed queue is not a busy one, and no retry
// will ever succeed.
//
// Since Rust 1.67, std's `mpsc` is a port of the `crossbeam-channel` crate.
// Tokio's `mpsc::channel(n)` has the same shape: `send(..).await` waits for
// space, `try_send` fails with `TrySendError::Full` or `Closed`, and the
// unbounded version is a separate, deliberately named `unbounded_channel`.
//
// Interviewers ask: bounded or unbounded, and why? What do you do when the
// queue is full: block, drop the new item, drop the oldest one, or grow?
// (`sync_channel` gives you the first two, and `channel()` is the last one.
// Dropping the oldest needs a ring buffer of your own.) And how would you
// notice overload in production? Count the refusals.

use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

/// Why an offer was refused. Both variants hand the item back.
#[derive(Debug, PartialEq, Eq)]
enum OfferError<T> {
    /// The buffer is full, so the consumer is behind. Retry later, or shed
    /// the item.
    Busy(T),
    /// The consumer is gone. No retry will ever succeed.
    Closed(T),
}

/// The front door of a work queue. Producers `offer` items, and one consumer
/// drains the `Receiver` that `Ingest::bounded` returns.
struct Ingest<T> {
    // The sending half of a bounded channel: it knows the buffer's capacity.
    tx: SyncSender<T>,
}

// Every producer gets its own clone, and all clones feed the same buffer.
// (`#[derive(Clone)]` would add a `T: Clone` bound, but cloning the sending
// half never clones an item.)
impl<T> Clone for Ingest<T> {
    fn clone(&self) -> Self {
        Ingest {
            tx: self.tx.clone(),
        }
    }
}

impl<T> Ingest<T> {
    /// Creates a queue that buffers at most `capacity` items. With
    /// `capacity == 0` there is no buffer: an offer gets through only while
    /// the consumer is already waiting in `recv`.
    fn bounded(capacity: usize) -> (Ingest<T>, Receiver<T>) {
        // `sync_channel` allocates a buffer of exactly `capacity` slots, and 0
        // makes it a rendezvous. Clones of a `SyncSender` share that buffer.
        let (tx, rx) = mpsc::sync_channel(capacity);
        (Ingest { tx }, rx)
    }

    /// Offers `item` without ever blocking. Returns `Ok(())` if it was
    /// queued, `Err(Busy(item))` if the buffer is full and `Err(Closed(item))`
    /// if the consumer is gone.
    fn offer(&self, item: T) -> Result<(), OfferError<T>> {
        // `try_send` never blocks. Both of its errors carry the item back,
        // so it is moved into our error, never copied. A receiver that is
        // gone wins over a full buffer: std reports `Disconnected` first.
        self.tx.try_send(item).map_err(|err| match err {
            TrySendError::Full(item) => OfferError::Busy(item),
            TrySendError::Disconnected(item) => OfferError::Closed(item),
        })
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;
    use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
    use std::thread;
    use std::time::{Duration, Instant};

    const WATCHDOG: Duration = Duration::from_secs(10);

    // Runs a test body on a thread of its own, so that an `offer` that blocks
    // fails the test instead of hanging it: rustlings has no test timeout.
    fn without_blocking(body: impl FnOnce() + Send + 'static) {
        let (done_tx, done_rx) = mpsc::channel();
        // Named like the test's own thread, so a panic message names the test.
        let name = thread::current().name().unwrap_or("test").to_owned();
        let worker = thread::Builder::new()
            .name(name)
            .spawn(move || {
                body();
                let _ = done_tx.send(());
            })
            .unwrap();
        match done_rx.recv_timeout(WATCHDOG) {
            Ok(()) => {}
            // The body panicked, and `done_tx` was dropped while it unwound:
            // fail this test with the body's own panic.
            Err(RecvTimeoutError::Disconnected) => {
                if let Err(payload) = worker.join() {
                    panic::resume_unwind(payload);
                }
            }
            Err(RecvTimeoutError::Timeout) => panic!(
                "`offer` blocked for {WATCHDOG:?}: it must answer at once, \
                 with `Busy` when the buffer is full, instead of waiting for space"
            ),
        }
    }

    // An item that cannot be cloned, so an item that comes back inside an
    // error must be the very one that was offered.
    #[derive(Debug, PartialEq, Eq)]
    struct Job(String);

    #[test]
    fn the_third_offer_at_capacity_two_is_busy() {
        without_blocking(|| {
            let (ingest, rx) = Ingest::bounded(2);
            assert_eq!(ingest.offer(1), Ok(()));
            assert_eq!(ingest.offer(2), Ok(()));
            assert_eq!(
                ingest.offer(3),
                Err(OfferError::Busy(3)),
                "two items fill a buffer of capacity 2: the third offer must be refused"
            );
            // The refused item was not queued, and taking one item frees one
            // slot.
            assert_eq!(rx.try_recv(), Ok(1));
            assert_eq!(
                ingest.offer(4),
                Ok(()),
                "the consumer took an item, so one slot is free again"
            );
            assert_eq!(ingest.offer(5), Err(OfferError::Busy(5)));
            assert_eq!(rx.try_recv(), Ok(2));
            assert_eq!(rx.try_recv(), Ok(4));
            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        });
    }

    #[test]
    fn a_busy_offer_hands_back_the_same_item() {
        without_blocking(|| {
            let (ingest, rx) = Ingest::bounded(1);
            assert_eq!(ingest.offer(Job("first".to_owned())), Ok(()));
            let job = Job("second".to_owned());
            let text = job.0.as_ptr();
            match ingest.offer(job) {
                Err(OfferError::Busy(back)) => assert_eq!(
                    back.0.as_ptr(),
                    text,
                    "`Busy` must carry the offered job itself, not a copy"
                ),
                other => panic!("expected `Busy` from a full buffer, got {other:?}"),
            }
            assert_eq!(rx.try_recv(), Ok(Job("first".to_owned())));
        });
    }

    #[test]
    fn capacity_zero_is_a_rendezvous() {
        without_blocking(|| {
            let (ingest, rx) = Ingest::bounded(0);
            assert_eq!(
                ingest.offer(1),
                Err(OfferError::Busy(1)),
                "capacity 0 has no buffer: with no consumer waiting in `recv`, \
                 the offer must be refused"
            );
            let consumer = thread::spawn(move || rx.recv());
            // As soon as the consumer waits in `recv`, an offer hands the item
            // straight to it. Nobody can tell when that is, so retry, the way
            // a real producer would, and back off a little between tries.
            let deadline = Instant::now() + WATCHDOG / 2;
            let mut item = 2;
            loop {
                match ingest.offer(item) {
                    Ok(()) => break,
                    Err(OfferError::Busy(back)) => {
                        assert!(
                            Instant::now() < deadline,
                            "no offer ever reached the consumer waiting in `recv`"
                        );
                        item = back;
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(OfferError::Closed(_)) => {
                        panic!("the consumer is still waiting, so the queue is not closed")
                    }
                }
            }
            assert_eq!(consumer.join().unwrap(), Ok(2));
            assert_eq!(
                ingest.offer(3),
                Err(OfferError::Closed(3)),
                "the consumer thread has ended, and its `Receiver` with it: the queue is closed"
            );
        });
    }

    #[test]
    fn a_dropped_consumer_means_closed_even_when_full() {
        without_blocking(|| {
            let (ingest, rx) = Ingest::bounded(1);
            assert_eq!(ingest.offer(Job("queued".to_owned())), Ok(()));
            drop(rx);
            // The buffer is full AND the consumer is gone. Report `Closed`:
            // waiting for space can never help.
            let job = Job("late".to_owned());
            let text = job.0.as_ptr();
            match ingest.offer(job) {
                Err(OfferError::Closed(back)) => assert_eq!(
                    back.0.as_ptr(),
                    text,
                    "`Closed` must carry the offered job itself, not a copy"
                ),
                other => panic!("expected `Closed` once the consumer is gone, got {other:?}"),
            }
        });
    }

    #[test]
    fn clones_share_one_buffer_that_drains_after_hang_up() {
        without_blocking(|| {
            let (first, rx) = Ingest::bounded(3);
            let second = first.clone();
            assert_eq!(first.offer(1), Ok(()));
            assert_eq!(second.offer(2), Ok(()));
            assert_eq!(first.offer(3), Ok(()));
            assert_eq!(
                second.offer(4),
                Err(OfferError::Busy(4)),
                "all clones of an `Ingest` share one buffer of 3"
            );
            drop(first);
            // One producer is left, so the queue is still open.
            assert_eq!(rx.try_recv(), Ok(1));
            drop(second);
            // Hanging up does not throw away what is already queued...
            assert_eq!(rx.try_recv(), Ok(2));
            assert_eq!(rx.try_recv(), Ok(3));
            // ...and only then does the consumer see the disconnect (this is
            // where `for item in rx` would end).
            assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
        });
    }

    #[test]
    fn a_stalled_consumer_caps_the_queue_and_sheds_the_rest() {
        without_blocking(|| {
            // A burst of 100 offers while the consumer reads nothing.
            let (ingest, rx) = Ingest::bounded(4);
            let (mut accepted, mut shed) = (Vec::new(), Vec::new());
            for n in 0..100 {
                match ingest.offer(n) {
                    Ok(()) => accepted.push(n),
                    Err(OfferError::Busy(n)) => shed.push(n),
                    Err(OfferError::Closed(n)) => panic!("offer {n}: the consumer is still alive"),
                }
            }
            assert_eq!(
                (accepted.len(), shed.len()),
                (4, 96),
                "(accepted, shed): a buffer of 4 must take the first 4 offers and refuse the other 96"
            );
            assert_eq!(accepted, [0, 1, 2, 3]);
            assert!(
                shed.iter().copied().eq(4..100),
                "the refused offers must be 4..100"
            );
            drop(ingest);
            assert_eq!(rx.iter().collect::<Vec<_>>(), [0, 1, 2, 3]);
        });
    }
}
