// Traits & Abstraction · Testing seams — part 2: a token-bucket rate limiter on an injected clock (E0277).
//
// "Design a rate limiter" is a standard backend round, and "now test it" is
// the follow-up. A TOKEN BUCKET holds up to `capacity` tokens. Every request
// takes one or is denied, and a new token drips in every `refill_every`. So
// the limiter allows bursts of up to `capacity` requests while holding the
// long-run rate at one request per `refill_every`. The textbook definition
// treats the level as continuous: after `t` of clock time it is
// `min(capacity, level + t / refill_every)`, and a request needs a whole
// token.
//
// Time is a dependency like any other. A limiter that calls `Instant::now()`
// directly can only be tested with `thread::sleep`, and such a test is slow
// and flaky: it depends on how busy the machine is. The fix is the seam from
// part 1, a `Clock` trait. Production passes `MonotonicClock`; tests pass a
// `FakeClock` and move time by hand, so "an hour passes" takes no time at all
// and every run sees exactly the same instants. There is no sleep anywhere in
// this file.
//
// The fake is a handle: the bucket owns one clone, the test keeps another,
// and both see the same time. `now` takes `&self`, so moving the time needs
// shared ownership plus interior mutability, as in part 1. In one thread
// that is `Rc<Cell<Duration>>`.
//
// Part A is the refill arithmetic, and the starter gets it wrong in the most
// common way. It stores whole tokens plus `last`, the time up to which it
// has credited refills. It divides the time since `last` by `refill_every`,
// adds the whole intervals, and then sets `last = now`. That throws away the
// leftover fraction of an interval on EVERY call. At 1 token per second, a
// client that polls every 400 ms is credited nothing for each 400 ms step,
// forever. It asks more often than the rate allows, so it should get exactly
// the rate, one token per second; instead it is starved completely. `last`
// may only move forward by the time that was actually turned into tokens.
// The starter also forgets the capacity: an idle hour must not turn into a
// burst of 3600 requests. Time spent FULL is the one exception to "keep the
// leftover": a full bucket has nowhere to put it, so that time is simply
// gone (the `min` in the definition above).
//
// Part B shares the limiter. A server keeps one bucket per client key, and
// many request threads call `try_acquire(&self, key)` on one limiter at once.
// A type can be shared between threads only if it is `Sync`, and a `RefCell`
// is not: its borrow flag is a plain, unsynchronized counter. `Rc` is neither
// `Send` nor `Sync` (`35_error_design/err5` met both markers; `30_send_sync`,
// later in the course, covers them in depth). So the per-key map needs a
// lock, and the fake clock, which the test keeps moving while the limiter
// holds a clone, needs a thread-safe handle. Interior mutability comes in a
// single-threaded flavor (`Cell`, `RefCell`, with `Rc` for sharing) and a
// thread-safe one (`Mutex`, `RwLock`, atomics, with `Arc`), and a test double
// must be as thread-safe as the code it stands in for. rustc rejects the
// tests over Part B first; once they compile, they report Part A.
//
// How interviewers probe this: "Token bucket or sliding window, and what
// does each cost in memory?", "How do you test it without sleeping?",
// "Generic `C: Clock` or `Arc<dyn Clock + Send + Sync>`?" (generic: static
// dispatch and no allocation, but the type parameter spreads into every type
// that holds a limiter; `dyn`: one concrete type and a vtable call per
// read), "What happens to the refill while the bucket is full?", "What if
// the clock goes backwards?", and "Where does the lock go, and what does it
// serialize?".

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// The seam: where "now" comes from.
trait Clock {
    // Time since some fixed, arbitrary origin. Only differences matter.
    fn now(&self) -> Duration;
}

// The production clock. `Instant` is monotonic: it never goes backwards.
#[derive(Clone, Copy)]
struct MonotonicClock {
    origin: Instant,
}

impl MonotonicClock {
    fn new() -> Self {
        MonotonicClock {
            origin: Instant::now(),
        }
    }
}

impl Clock for MonotonicClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

// The fake clock. Time only moves when a test says so.
//
// The thread-safe twin of `Rc<Cell<..>>`: `Arc` shares the time between the
// clones with an atomic reference count, and the `Mutex` lets any of them
// read or move it through `&self`, from any thread. A `Mutex<T>` is
// `Send + Sync` whenever `T: Send`, so `Arc<Mutex<Duration>>` is both. Each
// `new()` allocates its own `Arc`, so separate clocks stay independent. (An
// `Arc<AtomicU64>` of nanoseconds works too, without a lock.)
#[derive(Clone)]
struct FakeClock {
    now: Arc<Mutex<Duration>>,
}

impl FakeClock {
    fn new() -> Self {
        FakeClock {
            now: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }

    fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }

    // Jumps to `to`, which may be in the past.
    fn set(&self, to: Duration) {
        *self.now.lock().unwrap() = to;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}

// ---- Part A: the bucket ----

struct TokenBucket<C: Clock> {
    clock: C,
    capacity: u32,
    refill_every: Duration,
    tokens: u32,
    // The point in time up to which refills have been credited.
    last: Duration,
}

impl<C: Clock> TokenBucket<C> {
    // A new bucket starts full.
    fn new(clock: C, capacity: u32, refill_every: Duration) -> Self {
        assert!(!refill_every.is_zero(), "refill_every must be positive");
        let last = clock.now();
        TokenBucket {
            clock,
            capacity,
            refill_every,
            tokens: capacity,
            last,
        }
    }

    // Takes one token if there is one.
    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }

    fn refill(&mut self) {
        let now = self.clock.now();
        // A reading earlier than `last` counts as "no time passed".
        let elapsed = now.saturating_sub(self.last);
        let intervals = elapsed.as_nanos() / self.refill_every.as_nanos();
        let added = u32::try_from(intervals).unwrap_or(u32::MAX);
        self.tokens = self.tokens.saturating_add(added).min(self.capacity);
        if self.tokens == self.capacity {
            // Full: there is nowhere to put the leftover, so it is dropped and
            // the next interval starts now. `max` keeps a backwards reading
            // from moving `last` into the past, where the same stretch of time
            // would be credited again.
            self.last = self.last.max(now);
        } else {
            // Not full, so `added` is below `capacity` (no saturation) and
            // `last` moves forward by exactly the time that became tokens. The
            // leftover part of an interval stays between `last` and `now`.
            self.last += self.refill_every * added;
        }
    }
}

// ---- Part B: one bucket per key, shared by many threads ----

// A `Mutex` is `Sync` whenever what it guards is `Send`, and the clock field
// is `Sync` now too, so the whole limiter can be shared by `&` between
// threads. One lock guard covers the lookup, the refill and the decrement, so
// no other thread can get between "is there a token?" and "take it".
struct KeyedLimiter<C: Clock + Clone> {
    clock: C,
    capacity: u32,
    refill_every: Duration,
    buckets: Mutex<HashMap<String, TokenBucket<C>>>,
}

impl<C: Clock + Clone> KeyedLimiter<C> {
    fn new(clock: C, capacity: u32, refill_every: Duration) -> Self {
        KeyedLimiter {
            clock,
            capacity,
            refill_every,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    // Each key gets its own bucket, created full on first use.
    fn try_acquire(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(self.clock.clone(), self.capacity, self.refill_every)
        });
        bucket.try_acquire()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;

    const SECOND: Duration = Duration::from_secs(1);

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    // Takes tokens until the bucket says no; returns how many it got.
    fn drain<C: Clock>(bucket: &mut TokenBucket<C>) -> u32 {
        let mut taken = 0;
        while bucket.try_acquire() {
            taken += 1;
            assert!(taken <= 10_000, "the bucket never runs dry");
        }
        taken
    }

    // ---- Part A ----

    #[test]
    fn a_new_bucket_allows_a_burst_up_to_capacity_then_denies() {
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 3, SECOND);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn a_token_comes_back_exactly_after_refill_every() {
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 2, SECOND);
        assert_eq!(drain(&mut bucket), 2);
        clock.advance(SECOND - Duration::from_nanos(1));
        assert!(!bucket.try_acquire(), "one nanosecond too early");
        clock.advance(Duration::from_nanos(1));
        assert!(bucket.try_acquire(), "exactly `refill_every` mints a token");
        assert!(!bucket.try_acquire(), "and only one");
    }

    #[test]
    fn polling_faster_than_the_refill_rate_still_gets_tokens() {
        // 1 token per second, and a client that asks every 400 ms. It asks
        // more often than the rate allows, so some polls fail, but it must get
        // a token whenever another whole second of clock time has built up.
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 2, SECOND);
        assert_eq!(drain(&mut bucket), 2);
        let mut successes = Vec::new();
        for step in 1..=10 {
            clock.advance(ms(400));
            if bucket.try_acquire() {
                successes.push(step * 400);
            }
        }
        // Tokens are minted at 1 s, 2 s, 3 s and 4 s of clock time, and the
        // client picks each one up at its next poll.
        assert_eq!(successes, [1200, 2000, 3200, 4000]);
    }

    #[test]
    fn the_long_run_rate_is_exact() {
        // 1 token per 300 ms, polled every 70 ms: 900 polls cover 63 s of
        // clock time, so after the initial burst the client must get exactly
        // 63_000 / 300 = 210 more tokens. Every leftover counts.
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 2, ms(300));
        assert_eq!(drain(&mut bucket), 2);
        let mut granted = 0;
        for _ in 0..900 {
            clock.advance(ms(70));
            if bucket.try_acquire() {
                granted += 1;
            }
        }
        assert_eq!(granted, 210);
    }

    #[test]
    fn an_idle_hour_refills_to_capacity_and_no_further() {
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 3, SECOND);
        assert_eq!(drain(&mut bucket), 3);
        clock.advance(Duration::from_secs(3600) + ms(500));
        assert_eq!(drain(&mut bucket), 3, "at most `capacity` after idling");
        // The idle hour was spent, not saved up, and so was the extra half
        // second: the bucket had been full for most of the hour. The next
        // token needs a whole `refill_every` after the drain.
        clock.advance(ms(999));
        assert!(!bucket.try_acquire(), "the next token is due at 3601.5 s");
        clock.advance(ms(1));
        assert_eq!(drain(&mut bucket), 1);
    }

    #[test]
    fn a_full_bucket_does_not_bank_idle_time() {
        // Never used: the bucket sits full for 10.5 s. When it is drained at
        // 10.5 s, it has nothing saved up, not even the extra half second.
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 2, SECOND);
        clock.advance(ms(10_500));
        assert_eq!(drain(&mut bucket), 2);
        clock.advance(ms(999));
        assert!(!bucket.try_acquire(), "the next token is due at 11.5 s");
        clock.advance(ms(1));
        assert_eq!(drain(&mut bucket), 1);
    }

    #[test]
    fn a_full_bucket_does_not_bank_part_of_an_interval_either() {
        // Full for 0.7 s, less than one interval, so no whole token is due
        // yet when it is drained. Those 0.7 s are gone all the same: the next
        // token is due a whole second after the drain, at 1.7 s, not at 1 s.
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 2, SECOND);
        clock.advance(ms(700));
        assert_eq!(drain(&mut bucket), 2);
        clock.advance(ms(300));
        assert!(!bucket.try_acquire(), "0.3 s after the drain is too early");
        clock.advance(ms(699));
        assert!(!bucket.try_acquire(), "the next token is due at 1.7 s");
        clock.advance(ms(1));
        assert_eq!(drain(&mut bucket), 1);
    }

    #[test]
    fn a_very_long_idle_period_does_not_overflow() {
        // A year at one token per nanosecond is about 3 * 10^16 intervals,
        // far more than `u32::MAX`. Still just `capacity`, and no panic.
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 4, Duration::from_nanos(1));
        assert_eq!(drain(&mut bucket), 4);
        clock.advance(Duration::from_secs(365 * 24 * 3600));
        assert_eq!(drain(&mut bucket), 4);
        clock.advance(Duration::from_nanos(3));
        assert_eq!(drain(&mut bucket), 3);
    }

    #[test]
    fn a_clock_that_goes_backwards_mints_nothing() {
        let clock = FakeClock::new();
        clock.set(Duration::from_secs(10));
        let mut bucket = TokenBucket::new(clock.clone(), 2, SECOND);
        assert_eq!(drain(&mut bucket), 2);
        // Seven seconds back: no panic, and no token.
        clock.set(Duration::from_secs(3));
        assert!(!bucket.try_acquire());
        // Back where it was: that time was already credited once.
        clock.set(Duration::from_secs(10));
        assert!(!bucket.try_acquire());
        // Only NEW time mints tokens.
        clock.set(Duration::from_secs(11));
        assert_eq!(drain(&mut bucket), 1);
    }

    #[test]
    fn a_jittery_clock_cannot_mint_tokens() {
        // A clock that keeps jumping back and forth (two unsynchronized time
        // sources, say) must not become a token printer, whether the bucket
        // is empty or full while it jumps.
        let clock = FakeClock::new();
        clock.set(Duration::from_secs(100));
        let mut bucket = TokenBucket::new(clock.clone(), 5, SECOND);
        // Full: one token taken at 90 s, one at 100 s. Nothing is minted in
        // between, because 90..100 s is not new time.
        clock.set(Duration::from_secs(90));
        assert!(bucket.try_acquire());
        clock.set(Duration::from_secs(100));
        assert!(bucket.try_acquire());
        assert_eq!(drain(&mut bucket), 3);
        // Empty: ten more round trips, still nothing.
        for _ in 0..10 {
            clock.set(Duration::from_secs(90));
            assert!(!bucket.try_acquire());
            clock.set(Duration::from_secs(100));
            assert!(!bucket.try_acquire());
        }
        clock.set(Duration::from_secs(102));
        assert_eq!(drain(&mut bucket), 2);
    }

    #[test]
    fn a_zero_capacity_bucket_never_allows_anything() {
        let clock = FakeClock::new();
        let mut bucket = TokenBucket::new(clock.clone(), 0, SECOND);
        assert!(!bucket.try_acquire());
        clock.advance(Duration::from_secs(3600));
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn works_with_the_real_clock() {
        // A burst never waits for the clock, so this is deterministic.
        let mut bucket = TokenBucket::new(MonotonicClock::new(), 2, Duration::from_secs(3600));
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }

    // ---- Part B ----

    #[test]
    fn fake_clock_clones_share_time_and_new_clocks_are_independent() {
        let clock = FakeClock::new();
        let handle = clock.clone();
        clock.advance(ms(1500));
        assert_eq!(handle.now(), ms(1500));
        handle.set(ms(200));
        assert_eq!(clock.now(), ms(200));
        let other = FakeClock::new();
        assert_eq!(other.now(), Duration::ZERO);
        other.advance(SECOND);
        assert_eq!(clock.now(), ms(200));
    }

    #[test]
    fn keys_have_independent_buckets() {
        let clock = FakeClock::new();
        let limiter = KeyedLimiter::new(clock.clone(), 2, SECOND);
        assert!(limiter.try_acquire("alice"));
        assert!(limiter.try_acquire("alice"));
        assert!(!limiter.try_acquire("alice"));
        // Alice's burst does not touch Bob's bucket.
        assert!(limiter.try_acquire("bob"));
        clock.advance(SECOND);
        assert!(limiter.try_acquire("alice"));
        assert!(!limiter.try_acquire("alice"));
        assert!(limiter.try_acquire("bob"));
        assert!(limiter.try_acquire("bob"));
        assert!(!limiter.try_acquire("bob"));
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn the_limiter_is_send_and_sync() {
        assert_send_sync::<FakeClock>();
        assert_send_sync::<KeyedLimiter<FakeClock>>();
        assert_send_sync::<KeyedLimiter<MonotonicClock>>();
    }

    #[test]
    fn eight_threads_at_the_same_instant_get_exactly_capacity_tokens() {
        const THREADS: usize = 8;
        let clock = FakeClock::new();
        let limiter = KeyedLimiter::new(clock.clone(), 3, SECOND);
        let start = Barrier::new(THREADS);
        for round in 0..20 {
            // A fresh key each round, and all threads ask for it at once.
            let key = format!("client-{round}");
            let granted = thread::scope(|s| {
                let workers: Vec<_> = (0..THREADS)
                    .map(|_| {
                        s.spawn(|| {
                            start.wait();
                            limiter.try_acquire(&key)
                        })
                    })
                    .collect();
                workers
                    .into_iter()
                    .map(|worker| worker.join().unwrap())
                    .filter(|&allowed| allowed)
                    .count()
            });
            assert_eq!(granted, 3, "round {round}");
        }
        // The test thread moves the shared clock, and the buckets, which
        // are used from other threads, see it.
        clock.advance(SECOND);
        let granted = thread::scope(|s| {
            let workers: Vec<_> = (0..THREADS)
                .map(|_| s.spawn(|| limiter.try_acquire("client-0")))
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .filter(|&allowed| allowed)
                .count()
        });
        assert_eq!(granted, 1);
    }
}
