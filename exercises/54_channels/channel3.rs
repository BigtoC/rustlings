// Module 4 · Channels — part 3: an actor that owns its state and answers through reply channels (E0559, E0026).
//
// `Arc<Mutex<T>>` shares state by letting every thread lock it. An ACTOR
// shares state by not sharing it at all: one thread owns the state outright
// and is the only code that ever touches it. Every other thread sends it
// commands over a channel, and the actor handles them one at a time, in
// arrival order. There is no lock to forget, to hold too long or to take in
// the wrong order, and no one can keep a reference into the map, because no
// one else has one. Each command runs from start to finish before the next
// one begins, so it is as atomic as a critical section.
//
// A channel carries messages one way, though, and a lookup needs an answer.
// The standard trick is to put the way back INTO the request: the caller
// creates a fresh channel for each question, sends its `Sender` half along
// with the key, and waits on the `Receiver` half. The actor answers on the
// channel that came with the command, so an answer can never reach the
// wrong caller, however many callers wait at once and in whatever order the
// actor answers them. This is the tokio actor pattern exactly: an `mpsc`
// mailbox, and a `oneshot` channel inside each request. std has no stable
// one-shot channel (`std::sync::oneshot` is still unstable), so a plain
// `mpsc::channel` that is used once stands in for it.
//
// The reply channel also reports failure. If the actor dies, or drops the
// request without answering, the reply `Sender` is dropped, and the
// caller's `recv()` fails with `RecvError` instead of waiting forever. The
// other direction matters just as much. A caller that gave up (it timed out,
// or was canceled) has dropped its `Receiver`, so the actor's reply fails.
// The actor must shrug that off: if it `unwrap()`ed the reply, one impatient
// client would take the service down for everybody.
//
// Shutdown comes for free, as in `channel2`: the actor's
// `for cmd in commands` loop ends when the last `Handle`, which holds the
// last `Sender<Cmd>`, is dropped.
//
// Interviewers ask: channels or `Arc<Mutex<T>>`? How does a caller get an
// answer back from an actor? What happens to a waiting caller when the
// actor dies? And is a `get` followed by a `put` from one client atomic? (No:
// another client's command can land between the two. Whatever must be
// atomic, such as "add 1 to this counter", has to be ONE command.)

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

/// The actor is gone: it shut down or died before it answered.
#[derive(Debug, PartialEq, Eq)]
struct Gone;

/// The commands the actor understands.
enum Cmd {
    /// Store `value` under `key`. Fire and forget: there is no answer.
    Put { key: String, value: u32 },
    // TODO: The tests build `Cmd::Get { key, reply }`, so they fail to
    // compile with E0559 "variant `Cmd::Get` has no field named `reply`"
    // (and E0026 "variant `Cmd::Get` does not have a field named `reply`"
    // where they match on it). The actor can look the key up, but a command
    // on a one-way channel has no way back to whoever asked. Requirements:
    //   - the request itself carries the way back: the tests pass a
    //     `Sender<Option<u32>>` as `reply`, and read the answer from its
    //     `Receiver`;
    //   - one answer per request, sent on that request's `reply` only.
    // rustc then points at the two places that must use the new field (the
    // TODOs in `run_actor` and `Handle::get`). Until you have fixed all
    // three, this exercise will not compile.
    /// Look `key` up.
    Get { key: String },
}

/// The actor. It is the only code that ever touches `map`: it handles one
/// command at a time, in arrival order, and returns the map once every
/// Sender of `commands` is gone.
fn run_actor(commands: Receiver<Cmd>) -> HashMap<String, u32> {
    let mut map = HashMap::new();
    for cmd in commands {
        match cmd {
            Cmd::Put { key, value } => {
                map.insert(key, value);
            }
            Cmd::Get { key } => {
                // TODO: once `Get` has its `reply` field, this pattern gives
                // E0027 "pattern does not mention field `reply`": the actor
                // has the answer, but nowhere to send it. Requirements:
                // answer on the request's own `reply`, and keep serving if
                // that caller has stopped waiting (a test drops its
                // `Receiver` before the actor answers, then expects the actor
                // to go on serving everyone else). Until you make the actor
                // answer on `reply`, this exercise will not compile.
                let _found = map.get(&key).copied();
            }
        }
    }
    map
}

/// A client's end of the actor. Cloning one only clones a Sender, so every
/// thread that talks to the actor gets its own clone.
#[derive(Clone)]
struct Handle {
    tx: Sender<Cmd>,
}

impl Handle {
    /// Stores `value` under `key` without waiting for the actor. A later
    /// `get` from the same thread still sees it: the messages of one thread
    /// arrive in the order it sent them.
    fn put(&self, key: &str, value: u32) -> Result<(), Gone> {
        let cmd = Cmd::Put {
            key: key.to_owned(),
            value,
        };
        self.tx.send(cmd).map_err(|_| Gone)
    }

    /// Asks the actor for the value under `key`, and waits for the answer.
    fn get(&self, key: &str) -> Result<Option<u32>, Gone> {
        // TODO: once `Get` has its `reply` field, this initializer gives
        // E0063 "missing field `reply` in initializer of `Cmd`": the question
        // goes out, but no answer can come back, so this always says
        // `Ok(None)`. Requirements:
        //   - return what the actor answered: `Ok(Some(value))`, or
        //     `Ok(None)` for a key it does not have;
        //   - return `Err(Gone)` if the actor is gone before it gets the
        //     request or drops it unanswered, and never hang or panic then;
        //   - keep `Handle` cheap to clone and shareable between threads: no
        //     `Receiver` or lock inside it, just its one `tx` field (the tests
        //     build a `Handle { tx }` of their own).
        // Until you make `get` wait for the actor's answer, this exercise will
        // not compile.
        let cmd = Cmd::Get {
            key: key.to_owned(),
        };
        self.tx.send(cmd).map_err(|_| Gone)?;
        Ok(None)
    }
}

/// Starts the actor on a thread of its own. The actor ends once every
/// `Handle` has been dropped, and joining it returns its final map.
fn spawn_actor() -> (Handle, JoinHandle<HashMap<String, u32>>) {
    let (tx, rx) = mpsc::channel();
    let actor = thread::Builder::new()
        .name("actor".to_owned())
        .spawn(move || run_actor(rx))
        .expect("failed to spawn the actor thread");
    (Handle { tx }, actor)
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;
    use std::sync::mpsc::{RecvError, RecvTimeoutError};
    use std::time::Duration;

    const WATCHDOG: Duration = Duration::from_secs(10);

    // Runs `f` on a thread of its own and returns its result, or fails the
    // test with `hung` if `f` has not returned within `WATCHDOG`. A caller
    // waiting for an answer that never comes would otherwise hang the test
    // forever.
    fn within<R: Send + 'static>(hung: &str, f: impl FnOnce() -> R + Send + 'static) -> R {
        let (done_tx, done_rx) = mpsc::channel();
        // Named like the test's own thread, so a panic message names the test.
        let name = thread::current().name().unwrap_or("test").to_owned();
        let worker = thread::Builder::new()
            .name(name)
            .spawn(move || {
                let _ = done_tx.send(f());
            })
            .unwrap();
        match done_rx.recv_timeout(WATCHDOG) {
            Ok(result) => result,
            // `f` panicked, and `done_tx` was dropped while it unwound: fail
            // this test with `f`'s own panic.
            Err(RecvTimeoutError::Disconnected) => match worker.join() {
                Err(payload) => panic::resume_unwind(payload),
                Ok(()) => unreachable!("the worker sends its result before it ends"),
            },
            Err(RecvTimeoutError::Timeout) => panic!("{hung} (watchdog: {WATCHDOG:?})"),
        }
    }

    fn join_or_rethrow<T>(thread: JoinHandle<T>) -> T {
        thread
            .join()
            .unwrap_or_else(|payload| panic::resume_unwind(payload))
    }

    #[test]
    fn a_get_sees_the_latest_put() {
        within("a `get` never got its answer", || {
            let (handle, _actor) = spawn_actor();
            assert_eq!(
                handle.get("a"),
                Ok(None),
                "nothing is stored under \"a\" yet"
            );
            handle.put("a", 1).unwrap();
            assert_eq!(handle.get("a"), Ok(Some(1)));
            handle.put("a", 2).unwrap();
            handle.put("b", 3).unwrap();
            assert_eq!(handle.get("a"), Ok(Some(2)));
            assert_eq!(handle.get("b"), Ok(Some(3)));
        });
    }

    #[test]
    fn every_request_carries_its_own_reply_channel() {
        within(
            "the actor never answered on the request's `reply` channel, or kept it open after answering",
            || {
                let (handle, _actor) = spawn_actor();
                handle.put("k", 7).unwrap();
                let (reply, answer) = mpsc::channel();
                let request = Cmd::Get {
                    key: "k".to_owned(),
                    reply,
                };
                handle.tx.send(request).unwrap();
                assert_eq!(answer.recv(), Ok(Some(7)));
                // One answer per request. Then the actor drops the `reply`
                // Sender, so another `recv` reports the disconnect instead of
                // waiting.
                assert_eq!(answer.recv(), Err(RecvError));
            },
        );
    }

    #[test]
    fn replies_reach_their_own_caller_whatever_the_order() {
        within("a `get` never got its answer", || {
            // A stand-in actor: it collects two requests, then answers the
            // second one first.
            let (tx, commands) = mpsc::channel();
            let handle = Handle { tx };
            let actor = thread::spawn(move || {
                let mut pending = Vec::new();
                for cmd in commands.iter().take(2) {
                    match cmd {
                        Cmd::Get { key, reply } => pending.push((key, reply)),
                        Cmd::Put { .. } => panic!("this test sends only gets"),
                    }
                }
                for (key, reply) in pending.into_iter().rev() {
                    let value = if key == "a" { 10 } else { 20 };
                    reply.send(Some(value)).unwrap();
                }
            });
            let a = {
                let handle = handle.clone();
                thread::spawn(move || handle.get("a"))
            };
            let b = {
                let handle = handle.clone();
                thread::spawn(move || handle.get("b"))
            };
            assert_eq!(
                join_or_rethrow(a),
                Ok(Some(10)),
                "the caller asking for \"a\" got another caller's answer"
            );
            assert_eq!(
                join_or_rethrow(b),
                Ok(Some(20)),
                "the caller asking for \"b\" got another caller's answer"
            );
            join_or_rethrow(actor);
        });
    }

    // One client for the next test. It writes keys of its own, reads each one
    // back at once, and keeps overwriting the shared key "last".
    fn client(handle: Handle, t: u32) {
        for i in 0..250 {
            let key = format!("t{t}/{i}");
            handle.put(&key, t * 1000 + i).unwrap();
            // Our `put` entered the same queue earlier, so the actor has
            // handled it by the time it reads this `get`.
            assert_eq!(
                handle.get(&key),
                Ok(Some(t * 1000 + i)),
                "a client must read its own writes"
            );
            handle.put("last", t).unwrap();
        }
    }

    #[test]
    fn four_threads_share_one_actor_and_read_their_own_writes() {
        within(
            "a `get` never got its answer, or the actor never ended",
            || {
                let (handle, actor) = spawn_actor();
                let clients: Vec<_> = (0..4)
                    .map(|t| {
                        let handle = handle.clone();
                        thread::spawn(move || client(handle, t))
                    })
                    .collect();
                for client in clients {
                    join_or_rethrow(client);
                }
                for t in 0..4 {
                    for i in 0..250 {
                        let key = format!("t{t}/{i}");
                        assert_eq!(handle.get(&key), Ok(Some(t * 1000 + i)));
                    }
                }
                let last = handle.get("last").unwrap();
                assert!(
                    matches!(last, Some(0..4)),
                    "\"last\" must hold the number of one client, got {last:?}"
                );
                drop(handle);
                let map = join_or_rethrow(actor);
                assert_eq!(map.len(), 4 * 250 + 1, "every key, plus \"last\"");
            },
        );
    }

    #[test]
    fn the_actor_ends_once_every_handle_is_dropped() {
        let (handle, actor) = spawn_actor();
        let clones = [handle.clone(), handle.clone(), handle.clone()];
        handle.put("a", 1).unwrap();
        drop(handle);
        clones[2].put("b", 2).unwrap();
        // Three Handles are still alive, so `for cmd in rx` cannot have ended.
        assert!(
            !actor.is_finished(),
            "the actor stopped while Handles were still alive"
        );
        drop(clones);
        let map = within(
            "the actor never ended after every Handle was dropped: a Sender<Cmd> is still alive",
            move || join_or_rethrow(actor),
        );
        assert_eq!(
            map,
            HashMap::from([("a".to_owned(), 1), ("b".to_owned(), 2)])
        );
    }

    #[test]
    fn a_caller_that_gave_up_does_not_take_the_actor_down() {
        within(
            "the actor stopped answering, or never ended after the last Handle was dropped",
            || {
                let (handle, actor) = spawn_actor();
                handle.put("k", 1).unwrap();
                // This caller stops waiting before the actor answers (it timed
                // out, or was canceled): its Receiver is already gone.
                let (reply, answer) = mpsc::channel();
                drop(answer);
                let request = Cmd::Get {
                    key: "k".to_owned(),
                    reply,
                };
                handle.tx.send(request).unwrap();
                // The actor's answer goes nowhere. Everyone else must still be
                // served.
                assert_eq!(
                    handle.get("k"),
                    Ok(Some(1)),
                    "the actor died answering a caller that had stopped waiting"
                );
                drop(handle);
                assert!(actor.join().is_ok(), "the actor panicked");
            },
        );
    }

    #[test]
    fn get_reports_gone_when_the_actor_dies_mid_request() {
        within("`get` kept waiting for an actor that had died", || {
            // A stand-in actor that takes one request and dies without
            // answering it.
            let (tx, commands) = mpsc::channel();
            let handle = Handle { tx };
            let actor = thread::spawn(move || {
                let request = commands.recv();
                drop(request);
            });
            assert_eq!(
                handle.get("k"),
                Err(Gone),
                "the actor dropped the request unanswered: that is `Gone`, \
                 not a missing key"
            );
            join_or_rethrow(actor);
            // Now nothing can reach the actor at all.
            assert_eq!(handle.put("k", 1), Err(Gone));
            assert_eq!(handle.get("k"), Err(Gone));
        });
    }

    #[test]
    fn handles_can_be_cloned_and_shared_between_threads() {
        // Compiles only while `Handle` is `Clone + Send + Sync`.
        fn shareable<T: Clone + Send + Sync>() {}
        shareable::<Handle>();
    }
}
