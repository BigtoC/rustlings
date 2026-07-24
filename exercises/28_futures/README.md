# Module 3 · Async (Part 1): `Future` and `Poll`

> The foundational part of the syllabus's "Module 3: A Complete Dissection of the Async Programming Model".
> This directory uses only **std** and **100% safe** code to take apart the machinery behind `async`/`await` and show you how it works.

## Core idea

`async`/`await` is just syntactic sugar. At its heart, any async computation is an ordinary value that implements the `Future` trait:

```rust
trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

An executor "drives" a future by calling `poll` repeatedly:

- `Poll::Ready(value)`: the computation is done, and the value is right here.
- `Poll::Pending`: not ready yet, so poll me again later. Before returning `Pending`, you **must** register a wake-up notification via `cx.waker()`; otherwise the executor will never poll you again and the task hangs forever.

## Exercise progression

1. **futures1** — Implement the simplest possible `Future`: one that is immediately `Ready`. Understand the signature of `poll`.
2. **futures2** — Return `Pending` for the first time, and learn to use the `Waker` to request "poll me again".
3. **futures3** — **Key exercise**: hand-write a state machine that "awaits A first, then awaits B". This is exactly what the compiler generates for an `async fn`.
4. **futures4** — Write the equivalent logic using `async`/`await` and see for yourself that "syntactic sugar == state machine".

## Why is the receiver of `poll` a `Pin<&mut Self>`?

For now this section is just about "being able to use it" (`self.0`, `self.get_mut()`, `Pin::new`). The **reason** `Pin` exists (self-referential structs cannot be moved) is covered in depth in the `29_async_runtime` module's `runtime4`. Actually hand-writing a self-referential state machine with `unsafe` is left to the `deep-dive/` experiments at the repository root.

## Further reading

- [Asynchronous Programming in Rust (the async book)](https://rust-lang.github.io/async-book/)
- [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html)
- [`std::task::Poll`](https://doc.rust-lang.org/std/task/enum.Poll.html)
