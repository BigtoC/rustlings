//! Part 1 · `select_cancel`: `read_exact` in a `select!` loop loses bytes
//!
//! The protocol is a stream of fixed-size 8-byte frames (think a 64-bit
//! sequence number, or a price tick). The reader also has to send a heartbeat
//! every 100 ms, so it races "read the next frame" against a ticker:
//!
//! ```text
//! loop {
//!     let mut buf = [0u8; 8];
//!     tokio::select! {
//!         res = reader.read_exact(&mut buf) => frames.push(buf),
//!         _ = heartbeat.tick() => send_heartbeat(),
//!     }
//! }
//! ```
//!
//! Frames arrive in two TCP segments with a pause in between. When the tick
//! wins while half a frame has been read, `select!` drops the `read_exact`
//! future, and with it the count of bytes it had already taken out of the
//! stream. The bytes are gone from the socket, so the next round starts in the
//! middle of a frame. One lost half frame misaligns every frame after it:
//! `read_frames_broken` turns frames `[0; 8], [1; 8], ...` into
//! `[0, 0, 0, 0, 1, 1, 1, 1], [1, 1, 1, 1, 2, 2, 2, 2], ...`.
//!
//! tokio documents this: its `select!` docs list `AsyncReadExt::read_exact`
//! among the methods that "are not cancellation safe and can lead to loss of
//! data", and `AsyncReadExt::read` among the cancel-safe ones.
//!
//! Two fixes, the same two as `57_async_combinators/cancel1`:
//!
//! 1. Don't cancel it ([`read_frames_pinned`]): create the `read_exact`
//!    future once per frame, pin it outside an inner loop, and race `&mut` to
//!    it against the ticker. Dropping a reference cancels nothing, so the same
//!    future resumes after every tick.
//! 2. Keep the progress outside the future ([`read_frames_stateful`] with a
//!    [`FrameReader`]): the reader owns the partial frame and a `filled`
//!    count, and uses the cancel-safe `read`. A canceled `read_frame` loses
//!    nothing, and the next call carries on with the same frame. This is how
//!    tokio-util's `FramedRead` works: its decoder buffer lives in the stream,
//!    so `StreamExt::next` on it is cancel safe.
//!
//! Invariants the tests check: with a tick in the middle of every frame, the
//! fixed readers deliver every frame intact and count every tick; the broken
//! reader is fine as long as no tick lands mid-frame, which is why the bug
//! survives happy-path tests.
//!
//! Timing (virtual time, `start_paused`): the writer's segments land on even
//! milliseconds (0, 100, 200, ...) and the ticks on odd ones (51, 151, ...),
//! so a tick and a segment never arrive at the same instant, and no test
//! depends on which of two ready branches `select!` picks first.

use std::io;
use std::pin::pin;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{Instant, Interval, interval_at, sleep};

/// Bytes per frame.
pub const FRAME_LEN: usize = 8;

/// One frame of the protocol.
pub type Frame = [u8; FRAME_LEN];

/// Frame number `i` of the test stream: eight copies of the byte `i`, so a
/// frame stitched together from two different frames is easy to spot.
pub fn frame(i: u8) -> Frame {
    [i; FRAME_LEN]
}

/// A frame is intact when all of its bytes came from the same frame.
pub fn is_intact(frame: &Frame) -> bool {
    frame.iter().all(|&byte| byte == frame[0])
}

/// What a reader saw: the frames it assembled and the heartbeats it sent.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    pub frames: Vec<Frame>,
    pub ticks: u32,
}

/// A heartbeat ticker whose first tick is `offset` from now, then one every
/// `period`. (`tokio::time::interval` would tick immediately.)
pub fn ticker(offset: Duration, period: Duration) -> Interval {
    interval_at(Instant::now() + offset, period)
}

/// Writes frames `0..count`, each as two segments of 4 bytes with `stall`
/// between them, then shuts the stream down. The second half of a frame and
/// the first half of the next one go out back to back, as one segment would.
pub async fn write_frames<W: AsyncWrite + Unpin>(
    mut writer: W,
    count: u8,
    stall: Duration,
) -> io::Result<()> {
    let half = FRAME_LEN / 2;
    for i in 0..count {
        let frame = frame(i);
        writer.write_all(&frame[..half]).await?;
        if !stall.is_zero() {
            sleep(stall).await;
        }
        writer.write_all(&frame[half..]).await?;
    }
    writer.shutdown().await
}

/// BROKEN: a fresh `read_exact` every round. When the tick wins mid-frame,
/// the bytes the canceled `read_exact` had consumed are lost.
///
/// A clean end of stream, and a stream that ends mid-frame, both end the
/// transcript: `read_exact` reports both as `UnexpectedEof`.
pub async fn read_frames_broken<R: AsyncRead + Unpin>(
    mut reader: R,
    mut heartbeat: Interval,
) -> io::Result<Transcript> {
    let mut transcript = Transcript::default();
    loop {
        let mut buf = [0u8; FRAME_LEN];
        tokio::select! {
            // Not cancel safe: if the tick wins, this future is dropped
            // together with its count of bytes already read into `buf`.
            res = reader.read_exact(&mut buf) => match res {
                Ok(_) => transcript.frames.push(buf),
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(transcript),
                Err(e) => return Err(e),
            },
            // `Interval::tick` is cancel safe: losing the race consumes no
            // tick.
            _ = heartbeat.tick() => transcript.ticks += 1,
        }
    }
}

/// FIX 1: never cancel the read. One `read_exact` future per frame is pinned
/// outside the inner loop, and each round races a `&mut` to it, so a tick
/// only drops the reference and the same future resumes next round.
pub async fn read_frames_pinned<R: AsyncRead + Unpin>(
    mut reader: R,
    mut heartbeat: Interval,
) -> io::Result<Transcript> {
    let mut transcript = Transcript::default();
    loop {
        let mut buf = [0u8; FRAME_LEN];
        let res = {
            let mut read = pin!(reader.read_exact(&mut buf));
            loop {
                tokio::select! {
                    res = &mut read => break res,
                    _ = heartbeat.tick() => transcript.ticks += 1,
                }
            }
            // `read` (and its borrow of `buf`) ends here.
        };
        match res {
            Ok(_) => transcript.frames.push(buf),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(transcript),
            Err(e) => return Err(e),
        }
    }
}

/// A reader that keeps its framing state (the partial frame and how much of
/// it is filled) in itself, not in a future.
pub struct FrameReader<R> {
    inner: R,
    buf: Frame,
    filled: usize,
}

impl<R: AsyncRead + Unpin> FrameReader<R> {
    pub fn new(inner: R) -> Self {
        FrameReader {
            inner,
            buf: [0; FRAME_LEN],
            filled: 0,
        }
    }

    /// Returns the next frame, `Ok(None)` at a clean end of stream, and an
    /// `UnexpectedEof` error if the stream ends in the middle of a frame.
    ///
    /// Cancel safe. The only `.await` is `read`, which is cancel safe (a
    /// canceled `read` has read nothing), and every byte it returns goes into
    /// `self.buf` / `self.filled` before the next `.await`. Dropping this
    /// future therefore loses nothing: the next call continues the frame.
    pub async fn read_frame(&mut self) -> io::Result<Option<Frame>> {
        while self.filled < FRAME_LEN {
            let n = self.inner.read(&mut self.buf[self.filled..]).await?;
            if n == 0 {
                return if self.filled == 0 {
                    Ok(None)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "stream ended in the middle of a frame",
                    ))
                };
            }
            self.filled += n;
        }
        self.filled = 0;
        Ok(Some(self.buf))
    }
}

/// FIX 2: the same `select!` loop as the broken version, over a cancel-safe
/// `FrameReader::read_frame`.
pub async fn read_frames_stateful<R: AsyncRead + Unpin>(
    reader: R,
    mut heartbeat: Interval,
) -> io::Result<Transcript> {
    let mut reader = FrameReader::new(reader);
    let mut transcript = Transcript::default();
    loop {
        tokio::select! {
            res = reader.read_frame() => match res? {
                Some(frame) => transcript.frames.push(frame),
                None => return Ok(transcript),
            },
            _ = heartbeat.tick() => transcript.ticks += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use tokio::io::{DuplexStream, duplex};

    const MS: Duration = Duration::from_millis(1);
    /// First tick at 51 ms, then every 100 ms: always an odd millisecond.
    const TICK_OFFSET: Duration = Duration::from_millis(51);
    const TICK_PERIOD: Duration = Duration::from_millis(100);
    /// The pause between a frame's two halves: always an even millisecond,
    /// and as long as the tick period, so exactly one tick lands in it.
    const STALL: Duration = Duration::from_millis(100);

    /// Runs `write_frames(count, stall)` against `read` over an in-memory
    /// duplex pipe (a socket without the network), in virtual time.
    async fn run<F, Fut>(read: F, count: u8, stall: Duration) -> Transcript
    where
        F: FnOnce(DuplexStream, Interval) -> Fut,
        Fut: Future<Output = io::Result<Transcript>>,
    {
        let (client, server) = duplex(64);
        let writer = tokio::spawn(write_frames(client, count, stall));
        let transcript = read(server, ticker(TICK_OFFSET, TICK_PERIOD))
            .await
            .expect("the reader returned an error");
        writer.await.unwrap().unwrap();
        transcript
    }

    fn frames(count: u8) -> Vec<Frame> {
        (0..count).map(frame).collect()
    }

    /// The frame the broken reader assembles from the second half of frame
    /// `i` and the first half of frame `i + 1`.
    fn stitched(i: u8) -> Frame {
        let mut f = [i; FRAME_LEN];
        f[FRAME_LEN / 2..].fill(i + 1);
        f
    }

    // ---- the bug, demonstrated ----------------------------------------

    #[tokio::test(start_paused = true)]
    async fn broken_one_tick_mid_frame_misaligns_every_later_frame() {
        let transcript = run(read_frames_broken, 10, STALL).await;
        // At 51 ms the tick cancels a `read_exact` holding the first half of
        // frame 0. Those 4 bytes are gone, so every later read starts in the
        // middle of a frame, and the last half frame runs into the end of the
        // stream.
        assert_eq!(transcript.frames, (0..9).map(stitched).collect::<Vec<_>>());
        assert!(transcript.frames.iter().all(|f| !is_intact(f)));
        assert_eq!(transcript.ticks, 10);
    }

    #[tokio::test(start_paused = true)]
    async fn happy_path_test_hides_the_bug() {
        // Every byte arrives at t = 0 and the first tick is at 51 ms: no
        // read is ever canceled, so the broken reader passes. This is the
        // happy-path test that lets the bug through code review.
        let transcript = run(read_frames_broken, 10, Duration::ZERO).await;
        assert_eq!(transcript.frames, frames(10));
        assert_eq!(transcript.ticks, 0);
    }

    // ---- the fixes ----------------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn pinned_frames_intact_across_100_ticks() {
        let transcript = run(read_frames_pinned, 100, STALL).await;
        assert_eq!(transcript.frames, frames(100));
        // One tick in the middle of every frame, and none was missed.
        assert_eq!(transcript.ticks, 100);
    }

    #[tokio::test(start_paused = true)]
    async fn stateful_frames_intact_across_100_ticks() {
        let transcript = run(read_frames_stateful, 100, STALL).await;
        assert_eq!(transcript.frames, frames(100));
        assert_eq!(transcript.ticks, 100);
    }

    #[tokio::test(start_paused = true)]
    async fn a_frame_survives_several_ticks() {
        // A 350 ms stall holds each frame open across 3 or 4 ticks.
        let stall = 350 * MS;
        for transcript in [
            run(read_frames_pinned, 5, stall).await,
            run(read_frames_stateful, 5, stall).await,
        ] {
            assert_eq!(transcript.frames, frames(5));
            assert!(transcript.ticks >= 15, "ticks: {}", transcript.ticks);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn frame_reader_tells_a_clean_end_from_a_truncated_frame() {
        let (mut client, server) = duplex(64);
        client.write_all(&frame(7)).await.unwrap();
        client.write_all(&[9, 9, 9]).await.unwrap();
        drop(client);
        let mut reader = FrameReader::new(server);
        assert_eq!(reader.read_frame().await.unwrap(), Some(frame(7)));
        let err = reader.read_frame().await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);

        let (client, server) = duplex(64);
        drop(client);
        assert_eq!(FrameReader::new(server).read_frame().await.unwrap(), None);
    }
}
