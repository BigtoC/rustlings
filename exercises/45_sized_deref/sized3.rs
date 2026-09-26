// Traits & Abstraction · `?Sized` and unsized types — part 7: a struct whose last field is unsized is itself unsized (E0277, E0599, E0308).
//
// A struct's type parameter gets the implicit `Sized` bound like any other
// (part 5). Relax it and the struct can do more than point at an unsized
// value: its LAST field can BE one. With `struct Packet<T: ?Sized>` ending
// in `payload: T`, a `Packet<[u8]>` is a user-defined dynamically sized
// type: an id followed by any number of bytes, stored together in one value.
//
// std builds its DSTs exactly this way. `Path` is a struct around an `OsStr`,
// and `Rc<T>` points at an `RcInner<T>` that holds both reference counts and
// then the `T`, which is why an `Rc<str>` needs one allocation instead of
// two (`borrow1`'s interner).
//
// The rules follow from how DSTs work:
//   - Only the last field may be unsized: "only the last field of a struct
//     may have a dynamically sized type". Every other field must sit at an
//     offset the compiler knows without reading any metadata.
//   - A pointer to the struct carries the metadata of its last field. A
//     `&Packet<[u8]>` is two words (address and payload length) and a
//     `&Packet<[u8; 4]>` is one (as for `&str` in
//     `41_memory_layout/layout1`, two words is what current rustc does).
//     `size_of_val` reads that metadata to compute the size of the value.
//   - You never build one directly. You build a sized `Packet<[u8; 4]>`
//     and let an UNSIZING COERCION turn a `Box<Packet<[u8; 4]>>` into a
//     `Box<Packet<[u8]>>`, or a `&Packet<f64>` into a `&Packet<dyn
//     Display>`. The value stays where it is; only the pointer gains its
//     metadata. That coercion is stable for a struct whose last field is
//     the only one whose type involves `T`.
//   - Every `impl<T>` block declares its own `T`, with its own implicit
//     `Sized` bound, so it needs its own `?Sized`. A method that takes or
//     returns a `T` by value, such as `new`, cannot work for an unsized `T`
//     and needs the bound back.
//
// Custom DSTs are rare in application code and common in std and low-level
// crates, so this one is more about reading such code than writing it.
//
// How interviewers probe it: "Can a struct have an unsized field?", "How big
// is a `&Packet<[u8]>`, and how does `size_of_val` know the length?", and
// "How do you create a value of an unsized struct without `unsafe`?"

use std::fmt::Display;

// A network packet: a payload and the id it was sent with.
// TODO: the tests, `checksum` and `describe` use `Packet<[u8]>` and
// `Packet<dyn Display>`, and rustc rejects them: E0277 "the size for values
// of type `[u8]` cannot be known at compilation time" (note: "required by an
// implicit `Sized` bound in `Packet`"), E0599 "the method `id` exists for
// struct `Box<Packet<[u8]>>`, but its trait bounds were not satisfied", and
// E0308 "mismatched types" at the unsizing coercions.
// Requirement: let the payload itself be unsized, so that a `Packet<[u8]>`
// holds the id and the bytes in one value, and make `id` and `payload` work
// on every packet, whatever its payload type (the tests also use `[u16]`
// and `dyn Debug` payloads). Expect two more errors on the way: "only the
// last field of a struct may have a dynamically sized type", and E0277 in
// `new`, which takes and returns a `T` by value.
// Constraints: keep both fields and their types (no `Box`, `&` or `Vec` in
// the struct; a test checks the size of the value) and keep `new`; don't
// change `checksum`, `describe` or the tests; no `unsafe`. Until `Packet`
// and its methods accept an unsized payload, this exercise will not compile.
struct Packet<T> {
    payload: T,
    id: u32,
}

impl<T> Packet<T> {
    fn new(id: u32, payload: T) -> Self {
        Packet { payload, id }
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn payload(&self) -> &T {
        &self.payload
    }
}

// One function for payloads of every length: the sum of the payload bytes.
fn checksum(packet: &Packet<[u8]>) -> u32 {
    packet.payload().iter().map(|&byte| u32::from(byte)).sum()
}

// One function for every printable payload, through a vtable.
fn describe(packet: &Packet<dyn Display>) -> String {
    format!("#{}: {}", packet.id(), packet.payload())
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    use std::ptr;

    const W: usize = size_of::<usize>();

    #[test]
    fn one_checksum_for_every_payload_length() {
        let four: Box<Packet<[u8]>> = Box::new(Packet::new(1, [1, 2, 3, 4]));
        assert_eq!(checksum(&four), 10);
        let big: Box<Packet<[u8]>> = Box::new(Packet::new(2, [255; 300]));
        assert_eq!(checksum(&big), 76_500);
        let empty: Box<Packet<[u8]>> = Box::new(Packet::new(3, []));
        assert_eq!(checksum(&empty), 0);
        assert_eq!((four.id(), big.id(), empty.id()), (1, 2, 3));
        assert_eq!(four.payload(), [1, 2, 3, 4]);
    }

    #[test]
    fn unsizing_keeps_the_value_where_it_is() {
        let sized: Box<Packet<[u8; 4]>> = Box::new(Packet::new(9, [5, 6, 7, 8]));
        let before: *const Packet<[u8; 4]> = &*sized;
        // The unsizing coercion: the pointer gains a length, nothing moves.
        let unsized_packet: Box<Packet<[u8]>> = sized;
        assert!(ptr::addr_eq(before, &*unsized_packet));
        assert_eq!(unsized_packet.payload().len(), 4);
        assert_eq!(checksum(&unsized_packet), 26);
    }

    #[test]
    fn a_pointer_to_the_packet_carries_the_length() {
        assert_eq!(size_of::<&Packet<[u8; 4]>>(), W);
        assert_eq!(size_of::<&Packet<[u8]>>(), 2 * W);
        assert_eq!(size_of::<Box<Packet<[u8]>>>(), 2 * W);
        // The id and the bytes live in the value itself: 4 + 4 bytes.
        let four: Box<Packet<[u8]>> = Box::new(Packet::new(1, [1, 2, 3, 4]));
        assert_eq!(size_of_val(&*four), 8);
        let empty: Box<Packet<[u8]>> = Box::new(Packet::new(1, []));
        assert_eq!(size_of_val(&*empty), 4);
    }

    #[test]
    fn a_dyn_payload_is_reached_through_a_vtable() {
        let price = Packet::new(7, 7.5);
        assert_eq!(describe(&price), "#7: 7.5");
        let word = Packet::new(8, "eight");
        assert_eq!(describe(&word), "#8: eight");
        let erased: &Packet<dyn Display> = &price;
        assert_eq!(erased.payload().to_string(), "7.5");
        assert_eq!(size_of::<&Packet<dyn Display>>(), 2 * W);
        // The vtable records the payload type's size and alignment.
        assert_eq!(size_of_val(erased), size_of::<Packet<f64>>());
    }

    #[test]
    fn every_unsized_payload_gets_the_methods() {
        // Not only `[u8]` and `dyn Display`: one impl serves them all.
        let wide: &Packet<[u16]> = &Packet::new(6, [1000, 2000]);
        assert_eq!(wide.id(), 6);
        assert_eq!(wide.payload(), [1000, 2000]);
        let debug: &Packet<dyn Debug> = &Packet::new(7, 'x');
        assert_eq!(format!("{:?}", debug.payload()), "'x'");
    }

    #[test]
    fn sized_packets_still_work() {
        let packet = Packet::new(4, 99_u64);
        assert_eq!((packet.id(), *packet.payload()), (4, 99));
        let text = Packet::new(5, String::from("owned"));
        assert_eq!(text.payload(), "owned");
    }
}
