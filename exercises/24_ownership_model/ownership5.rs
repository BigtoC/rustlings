// Module 1 · Ownership — part 5: enum state transitions on `&mut self` with `mem::replace` (E0507).
//
// State machines are where E0507 bites hardest in live coding. The state is an
// enum, the transition method takes `&mut self`, and the next state is built
// FROM the fields of the current one: `Connecting { addr }` becomes
// `Connected { addr, session }`. The first thing everyone writes is
//
//     if let Conn::Connecting { addr } = *self {
//         *self = Conn::Connected { addr, session };
//     }
//
// It looks harmless, because `*self` is overwritten on the very next line.
// But the pattern MOVES the `String` out of `*self`, which is only borrowed
// (part 4's E0507), and the assignment makes it worse: `*self = ...` first
// DROPS the old value of `*self`, and that old value would still contain the
// `String` that now lives in `addr`. That is a double free, no panic needed.
// (On a local variable the same code compiles, because the compiler knows the
// local is partly moved-from and does not drop that part again. It cannot know
// that about a place it only reaches through a reference.)
//
// The fix is part 4's rule applied to the whole enum: put a valid value in
// FIRST. `mem::replace(self, <placeholder>)` leaves a cheap variant that owns
// nothing (like `Closed`) in `*self` and hands you the old state BY VALUE.
// Now the old state is yours. Match on it, move its fields wherever you like,
// and write the real next state back with `*self = ...`. Every arm has to
// write something back, including "this event does not apply here, keep the
// old state". If a panic happens before the write-back, the object is simply
// left in the placeholder state: well defined, but possibly surprising, so
// pick a placeholder that means something. And when the placeholder IS the
// next state, the whole transition is one `mem::replace`.
//
// Two alternatives are worth knowing. If only one field has to move and its
// type has a cheap `Default` (a `String` does), you can match on `self` (so
// the binding is a `&mut String`) and `mem::take` that field. That does not
// work for a `Session`: it has no sensible default, and a dummy one invented
// to fill the hole is a bug waiting to happen. It gets dropped along with the
// old state, and a session's `Drop` is a real teardown (see `Session` below).
// Crates like `take_mut` and `replace_with` let a closure consume `*self`
// directly. To stay sound when the closure panics, they either abort the
// process or make you supply a fallback value, which is the placeholder again.
//
// How interviewers probe it: "Implement `on_connected(&mut self, ..)` without
// cloning `addr`. Why does the obvious `if let ... = *self` fail? What state
// is the object in if the transition panics halfway?"

use std::cell::Cell;

// A live resource (think of a TLS session and its keys). There is exactly one
// of it, so it is deliberately NOT `Clone`, and it has no `Default`.
#[derive(Debug, PartialEq)]
struct Session {
    id: u64,
    token: String,
}

impl Session {
    fn new(id: u64, token: &str) -> Self {
        Session {
            id,
            token: token.to_string(),
        }
    }
}

thread_local! {
    // How many sessions have been torn down on this thread. The tests use it
    // to check that no transition destroys a session or makes up a stand-in.
    static SESSIONS_DROPPED: Cell<usize> = const { Cell::new(0) };
}

// Dropping a session tears it down. A real one would notify the peer and wipe
// its keys; this one only counts.
impl Drop for Session {
    fn drop(&mut self) {
        SESSIONS_DROPPED.set(SESSIONS_DROPPED.get() + 1);
    }
}

#[derive(Debug, PartialEq)]
enum Conn {
    Idle,
    Connecting { addr: String },
    Connected { addr: String, session: Session },
    Closed,
}

impl Conn {
    // Idle -> Connecting. Already fine: nothing is moved OUT of `*self`. The
    // assignment drops the old (complete) `Idle` value and stores the new
    // state. In any other state the call is ignored.
    fn connect(&mut self, addr: String) {
        if matches!(self, Conn::Idle) {
            *self = Conn::Connecting { addr };
        }
    }

    // Connecting { addr } -> Connected { addr, session }. In any other state
    // the event is unexpected: nothing changes and the session is handed back
    // in `Err`, so the caller decides what to do with it.
    fn on_connected(&mut self, session: Session) -> Result<(), Session> {
        // TODO: E0507 "cannot move out of `self.addr` as enum variant
        // `Connecting` which is behind a mutable reference". rustc's help
        // ("consider removing the dereference here") only gets you a
        // `&mut String`, which is not the `String` that `Connected` needs, and
        // cloning it is exactly what the interviewer asked you not to do.
        // Requirement: `Connecting { addr }` becomes `Connected { addr,
        // session }` with the SAME `addr` allocation (a test compares heap
        // pointers). In every other state nothing changes and the very same
        // session comes back in `Err`.
        // Constraints: keep the signature, no `.clone()`, `.to_string()` or
        // `.to_owned()` of `addr`, don't derive `Clone`, no `unsafe`, and
        // don't change the tests.
        // Until you get `addr` out of `*self` without leaving a hole behind,
        // this exercise will not compile.
        if let Conn::Connecting { addr } = *self {
            *self = Conn::Connected { addr, session };
            Ok(())
        } else {
            Err(session)
        }
    }

    // Any state -> Closed. If the connection was up, its session is handed to
    // the caller (to log out, say); otherwise the result is `None`.
    fn close(&mut self) -> Option<Session> {
        // TODO: E0507 "cannot move out of `self.session` as enum variant
        // `Connected` which is behind a mutable reference". A `Session` has no
        // `Default` and must not be copied, so there is no "empty session" you
        // could leave in the field instead.
        // Requirement: every state ends up `Closed`, and a `Connected` state's
        // session is returned (the same one: a test compares heap pointers).
        // Constraints: no `.clone()`, don't construct a replacement or dummy
        // `Session` (a test counts dropped sessions), no `unsafe`, and don't
        // change the tests.
        // Until you take the session out without leaving a hole in `*self`,
        // this exercise will not compile.
        match *self {
            Conn::Connected { session, .. } => {
                *self = Conn::Closed;
                Some(session)
            }
            _ => {
                *self = Conn::Closed;
                None
            }
        }
    }

    // A read-only peek works fine through `&self`: nothing moves.
    fn addr(&self) -> Option<&str> {
        match self {
            Conn::Connecting { addr } | Conn::Connected { addr, .. } => Some(addr),
            Conn::Idle | Conn::Closed => None,
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connecting(addr: &str) -> Conn {
        let mut conn = Conn::Idle;
        conn.connect(addr.to_string());
        conn
    }

    fn addr_ptr(conn: &Conn) -> *const u8 {
        conn.addr().expect("this state has an address").as_ptr()
    }

    #[test]
    fn connect_moves_from_idle_to_connecting() {
        let conn = connecting("10.0.0.1:443");
        assert_eq!(
            conn,
            Conn::Connecting {
                addr: "10.0.0.1:443".to_string()
            }
        );
    }

    #[test]
    fn on_connected_moves_the_address_into_the_new_state() {
        let mut conn = connecting("10.0.0.1:443");
        let addr_before = addr_ptr(&conn);

        assert_eq!(conn.on_connected(Session::new(7, "tok-7")), Ok(()));
        assert_eq!(
            conn,
            Conn::Connected {
                addr: "10.0.0.1:443".to_string(),
                session: Session::new(7, "tok-7"),
            }
        );
        // Same heap buffer: the `String` was moved, not copied.
        assert_eq!(addr_ptr(&conn), addr_before);
    }

    #[test]
    fn on_connected_keeps_the_session_it_was_given() {
        let mut conn = connecting("db:5432");
        let session = Session::new(1, "tok-1");
        let token_ptr = session.token.as_ptr();
        assert!(conn.on_connected(session).is_ok());
        match &conn {
            Conn::Connected { session, .. } => assert_eq!(session.token.as_ptr(), token_ptr),
            other => panic!("expected Connected, got {other:?}"),
        }
    }

    fn assert_on_connected_is_ignored(mut conn: Conn, expected: Conn) {
        let session = Session::new(9, "tok-9");
        let token_ptr = session.token.as_ptr();

        let returned = conn.on_connected(session).unwrap_err();
        // The very same session comes back, and the state is untouched.
        assert_eq!(returned.token.as_ptr(), token_ptr);
        assert_eq!(returned, Session::new(9, "tok-9"));
        assert_eq!(conn, expected);
    }

    #[test]
    fn on_connected_is_ignored_when_idle_or_closed() {
        assert_on_connected_is_ignored(Conn::Idle, Conn::Idle);
        assert_on_connected_is_ignored(Conn::Closed, Conn::Closed);
    }

    #[test]
    fn a_second_on_connected_keeps_the_first_session() {
        let mut conn = connecting("10.0.0.1:443");
        conn.on_connected(Session::new(1, "first")).unwrap();
        let addr_before = addr_ptr(&conn);

        let rejected = conn.on_connected(Session::new(2, "second")).unwrap_err();
        assert_eq!(rejected, Session::new(2, "second"));
        assert_eq!(
            conn,
            Conn::Connected {
                addr: "10.0.0.1:443".to_string(),
                session: Session::new(1, "first"),
            }
        );
        // Putting the old state back must not copy it either.
        assert_eq!(addr_ptr(&conn), addr_before);
    }

    #[test]
    fn close_hands_back_the_live_session() {
        let mut conn = connecting("10.0.0.1:443");
        let session = Session::new(7, "tok-7");
        let token_ptr = session.token.as_ptr();
        conn.on_connected(session).unwrap();

        let session = conn.close().expect("a connected Conn has a session");
        assert_eq!(session, Session::new(7, "tok-7"));
        assert_eq!(session.token.as_ptr(), token_ptr);
        assert_eq!(conn, Conn::Closed);
    }

    #[test]
    fn close_from_any_other_state_returns_none() {
        for mut conn in [Conn::Idle, connecting("10.0.0.1:443"), Conn::Closed] {
            assert_eq!(conn.close(), None);
            assert_eq!(conn, Conn::Closed);
        }
    }

    #[test]
    fn transitions_never_drop_or_invent_a_session() {
        let mut conn = connecting("10.0.0.1:443");
        let before = SESSIONS_DROPPED.get();

        conn.on_connected(Session::new(5, "tok-5")).unwrap();
        // The session was stored in the new state, not dropped...
        assert_eq!(SESSIONS_DROPPED.get(), before);

        let session = conn.close().expect("a connected Conn has a session");
        // ...and `close` handed that same session out. A stand-in session
        // swapped into the old state would be dropped along with it.
        assert_eq!(SESSIONS_DROPPED.get(), before);
        assert_eq!(session.id, 5);

        drop(session);
        assert_eq!(SESSIONS_DROPPED.get(), before + 1);
    }

    #[test]
    fn closed_is_final() {
        let mut conn = connecting("10.0.0.1:443");
        conn.on_connected(Session::new(3, "tok-3")).unwrap();
        assert!(conn.close().is_some());
        // Closing twice has nothing left to hand out.
        assert_eq!(conn.close(), None);
        // A closed connection cannot be reopened.
        conn.connect("10.0.0.2:443".to_string());
        assert_eq!(conn, Conn::Closed);
        assert_eq!(conn.addr(), None);
    }
}
