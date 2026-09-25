// Closures - bounds beyond the basics, part 8: async closures and
// `AsyncFnMut`.
//
// Before Rust 1.85 an "async callback" was spelled `F: FnMut() -> Fut` with
// `Fut: Future`. `Fut` is a type parameter picked once, outside the call, just
// like `K` in `sort_by_key` (`closure5`), so the future that one call returns
// cannot borrow from the closure that returned it. That matters because an
// `FnMut` closure has `&mut` access to its captures only WHILE a call runs,
// and the future is awaited AFTER the call has returned. A closure such as
// `|| attempt(log)` hands its captured `&mut Vec` to a future that escapes the
// call: "captured variable cannot escape `FnMut` closure body". (Were it
// allowed, two calls could produce two live futures holding the same `&mut`.)
//
// Async closures (`async || ..`, stable since 1.85) come with their own traits,
// `AsyncFn`, `AsyncFnMut` and `AsyncFnOnce`, which are in the prelude. They
// mirror the `Fn` hierarchy, but a call returns a future that may BORROW the
// closure: `AsyncFnMut` keeps the closure mutably borrowed until the returned
// future is gone, so every attempt may reuse the captured `&mut`, and the
// borrow checker still stops two of those futures from being alive at once.
// The bound names the output, not the future type: `F: AsyncFnMut() -> T`.
// Plain closures that return a future implement the `AsyncFn*` traits too, so
// callers that pass one keep compiling after the bound is switched over.
//
// How interviewers probe this: "What do async closures solve that
// `F: FnMut() -> Fut` cannot?" A common follow-up is `Send`: on stable you
// cannot yet require the future of an `AsyncFn*` call to be `Send`, because
// the associated type that names it (`CallRefFuture`) is unstable.

// A flaky operation: it records every call in `log` and succeeds only on the
// third call overall, with the value 7.
async fn attempt(log: &mut Vec<usize>) -> Option<u32> {
    log.push(log.len() + 1);
    (log.len() == 3).then_some(7)
}

// Calls `op` until it returns `Some`, at most `max_attempts` times.
async fn retry<F, Fut>(mut op: F, max_attempts: usize) -> Option<u32>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<u32>>,
{
    // TODO: `fetch` below fails with "captured variable cannot escape `FnMut`
    // closure body", and the test `retry_lends_captures_to_each_call` fails
    // with "async closure does not implement `FnMut` because it captures state
    // from its environment" (neither has an error code). One future type
    // `Fut`, fixed outside the call, can never borrow the closure that
    // produced it. Rebound `op` with the async counterpart of `FnMut`, whose
    // calls may lend the closure's captures to the future they return. Keep
    // the behavior: at most `max_attempts` calls, stop at the first `Some`,
    // and plain closures that return a future must still be accepted. Until
    // `retry` accepts a closure whose futures borrow its captures, this
    // exercise will not compile.
    for _ in 0..max_attempts {
        if let Some(value) = op().await {
            return Some(value);
        }
    }
    None
}

// Retries the flaky operation up to five times, recording each call in `log`.
async fn fetch(log: &mut Vec<usize>) -> Option<u32> {
    // TODO: even with `retry` rebound, this plain closure still returns a
    // future that holds its captured `log`, which is the same "cannot escape
    // `FnMut` closure body" error. Turn it into a closure whose returned future
    // may borrow its captures. No `RefCell`, `Rc`, `Mutex`, cloning or leaking
    // of the log. Until the closure itself is async, this exercise will not
    // compile.
    retry(|| attempt(log), 5).await
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::ready;
    use std::task::{Context, Poll, Waker};

    // A minimal executor, as in `28_futures`, cut down to a single poll.
    // Nothing in this file ever returns `Pending`, so the first poll already
    // finishes; a future that did return `Pending` fails the test instead of
    // spinning forever.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut cx = Context::from_waker(Waker::noop());
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("nothing in this exercise should ever be pending"),
        }
    }

    #[test]
    fn attempt_logs_every_call() {
        let mut log = Vec::new();
        assert_eq!(block_on(attempt(&mut log)), None);
        assert_eq!(block_on(attempt(&mut log)), None);
        assert_eq!(block_on(attempt(&mut log)), Some(7));
        assert_eq!(log, [1, 2, 3]);
    }

    #[test]
    fn fetch_succeeds_on_the_third_attempt() {
        let mut log = Vec::new();
        assert_eq!(block_on(fetch(&mut log)), Some(7));
        // Exactly three calls: `retry` stops at the first success.
        assert_eq!(log, [1, 2, 3]);
    }

    #[test]
    fn fetch_gives_up_after_five_attempts() {
        // Three calls are already logged, so no new call can be the third.
        let mut log = vec![1, 2, 3];
        assert_eq!(block_on(fetch(&mut log)), None);
        assert_eq!(log, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn retry_lends_captures_to_each_call() {
        // Every future this async closure returns borrows the captured `log`
        // and `calls`, which only a lending bound accepts.
        let mut log = Vec::new();
        let mut calls = 0;
        let result = block_on(retry(
            async || {
                calls += 1;
                attempt(&mut log).await
            },
            5,
        ));
        assert_eq!(result, Some(7));
        assert_eq!(calls, 3);
        assert_eq!(log, [1, 2, 3]);
    }

    #[test]
    fn retry_still_accepts_plain_closures() {
        // A plain closure returning a future that owns its data.
        let mut calls = 0;
        let result = block_on(retry(
            || {
                calls += 1;
                ready(None)
            },
            4,
        ));
        assert_eq!(result, None);
        assert_eq!(calls, 4);
    }

    #[test]
    fn zero_attempts_never_call_the_operation() {
        let mut calls = 0;
        let result = block_on(retry(
            || {
                calls += 1;
                ready(Some(1))
            },
            0,
        ));
        assert_eq!(result, None);
        assert_eq!(calls, 0);
    }
}
