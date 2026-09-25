// Module 1 · Ownership — part 6: moving a field out of a type that implements `Drop` (E0509).
//
// Parts 4 and 5 were about places you only BORROW. Here `self` is taken BY
// VALUE, so you own the whole struct, and moving one field out of it is
// normally fine. That is a "partial move": the compiler remembers which field
// is gone, drops the other fields when the function ends, and never touches
// the moved one again. `Frame` below does exactly that, and it compiles.
//
// A type with its own `Drop` impl changes the picture. `Drop::drop(&mut self)`
// is handed the WHOLE value, and it runs whenever that value goes away,
// including at the end of a method that took `self` by value. After a partial
// move there is no whole value left to hand it, so rustc forbids moving out of
// such a type at all: E0509 "cannot move out of type `Connection<'_>`,
// which implements the `Drop` trait". A destructuring pattern like
// `let Connection { pending, .. } = self;` is rejected the same way. What
// matters is the type that has the `Drop` IMPL: `Frame` has none, so it can be
// taken apart, even though its `String` and `Vec` fields have destructors of
// their own.
//
// The fix is part 4's rule once more: put something valid back in place of
// what you take. Then `drop` still runs, on the leftover, and it sees exactly
// what you left. Here that is an empty buffer, which is the truth: the bytes
// belong to the caller now, so the close log must say "0 pending bytes".
// Changing how the by-value `self` is bound (for example to make it mutable)
// is not an API change. It only affects the local variable inside the
// function, so callers cannot tell, and an impl may even write it differently
// from the trait declaration of the same method.
//
// When the field has no cheap placeholder, store it as an `Option<T>` and
// `take()` it, and let `drop` handle `None`. std's own
// `io::BufWriter::into_parts` wraps `self` in `ManuallyDrop` and reads the
// inner writer out with `unsafe { ptr::read(..) }`. That is not available in
// this `forbid(unsafe_code)` course, and it is not the first answer an
// interviewer wants to hear.
//
// How interviewers probe it: "Why can't I move a field out of a type that
// implements `Drop`, when I own it?", "Does `drop` still run after you take a
// field?" (yes, on what is left), and "What if the field has no `Default`?"

use std::mem;

// A connection that must report, when it closes, how much it never sent. The
// report goes to a log the caller owns, so the tests can read it.
struct Connection<'a> {
    peer: String,
    pending: Vec<u8>,
    log: &'a mut Vec<String>,
}

impl Drop for Connection<'_> {
    fn drop(&mut self) {
        self.log.push(format!(
            "{}: closed with {} pending bytes",
            self.peer,
            self.pending.len()
        ));
    }
}

impl<'a> Connection<'a> {
    fn new(peer: &str, log: &'a mut Vec<String>) -> Self {
        Connection {
            peer: peer.to_string(),
            pending: Vec::new(),
            log,
        }
    }

    // Queues bytes; nothing is actually sent in this exercise.
    fn send(&mut self, bytes: &[u8]) {
        self.pending.extend_from_slice(bytes);
    }

    fn pending_len(&self) -> usize {
        self.pending.len()
    }

    // Closes the connection and hands its unsent bytes to the caller (to
    // retry them on a new connection, say).
    // `mut self` only makes the local binding mutable (callers see the same
    // by-value signature). `mem::take` swaps an empty `Vec` into the field, so
    // the struct stays whole and `drop` runs on it as usual, reporting the 0
    // bytes that are left, while the caller gets the original buffer.
    fn into_pending(mut self) -> Vec<u8> {
        mem::take(&mut self.pending)
    }
}

// For contrast (already fine, nothing to fix): no `Drop` impl, so taking a
// field out of an owned `Frame` is an ordinary partial move. `header` is
// dropped when `into_body` returns.
struct Frame {
    header: String,
    body: Vec<u8>,
}

impl Frame {
    fn into_body(self) -> Vec<u8> {
        self.body
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_a_connection_reports_what_was_never_sent() {
        let mut log = Vec::new();
        let mut conn = Connection::new("db", &mut log);
        conn.send(b"hello");
        assert_eq!(conn.pending_len(), 5);
        drop(conn);
        assert_eq!(log, ["db: closed with 5 pending bytes"]);
    }

    #[test]
    fn into_pending_returns_the_unsent_bytes() {
        let mut log = Vec::new();
        let mut conn = Connection::new("db", &mut log);
        conn.send(b"abc");
        conn.send(b"de");
        assert_eq!(conn.into_pending(), b"abcde");
    }

    #[test]
    fn into_pending_hands_over_the_same_heap_buffer() {
        let mut log = Vec::new();
        let mut conn = Connection::new("cache", &mut log);
        conn.send(b"GET key");
        let buf_ptr = conn.pending.as_ptr();

        let bytes = conn.into_pending();
        assert_eq!(bytes.as_ptr(), buf_ptr);
        assert_eq!(bytes, b"GET key");
    }

    #[test]
    fn the_connection_still_closes_exactly_once_with_nothing_pending() {
        let mut log = Vec::new();
        let mut conn = Connection::new("db", &mut log);
        conn.send(b"abc");
        let bytes = conn.into_pending();
        assert_eq!(bytes.len(), 3);
        // `drop` ran once, at the end of `into_pending`, on the leftover
        // connection, whose buffer was already handed over.
        assert_eq!(log, ["db: closed with 0 pending bytes"]);
    }

    #[test]
    fn into_pending_with_nothing_queued() {
        let mut log = Vec::new();
        let conn = Connection::new("idle", &mut log);
        assert!(conn.into_pending().is_empty());
        assert_eq!(log, ["idle: closed with 0 pending bytes"]);
    }

    #[test]
    fn each_connection_logs_its_own_close() {
        let mut log = Vec::new();
        {
            let mut conn = Connection::new("a", &mut log);
            conn.send(b"xy");
            assert_eq!(conn.into_pending(), b"xy");
        }
        {
            let mut conn = Connection::new("b", &mut log);
            conn.send(b"zz");
        }
        assert_eq!(
            log,
            [
                "a: closed with 0 pending bytes",
                "b: closed with 2 pending bytes",
            ]
        );
    }

    #[test]
    fn a_type_without_drop_can_be_taken_apart() {
        let frame = Frame {
            header: "len=3".to_string(),
            body: vec![1, 2, 3],
        };
        assert_eq!(frame.header, "len=3");
        let body_ptr = frame.body.as_ptr();
        let body = frame.into_body();
        assert_eq!(body, [1, 2, 3]);
        assert_eq!(body.as_ptr(), body_ptr);
    }
}
