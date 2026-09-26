// Module 3 · Leaf futures — part 2: a non-blocking `Sleep` woken by a timer thread.
//
// "What happens if you call `std::thread::sleep` inside an async fn?" is one
// of the most common async interview questions. The answer: `poll` runs ON
// the executor's thread, so while it blocks, that thread polls nothing else.
// On a single-threaded runtime every other task freezes. On tokio's
// multi-threaded runtime you lose a whole worker thread until the call
// returns; block enough of them and everything stalls. `.await` does not
// protect you: the blocking call happens inside a `poll`, and the executor
// cannot take the thread back until `poll` returns. The task is not
// "waiting", it is holding the thread hostage.
//
// A real sleep never blocks. `tokio::time::sleep` returns a leaf future that,
// on its first poll, hands its deadline and its waker to the runtime's TIMER
// (a hierarchical timing wheel, which the runtime checks every few dozen
// tasks and whenever a worker thread parks) and returns `Pending`. The thread
// is free at once. When the deadline passes, the timer calls `wake()`, the
// task goes back on the ready queue, and the next poll sees that the timer
// has fired and returns `Ready`.
//
// Here the timer is a background thread (given below, complete). `register`
// sends it a deadline plus an `Arc<Mutex<TimerEntry>>`. When the deadline
// passes, the thread sets `fired` and wakes whatever waker the entry holds at
// that moment. Blocking is fine on THAT thread: it is not an executor, it
// exists to wait. (That is also what `tokio::task::spawn_blocking` is for:
// run blocking work on a separate thread pool and `.await` its result.)
//
// Your `Sleep` must follow the leaf-future rules from part 1 (`oneshot1`):
//
//   - Register once. A `Sleep` can be polled many times before it fires (a
//     `select!` loop over a pinned `&mut sleep` polls it on every round), and
//     each registration is one more entry for the timer to track.
//   - The latest waker wins. The timer wakes the waker stored in the entry, so
//     every early poll must leave its own waker there.
//   - Check and update under one lock. If you read `fired == false`, release
//     the lock, then store your waker, the timer can fire in between, wake the
//     OLD waker, and leave the new one stranded.
//
// Interviewers follow up with "how does tokio's timer wake the task?", "how
// would you run a blocking or CPU-heavy call from async code?"
// (`spawn_blocking`, or `block_in_place` on the multi-threaded runtime), and
// "what does a timer do with a `Sleep` that is dropped before it fires?"
// (tokio removes the entry; this one lets it fire and wake a task that no
// longer cares, which is harmless).

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

/// What a `Sleep` shares with the timer thread.
struct TimerEntry {
    /// Set by the timer thread once the deadline has passed.
    fired: bool,
    /// The waker the timer thread will wake when it fires.
    waker: Option<Waker>,
}

struct Registration {
    deadline: Instant,
    entry: Arc<Mutex<TimerEntry>>,
}

impl Registration {
    fn fire(self) {
        let waker = {
            let mut entry = self.entry.lock().unwrap();
            entry.fired = true;
            entry.waker.take()
        };
        // Wake with the lock released: the task may be polled on another
        // thread at once, and it will lock this entry.
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// A handle to the timer thread. Cloning it is cheap; the thread exits once
/// every handle (including the ones inside `Sleep`s) is gone.
#[derive(Clone)]
struct Timer {
    registrations: mpsc::Sender<Registration>,
    /// How many times `register` has been called (for the tests).
    registered: Arc<AtomicUsize>,
    /// How many registrations have not fired yet (for the tests).
    unfired: Arc<AtomicUsize>,
}

impl Timer {
    fn new() -> Timer {
        let (registrations, requests) = mpsc::channel();
        let unfired = Arc::new(AtomicUsize::new(0));
        let fired = Arc::clone(&unfired);
        thread::spawn(move || timer_thread(requests, &fired));
        Timer {
            registrations,
            registered: Arc::new(AtomicUsize::new(0)),
            unfired,
        }
    }

    /// Asks the timer thread to fire `entry` once `deadline` has passed.
    fn register(&self, deadline: Instant, entry: Arc<Mutex<TimerEntry>>) {
        self.registered.fetch_add(1, Ordering::SeqCst);
        self.unfired.fetch_add(1, Ordering::SeqCst);
        self.registrations
            .send(Registration { deadline, entry })
            .expect("the timer thread is gone");
    }

    /// How many times `register` has been called so far.
    fn registrations(&self) -> usize {
        self.registered.load(Ordering::SeqCst)
    }

    /// How many registered entries have not fired yet. An entry counts as
    /// fired only after its waker (if any) has been woken.
    fn unfired(&self) -> usize {
        self.unfired.load(Ordering::SeqCst)
    }

    /// A future that completes `duration` from now.
    fn sleep(&self, duration: Duration) -> Sleep {
        self.sleep_until(Instant::now() + duration)
    }

    /// A future that completes at `deadline`.
    fn sleep_until(&self, deadline: Instant) -> Sleep {
        Sleep {
            deadline,
            timer: self.clone(),
            entry: None,
        }
    }
}

// The timer thread: fires due entries, earliest deadline first, then blocks
// until the next deadline or the next registration, whichever comes first.
fn timer_thread(requests: mpsc::Receiver<Registration>, unfired: &AtomicUsize) {
    let mut pending: Vec<Registration> = Vec::new();
    loop {
        pending.sort_by_key(|r| r.deadline);
        let now = Instant::now();
        let due = pending.partition_point(|r| r.deadline <= now);
        for registration in pending.drain(..due) {
            registration.fire();
            unfired.fetch_sub(1, Ordering::SeqCst);
        }
        let request = match pending.first() {
            Some(next) => {
                requests.recv_timeout(next.deadline.saturating_duration_since(Instant::now()))
            }
            None => requests.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match request {
            Ok(registration) => pending.push(registration),
            Err(RecvTimeoutError::Timeout) => {}
            // Every `Timer` handle is gone. Each `Sleep` holds one, so nobody
            // is waiting for the entries that are still pending.
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

struct Sleep {
    deadline: Instant,
    timer: Timer,
    /// `None` until the first poll that has to wait registers with the timer.
    entry: Option<Arc<Mutex<TimerEntry>>>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        // TODO: this blocks the executor's thread until the deadline and then
        // reports `Ready`, so while one task sleeps no other task can run.
        // `a_sleeping_task_lets_other_tasks_run` sees
        // `["a:start", "a:done", "b:ran"]` instead of
        // `["a:start", "b:ran", "a:done"]`, and `shorter_sleeps_finish_first`
        // sees the tasks finish in spawn order instead of deadline order.
        // `each_sleep_registers_once_and_the_task_is_not_busy_polled` counts
        // 1 poll instead of 3, and
        // `repolling_with_a_new_waker_wakes_only_the_new_one` gets `Ready`
        // where it expects `Pending`. Make `Sleep` a leaf future that the
        // timer thread wakes. Requirements:
        //   - never block in `poll` (no `thread::sleep`, no blocking `recv`),
        //     and never wake yourself: a task that awaits one sleep is polled
        //     exactly twice, once to start it and once after the timer fired
        //     (no busy-polling);
        //   - a deadline that has already passed is `Ready` on the first poll;
        //   - otherwise register with `self.timer` exactly ONCE per `Sleep`,
        //     through a `TimerEntry` that holds the waker, keep the entry in
        //     `self.entry`, and return `Pending`;
        //   - a later poll is `Ready` once the entry has `fired`; while it is
        //     early, it leaves ITS waker in the entry (the timer wakes
        //     whatever is stored when it fires), checking `fired` and storing
        //     the waker under one lock;
        //   - don't spawn threads, don't change `Timer` or the tests; no
        //     `unsafe`.
        // Until `Sleep` registers with the timer instead of blocking, the
        // tests will fail.
        let now = Instant::now();
        if now < self.deadline {
            thread::sleep(self.deadline - now);
        }
        Poll::Ready(())
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Wake;

    // `runtime3`'s single-threaded executor, with two guards so that a broken
    // `Sleep` fails the test instead of hanging it. `run` panics if the ready
    // queue is empty while tasks are unfinished and the timer has no entry
    // left to fire: nobody will ever wake those tasks. It also panics after
    // `POLL_BUDGET` polls (someone is busy-polling). `STUCK_AFTER` only
    // bounds a timer thread that never gets to run.
    const STUCK_AFTER: Duration = Duration::from_secs(30);
    const POLL_BUDGET: usize = 10_000;

    type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

    struct Task {
        id: usize,
        future: Mutex<Option<BoxFuture>>,
        queue: mpsc::Sender<Arc<Task>>,
    }

    impl Wake for Task {
        fn wake(self: Arc<Self>) {
            // Fails only once `run` has returned; nobody is listening then.
            let _ = self.queue.send(Arc::clone(&self));
        }
    }

    // Waits for the next message on `rx`. Returns `None` once nothing is
    // queued and `timer` has no entry left to fire: the timer counts an
    // entry as fired only after waking it, so everything it will ever send
    // is on `rx` by then.
    fn next_wake<T>(timer: &Timer, rx: &mpsc::Receiver<T>) -> Option<T> {
        let give_up = Instant::now() + STUCK_AFTER;
        loop {
            if let Ok(message) = rx.recv_timeout(ms(10)) {
                return Some(message);
            }
            if timer.unfired() == 0 {
                return rx.try_recv().ok();
            }
            assert!(
                Instant::now() < give_up,
                "the timer did not fire within {STUCK_AFTER:?}"
            );
        }
    }

    /// Runs every future on this thread, one poll at a time, until all are
    /// done. Returns how often each one was polled.
    fn run(timer: &Timer, futures: Vec<BoxFuture>) -> Vec<usize> {
        let (queue, ready) = mpsc::channel();
        let mut polls = vec![0; futures.len()];
        let mut unfinished = futures.len();
        for (id, future) in futures.into_iter().enumerate() {
            let task = Arc::new(Task {
                id,
                future: Mutex::new(Some(future)),
                queue: queue.clone(),
            });
            queue.send(task).unwrap();
        }
        let mut total = 0;
        while unfinished > 0 {
            let Some(task) = next_wake(timer, &ready) else {
                panic!(
                    "{unfinished} task(s) returned Pending, but the timer has no \
                     entry left to fire, so nothing will ever wake them (does \
                     `Sleep` register with `self.timer` and leave its waker in \
                     the entry?)"
                );
            };
            let mut slot = task.future.lock().unwrap();
            // A wake-up for a task that already finished: nothing to do.
            let Some(future) = slot.as_mut() else {
                continue;
            };
            total += 1;
            assert!(
                total <= POLL_BUDGET,
                "more than {POLL_BUDGET} polls: a future is waking itself on \
                 every poll (busy-polling) instead of waiting for the timer"
            );
            polls[task.id] += 1;
            let waker = Waker::from(Arc::clone(&task));
            if future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_ready()
            {
                *slot = None;
                unfinished -= 1;
            }
        }
        polls
    }

    type Log = Arc<Mutex<Vec<&'static str>>>;

    fn note(log: &Log, event: &'static str) {
        log.lock().unwrap().push(event);
    }

    fn events(log: &Log) -> Vec<&'static str> {
        log.lock().unwrap().clone()
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn poll_once(sleep: &mut Sleep, waker: &Waker) -> Poll<()> {
        Pin::new(sleep).poll(&mut Context::from_waker(waker))
    }

    // A waker that reports its name on a channel when it is woken.
    struct NamedWaker {
        name: &'static str,
        woken: mpsc::Sender<&'static str>,
    }

    impl Wake for NamedWaker {
        fn wake(self: Arc<Self>) {
            let _ = self.woken.send(self.name);
        }
    }

    fn named_waker(name: &'static str, woken: &mpsc::Sender<&'static str>) -> Waker {
        Waker::from(Arc::new(NamedWaker {
            name,
            woken: woken.clone(),
        }))
    }

    #[test]
    fn a_sleeping_task_lets_other_tasks_run() {
        let timer = Timer::new();
        let log = Log::default();
        let (t, l) = (timer.clone(), Arc::clone(&log));
        let a: BoxFuture = Box::pin(async move {
            note(&l, "a:start");
            t.sleep(ms(100)).await;
            note(&l, "a:done");
        });
        let l = Arc::clone(&log);
        let b: BoxFuture = Box::pin(async move {
            note(&l, "b:ran");
        });
        run(&timer, vec![a, b]);
        assert_eq!(
            events(&log),
            ["a:start", "b:ran", "a:done"],
            "while `a` sleeps, the executor must be free to run `b`"
        );
    }

    #[test]
    fn shorter_sleeps_finish_first() {
        // Three tasks sleep at the same time. The earliest deadline must end
        // first, whatever order the tasks were spawned in. The deadlines are
        // fixed up front, so the order does not depend on how fast the tasks
        // get polled (as long as all three start within the first 200 ms).
        let timer = Timer::new();
        let log = Log::default();
        let base = Instant::now();
        let plan = [
            ("a:start", 600, "a:done"),
            ("b:start", 200, "b:done"),
            ("c:start", 400, "c:done"),
        ];
        let tasks = plan
            .into_iter()
            .map(|(start, millis, done)| {
                let (t, l) = (timer.clone(), Arc::clone(&log));
                Box::pin(async move {
                    note(&l, start);
                    t.sleep_until(base + ms(millis)).await;
                    note(&l, done);
                }) as BoxFuture
            })
            .collect();
        run(&timer, tasks);
        assert_eq!(
            events(&log),
            [
                "a:start", "b:start", "c:start", "b:done", "c:done", "a:done"
            ],
            "the three sleeps must overlap, so they end in deadline order"
        );
    }

    #[test]
    fn a_sleep_lasts_at_least_its_duration() {
        let timer = Timer::new();
        let started = Instant::now();
        let t = timer.clone();
        run(&timer, vec![Box::pin(async move { t.sleep(ms(80)).await })]);
        let elapsed = started.elapsed();
        assert!(
            elapsed >= ms(80),
            "the sleep ended after {elapsed:?}, before its 80 ms were up"
        );
    }

    #[test]
    fn each_sleep_registers_once_and_the_task_is_not_busy_polled() {
        let timer = Timer::new();
        let t = timer.clone();
        let polls = run(
            &timer,
            vec![Box::pin(async move {
                t.sleep(ms(30)).await;
                t.sleep(ms(30)).await;
            })],
        );
        // Poll 1 starts the first sleep, poll 2 comes after it fired and
        // starts the second, and poll 3 comes after that one fired.
        assert_eq!(
            polls,
            [3],
            "a sleeping task is polled once to start each sleep and once \
             after the last one fired, never in between"
        );
        assert_eq!(timer.registrations(), 2, "one registration per `Sleep`");
    }

    #[test]
    fn a_zero_duration_sleep_is_ready_on_the_first_poll() {
        let timer = Timer::new();
        let mut sleep = timer.sleep(Duration::ZERO);
        assert_eq!(poll_once(&mut sleep, Waker::noop()), Poll::Ready(()));
    }

    #[test]
    fn repolling_with_a_new_waker_wakes_only_the_new_one() {
        let timer = Timer::new();
        let (woken_tx, woken_rx) = mpsc::channel();
        let waker_a = named_waker("A", &woken_tx);
        let waker_b = named_waker("B", &woken_tx);
        let mut sleep = timer.sleep(ms(1_000));

        assert_eq!(poll_once(&mut sleep, &waker_a), Poll::Pending);
        // Polled again early, this time with waker B (say, the `Sleep` was
        // moved into another task, or a combinator now polls it).
        assert_eq!(
            poll_once(&mut sleep, &waker_b),
            Poll::Pending,
            "the second has not passed yet"
        );
        assert_eq!(
            timer.registrations(),
            1,
            "an early re-poll must not register again"
        );

        assert_eq!(
            next_wake(&timer, &woken_rx),
            Some("B"),
            "when it fires, the timer must wake the waker of the LATEST poll"
        );
        assert_eq!(poll_once(&mut sleep, &waker_b), Poll::Ready(()));
        assert!(
            woken_rx.try_recv().is_err(),
            "waker A is stale and must not be woken"
        );
    }
}
