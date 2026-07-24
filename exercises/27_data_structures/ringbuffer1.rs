// Module 2 · Data structures — part 2: a fixed-capacity ring buffer.
//
// A ring buffer (circular buffer) is the core idea behind `VecDeque`. We keep a
// `Vec<Option<i32>>` of fixed length `capacity`, plus a `head` index (where the
// front element lives) and a `len` (how many slots are in use). The "circular"
// part is pure arithmetic: when an index walks off the end it wraps back to the
// start with `% capacity`. No data ever moves — only the indices advance.
//
// The back of the queue is therefore NOT stored explicitly; it is computed:
//
//     tail = (head + len) % capacity
//
// (No `unsafe` here — that is exactly the point. The raw-pointer versions of
// these structures live in `deep-dive/`.)

struct RingBuffer {
    buf: Vec<Option<i32>>,
    head: usize,
    len: usize,
}

impl RingBuffer {
    fn with_capacity(capacity: usize) -> Self {
        RingBuffer {
            buf: vec![None; capacity],
            head: 0,
            len: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.buf.len()
    }

    fn len(&self) -> usize {
        self.len
    }

    // Push onto the back. Returns `false` (without changing anything) if the
    // buffer is already full, otherwise stores the value and returns `true`.
    fn push_back(&mut self, value: i32) -> bool {
        // TODO: If `self.len == self.capacity()`, return `false`. Otherwise
        // compute the tail slot `(self.head + self.len) % self.capacity()`,
        // store `Some(value)` there, increment `self.len`, and return `true`.
        //
        // Until you return a `bool` from every path this will not compile.
    }

    // Pop from the front. Returns `None` if empty.
    fn pop_front(&mut self) -> Option<i32> {
        // TODO: If `self.len == 0`, return `None`. Otherwise take the value out
        // of `self.buf[self.head]` (use `.take()`), advance
        // `self.head = (self.head + 1) % self.capacity()`, decrement `self.len`,
        // and return the value you took.
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pop_is_none() {
        let mut rb = RingBuffer::with_capacity(3);
        assert_eq!(rb.pop_front(), None);
        assert_eq!(rb.len(), 0);
    }

    #[test]
    fn fills_up_and_rejects_when_full() {
        let mut rb = RingBuffer::with_capacity(2);
        assert!(rb.push_back(1));
        assert!(rb.push_back(2));
        // Full now: further pushes are refused and leave the buffer untouched.
        assert!(!rb.push_back(3));
        assert_eq!(rb.len(), 2);
        assert_eq!(rb.pop_front(), Some(1));
        assert_eq!(rb.pop_front(), Some(2));
    }

    #[test]
    fn indices_wrap_around() {
        let mut rb = RingBuffer::with_capacity(3);
        rb.push_back(1);
        rb.push_back(2);
        rb.push_back(3);
        // Drain two from the front so `head` moves forward...
        assert_eq!(rb.pop_front(), Some(1));
        assert_eq!(rb.pop_front(), Some(2));
        // ...then push two more, which must wrap around to the freed slots.
        assert!(rb.push_back(4));
        assert!(rb.push_back(5));
        assert_eq!(rb.pop_front(), Some(3));
        assert_eq!(rb.pop_front(), Some(4));
        assert_eq!(rb.pop_front(), Some(5));
        assert_eq!(rb.pop_front(), None);
    }
}
