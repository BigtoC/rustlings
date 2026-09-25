# Module 2 · Data Structures: Implementing std-style Containers by Hand (safe)

> Corresponds to "Module 2: Rust Implementations of Common Data Structures" in the
> course outline. This directory uses only **std**, **100% safe** code, building
> the "kernel" of the standard library's containers by hand to appreciate their
> design tradeoffs — rather than getting bogged down in raw pointers.
>
> It ends with the linked-list **interview classics** on LeetCode's `ListNode`
> (reverse, merge, split in half, remove the nth from the end). As in
> `37_borrowck_errors`, their `// TODO` comments name the error and the
> requirements but not the fix; press `h` for the full answer.

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
- "Relinking a list in place" = moving `Box`es from one link to another. The
  nodes themselves never move, so no node is copied, allocated or freed.
- "A pointer you can write through" = a `&mut Option<Box<Node>>` cursor that
  points at a **link** (the place where the next node goes), not at a node.
- "Two pointers into one list" does not port: a `&mut` cursor is exclusive
  access to the whole rest of the list, so a `&` cursor reading ahead of it is
  E0502. Count first, then walk a single `&mut` cursor.

## Exercise Path

1. **linkedlist1** — Implement a **singly-linked-list stack** using
   `Option<Box<Node>>`. Implement `push` and `pop`, using `Option::take` to crack
   the classic ownership puzzle of "you can't move a value out of a borrow." The
   tests verify LIFO (last-in, first-out) order, plus that a 200 000-node stack
   can be dropped at all — the derived drop glue is recursive, so the exercise
   ships a hand-written iterative `Drop` to stop it overflowing the stack.
2. **ringbuffer1** — Implement a fixed-length **ring buffer** (the kernel of
   `VecDeque`) on top of a `Vec<Option<i32>>`. Implement `push_back` /
   `pop_front`, handling index wrap-around with `% capacity`. The tests verify
   wrap-around behavior.
3. **hashtable1** — Implement a **separate-chaining hash table** using
   `Vec<Vec<(String, i32)>>` + `DefaultHasher`. Implement `insert` (overwrite on
   an existing key) and `get`. The tests verify insertion / lookup / overwrite /
   correct retrieval even under hash collisions.
4. **linkedlist2** — **Reverse** a LeetCode `ListNode` list **in place**,
   iteratively, by moving each node from the input onto the reversed part. The
   tests check that the result is made of the original nodes (a private serial
   number plus the address of every node, because a freed address can come back
   from the allocator), and they reverse a 200 000-node list, which a recursive
   version cannot survive on a test thread's stack.
5. **linkedlist3** — **Merge two sorted lists** without allocating: splice the
   input nodes onto the end of the result through a `&mut` **tail cursor**
   (`Option::insert` appends and advances in one step), keep ties **stable**, and
   attach the leftover list with one assignment. Node identity and a
   200 000-node merge rule out copying and recursion.
6. **linkedlist4** — **Split a list in half** and **remove the nth node from the
   end**. The fast/slow-pointer starter fails with E0502, because a `&` cursor
   reads nodes that the `&mut` cursor borrows exclusively. Count the nodes
   first, then walk one `&mut` cursor and cut; use `checked_sub` so that
   `n > len` leaves the list unchanged instead of underflowing.

## Linked-List Classics: Cursors, Not Pointers

The C versions of the interview classics juggle several raw pointers into one
list. In safe Rust each node has exactly one owner, so the same algorithms are
written with **moves** and a single **`&mut` cursor**:

| C idiom                                   | Safe Rust                                                                 | What goes wrong otherwise                                                   |
| ----------------------------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `prev` / `cur` / `next` pointer shuffle   | move each `Box` from the input onto the reversed part (`take`, `replace`) | copying the values into a `Vec` and rebuilding: new nodes, O(n) extra space |
| `tail->next = node; tail = node;`         | `tail: &mut Option<Box<Node>>` at the end link, advanced with a reborrow  | a second `&mut` to the list while `tail` is live                            |
| fast/slow pointers to find the middle     | count with a `&` cursor, then walk one `&mut` cursor                      | E0502: a `&` cursor reads nodes the `&mut` cursor borrows exclusively       |
| lead pointer `n` nodes ahead of the trail | the same: count, then index with `len.checked_sub(n)`                     | E0502 again, and `len - n` panics in debug builds when `n > len`            |
| recursion over the list                   | a loop                                                                    | one stack frame per node: a long list overflows the stack                   |

Interviewers ask these in Rust *because* the C answer does not carry over. The
strong answer names the rule that rejects it (aliasing XOR mutability; one owner
per node), then gives the restructured version and its complexity: all of them
stay O(n) time and O(1) extra space.

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
- [`Option::insert`](https://doc.rust-lang.org/std/option/enum.Option.html#method.insert)
  and [`std::mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html)
- [`usize::checked_sub`](https://doc.rust-lang.org/std/primitive.usize.html#method.checked_sub)
- LeetCode [206. Reverse Linked List](https://leetcode.com/problems/reverse-linked-list/),
  [21. Merge Two Sorted Lists](https://leetcode.com/problems/merge-two-sorted-lists/),
  [876. Middle of the Linked List](https://leetcode.com/problems/middle-of-the-linked-list/)
  and [19. Remove Nth Node From End of List](https://leetcode.com/problems/remove-nth-node-from-end-of-list/)
