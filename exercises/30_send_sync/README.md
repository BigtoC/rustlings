# Module 4 · Send / Sync and Fearless Concurrency

> Corresponds to "Module 4: The Type-System Foundations of Concurrency Safety" in the syllabus.
> This directory uses only **std** and **100% safe** code (`unsafe` is forbidden); threads are finite and always `join`ed, so the tests run instantly.

## Core idea

Rust's "fearless concurrency" doesn't rely on runtime checks. Instead, two **auto marker traits** keep data races out at compile time:

- **`Send`**: a value of the type can be **moved** to another thread.
- **`Sync`**: the type can be **shared by reference** across multiple threads (`&T` is `Send` if and only if `T: Sync`).

They are _auto traits_: a type is automatically `Send`/`Sync` as long as **all of its fields** are. Conversely, a single field that isn't "poisons" the whole type, making it not. `thread::spawn` requires the closure (and everything it captures) to be `Send` — that requirement is exactly where the compile-time interception happens.

## `Rc` vs `Arc`

| Type     | Reference count            | `Send`/`Sync`?                  | Use case                         |
| -------- | -------------------------- | ------------------------------- | -------------------------------- |
| `Rc<T>`  | Plain integer (non-atomic) | **No**                          | Single-threaded shared ownership |
| `Arc<T>` | Atomic integer             | **Yes** (when `T: Send + Sync`) | Cross-thread shared ownership    |

`Rc`'s count is not atomic, so two threads mutating it at once would be a data race. That's why `Rc` is marked `!Send`, and the compiler reports the error directly: `` `Rc<T>` cannot be sent between threads safely ``. Just switch it to `Arc`.

## `Arc<Mutex<T>>`: shared mutable state

`Arc<T>` only gives you a `&T`, so you can't modify the inner value. To **safely mutate** shared state across threads you need interior mutability plus a lock:

```rust
let counter = Arc::new(Mutex::new(0usize));
// In each thread:
*counter.lock().unwrap() += 1; // lock() blocks until the lock is acquired; the guard releases it automatically when dropped
```

- `Arc`: multiple threads own it together (`Send + Sync`).
- `Mutex`: only one thread can hold `&mut T` at any moment, eliminating data races.

## Exercise progression

1. **send_sync1** — `Rc` is `!Send` and can't cross threads; switch to `Arc` so a shared read-only value can pass the thread boundary.
2. **send_sync2** — `Arc` alone can't mutate; use `Arc<Mutex<T>>` so N threads can safely increment the same counter.
3. **send_sync3** — Use `fn assert_send<T: Send>()` / `assert_sync` to turn "is this type thread-safe?" into a compile-time assertion, then fix the field types so `Shared` becomes `Send + Sync`.

## Further reading

- [The Rust Book · Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [`std::marker::Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) / [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html)
- [`std::sync::Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) / [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
