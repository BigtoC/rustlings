# Module 2 · LRU Cache: From a Tick Map to an O(1) Index-Linked List

> Module 2 (data structures), right after the arenas of `59_arena`. "Implement
> an LRU cache" (LeetCode 146) is one of the most-asked live-coding problems in
> Rust interviews, because the textbook answer, a hash map plus a doubly linked
> list, runs straight into ownership. You build it three times: an O(log n)
> warm-up, the O(1) version interviewers want, and then the API polish they
> ask about next. All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Press `h` when you want
> the full answer.

## Core Ideas

- **Two indexes over the same entries.** A cache needs "key to value" and
  "which key was used longest ago". Every operation must keep both in step:
  every cached key appears exactly once in each. The tests check that
  invariant after every step, against a naive model cache.
- **A hit is a write.** `get` moves its key to the front of the recency order,
  so it takes `&mut self`. That one fact decides how the cache can be shared
  (see [below](#why-get-takes-mut-self)).
- **The O(log n) answer has no links.** Stamp every use with a growing tick
  and keep a `BTreeMap<tick, key>`. The least recently used key is the first
  entry, and `pop_first` removes it. The trap: a touch must remove the key's
  old tick, or stale ticks pile up and a hot key gets evicted.
- **The O(1) answer links by index.** A doubly linked list needs every
  middle node to be pointed at by two neighbors: two `&mut` links to one node
  are not allowed, and `&` links cannot be relinked (short of a `Cell` in
  every link). Put the nodes in one `Vec` and make every link a `usize`: the
  arena of `59_arena`, used as an intrusive list.
- **Eviction recycles a slot.** A full cache moves the new key into the
  evicted entry's slot with `mem::replace`, which also hands the old key and
  value back by value (a plain move out of `nodes[i]` is E0507). No other
  index changes, and `nodes.len()` never exceeds the capacity.
- **Look up by a borrowed key.** `get<Q>(&mut self, key: &Q) where K:
  Borrow<Q>, Q: Hash + Eq + ?Sized` lets an `LruCache<String, V>` be queried
  with a `&str`, exactly like `HashMap::get` (`45_sized_deref/borrow1`).

## Four Ways to Link the List

| Links                             | Safe Rust?                             | Allocations  | A bug means                                           | `Send` / `Sync`          | Seen in                       |
| --------------------------------- | -------------------------------------- | ------------ | ----------------------------------------------------- | ------------------------ | ----------------------------- |
| `&mut Node` / `&Node`             | no: the cache would borrow from itself | —            | —                                                     | —                        | nobody                        |
| `Rc<RefCell<Node>>`               | yes                                    | one per node | a "RefCell already borrowed" panic, or a leaked cycle | neither                  | "A Bad Safe Deque"            |
| raw pointers (`*mut` / `NonNull`) | no, `unsafe`                           | one per node | undefined behavior                                    | only by `unsafe impl`    | the `lru` crate, `LinkedList` |
| `usize` indices into one `Vec`    | yes                                    | one `Vec`    | a wrong entry or an index panic                       | whenever `K` and `V` are | `lru2`, `lru3`                |

Two smaller design choices come up in the follow-up questions:

- **The key is stored twice** (in the map and in its entry, so an eviction
  can find the map entry to remove), hence `K: Clone`. Cheap-to-clone keys
  such as `Arc<str>` make that a counter bump. Storing it once needs a raw
  hash table that hashes `nodes[i].key` itself, such as
  `hashbrown::HashTable<usize>`.
- **`Option<usize>` links cost 16 bytes each** on a 64-bit target, because a
  `usize` has no spare bit pattern to encode `None` in. A plain `usize` with
  `usize::MAX` meaning "none" halves that, at the price of a value that must
  never be used as an index by mistake. `Option<NonZeroU32>` holding the index
  plus one takes 4 bytes and caps the cache at about four billion entries.

## Why `get` Takes `&mut self`

`get` only returns a reference to a value, but it also relinks the entry at
the front, and relinking is a write. The consequences are what interviewers
are really asking about:

- **No `get` through `&LruCache`.** A function that receives the cache by
  shared reference, or an `Arc<LruCache>`, can only call methods that take
  `&self`. That is why `lru3` adds `peek(&self)`: a read that does not count
  as a use.
- **Shared between threads, the cache goes behind a `Mutex`.** A `RwLock`
  buys little: every `get` needs the write lock, and only `peek` can use a
  read guard. `53_lock_hazards/rwlock1`, later in the course, covers what a
  `RwLock` does and does not allow.
- **Interior mutability moves the write, it does not remove it.** A `RefCell`
  around the list gives you a `get(&self)`, but the cache stops being `Sync`,
  and overlapping borrows become run-time panics
  (`26_smart_pointers_deep/smartptr3`, and `31_debugging/debugging4` for
  tracking one down).
  A `Cell` is enough for a hit counter (`40_interior_mutability/cell1`), not
  for a linked list. A `Mutex` inside the cache gives a thread-safe
  `get(&self)`, which is the usual shape of a concurrent cache API, but every
  hit still takes that lock, and it returns a clone (often of an `Arc<V>`)
  instead of a `&V`, because a reference into the data cannot outlive the
  guard.
- **Production caches relax "exact LRU" to scale.** They shard the cache
  (one lock per shard, picked by the key's hash), approximate it (Redis
  samples a few keys and evicts the oldest of them), or buffer the hits and
  apply them to the list in batches (Caffeine's design, which `moka` follows).

## Exercise Path

1. **lru1** — Two empty bodies (E0308) in an `LruCache` over
   `HashMap<K, (V, u64)>` plus `BTreeMap<u64, K>`. Write `get` and `put`: every
   touch gives the key a new tick and removes its old one, an update never
   evicts, and a new key in a full cache evicts `order`'s first entry with
   `pop_first`. The short warm-up: O(log n), no links at all.
2. **lru2** — The O(1) cache: entries in a `Vec` with `prev` / `next` indices,
   plus `HashMap<K, usize>` and `head` / `tail`. `get` and `put` are empty
   (E0308), and once they compile the tests fail until the empty `unlink` and
   `push_front` helpers relink correctly. A full cache moves the new key into
   the tail's slot with `mem::replace`; 10_000 puts into a capacity-3 cache
   must leave exactly 3 entries, and the forward and backward walks must
   agree after every step.
3. **lru3** — The tests look up `String` keys with a `&str`, `Vec<u8>` keys
   with a `&[u8]` and `Arc<str>` keys with a `&str` (E0308), and call a `peek`
   that does not exist yet (E0599). Give `get` the `K: Borrow<Q>` signature of
   `HashMap::get` (an application of `45_sized_deref/borrow1`), and add a
   `peek(&self)` that does not count as a use, makes one hash lookup (a test
   counts the key comparisons) and works through a `RwLock` read guard.

Related: `59_arena` introduces index links and generations, and
`37_borrowck_errors/borrowck1` explains why two `&mut` into one `Vec` need
proof of disjointness. The raw-pointer doubly linked list is in
`deep-dive/src/unsafe_list.rs`, and `deep-dive/src/self_referential.rs` shows
why a struct cannot simply hold pointers into itself. Index-based graphs come
back in `62_graphs`.

## Further Reading

- LeetCode [146. LRU Cache](https://leetcode.com/problems/lru-cache/)
- [`BTreeMap::pop_first`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.pop_first) and [`std::mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html)
- [`HashMap::get`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get) and [`std::borrow::Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html), including why `Borrow` requires matching `Hash` and `Eq`
- [The `lru` crate's `LruCache`](https://docs.rs/lru/latest/lru/struct.LruCache.html): the same `get` / `peek` split, built on raw pointers
- [A Bad Safe Deque](https://rust-unofficial.github.io/too-many-lists/fourth.html) in *Learning Rust With Entirely Too Many Linked Lists*: the `Rc<RefCell>` doubly linked list, and why it hurts
- [Modeling graphs in Rust using vector indices](https://smallcultfollowing.com/babysteps/blog/2015/04/06/modeling-graphs-in-rust-using-vector-indices/) (Niko Matsakis)
- [`std::sync::RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html)
- [Redis key eviction: the approximated LRU algorithm](https://redis.io/docs/latest/develop/reference/eviction/)
- [Caffeine's design](https://github.com/ben-manes/caffeine/wiki/Design) (read buffers and batched policy updates) and [`moka`](https://docs.rs/moka)
- [`hashbrown::HashTable`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashTable.html)
