// Module 3 · Async combinators — part 3: cancel safety, a partial line lost to a `select` timeout.
//
// `select1` showed that the loser of a race is dropped, and that dropping a
// future cancels it at the `.await` where it last stopped. Now look at what
// the loser had done by then. Its side effects stay done (bytes it took out
// of a socket are out of the socket), but whatever it kept in its OWN local
// variables is dropped with it. A future is CANCEL SAFE if dropping it at any
// of its `.await`s and then starting the same operation again loses nothing.
// tokio's rule of thumb: look at every `.await` in the function; if it
// behaves correctly even if it is restarted while waiting there, it is cancel
// safe.
//
// The classic bug is a `select!` loop with a timer:
//
//     loop {
//         tokio::select! {
//             line = reader.read_line(&mut buf) => handle(line),
//             _ = tokio::time::sleep(IDLE) => send_heartbeat(),
//         }
//     }
//
// Every time the timer wins, the half-finished `read_line` future is dropped
// and the next iteration starts a NEW one. tokio documents `read_line` as not
// cancel safe: "some data may have been partially read, and this data is
// lost". Its `select!` docs list `read_exact`, `read_to_end`, `read_to_string`
// and `write_all` as not cancel safe either, and even `Mutex::lock`, which
// loses its place in the lock's fair queue. `mpsc::Receiver::recv`,
// `AsyncReadExt::read`, `Lines::next_line` and `StreamExt::next` ARE cancel
// safe: their progress lives in the channel, reader or stream they borrow,
// not in the future.
//
// Here `LineReader::read_line` pulls bytes from a `Source` that goes
// `Pending` in the middle of a line (think TCP segments trickling in), and it
// collects the partial line in a LOCAL `Vec`. `read_lines_with_heartbeat`
// races it against a `Timeout` in a loop. When the timeout wins while "hel"
// of "hello\n" sits in that local buffer, the bytes are dropped with the
// future, and the next round reads "lo".
//
// There are two standard fixes, and interviewers want to hear both:
//
//   1. Make the operation cancel safe: keep its progress somewhere that
//      outlives the future, such as the reader it borrows. A restarted
//      `read_line` then continues where the canceled one stopped. tokio's
//      `Lines::next_line` and tokio-util's `FramedRead` work this way, and
//      so does `read_until`, which appends every byte to the caller's `buf`
//      at once. tokio's `read_line` keeps the bytes inside its future until
//      the line is complete, because a `String` may hold only valid UTF-8,
//      and that is exactly why it is not cancel safe.
//   2. Don't cancel it: create the `read_line` future once, pin it outside
//      the loop, and race a `Pin<&mut _>` to it against a FRESH timer each
//      round. Dropping a reference cancels nothing, so the same future
//      resumes next round, and a new one is created only after it completed.
//
// (A `Timeout` counts polls instead of reading a clock, and every leaf wakes
// itself, so "time" is just the number of polls and the tests are
// deterministic. `58_leaf_futures/timer1` builds a real timer.)
//
// How interviewers probe this: "What does cancel safe mean?", "Why is
// `read_exact` not cancel safe inside a `select!` loop, and how do you fix
// it?", and "Is `mpsc::Receiver::recv` cancel safe? Is `Mutex::lock`?".

use std::cell::Cell;
use std::collections::VecDeque;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

// ---- Given: the race from `select1` ---------------------------------------

#[derive(Debug, PartialEq)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

// Polls `a` first and `b` only while `a` is `Pending`; the loser is dropped
// on the poll that decides the race.
struct Select<A, B> {
    a: Option<A>,
    b: Option<B>,
}

fn select<A, B>(a: A, b: B) -> Select<A, B> {
    Select {
        a: Some(a),
        b: Some(b),
    }
}

impl<A: Future + Unpin, B: Future + Unpin> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let a = this.a.as_mut().expect("`Select` polled after completion");
        if let Poll::Ready(output) = Pin::new(a).poll(cx) {
            this.a = None;
            this.b = None;
            return Poll::Ready(Either::Left(output));
        }
        let b = this.b.as_mut().expect("`Select` polled after completion");
        if let Poll::Ready(output) = Pin::new(b).poll(cx) {
            this.a = None;
            this.b = None;
            return Poll::Ready(Either::Right(output));
        }
        Poll::Pending
    }
}

// ---- Given: a timer and a byte source --------------------------------------

// Stands in for `tokio::time::sleep`: it returns `Pending` `polls_left` times
// and then `Ready(())`, so it fires on its (polls_left + 1)-th poll.
struct Timeout {
    polls_left: u32,
}

thread_local! {
    // How many `Timeout`s have fired on this thread. Only a guard for the
    // tests: a loop in which the timeout wins every round, and the line is
    // never polled, would otherwise spin forever without returning `Pending`.
    static TIMEOUTS_FIRED: Cell<u32> = const { Cell::new(0) };
}

impl Timeout {
    fn after(polls: u32) -> Self {
        Timeout { polls_left: polls }
    }
}

impl Future for Timeout {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.polls_left == 0 {
            TIMEOUTS_FIRED.set(TIMEOUTS_FIRED.get() + 1);
            assert!(
                TIMEOUTS_FIRED.get() <= 1_000,
                "1000 timeouts fired: does the line still get polled every round?"
            );
            return Poll::Ready(());
        }
        self.polls_left -= 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

// Stands in for a socket. Its script is a byte string in which every `~` is
// one poll on which no byte has arrived yet: `poll_byte` returns `Pending`
// there. After the last byte it reports the end of the stream, `Ready(None)`,
// on every poll.
struct Source {
    script: VecDeque<u8>,
    // How often the end of the stream was reported. Only a guard for the
    // tests: a reader that keeps asking long after the end is stuck in a loop.
    polls_after_end: u32,
}

impl Source {
    fn new(script: &[u8]) -> Self {
        Source {
            script: script.iter().copied().collect(),
            polls_after_end: 0,
        }
    }

    fn poll_byte(&mut self, cx: &mut Context<'_>) -> Poll<Option<u8>> {
        match self.script.pop_front() {
            Some(b'~') => {
                // A real socket would register the waker with the OS; this
                // one has the next byte ready on the next poll.
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Some(byte) => Poll::Ready(Some(byte)),
            None => {
                self.polls_after_end += 1;
                assert!(
                    self.polls_after_end <= 100,
                    "the stream ended 100 polls ago: does a loop miss the end of the stream?"
                );
                Poll::Ready(None)
            }
        }
    }
}

// ---- The code under test ---------------------------------------------------

// Reads '\n'-terminated lines from a `Source`.
struct LineReader {
    source: Source,
    // The bytes of the line being read. They belong to the reader, not to a
    // `read_line` future, so they outlive a canceled `read_line`, and the
    // next call simply continues the same line.
    pending: Vec<u8>,
}

impl LineReader {
    fn new(source: Source) -> Self {
        LineReader {
            source,
            pending: Vec::new(),
        }
    }

    // Returns the next line without its '\n', or `None` at the end of the
    // stream. A last line without a '\n' is still returned. Lines are UTF-8,
    // but a character may arrive split across two polls, so the bytes are
    // collected first and decoded once the line is complete.
    //
    // Cancel safe: the only `.await` is the wait for the next byte, and no
    // part of the line lives in the future's own state across it. A byte
    // taken out of the source goes straight into `self.pending`, with no
    // `.await` in between, so dropping this future at that `.await` loses
    // nothing, and the next call picks up the same line.
    async fn read_line(&mut self) -> Option<String> {
        loop {
            match poll_fn(|cx| self.source.poll_byte(cx)).await {
                Some(b'\n') => break,
                Some(byte) => self.pending.push(byte),
                None if self.pending.is_empty() => return None,
                None => break,
            }
        }
        // The line is complete: hand its bytes out and leave an empty buffer
        // behind for the next line.
        let line = std::mem::take(&mut self.pending);
        Some(String::from_utf8(line).expect("the peer sent invalid UTF-8"))
    }
}

#[derive(Debug, PartialEq)]
struct Transcript {
    lines: Vec<String>,
    heartbeats: u32,
}

// Reads every line from `reader`. Each round waits for the next line, but for
// no more than `patience` idle polls: it races the line against a fresh
// `Timeout::after(patience)`. Whenever the timeout wins, it counts a
// heartbeat (a real server would ping the client or check for shutdown here)
// and starts the next round.
async fn read_lines_with_heartbeat(reader: &mut LineReader, patience: u32) -> Transcript {
    let mut transcript = Transcript {
        lines: Vec::new(),
        heartbeats: 0,
    };
    loop {
        // `read_line`'s future is `!Unpin`; pinning it on the stack gives
        // `Select` the `Unpin` handle it needs. The future itself lives in
        // this loop body: when the timeout wins, `Select` drops its handle,
        // and the future is dropped at the end of the round. `read_line` is
        // cancel safe now, so that costs nothing: the partial line waits in
        // `reader.pending`. (The other fix leaves `read_line` alone and keeps
        // ONE future alive across heartbeats instead: pin it before an inner
        // loop that races `line.as_mut()` against a fresh `Timeout` each
        // round, and break out of that loop only when the line is done.)
        let line = pin!(reader.read_line());
        match select(line, Timeout::after(patience)).await {
            Either::Left(Some(line)) => transcript.lines.push(line),
            Either::Left(None) => return transcript,
            Either::Right(()) => transcript.heartbeats += 1,
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Waker;

    // Polls `future` until it is `Ready`. Every leaf wakes itself before it
    // returns `Pending`, so re-polling at once is what a real executor would
    // do too. Still `Pending` after 10_000 polls means something is stuck.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        for _ in 0..10_000 {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
        panic!("not Ready after 10000 polls: a future is stuck in Pending");
    }

    fn transcript(script: &[u8], patience: u32) -> Transcript {
        let mut reader = LineReader::new(Source::new(script));
        block_on(read_lines_with_heartbeat(&mut reader, patience))
    }

    fn expected(lines: &[&str], heartbeats: u32) -> Transcript {
        Transcript {
            lines: lines.iter().map(|line| line.to_string()).collect(),
            heartbeats,
        }
    }

    #[test]
    fn a_line_split_by_a_heartbeat_arrives_whole() {
        // Patience 2: a round's timeout fires on the round's 3rd poll.
        // Round 1, poll 1: "hel" arrives, then the reader waits (`~`). Polls
        // 2 and 3: still waiting, and on poll 3 the timeout fires (heartbeat
        // 1). Round 2, poll 1: "lo\n" completes the line. "world" goes the
        // same way (heartbeat 2). Dropping the reader's progress with the
        // canceled future turns the lines into "lo" and "ld".
        assert_eq!(
            transcript(b"hel~~~lo\nwor~~~ld\n", 2),
            expected(&["hello", "world"], 2)
        );
    }

    #[test]
    fn a_line_survives_several_heartbeats() {
        // Two stalls of 3 polls inside one line, patience 2: two heartbeats
        // fire before the line is complete.
        assert_eq!(transcript(b"a~~~b~~~c\n", 2), expected(&["abc"], 2));
    }

    #[test]
    fn patience_zero_fires_on_every_stall() {
        // With patience 0 the timeout is `Ready` on its first poll, so every
        // round in which the reader has to wait ends in a heartbeat: 3 here.
        // The line still gets one poll per round because `Select` polls it
        // first; a loop that checked the timeout first would starve it.
        assert_eq!(transcript(b"hel~~~lo\n", 0), expected(&["hello"], 3));
    }

    #[test]
    fn without_a_heartbeat_there_is_nothing_to_lose() {
        // Patience 5 outlasts every stall, so no round is canceled. (This
        // passes even with the original `read_line`: the bug only shows up
        // under cancellation, which is why it survives code review and happy
        // path tests.)
        assert_eq!(
            transcript(b"hel~~~lo\nwor~~~ld\n", 5),
            expected(&["hello", "world"], 0)
        );
    }

    #[test]
    fn a_line_that_completes_on_the_timeouts_last_poll_wins() {
        // Patience 2, a stall of only 2 polls: on the round's 3rd poll the
        // line completes AND the timeout would fire. `Select` polls the line
        // first, so the line wins and no heartbeat is counted.
        assert_eq!(transcript(b"ab~~c\n", 2), expected(&["abc"], 0));
    }

    #[test]
    fn empty_lines_and_a_last_line_without_a_newline() {
        // Patience 1. Rounds: "" | heartbeat | "" | heartbeat (with "last"
        // read) | "lastline" at the end of the stream | `None`.
        assert_eq!(
            transcript(b"\n~~~\nlast~~~line", 1),
            expected(&["", "", "lastline"], 2)
        );
    }

    #[test]
    fn a_character_split_across_a_heartbeat_is_decoded_whole() {
        // 'é' is the two bytes 0xC3 0xA9, and the heartbeat fires between
        // them. Keeping bytes (not chars) until the line is complete is what
        // makes this work; losing 0xC3 leaves invalid UTF-8 behind.
        assert_eq!(
            transcript(b"caf\xC3~~~\xA9\nok\n", 2),
            expected(&["caf\u{e9}", "ok"], 1)
        );
    }

    #[test]
    fn two_connections_keep_their_own_partial_lines() {
        // One task serving two connections: both transcripts are polled in
        // turns, so both readers are in the middle of a line at the same
        // time. Each reader's progress must stay with that reader.
        let mut first = LineReader::new(Source::new(b"hel~~~lo\n"));
        let mut second = LineReader::new(Source::new(b"wor~~~ld\n"));
        let mut first = pin!(read_lines_with_heartbeat(&mut first, 2));
        let mut second = pin!(read_lines_with_heartbeat(&mut second, 2));
        let mut cx = Context::from_waker(Waker::noop());
        let (mut first_done, mut second_done) = (None, None);
        for _ in 0..100 {
            if first_done.is_none()
                && let Poll::Ready(transcript) = first.as_mut().poll(&mut cx)
            {
                first_done = Some(transcript);
            }
            if second_done.is_none()
                && let Poll::Ready(transcript) = second.as_mut().poll(&mut cx)
            {
                second_done = Some(transcript);
            }
        }
        assert_eq!(first_done, Some(expected(&["hello"], 1)));
        assert_eq!(second_done, Some(expected(&["world"], 1)));
    }

    #[test]
    fn the_end_of_the_stream_ends_the_transcript() {
        assert_eq!(transcript(b"", 2), expected(&[], 0));
        assert_eq!(transcript(b"~~~", 2), expected(&[], 1));
    }
}
