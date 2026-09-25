// Module 1 · Ownership — part 4: moving OUT of `&mut self` (E0507), `mem::take` and `mem::swap`.
//
// Part 1 said that a move transfers ownership and leaves the source unusable.
// For a local variable that is no problem: the compiler tracks which of its
// own variables (and fields of them) have been moved from, and it won't let you
// read them again or drop them a second time. A `&mut T` is different. It
// gives you exclusive ACCESS to a place, not ownership of it. The owner is
// somewhere else (up the call stack, or in a collection), and after your
// borrow ends it may read that place again, and it WILL drop it. The compiler
// cannot tell the owner "this place is empty now", so moving a value out from
// behind a `&mut` would leave the owner holding a moved-from value: a
// use-after-free or double free waiting to happen. rustc refuses with E0507
// "cannot move out of `...` which is behind a mutable reference".
//
// So the rule is not "never move out of a `&mut`". It is "never leave a place
// behind a `&mut` without a valid value in it". Moving out is fine if a valid
// value goes back in AS PART OF THE SAME OPERATION, and that is exactly what
// three small functions in `std::mem` do:
//
//   - `mem::replace(&mut place, new)` stores `new` and returns the old value.
//   - `mem::take(&mut place)` is `replace` with `T::default()`, so it needs
//     `T: Default`. For `Vec`, `String` and `Option` the default is empty and
//     does not allocate, so a `take` costs a few word copies.
//   - `mem::swap(&mut a, &mut b)` exchanges two values of the same type.
//
// (`Option::take()`, which `27_data_structures/linkedlist1` uses, is literally
// `mem::replace(self, None)`.) None of them copies heap data. Moving a `Vec`
// moves its three-word header (pointer, capacity, length). The elements stay
// where they are, in the same heap buffer. That is why the tests below can
// prove "moved, not copied" by comparing `as_ptr()` before and after.
//
// Part B is why `mem::swap` exists even though a swap is "just three
// assignments". `let tmp = a; a = b; b = tmp;` works on locals. On two fields
// behind `&mut self`, the first line already moves a field out with nothing
// put back, so it is E0507 again. rustc does not accept "I refill it two lines
// later": the check is per move, and in general a panic in between would
// unwind into the owner while the place is still empty.
//
// How interviewers probe this: "How do you move a value out of a `&mut T`?",
// "`take` vs `replace` vs `swap` vs `Option::take`?", and they watch whether
// you reach for `.clone()` first. rustc's own note says "if `Event`
// implemented `Clone`, you could clone the value". That is the trap: cloning
// copies every event (and allocates) only to throw the originals away, and a
// type that owns a unique resource should not be `Clone` at all.

// ---------- Part A: hand the whole buffer to the caller ----------

// Deliberately NOT `Clone`: an event owns its payload, and copying events is
// not the way out.
#[derive(Debug, PartialEq)]
struct Event {
    id: u32,
    payload: String,
}

impl Event {
    fn new(id: u32, payload: &str) -> Self {
        Event {
            id,
            payload: payload.to_string(),
        }
    }
}

// Collects events and ships them in batches.
struct Batch {
    buf: Vec<Event>,
    // How many events have been flushed so far, over the batch's lifetime.
    flushed: usize,
}

impl Batch {
    fn new() -> Self {
        Batch {
            buf: Vec::new(),
            flushed: 0,
        }
    }

    fn push(&mut self, event: Event) {
        self.buf.push(event);
    }

    // Hands every buffered event to the caller and leaves the batch empty,
    // ready to collect the next batch.
    fn flush(&mut self) -> Vec<Event> {
        self.flushed += self.buf.len();
        // TODO: E0507 "cannot move out of `self.buf` which is behind a mutable
        // reference". The batch is only borrowed, so its `buf` cannot simply be
        // moved away with nothing left in the field. rustc's note suggests
        // implementing `Clone` for `Event`; don't.
        // Requirement: the caller gets the batch's OWN `Vec` (a test compares
        // heap pointers), and the batch is left empty and reusable.
        // Constraints: keep the signature (`&mut self`: the tests flush the
        // same batch several times), keep `Event` non-`Clone`, don't move the
        // events one by one into a new `Vec` (`drain`, `split_off`, ...), no
        // `unsafe`, and don't change the tests.
        // Until you move the buffer out while leaving a valid value in its
        // place, this exercise will not compile.
        self.buf
    }
}

// ---------- Part B: a double buffer ----------

// Writers fill `back` while readers look at `front`. `present` publishes the
// back buffer. Renderers (front and back framebuffers), audio and log
// shippers all use this shape. Both buffers are allocated once, up front.
struct DoubleBuffer {
    front: Vec<Event>,
    back: Vec<Event>,
}

impl DoubleBuffer {
    fn with_capacity(capacity: usize) -> Self {
        DoubleBuffer {
            front: Vec::with_capacity(capacity),
            back: Vec::with_capacity(capacity),
        }
    }

    fn write(&mut self, event: Event) {
        self.back.push(event);
    }

    // Publishes what was written since the last `present`: the back buffer
    // becomes the front, and the old front becomes the next (cleared) back
    // buffer. The two allocations just trade places, so a running double
    // buffer never allocates.
    fn present(&mut self) {
        // TODO: two E0507s, for `self.front` and `self.back`. A swap written
        // with a temporary has a moment in which a field has been moved out and
        // nothing is back in it yet, and behind `&mut self` that moment is not
        // allowed.
        // Requirement: afterwards `front` is the old back buffer and `back` is
        // the old front buffer (the tests check that the two heap pointers
        // trade places).
        // Constraints: no `.clone()`, don't copy or move the events from one
        // `Vec` into the other (`append`, `extend`, `drain`, ...), don't give
        // either field a freshly allocated `Vec`, no `unsafe`, and don't change
        // the tests.
        // Until you exchange the two buffers in one step, this exercise will
        // not compile.
        let old_front = self.front;
        self.front = self.back;
        self.back = old_front;

        // The stale frame is dropped, but the allocation is kept for reuse.
        self.back.clear();
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(events: &[Event]) -> Vec<u32> {
        events.iter().map(|e| e.id).collect()
    }

    // ----- Part A -----

    #[test]
    fn flush_returns_every_event_in_order() {
        let mut batch = Batch::new();
        batch.push(Event::new(1, "login"));
        batch.push(Event::new(2, "click"));
        batch.push(Event::new(3, "logout"));

        let events = batch.flush();
        assert_eq!(
            events,
            [
                Event::new(1, "login"),
                Event::new(2, "click"),
                Event::new(3, "logout"),
            ]
        );
    }

    #[test]
    fn flush_hands_over_the_same_heap_buffer() {
        let mut batch = Batch::new();
        batch.push(Event::new(1, "a"));
        batch.push(Event::new(2, "b"));
        let buf_ptr = batch.buf.as_ptr();
        let payload_ptr = batch.buf[0].payload.as_ptr();

        let events = batch.flush();
        // The `Vec` itself was moved: same heap buffer, not a copy of it...
        assert_eq!(events.as_ptr(), buf_ptr);
        // ...and so were the events inside it.
        assert_eq!(events[0].payload.as_ptr(), payload_ptr);
    }

    #[test]
    fn flush_leaves_the_batch_empty_and_reusable() {
        let mut batch = Batch::new();
        batch.push(Event::new(1, "a"));
        assert_eq!(batch.flush().len(), 1);
        assert!(batch.buf.is_empty());

        // A second flush right away has nothing to hand out.
        assert!(batch.flush().is_empty());

        // The batch keeps working, and a flush returns only the new events.
        batch.push(Event::new(2, "b"));
        batch.push(Event::new(3, "c"));
        assert_eq!(ids(&batch.flush()), [2, 3]);
        assert!(batch.buf.is_empty());
    }

    #[test]
    fn flushing_an_empty_batch_is_fine() {
        let mut batch = Batch::new();
        assert!(batch.flush().is_empty());
        assert_eq!(batch.flushed, 0);
    }

    #[test]
    fn flush_keeps_counting_across_batches() {
        let mut batch = Batch::new();
        batch.push(Event::new(1, "a"));
        batch.push(Event::new(2, "b"));
        batch.flush();
        batch.push(Event::new(3, "c"));
        batch.flush();
        batch.flush();
        assert_eq!(batch.flushed, 3);
    }

    // ----- Part B -----

    #[test]
    fn present_publishes_what_was_written() {
        let mut screen = DoubleBuffer::with_capacity(4);
        screen.write(Event::new(1, "draw"));
        screen.write(Event::new(2, "text"));
        assert!(screen.front.is_empty());

        screen.present();
        assert_eq!(ids(&screen.front), [1, 2]);
        assert!(screen.back.is_empty());
    }

    #[test]
    fn present_makes_the_two_heap_buffers_trade_places() {
        let mut screen = DoubleBuffer::with_capacity(4);
        screen.write(Event::new(1, "draw"));
        let front_ptr = screen.front.as_ptr();
        let back_ptr = screen.back.as_ptr();
        let payload_ptr = screen.back[0].payload.as_ptr();

        screen.present();
        assert_eq!(screen.front.as_ptr(), back_ptr);
        assert_eq!(screen.back.as_ptr(), front_ptr);
        // The published event was not copied either.
        assert_eq!(screen.front[0].payload.as_ptr(), payload_ptr);
        // No buffer lost its allocation.
        assert!(screen.front.capacity() >= 4);
        assert!(screen.back.capacity() >= 4);
    }

    #[test]
    fn each_present_replaces_the_previous_frame() {
        let mut screen = DoubleBuffer::with_capacity(4);
        screen.write(Event::new(1, "frame one"));
        screen.present();
        screen.write(Event::new(2, "frame two"));
        screen.write(Event::new(3, "frame two"));
        screen.present();
        assert_eq!(ids(&screen.front), [2, 3]);
        assert!(screen.back.is_empty());

        // Presenting with nothing written shows an empty frame.
        screen.present();
        assert!(screen.front.is_empty());
        assert!(screen.back.is_empty());
    }

    #[test]
    fn a_running_double_buffer_reuses_its_two_allocations() {
        let mut screen = DoubleBuffer::with_capacity(4);
        let a = screen.front.as_ptr();
        let b = screen.back.as_ptr();
        for frame in 0..6 {
            screen.write(Event::new(frame, "tick"));
            screen.present();
            // The two buffers ping-pong. No third allocation ever appears.
            let (front, back) = if frame % 2 == 0 { (b, a) } else { (a, b) };
            assert_eq!(screen.front.as_ptr(), front);
            assert_eq!(screen.back.as_ptr(), back);
            assert_eq!(ids(&screen.front), [frame]);
        }
    }
}
