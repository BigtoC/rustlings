// Module 3 · Async — the `Future` trait, part 4: the sugar.
//
// You just wrote `Sequence` by hand. The good news: you almost never have to.
// An `async fn` (or `async {}` block) is compiled into exactly that kind of
// anonymous `Future` state machine, and every `.await` is one of its suspension
// points. `double(a).await` polls the `double` future until it is `Ready` and
// then evaluates to the value inside.
//
// Prove the sugar works by finishing `sum_of_doubles`.

use std::future::Future;

async fn double(x: i32) -> i32 {
    x * 2
}

// TODO: Complete this `async fn` so that it awaits `double(a)` and `double(b)`
// and returns their sum. Use `.await` on each call, e.g. `let x = double(a).await;`.
async fn sum_of_doubles(a: i32, b: i32) -> i32 {
    // The function currently returns `()`, but it is declared to return `i32`,
    // so it will not compile until you await the two doublings and add them.
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::thread::{self, Thread};

    fn block_on<F: Future>(future: F) -> F::Output {
        struct ThreadWaker(Thread);
        impl Wake for ThreadWaker {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }

        let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(value) => return value,
                Poll::Pending => thread::park(),
            }
        }
    }

    #[test]
    fn awaits_and_sums() {
        assert_eq!(block_on(sum_of_doubles(3, 4)), 14);
    }
}
