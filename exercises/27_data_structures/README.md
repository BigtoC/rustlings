# Module 2 · Data Structures: Implementing std-style Containers by Hand (safe)

> Corresponds to "Module 2: Rust Implementations of Common Data Structures" in the
> course outline. This directory uses only **std**, **100% safe** code, building
> the "kernel" of the standard library's containers by hand to appreciate their
> design tradeoffs — rather than getting bogged down in raw pointers.

## Core Ideas

Safely expressing data structures in Rust relies on **ownership** rather than raw
pointers:

- A "nullable, owning pointer" = `Option<Box<T>>`; `None` is the null pointer.
- "Moving an owned value out of `&mut self`" = `Option::take` (swaps in `None` in
  place and hands you the old value) — the key move for a linked list's `pop`.
- A "wrap-around buffer" = a fixed-length `Vec` + indexing modulo `% capacity`;
  the data never moves.
- A "hash table" = hash into a bucket + linear search within the bucket (separate
  chaining).

## Exercise Path

1. **linkedlist1** — Implement a **singly-linked-list stack** using
   `Option<Box<Node>>`. Implement `push` and `pop`, using `Option::take` to crack
   the classic ownership puzzle of "you can't move a value out of a borrow." The
   tests verify LIFO (last-in, first-out) order.
2. **ringbuffer1** — Implement a fixed-length **ring buffer** (the kernel of
   `VecDeque`) on top of a `Vec<Option<i32>>`. Implement `push_back` /
   `pop_front`, handling index wrap-around with `% capacity`. The tests verify
   wrap-around behavior.
3. **hashtable1** — Implement a **separate-chaining hash table** using
   `Vec<Vec<(String, i32)>>` + `DefaultHasher`. Implement `insert` (overwrite on
   an existing key) and `get`. The tests verify insertion / lookup / overwrite /
   correct retrieval even under hash collisions.

## Why Is Everything Here safe Code?

The **production implementations** of `LinkedList`, `VecDeque`, and `HashMap` in
the standard library use raw pointers and `unsafe` for performance. But their
**design ideas** can be fully expressed in safe Rust, which is also better for
building intuition first. For versions that truly use raw pointers (`NonNull`,
`unsafe`) to hand-write things like a **doubly-linked list**, see the `deep-dive/`
experiments in the repository root.

## Further Reading

- [Learning Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/)
- [`std::collections`](https://doc.rust-lang.org/std/collections/index.html)
- [`Option::take`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take)
