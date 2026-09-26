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

// Sizes in bytes for a 64-bit target. Each answer says why, and whether the
// language GUARANTEES that size or it is only what the CURRENT RUSTC does.

// Q1: 8. A `u32` uses all 2^32 bit patterns, so there is no niche, and
// `Option` needs a separate tag. The `u32` must stay 4-aligned, so the tag
// takes a 4-byte slot in front of it (rustc widens the tag itself to 4 bytes
// rather than leaving 3 bytes of padding). At least 8 is forced: 2^32 + 1
// states need more than 4 bytes, and the size must be a multiple of the
// alignment (4). Exactly 8 is CURRENT RUSTC.
const Q_OPTION_U32: usize = 8;
// Q2: 4. A `NonZero<u32>` is never 0, so `None` is stored as 0 and there is
// no tag. GUARANTEED: `num::NonZero*` is in the `core::option`
// "Representation" table. This is why IDs and handles are often `NonZero`.
const Q_OPTION_NONZERO_U32: usize = 4;
// Q3: 16. Every 64-bit pattern is some `f64` (NaNs included), so there is no
// niche, and the tag takes an 8-byte slot, because the `f64` must stay
// 8-aligned. At least 16 is forced; exactly 16 is CURRENT RUSTC. On 32-bit x86
// Linux (i686) an `f64` is only 4-byte aligned, and this is 12.
const Q_OPTION_F64: usize = 16;
// Q4: 12. In declaration order: `tag` at 0, 3 bytes of padding, `value` at
// 4..8, `flags` at 8, then 3 bytes of tail padding so that the size is a
// multiple of the alignment (4). Half of it is padding. GUARANTEED: this is
// the `repr(C)` algorithm in the Reference.
const Q_SAMPLE_C: usize = 12;
// Q5: 8. rustc reorders the fields to `value`, `tag`, `flags` (offsets 0, 4,
// 5), plus 2 bytes of tail padding. The tuple `(u8, u32, u8)` is 8 for the
// same reason. CURRENT RUSTC: `repr(Rust)` only promises aligned,
// non-overlapping fields, not any particular order.
const Q_SAMPLE_RUST: usize = 8;
// Q6: 6. `packed` sets the alignment to 1, so there is no padding at all, and
// `value` sits at offset 1, misaligned (hence E0793 for `&s.value`).
// GUARANTEED: `packed(1)` leaves no padding between fields, and `C` keeps the
// declared order.
const Q_SAMPLE_PACKED: usize = 6;
// Q7: 1025. The 1024-byte payload plus a 1-byte tag; the alignment is 1, so
// there is no padding. EVERY `Message`, even a `Ping`, takes 1025 bytes: a
// `Vec` of a thousand pings is a megabyte, and moving one is a 1025-byte copy
// unless the optimizer removes it. At least 1025 is forced (a `u8` has no
// niche); exactly 1025 is CURRENT RUSTC.
const Q_MESSAGE: usize = 1025;
// Q8: 8. The payload is now a `Box`, which is never null, so `Ping` is
// stored as the null pointer and there is no tag: one word. A `Ping` costs
// 8 bytes; a `Data` costs 8 plus a 1024-byte heap allocation. GUARANTEED:
// the Nomicon's FFI chapter promises this "nullable pointer optimization"
// for any enum with exactly two variants, one data-less and one holding a
// non-nullable pointer. On a 32-bit target this is 4.
const Q_BOXED_MESSAGE: usize = 8;

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
// Sorted from the largest alignment to the smallest, every field lands on an
// offset its alignment already divides, so `repr(C)` inserts no padding:
// `id` 0..8, `len` 8..12, `kind` 12..14, `flag` 14, `ok` 15. That works
// because each field's size is a multiple of its alignment, and the fields
// before it are all at least as aligned. Before, `flag` wasted 7 bytes in
// front of `id`, and `ok` wasted 1 in front of `len`. Dropping `repr(C)`
// would also give 16 bytes on current rustc (it happens to pick this order),
// but then nothing would promise C code that layout, and `on_packet` would
// be an error.
#[repr(C)]
struct PacketHeader {
    id: u64,
    len: u32,
    kind: u16,
    flag: bool,
    ok: bool,
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
