# Module 1 · Memory Layout: Predict `size_of`, Then Explain It

> The last drill of Module 1, after the smart-pointer, `Drop` and
> interior-mutability modules. Two quizzes ask how many words or bytes a type
> takes, and a small fix squeezes the padding out of a `repr(C)` header. All
> **std**, **100% safe**, **stable** Rust, edition 2024.
>
> The `// TODO` comments say what is checked and the rules for your answers,
> but not the answers themselves, as in an interview. Predict first, then let
> the checks grade you. Press `h` for the full answer key: every answer is
> explained and labeled **guaranteed** or **current rustc**.

## Core Ideas

- **Size and alignment.** Every sized type has both, and its size is always a
  multiple of its alignment, so that the elements of an array stay aligned.
  Each field sits at an offset its alignment divides. The unused bytes in
  between, and at the end, are **padding**.
- **Thin and fat pointers.** A pointer to a sized type (`&T`, `&mut T`,
  `Box<T>`, `NonNull<T>`, a `fn` pointer) is one `usize`. A pointer to a
  dynamically sized type carries **metadata** next to the address: the length
  for `[T]` and `str`, a pointer to the vtable for `dyn Trait`. That makes
  `Box<[u8]>`, `&mut [T]` and `&mut dyn Write` two words each.
- **Niches.** If a type has bit patterns that no valid value uses, an enum can
  store its tag there for free. References, `Box` and `NonNull` are never
  null, `NonZero<_>` is never 0, a `bool` is only 0 or 1, and a `char` is
  never a surrogate or above `0x10FFFF`. So `Option<&mut T>` is one word and
  `Option<bool>` is one byte. Nesting stays free only while spare values are
  left: a `bool` has 254 of them, so `Option<Option<bool>>` is still one byte.
- **Enum tags.** Without a niche, an enum is a tag plus room for its largest
  variant, and the tag is padded like any other field: `Option<u16>` is 4
  bytes. Every value pays for the largest variant, which is why clippy's
  `large_enum_variant` lint suggests boxing the big payload once the largest
  variant is more than 200 bytes bigger than the second largest.
- **Representations.** The default `repr(Rust)` promises only aligned,
  non-overlapping fields, so rustc may reorder them, and does. `repr(C)` keeps
  the declared order with C's padding rules, for FFI and wire formats (an
  `extern "C" fn` that takes a `repr(Rust)` struct gets the
  `improper_ctypes_definitions` warning: "not FFI-safe").
  `repr(packed)` drops the alignment to 1 and with it all padding.
  `repr(transparent)` gives a one-field wrapper exactly its field's layout,
  and `repr(align(N))` raises a type's alignment.
- **Closures** are anonymous structs with one field per captured variable (or
  captured place, such as `point.x`, since edition 2021): a reference for each
  capture by reference, the value itself for each capture by value.
- **Zero-sized types.** `()`, unit structs, `PhantomData<T>`, `[T; 0]` and
  closures that capture nothing take no bytes, so `HashSet<K>` (a
  `HashMap<K, ()>` inside) stores no value bytes at all.
- **Interior mutability hides niches.** `UnsafeCell` (inside `Cell`,
  `RefCell`, `Mutex`, ...) blocks niche use, so `Option<Cell<bool>>` is two
  bytes while `Option<bool>` is one. The cells themselves are the subject of
  `40_interior_mutability`.

## Guaranteed or Current rustc?

The follow-up interviewers like best is "is that guaranteed?". The Reference
states the rule: "Type layout can be changed with each compilation", and only
what is written down is promised.

| Fact                                                                                                         | Status        | Where it is written                                   |
| ------------------------------------------------------------------------------------------------------------ | ------------- | ----------------------------------------------------- |
| `&T`, `&mut T` and raw pointers to a sized `T` have the size and alignment of `usize`                        | guaranteed    | Reference, Pointers and references layout             |
| `Box<T>` with `T: Sized` is a single pointer                                                                 | guaranteed    | `std::boxed`, Memory layout                           |
| `Option<T>` is the size of `T` for `&U`, `&mut U`, `Box<U>`, `NonNull<U>`, `fn` pointers and `NonZero<_>`    | guaranteed    | `core::option`, Representation                        |
| A two-variant enum with one data-less variant and one holding a non-nullable pointer uses null for the first | guaranteed    | Nomicon, FFI: "The nullable pointer optimization"     |
| Pointers to DSTs are two words                                                                               | current rustc | Reference: "Though you should not rely on this"       |
| `String` and `Vec<T>` are three words, and their capacity field has a niche (values above `isize::MAX`)      | current rustc | std source only                                       |
| `repr(Rust)` reorders fields to save padding                                                                 | current rustc | Reference, The Rust representation (allowed, no more) |
| `repr(C)` keeps the declared order and C's padding                                                           | guaranteed    | Reference, `#[repr(C)]` structs                       |
| `repr(packed)` leaves no padding between fields                                                              | guaranteed    | Reference, The alignment modifiers                    |
| Anything about a closure's layout                                                                            | current rustc | Reference: "Closures have no layout guarantees"       |
| `()` has size 0 and alignment 1                                                                              | guaranteed    | Reference, Tuple layout                               |

Primitive **sizes** are fixed (`u32` is 4 bytes, `char` 4, `bool` 1), but
primitive **alignment** is platform-specific: on 32-bit x86 Linux (i686), `u64`
and `f64` are only 4-byte aligned. That is why `layout2` says its byte answers
are for a 64-bit target.

## Padding and Field Order

`repr(C)` lays fields out in the order you wrote them, so a small field in
front of a big one costs padding:

```rust
#[repr(C)]
struct Event {
    kind: u8,  // offset 0, then 7 bytes of padding
    at: u64,   // offsets 8..16
    level: u8, // offset 16, then 7 bytes of tail padding
} // 24 bytes, alignment 8

#[repr(C)]
struct EventSorted {
    at: u64,   // offsets 0..8
    kind: u8,  // offset 8
    level: u8, // offset 9, then 6 bytes of tail padding
} // 16 bytes
```

Sorting fields from the largest alignment to the smallest removes all padding
**between** fields, because every size is a multiple of its alignment. Tail
padding can remain, and it is part of the size. With the default
`repr(Rust)`, rustc does this sorting for you today; with `repr(C)` it is your
job. `repr(packed)` is not the fix: it removes the padding by misaligning
fields, so every read or write of a multi-byte field compiles to an unaligned
load or store (slower on some CPUs), and references to those fields are
forbidden.

## E0793: Borrowing a Packed Field

```text
error[E0793]: reference to field of packed struct is unaligned
  = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
  = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
```

A reference must always be aligned, and a field of a packed struct may not be.
Reading the field **by value** is fine: `let v = s.value;`. Watch out for
macros, which borrow their arguments: `println!("{}", s.value)` and
`assert_eq!(s.value, 1)` are both E0793, while `println!("{}", { s.value })`
compiles, because the extra braces make a block that copies the field out
first. `&raw const s.value` (stable since Rust 1.82) makes a raw pointer
without a reference, but reading through it needs `unsafe` and
`read_unaligned`.

## Asking the Compiler

- `size_of::<T>()`, `align_of::<T>()` and `size_of_val(&value)` are in the
  prelude since Rust 1.80. Use `size_of_val` for a closure, whose type you
  cannot name, and for a DST behind a reference.
- `std::mem::offset_of!(Type, field)` (stable since Rust 1.77) gives a field's
  offset.
- `const _: () = assert!(size_of::<T>() == 16);` is a **layout guard**: it
  is checked at compile time, and a failing one is E0080 "evaluation
  panicked". The `static_assertions` crate packages the same idea as
  `const_assert!` and `assert_eq_size!`.
- On nightly, `rustc -Zprint-type-sizes` prints the layout of every type in a
  crate, padding included.

## Exercise Path

1. **layout1** — Nine predictions in words (`W = size_of::<usize>()`), so the
   answers hold on 32-bit and 64-bit targets alike: `Option<Box<u64>>`,
   `&str`, `&dyn Display`, `Rc<str>`, `Option<String>`, `Option<Option<&u8>>`,
   and three closures (one captures nothing, one `move`s a `String` in, one
   borrows two `String`s). Every answer starts at 999, and each failing test
   names only its question, never the real size. Replace each 999 with your
   prediction.
2. **layout2** — Part A: eight predictions in bytes for a 64-bit target:
   `Option<u32>`, `Option<NonZero<u32>>`, `Option<f64>`, one struct as
   `repr(C)`, as the default `repr(Rust)` and as `repr(C, packed)`, and an
   enum with a 1 KiB variant, unboxed and boxed. The checks are compile-time
   `const` assertions, so a wrong answer is an E0080 error that prints no
   sizes. Part B: a `repr(C)` `PacketHeader` is 24 bytes, and its layout guard
   demands 16. Reorder the fields by alignment, keeping `repr(C)` and without
   `packed`: the tests check the offsets, the alignment and that each field
   can still be borrowed, and an `extern "C" fn` under
   `#[deny(improper_ctypes_definitions)]` refuses the struct without
   `repr(C)`.

## Related Modules

- `24_ownership_model/ownership4`: a `Vec` or `String` is a three-word header,
  and moving one copies only the header.
- `26_smart_pointers_deep`: `Box` and recursive types, `Rc` and `Weak`.
- `32_dispatch` (next in the path) and `deep-dive/src/vtable_lab.rs`: what
  the second word of a `&dyn Trait` points to, built by hand in the lab.
- `33_closures`: the capture modes that decide a closure's fields, in depth.
- `45_sized_deref/borrow1`: an `Rc<str>` string interner, and why
  `HashSet<Rc<String>>` cannot be looked up with a `&str`.
- `deep-dive/src/ordering_lab.rs`: `repr(align(128))` cache-line padding
  (`CachePadded`) against false sharing, with its layout asserted and a
  benchmark.

## Further Reading

- [Type layout (The Reference)](https://doc.rust-lang.org/reference/type-layout.html), including [pointer layout](https://doc.rust-lang.org/reference/type-layout.html#pointers-and-references-layout), [the Rust representation](https://doc.rust-lang.org/reference/type-layout.html#the-rust-representation) and [the alignment modifiers](https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers)
- [`Option` representation (`core::option`)](https://doc.rust-lang.org/std/option/index.html#representation)
- [`Box` memory layout](https://doc.rust-lang.org/std/boxed/index.html#memory-layout)
- [The nullable pointer optimization (The Nomicon, FFI)](https://doc.rust-lang.org/nomicon/ffi.html#the-nullable-pointer-optimization)
- [repr(Rust)](https://doc.rust-lang.org/nomicon/repr-rust.html) and [Alternative representations](https://doc.rust-lang.org/nomicon/other-reprs.html) (The Nomicon)
- [Closure types and capture modes (The Reference)](https://doc.rust-lang.org/reference/types/closure.html)
- [`std::mem::size_of`](https://doc.rust-lang.org/std/mem/fn.size_of.html), [`size_of_val`](https://doc.rust-lang.org/std/mem/fn.size_of_val.html) and [`offset_of!`](https://doc.rust-lang.org/std/mem/macro.offset_of.html)
- [`std::num::NonZero`](https://doc.rust-lang.org/std/num/struct.NonZero.html)
- [Clippy: `large_enum_variant`](https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant)
- [E0080](https://doc.rust-lang.org/error_codes/E0080.html) and [E0793](https://doc.rust-lang.org/error_codes/E0793.html) in the error code index
- [Unsafe Code Guidelines: data layout](https://rust-lang.github.io/unsafe-code-guidelines/layout.html)
- [The `static_assertions` crate](https://docs.rs/static_assertions)
- [cheats.rs: Memory Layout, Basic Types](https://cheats.rs/#basic-types) (and the sections after it on references, closures and std types)
