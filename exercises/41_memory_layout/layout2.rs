// Module 1 · Memory layout — part 2: sizes in bytes, padding and a `repr(C)` field reorder (E0080).
//
// Part 1 counted words. This part counts BYTES, because that is where
// alignment and padding show up. The rules:
//
//   - Every type has a size and an alignment, and its size is always a
//     multiple of its alignment (so that the elements of an array stay
//     aligned). A `u8` has alignment 1, a `u32` 4, and a `u64`, an `f64` or a
//     pointer 8 on a 64-bit target.
//   - Each field sits at an offset that is a multiple of its own alignment.
//     The unused bytes in between, and at the end, are PADDING.
//   - The default representation (`repr(Rust)`) promises nothing more than
//     that: rustc may reorder the fields, and does, to waste less padding.
//   - `#[repr(C)]` lays the fields out in declaration order, with the padding
//     C would insert. That is what FFI and wire formats need, and it makes
//     the order of your fields your problem.
//   - `#[repr(C, packed)]` lowers the alignment to 1, so there is no padding
//     at all. The price: a field may now sit at a misaligned address, so
//     taking a reference to it is error E0793 "reference to field of packed
//     struct is unaligned". Reading it BY VALUE (a copy) is fine.
//   - An enum is a tag plus room for its largest variant, and the tag is
//     padded like any other field, unless a niche (part 1) can hold it. A
//     `NonZero<u32>` is never 0, a `char` is never above `0x10FFFF` or a
//     surrogate, a `bool` is only 0 or 1. An `f64`, like a `u32`, has no
//     invalid bit patterns at all: every 64-bit pattern is some number, an
//     infinity or a NaN.
//
// The answers below are for a 64-bit target (x86_64, aarch64), which is what
// the course's CI runs on. Some of them differ on 32-bit targets; the hint
// says which.
//
// Here the checks run at COMPILE time. `const _: () = assert!(..);` is
// evaluated by the compiler, and a failing one is error E0080 "evaluation
// panicked: <message>". Real code uses the same idiom as a layout guard
// (the `static_assertions` crate packages the same idea as `const_assert!`
// and `assert_eq_size!`): if someone adds a field to a type that must stay
// small, the build breaks instead of every value quietly growing. Part B has
// one.
//
// How interviewers probe this: "Why is `Option<u32>` 8 bytes but
// `Option<NonZeroU32>` 4?", "Does Rust reorder struct fields? What does
// `repr(C)` guarantee?", "Why would you box one variant of an enum?", "How
// would you shrink this `repr(C)` struct without `packed`?".

use std::num::NonZero;

// ---------- Part A: predict the size in bytes ----------

// The types the questions ask about.

#[repr(C)]
struct SampleC {
    tag: u8,
    value: u32,
    flags: u8,
}

struct SampleRust {
    tag: u8,
    value: u32,
    flags: u8,
}

#[repr(C, packed)]
struct SamplePacked {
    tag: u8,
    value: u32,
    flags: u8,
}

// Clippy's `large_enum_variant` lint flags this enum, and its message would
// give the answer away, so it is allowed here.
#[allow(clippy::large_enum_variant)]
enum Message {
    Ping,
    Data([u8; 1024]),
}

enum BoxedMessage {
    Ping,
    Data(Box<[u8; 1024]>),
}

// TODO: every answer below is the placeholder 999, so each check under the
// answers fails with E0080 "evaluation panicked: Q_... is wrong: how many
// bytes is `...`?". The checks run at compile time and name the question
// only, never the real size. Replace each 999 with your prediction in bytes,
// for a 64-bit target.
// Requirements:
//   - predict first, then compile and rethink only the checks still failing;
//   - write each answer as a plain integer literal: computing it with
//     `size_of` passes the check and teaches you nothing;
//   - don't change the checks or the types above.
// Until you replace every 999 with the right number of bytes, this exercise
// will not compile.

// Q1: `Option<u32>`
const Q_OPTION_U32: usize = 999;
// Q2: `Option<NonZero<u32>>` (`NonZeroU32` is another name for `NonZero<u32>`)
const Q_OPTION_NONZERO_U32: usize = 999;
// Q3: `Option<f64>`
const Q_OPTION_F64: usize = 999;
// Q4: `SampleC`
const Q_SAMPLE_C: usize = 999;
// Q5: `SampleRust`, the same fields without a `repr`
const Q_SAMPLE_RUST: usize = 999;
// Q6: `SamplePacked`
const Q_SAMPLE_PACKED: usize = 999;
// Q7: `Message`
const Q_MESSAGE: usize = 999;
// Q8: `BoxedMessage`
const Q_BOXED_MESSAGE: usize = 999;

// The checks. Don't change them.
const _: () = assert!(
    size_of::<Option<u32>>() == Q_OPTION_U32,
    "Q_OPTION_U32 is wrong: how many bytes is `Option<u32>`?"
);
const _: () = assert!(
    size_of::<Option<NonZero<u32>>>() == Q_OPTION_NONZERO_U32,
    "Q_OPTION_NONZERO_U32 is wrong: how many bytes is `Option<NonZero<u32>>`?"
);
const _: () = assert!(
    size_of::<Option<f64>>() == Q_OPTION_F64,
    "Q_OPTION_F64 is wrong: how many bytes is `Option<f64>`?"
);
const _: () = assert!(
    size_of::<SampleC>() == Q_SAMPLE_C,
    "Q_SAMPLE_C is wrong: how many bytes is `SampleC`?"
);
const _: () = assert!(
    size_of::<SampleRust>() == Q_SAMPLE_RUST,
    "Q_SAMPLE_RUST is wrong: how many bytes is `SampleRust`?"
);
const _: () = assert!(
    size_of::<SamplePacked>() == Q_SAMPLE_PACKED,
    "Q_SAMPLE_PACKED is wrong: how many bytes is `SamplePacked`?"
);
const _: () = assert!(
    size_of::<Message>() == Q_MESSAGE,
    "Q_MESSAGE is wrong: how many bytes is `Message`?"
);
const _: () = assert!(
    size_of::<BoxedMessage>() == Q_BOXED_MESSAGE,
    "Q_BOXED_MESSAGE is wrong: how many bytes is `BoxedMessage`?"
);

// ---------- Part B: shrink a `repr(C)` header ----------

// The fixed-size header in front of every packet: C code reads it, and it
// goes over the wire as is, so it has to be `repr(C)`. Millions of these sit
// in queues, so every byte counts.
//
// TODO: the layout guard under the struct fails with E0080 "evaluation
// panicked: PacketHeader must be 16 bytes". The fields add up to 16 bytes,
// but in this declaration order `repr(C)` has to pad the header to 24.
// Get it down to 16 by changing only the ORDER of the fields.
// Requirements:
//   - keep `#[repr(C)]` (in real life you would reorder the C struct too,
//     and bump the protocol version); without it, `on_packet` below fails
//     to compile, because nothing would promise C that layout;
//   - keep every field's name and type;
//   - no `packed`: it would get to 16 bytes by misaligning `id`, and the
//     tests borrow the fields (E0793) and check the alignment;
//   - order the fields from the largest alignment to the smallest: the tests
//     check the offsets that gives (either `bool` may come first).
// Until you get the header down to 16 bytes, this exercise will not compile,
// and until its fields are in that order, the tests will fail.
#[repr(C)]
struct PacketHeader {
    flag: bool,
    id: u64,
    kind: u16,
    ok: bool,
    len: u32,
}

// The layout guard. Don't change it.
const _: () = assert!(
    size_of::<PacketHeader>() == 16,
    "PacketHeader must be 16 bytes"
);
const _: () = assert!(
    align_of::<PacketHeader>() == align_of::<u64>(),
    "PacketHeader must keep the alignment of its `u64` field"
);

// The entry point the C side calls with each header, by value. rustc's
// `improper_ctypes_definitions` lint checks that every type in an
// `extern "C" fn` signature has a layout C can rely on. It is only a warning
// by default; the `deny` makes a `PacketHeader` without `#[repr(C)]` an
// error: "`extern` fn uses type `PacketHeader`, which is not FFI-safe".
// Don't change it.
#[deny(improper_ctypes_definitions)]
extern "C" fn on_packet(header: PacketHeader) -> u32 {
    header.len
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;

    #[test]
    fn header_fields_go_from_largest_to_smallest_alignment() {
        // `id` (align 8), then `len` (align 4), then `kind` (align 2)...
        assert_eq!(offset_of!(PacketHeader, id), 0);
        assert_eq!(offset_of!(PacketHeader, len), 8);
        assert_eq!(offset_of!(PacketHeader, kind), 12);
        // ...then the two `bool`s (align 1), in either order.
        let mut bools = [offset_of!(PacketHeader, flag), offset_of!(PacketHeader, ok)];
        bools.sort_unstable();
        assert_eq!(bools, [14, 15]);
    }

    #[test]
    fn header_has_no_padding_left() {
        let fields = size_of::<u64>() + size_of::<u32>() + size_of::<u16>() + 2 * size_of::<bool>();
        assert_eq!(size_of::<PacketHeader>(), fields);
        assert_eq!(align_of::<PacketHeader>(), align_of::<u64>());
        // No padding inside means none between the elements of an array.
        assert_eq!(size_of::<[PacketHeader; 4]>(), 4 * fields);
    }

    #[test]
    fn header_fields_can_still_be_borrowed() {
        let h = PacketHeader {
            flag: true,
            id: 0x0123_4567_89ab_cdef,
            kind: 0xbeef,
            ok: false,
            len: 0xdead_beef,
        };
        // References to the fields. With `packed`, each would be E0793.
        let id: &u64 = &h.id;
        let len: &u32 = &h.len;
        let kind: &u16 = &h.kind;
        assert_eq!(*id, 0x0123_4567_89ab_cdef);
        assert_eq!(*len, 0xdead_beef);
        assert_eq!(*kind, 0xbeef);
        assert!(h.flag);
        assert!(!h.ok);
    }

    #[test]
    fn c_side_takes_the_header_by_value() {
        let h = PacketHeader {
            flag: false,
            id: 7,
            kind: 1,
            ok: true,
            len: 1500,
        };
        assert_eq!(on_packet(h), 1500);
    }

    #[test]
    fn packed_fields_can_be_read_by_value() {
        let s = SamplePacked {
            tag: 1,
            value: 0x1234_5678,
            flags: 2,
        };
        // `assert_eq!(s.value, ..)` would not compile: the macro borrows its
        // arguments, and `&s.value` may be misaligned (E0793). Copying the
        // field into a local first is always fine.
        let value = s.value;
        assert_eq!(value, 0x1234_5678);
        assert_eq!((s.tag, s.flags), (1, 2));
    }
}
