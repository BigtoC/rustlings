# Traits & Abstraction · Testing Seams: Trait-Injected Dependencies, Recording Mocks and Fake Clocks

> The last module of the "Traits & Abstraction" group, after `49_panics`.
> It puts the interior mutability of `26_smart_pointers_deep` and
> `40_interior_mutability` and the dispatch trade-offs of `32_dispatch` to
> work. "How would you test this?" follows almost every design question, and
> this module drills the answer: put each dependency (a mailer, a clock)
> behind a trait, hand the code a test double, and test time-based code, a
> token-bucket rate limiter and a TTL cache, deterministically and without a
> single `sleep`. Entirely **std**, **100% safe**, **stable** Rust, edition 2024.
>
> As in the other interview modules, the `// TODO` comments name the error and
> the requirements but **not** the fix. Read what rustc (or the failing test)
> says first. Press `h` when you want the full answer.

## Core Ideas

- **A seam is a trait.** A seam is a place where you can change what code does
  without editing the code there (Michael Feathers' term). The service is
  generic over the dependency (`SignupService<M: Mailer>`,
  `TokenBucket<C: Clock>`): production passes the real implementation, tests
  pass a double. Nothing about the service changes, and a generic parameter
  costs nothing at run time.
- **The receiver belongs to the production contract.** Real dependencies are
  handles (a connection pool, an HTTP client, a channel `Sender`) that do their
  own synchronization, so their methods take `&self`, and the services that
  hold them are shared between request handlers. Don't make a trait method take
  `&mut self` just so a test double can push into a `Vec`, even when rustc's
  help suggests exactly that.
- **So the double needs interior mutability.** Something that records or
  counts through `&self` must use `RefCell`, `Cell`, a `Mutex` or atomics
  inside. Pick the smallest tool that fits:

  | Double's state                           | One thread                 | Shared between threads                   |
  | ---------------------------------------- | -------------------------- | ---------------------------------------- |
  | a call counter (`Copy`)                  | `Cell<u32>`                | `AtomicU32`, or `Mutex<u32>`             |
  | a one-shot scripted failure (not `Copy`) | `Cell<Option<E>>` + `take` | `Mutex<Option<E>>`                       |
  | a log of calls                           | `RefCell<Vec<T>>`          | `Mutex<Vec<T>>`                          |
  | a handle the test keeps (a fake clock)   | `Rc<Cell<Duration>>`       | `Arc<Mutex<Duration>>`, `Arc<AtomicU64>` |

  A double has to be as thread-safe as the code it stands in for: if the real
  limiter is shared by `&` between threads, it must be `Sync`, and so must the
  fake clock inside it. So a `Send + Sync` double trades `Cell` and `RefCell`
  for a `Mutex` (or atomics), and `Rc` for `Arc`.
- **Time is a dependency.** Code that calls `Instant::now()` can only be tested
  with `thread::sleep`, which is slow and flaky. Inject a `Clock` instead, and
  make the fake a handle: the code under test owns one clone, the test keeps
  another and moves time by hand. Use a monotonic clock (`Instant`) in
  production; wall-clock time (`SystemTime`) can jump backwards, and the code
  should survive that anyway.
- **Generic or `dyn`?** Both are fine seams; interviewers want the trade-off.

  | Injection                                           | Dispatch        | Costs                                                                                    |
  | --------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------- |
  | `struct S<M: Mailer> { mailer: M }`                 | static, inlined | the type parameter spreads to every type that holds an `S`; one copy of the code per `M` |
  | `Box<dyn Mailer>` / `Arc<dyn Mailer + Send + Sync>` | vtable call     | one concrete `S`; a heap allocation; the auto traits must be written out                 |
  | `&dyn Mailer` / `&M` borrowed                       | either          | a lifetime parameter on the service                                                      |

  See `32_dispatch` for the mechanics. The forwarding impl that lets a service
  take `&mock` directly (`impl<T: Mailer + ?Sized> Mailer for &T`) is
  `45_sized_deref/sized2`.

## The Token Bucket (seams2)

A bucket holds at most `capacity` tokens and gains one every `refill_every`.
The textbook definition treats the level as continuous: after `t` of clock
time it is `min(capacity, level + t / refill_every)`, and a request needs a
whole token. An implementation with whole tokens must keep what that formula
keeps:

- **The leftover.** Credit only the whole intervals and move `last` forward by
  exactly that much. Setting `last = now` after every refill throws the
  fraction away, and a client that polls faster than the refill rate is never
  credited anything at all.
- **The cap.** An idle hour is worth `capacity` tokens, not 3600. Time spent
  full is lost (that is the `min`): once a full bucket is drained, its next
  token needs a whole `refill_every`.
- **Monotonic credit.** A clock reading earlier than `last` is "no time
  passed". Never move `last` backwards, or a jittery clock becomes a token
  printer.
- **Sharing.** A per-key limiter is `Mutex<HashMap<Key, Bucket>>`: one lock
  guard covers the lookup, the refill and the decrement, so "is there a token?"
  and "take it" are one step.

Alternatives worth naming in an interview: a fixed-window counter (cheap, but
allows 2x bursts at window edges), a sliding-window log (exact, O(requests)
memory), and GCRA, which stores a single "theoretical arrival time" per key.
The planned `backend-tokio-lab` (ROADMAP: sibling crate `backend-lab/`) tests
async code the same way, with tokio's paused test clock
(`#[tokio::test(start_paused = true)]`) instead of a hand-written fake.

## The TTL Cache (seams3)

Each entry stores `expires_at = now + ttl` (saturating: `Duration + Duration`
panics on overflow) and is expired from `now >= expires_at` on.

- **Lazy expiry**: `get` removes a dead entry when it finds one. O(1) per
  lookup and no background work, but untouched dead entries stay in memory,
  and `get` needs `&mut self` (behind a lock, a `Mutex`, not an `RwLock` read
  guard).
- **Eager expiry**: `purge_expired` drops every dead entry in one O(n)
  `HashMap::retain` pass. For big maps, keep a min-heap of deadlines
  (`BinaryHeap<Reverse<(Duration, K)>>`) or a timer wheel and pop only what is
  due. Real caches such as Redis combine both strategies.

The obvious single-lookup `get` does not compile. It is NLL problem case #3
again, the same false positive as in `37_borrowck_errors/borrowck3`, which
explains why and how to restructure:

```rust,ignore
fn get<Q>(&mut self, key: &Q) -> Option<&V>
where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
{
    let now = self.clock.now();
    if let Some(e) = self.entries.get(key) {
        if now < e.expires_at {
            return Some(&e.value); // this borrow MIGHT be returned...
        }
    }
    self.entries.remove(key); // ...so it is still live here: E0502
    None
}
```

## Errors You Will Meet

| Code  | rustc says                                                                       | Where                    | What it means                                                              |
| ----- | -------------------------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------------------- |
| E0596 | cannot borrow `self.sent` as mutable, as it is behind a `&` reference            | seams1, the mock         | a `&self` method tries to mutate a plain field                             |
| E0594 | cannot assign to `self.calls`, which is behind a `&` reference                   | seams1, the mock         | the same, for an assignment                                                |
| E0277 | `Rc<Cell<std::time::Duration>>` cannot be sent between threads safely            | seams2, the fake clock   | `Rc` and `Cell` are single-threaded; the handle must be `Send + Sync`      |
| E0277 | `RefCell<HashMap<..>>` cannot be shared between threads safely                   | seams2, the limiter      | a `RefCell` is `!Sync`; shared state needs a lock                          |
| E0308 | mismatched types: expected `Option<V>`, found `()`                               | seams3, the empty bodies | the method has to be written                                               |
| E0502 | cannot borrow `self.entries` as mutable because it is also borrowed as immutable | seams3, a first `get`    | NLL problem case #3: a borrow that might be returned is live on every path |

Once the double uses `RefCell`, its borrow rules are checked at run time. A
double that holds a `borrow_mut()` guard across a call that re-enters it
panics with "RefCell already borrowed" (or "RefCell already mutably borrowed"
if the re-entrant call only reads), the run-time version of E0499 and E0502
(`31_debugging/debugging4`, and `debugging6` for a guard that lives longer
than it looks). So keep each `borrow_mut()` to one statement, as seams1's
solution does.

## Exercise Path

1. **seams1** — A recording mock behind a `&self` trait (E0596, E0594).
   `SignupService<M: Mailer>` is finished; the `MockMailer` it is tested with
   tries to push, count and clear a scripted failure through `&self`. Keep the
   trait's `&self` (ignore rustc's help to change it) and rebuild the double
   with `RefCell` and `Cell`, so the tests can check that a duplicate sends
   nothing and a failed mail stores no user.
2. **seams2** — A token-bucket rate limiter on an injected clock (E0277, then
   failing tests). First make the fake clock and the per-key limiter safe to
   share between threads, so 8 threads racing for 3 tokens get exactly 3. Then
   fix the refill: keep the leftover time (a 400 ms poller at 1 token/s gets
   nothing from the starter), cap the level, drop time spent full, and never
   credit the same clock time twice.
3. **seams3** — A TTL cache with lazy and eager expiry (E0308). Write `insert`
   (saturating deadline; return the old value only if it was still live),
   `get` (inclusive deadline; remove the dead entry it finds and touch nothing
   else; return a reference into the map) and `purge_expired` (one `retain`
   pass that counts what it dropped).

## Further Reading

- [Testing with mock objects (The Book)](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html#testing-with-mock-objects): its `MockMessenger` is the seams1 pattern in miniature
- [`std::cell`](https://doc.rust-lang.org/std/cell/index.html): when to use `Cell`, `RefCell` and `OnceCell`
- [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html) and [`Send` and `Sync` (The Rustonomicon)](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html) (monotonic) vs [`SystemTime`](https://doc.rust-lang.org/std/time/struct.SystemTime.html) (can go backwards), and [`Duration::saturating_add`](https://doc.rust-lang.org/std/time/struct.Duration.html#method.saturating_add)
- [`HashMap::retain`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.retain)
- [Mocks Aren't Stubs (Martin Fowler)](https://martinfowler.com/articles/mocksArentStubs.html) and [Test Double](https://martinfowler.com/bliki/TestDouble.html)
- [Token bucket](https://en.wikipedia.org/wiki/Token_bucket) and [the generic cell rate algorithm (GCRA)](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm)
- [Redis `EXPIRE`](https://redis.io/docs/latest/commands/expire/), including "How Redis expires keys" (lazy on access plus active sampling)
- [RFC 2094: non-lexical lifetimes](https://rust-lang.github.io/rfcs/2094-nll.html), "Problem case #3: conditional control flow across functions"
- [`mockall`](https://docs.rs/mockall/latest/mockall/), the crate that generates doubles for you
- [`tokio::time::pause`](https://docs.rs/tokio/latest/tokio/time/fn.pause.html), the async world's fake clock
