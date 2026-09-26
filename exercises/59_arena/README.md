# Module 2 · Arenas: Indices Instead of Pointers

> Module 2 (data structures), right after the linked lists of
> `27_data_structures`. "How do you build a graph, or a tree with parent
> pointers, in safe Rust?" comes up in almost every Rust interview, and the
> answer interviewers want is an **arena**: every node lives in one `Vec`, and
> every link is an index. This module is the base for `60_lru_cache` (`lru2`
> is an index-linked list in a `Vec`) and for the planned `62_graphs` module
> (Tier 2 in `deep-dive/ROADMAP.md`). All **std**, **100% safe**, **stable**
> Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Press `h` when you want
> the full answer.

## Core Ideas

- **One owner for every node.** Rust ownership has to be a tree, so back
  edges (a child pointing at its parent, a cycle in a graph) cannot be owning
  pointers. An arena makes the container the single owner of every node, and
  a link becomes a plain `Copy` number that borrows nothing.
- **Typed indices.** `NodeId(usize)` is a newtype: it cannot be confused with
  a length, an offset or another kind of id, and (with a private field) only
  the arena can create one. rustc itself works this way (`IndexVec` and
  `newtype_index!`).
- **The checks move to run time.** The borrow checker no longer sees the
  links. A wrong index is caught by the bounds check (a panic) or not at all
  (the wrong node), but it is **never undefined behavior**. And the whole
  arena is one borrow: a `&mut` to two nodes at once needs
  `get_disjoint_mut` (`37_borrowck_errors/borrowck1`).
- **Removal makes ids stale.** Once a freed slot is reused, an old index names
  the new occupant, which is the ABA problem again. A **generational index**
  `(index, generation)` catches it: removal bumps the slot's generation, and a
  lookup with an old generation returns `None`.
- **Finite generations.** A `u32` generation runs out once one slot has held
  2^32 values. Either accept the (tiny) chance of a wrap-around, as `slotmap`
  does, or **retire** the slot, as `arena2` does.

## Arena vs `Rc<RefCell<Node>>`

The `Rc` / `Weak` tree is `26_smart_pointers_deep/smartptr2`, `RefCell` is
`smartptr3`, and its run-time borrow panic is `31_debugging/debugging4`.

|                 | `Rc<RefCell<Node>>` + `Weak` parent                              | Arena + typed ids                                                       |
| --------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Allocation      | one per node, plus two counts and a borrow flag                  | one growable `Vec`, nodes side by side                                  |
| A link is       | a counted pointer                                                | a `Copy` integer                                                        |
| Checked by      | `RefCell` at run time: `borrow_mut` during another borrow panics | bounds checks, plus generation checks in a slab                         |
| `Send` / `Sync` | neither: `Rc`'s counts are not atomic                            | yes, whenever `T` is                                                    |
| `clone()`       | copies one handle (shallow)                                      | copies the whole structure (deep), and every id still works in the copy |
| Drop            | recursive, one level at a time; a strong cycle leaks             | one flat loop over the `Vec`                                            |
| Removing a node | automatic when the last strong link goes                         | leaves a hole: reuse it with a free list and generations                |
| A wrong link    | cannot dangle (`Weak::upgrade` returns `None`)                   | a stale or foreign id: the wrong node, a panic, or `None`               |

## Removing Values: Why Generations

| Strategy                                         | Other ids                                                                         | Memory                | A stale id                |
| ------------------------------------------------ | --------------------------------------------------------------------------------- | --------------------- | ------------------------- |
| Never reuse a slot                               | stay valid                                                                        | a new slot per insert | finds an empty slot       |
| `Vec::remove(i)`                                 | every later id is off by one                                                      | compact               | reaches the wrong value   |
| `Vec::swap_remove(i)`                            | the last value moves to `i` and its id breaks (`petgraph`'s `Graph::remove_node`) | compact               | reaches the wrong value   |
| Free list, plain `usize` ids (the `slab` crate)  | stay valid                                                                        | reused                | reaches the next occupant |
| Free list plus generations (`slotmap`, `arena2`) | stay valid                                                                        | reused                | `None`                    |

Generations do not catch an id from a **different** slab that happens to have
the same index and generation. `slotmap`'s answer is a distinct key type per
map (`new_key_type!`).

## Two Kinds of Arena

- **Index arenas** (this module): a `Vec` plus ids. Links are data, so the
  structure can be mutated freely, moved to another thread, cloned and
  serialized. Examples: `petgraph`, `indextree`, `slotmap`, and ECS entity ids
  (a Bevy `Entity` is an index plus a generation).
- **Allocation arenas** (`typed-arena`, `bumpalo`): `alloc(&self, value)`
  returns a real `&mut T` that lives as long as the arena, and everything is
  freed at once when the arena is dropped (`bumpalo` frees the memory without
  running destructors). Links are `&'arena Node` (with `Cell` for links set
  later), so they are checked at compile time and cycles are fine, because
  nothing is freed on its own. The cost is a lifetime parameter on every type
  and no removal. rustc interns its types this way.

## Exercise Path

1. **arena1** — Three empty bodies (E0308) in a `Tree<T>` whose nodes live in
   one `Vec` and point at each other with `NodeId`s. Write `add_child`
   (sequential ids, both links set, an unknown parent panics before anything
   changes), `path_to_root` (a loop over parent links: one test walks a
   200_000-node chain, too deep for recursion) and `lca` (LeetCode 1650, the
   lowest common ancestor with parent links). The tests also show what the
   arena buys: `Tree<String>` is `Send + Sync`, `clone` is deep and ids carry
   over to the copy, and dropping a very deep tree needs no recursion.
2. **arena2** — Three empty bodies (E0308) in a generational `Slab<T>`. `get`
   and `get_mut` check the index, the occupancy and the generation. `remove`
   validates first (a double remove frees nothing), swaps the slot for a
   vacant one with `mem::replace` (moving the value out directly is E0507, as
   in `24_ownership_model/ownership4`), gives it a new generation and puts it
   on the free list. 10_000 insert/remove cycles reuse a single slot, and a
   slot at generation `u32::MAX` is retired instead of overflowing.

Related: `36_atomics/atomics5` meets the ABA problem in a lock-free free list
over an arena and fixes it with a version tag, the atomic cousin of a
generation. The `unsafe` pointer-based doubly linked list is in
`deep-dive/src/unsafe_list.rs`.

## Further Reading

- The Book, [Reference Cycles Can Leak Memory](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html): the `Rc` / `Weak` parent-link tree that an arena replaces
- [Modeling graphs in Rust using vector indices](https://smallcultfollowing.com/babysteps/blog/2015/04/06/modeling-graphs-in-rust-using-vector-indices/) (Niko Matsakis)
- [My RustConf 2018 closing keynote](https://kyren.github.io/2018/09/14/rustconf-talk.html) (Catherine West), especially "Generational indexes are awesome"
- [Arenas in Rust](https://manishearth.github.io/blog/2021/03/15/arenas-in-rust/) (Manish Goregaokar): allocation arenas and cyclic graphs of `&'arena` references
- [`slotmap`](https://docs.rs/slotmap), [`thunderdome`](https://docs.rs/thunderdome) and [`slab`](https://docs.rs/slab)
- [`petgraph`'s `Graph::remove_node`](https://docs.rs/petgraph/latest/petgraph/graph/struct.Graph.html#method.remove_node) and [`StableGraph`](https://docs.rs/petgraph/latest/petgraph/stable_graph/struct.StableGraph.html)
- [`std::mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html), [`std::iter::successors`](https://doc.rust-lang.org/std/iter/fn.successors.html) and [`u32::checked_add`](https://doc.rust-lang.org/std/primitive.u32.html#method.checked_add)
- [`slice::get_disjoint_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_disjoint_mut)
- LeetCode [236. Lowest Common Ancestor of a Binary Tree](https://leetcode.com/problems/lowest-common-ancestor-of-a-binary-tree/) and [1650. Lowest Common Ancestor of a Binary Tree III](https://leetcode.com/problems/lowest-common-ancestor-of-a-binary-tree-iii/) (with parent pointers)
