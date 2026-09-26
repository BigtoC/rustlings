// Traits & Abstraction · Testing seams — part 1: a recording mock behind a `&self` trait (E0596, E0594).
//
// "How would you test this?" follows almost every design question. For code
// that sends mail, charges a card or reads the clock, the answer starts with
// a SEAM: a place where you can change what the code does without editing
// the code itself (Michael Feathers' term). In Rust the idiomatic seam is a
// trait. `SignupService<M: Mailer>` does not know which mailer it talks to.
// Production passes the real one, a test passes a TEST DOUBLE:
//
//   - a STUB returns canned answers;
//   - a FAKE is a small working implementation (an in-memory store, or the
//     fake clock of parts 2 and 3);
//   - a SPY records how it was called, so the test can assert on the calls
//     afterwards; a MOCK checks the calls against expectations set up in
//     advance. In everyday speech both are "mocks".
//
// `MockMailer` below is a spy that can also be told to fail once, so a test
// can check how the service copes with a mail error. The service is finished
// and correct; the double is what you have to fix.
//
// The trouble is the receiver. `Mailer::send` takes `&self`, and it should:
// the receiver is part of the contract with EVERY implementation, not just
// with the mock. A real mailer is a handle to something that does its own
// synchronization (an SMTP connection pool, an HTTP client, the sending half
// of a channel), and one service is shared by many request handlers, often
// as an `Arc<SignupService<..>>`. With `&mut self` in the trait, every caller
// would need exclusive access to the service just to send one mail. And the
// only reason to change it would be that a test double wants to push into a
// `Vec`.
//
// So the double has to record through a shared reference, which is what
// interior mutability is for. `26_smart_pointers_deep/smartptr3` and
// `40_interior_mutability/cell1` drilled the tools (`RefCell` when you need a
// `&mut` to the contents, `Cell` when moving a value in and out is enough,
// `Copy` or not); here the question is which one fits each field of a
// double. Both are single-threaded (`!Sync`): a double that is shared between
// threads needs a `Mutex` or atomics instead, as in part 2.
//
// Read rustc's help on these errors carefully. It suggests "changing this to
// be a mutable reference in the `impl` method and the `trait` definition".
// That makes the errors go away by changing the production contract for the
// sake of a test. The interview answer is the opposite: keep the trait as it
// is and put the mutability inside the double.
//
// How interviewers probe this: "How do you unit-test code that sends mail?",
// "Why does your mock need a `RefCell`? Why not `&mut self`?", "Generics or
// `dyn` for the injected dependency?" (a generic `M` is monomorphized and
// costs nothing at run time; a `Box<dyn Mailer>` field gives one concrete
// service type whose mailer is picked at run time, for a vtable call, see
// `32_dispatch`), and "What changes if the service is shared between
// threads?" (the mailer must be `Send + Sync`, and so must its double).
// Crates like `mockall` generate doubles, but interviewers expect you to
// write one by hand.

// The dependency the service needs, as a trait: this is the seam.
trait Mailer {
    fn send(&self, to: &str, subject: &str) -> Result<(), MailError>;
}

#[derive(Debug, PartialEq, Eq)]
enum MailError {
    // The mail server could not be reached.
    Unreachable,
    // The mail server refused the message, with its reason.
    Rejected(String),
}

#[derive(Debug, PartialEq, Eq)]
enum SignupError {
    InvalidEmail,
    AlreadyRegistered,
    UnknownUser,
    Mail(MailError),
}

const WELCOME: &str = "Welcome aboard!";
const CONFIRM: &str = "Please confirm your address";

// ---- The code under test (finished; don't change it) ----

struct SignupService<M: Mailer> {
    mailer: M,
    users: Vec<String>,
}

impl<M: Mailer> SignupService<M> {
    fn new(mailer: M) -> Self {
        SignupService {
            mailer,
            users: Vec::new(),
        }
    }

    // Registers `email` and sends it a welcome mail. The user is stored only
    // once the mail went out, so a failed mail leaves nothing behind and the
    // caller can simply retry.
    fn register(&mut self, email: &str) -> Result<(), SignupError> {
        if !email.contains('@') {
            return Err(SignupError::InvalidEmail);
        }
        if self.is_registered(email) {
            return Err(SignupError::AlreadyRegistered);
        }
        self.mailer
            .send(email, WELCOME)
            .map_err(SignupError::Mail)?;
        self.users.push(email.to_string());
        Ok(())
    }

    // Sends the confirmation mail again. It changes nothing in the service,
    // so it takes `&self`, like most of a real service's methods.
    fn resend_confirmation(&self, email: &str) -> Result<(), SignupError> {
        if !self.is_registered(email) {
            return Err(SignupError::UnknownUser);
        }
        self.mailer.send(email, CONFIRM).map_err(SignupError::Mail)
    }

    fn is_registered(&self, email: &str) -> bool {
        self.users.iter().any(|user| user == email)
    }

    // Lets a test look at the double after the service has used it.
    fn mailer(&self) -> &M {
        &self.mailer
    }
}

// ---- The test double ----
//
// In a real crate it would live inside `#[cfg(test)] mod tests`. It sits out
// here so that you can fix it without touching the tests.

// One recorded mail. Deliberately not `Clone`: the double hands its record
// over by value.
#[derive(Debug, PartialEq, Eq)]
struct Mail {
    to: String,
    subject: String,
}

// TODO: every method below that changes the double is rejected, because it
// only has `&self`: E0596 "cannot borrow `self.sent` as mutable, as it is
// behind a `&` reference" (at the `push` and the `mem::take`), E0596 for
// `self.fail_next.take()`, and E0594 "cannot assign to `self.calls`, which is
// behind a `&` reference" (and to `self.fail_next`). Keep every receiver a
// `&self` and change what the double is made of, so it can record through a
// shared reference. Requirements:
//   - don't change the `Mailer` trait (ignore rustc's help to make `send`
//     take `&mut self`), the service or the tests;
//   - `send` counts every call (failed ones too), fails with the scripted
//     error if one is set (recording nothing and clearing the script), and
//     otherwise records the mail and returns `Ok(())`;
//   - `take_sent` hands over the recorded mails, oldest first, and leaves the
//     record empty; `Mail` must not gain `Clone`;
//   - no `unsafe`, no leaking, no `static`s: every test builds its own double.
// Until the double can record through `&self`, this exercise will not compile.
struct MockMailer {
    sent: Vec<Mail>,
    calls: u32,
    fail_next: Option<MailError>,
}

impl MockMailer {
    fn new() -> Self {
        MockMailer {
            sent: Vec::new(),
            calls: 0,
            fail_next: None,
        }
    }

    // Makes the next call to `send` fail with `err`.
    fn fail_next_with(&self, err: MailError) {
        self.fail_next = Some(err);
    }

    // Hands over everything sent so far, oldest first.
    fn take_sent(&self) -> Vec<Mail> {
        std::mem::take(&mut self.sent)
    }

    // How many times `send` was called, failed calls included.
    fn calls(&self) -> u32 {
        self.calls
    }
}

impl Mailer for MockMailer {
    fn send(&self, to: &str, subject: &str) -> Result<(), MailError> {
        self.calls += 1;
        if let Some(err) = self.fail_next.take() {
            return Err(err);
        }
        self.sent.push(Mail {
            to: to.to_string(),
            subject: subject.to_string(),
        });
        Ok(())
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(to: &str, subject: &str) -> Mail {
        Mail {
            to: to.to_string(),
            subject: subject.to_string(),
        }
    }

    #[test]
    fn registering_sends_exactly_one_welcome_mail() {
        let mut service = SignupService::new(MockMailer::new());
        assert_eq!(service.register("ada@example.com"), Ok(()));
        assert!(service.is_registered("ada@example.com"));
        assert_eq!(service.mailer().calls(), 1);
        assert_eq!(
            service.mailer().take_sent(),
            [mail("ada@example.com", WELCOME)]
        );
    }

    #[test]
    fn a_duplicate_registration_sends_no_second_mail() {
        let mut service = SignupService::new(MockMailer::new());
        assert_eq!(service.register("ada@example.com"), Ok(()));
        assert_eq!(
            service.register("ada@example.com"),
            Err(SignupError::AlreadyRegistered)
        );
        // The double proves what the return value cannot: the service did not
        // even try to mail the duplicate.
        assert_eq!(service.mailer().calls(), 1);
        assert_eq!(service.mailer().take_sent().len(), 1);
    }

    #[test]
    fn an_invalid_address_never_reaches_the_mailer() {
        let mut service = SignupService::new(MockMailer::new());
        assert_eq!(
            service.register("not-an-address"),
            Err(SignupError::InvalidEmail)
        );
        assert!(!service.is_registered("not-an-address"));
        assert_eq!(service.mailer().calls(), 0);
        assert!(service.mailer().take_sent().is_empty());
    }

    #[test]
    fn a_failed_mail_is_reported_and_the_user_is_not_stored() {
        let mut service = SignupService::new(MockMailer::new());
        // The service owns the double, and the test only gets it back through
        // `&M`. Scripting a failure therefore has to work through `&self` too.
        service
            .mailer()
            .fail_next_with(MailError::Rejected("mailbox full".to_string()));
        assert_eq!(
            service.register("bob@example.com"),
            Err(SignupError::Mail(MailError::Rejected(
                "mailbox full".to_string()
            )))
        );
        assert!(!service.is_registered("bob@example.com"));
        // The attempt counts, but nothing was delivered.
        assert_eq!(service.mailer().calls(), 1);
        assert!(service.mailer().take_sent().is_empty());

        // The script was for ONE call: a retry goes through.
        assert_eq!(service.register("bob@example.com"), Ok(()));
        assert!(service.is_registered("bob@example.com"));
        assert_eq!(service.mailer().calls(), 2);
        assert_eq!(
            service.mailer().take_sent(),
            [mail("bob@example.com", WELCOME)]
        );
    }

    #[test]
    fn resending_works_through_a_shared_reference_to_the_service() {
        let mut service = SignupService::new(MockMailer::new());
        service.register("ada@example.com").unwrap();
        // From here on the service is only borrowed, as it would be when
        // several request handlers share it.
        let shared = &service;
        assert_eq!(shared.resend_confirmation("ada@example.com"), Ok(()));
        assert_eq!(
            shared.resend_confirmation("eve@example.com"),
            Err(SignupError::UnknownUser)
        );
        shared.mailer().fail_next_with(MailError::Unreachable);
        assert_eq!(
            shared.resend_confirmation("ada@example.com"),
            Err(SignupError::Mail(MailError::Unreachable))
        );
        assert_eq!(shared.mailer().calls(), 3);
        assert_eq!(
            shared.mailer().take_sent(),
            [
                mail("ada@example.com", WELCOME),
                mail("ada@example.com", CONFIRM)
            ]
        );
    }

    #[test]
    fn the_double_records_through_shared_references() {
        // No service at all: two shared references to one double, the way a
        // real mailer is shared. This only compiles while `send` takes `&self`.
        let mock = MockMailer::new();
        let (first, second) = (&mock, &mock);
        assert_eq!(first.send("a@example.com", "one"), Ok(()));
        assert_eq!(second.send("b@example.com", "two"), Ok(()));
        second.fail_next_with(MailError::Unreachable);
        assert_eq!(
            first.send("c@example.com", "three"),
            Err(MailError::Unreachable)
        );
        assert_eq!(first.send("d@example.com", "four"), Ok(()));
        assert_eq!(mock.calls(), 4);
        assert_eq!(
            mock.take_sent(),
            [
                mail("a@example.com", "one"),
                mail("b@example.com", "two"),
                mail("d@example.com", "four")
            ]
        );
        // Taking the record empties it; the count is not reset.
        assert!(mock.take_sent().is_empty());
        assert_eq!(mock.calls(), 4);
    }

    #[test]
    fn a_new_script_replaces_an_unused_one() {
        let mock = MockMailer::new();
        mock.fail_next_with(MailError::Unreachable);
        mock.fail_next_with(MailError::Rejected("spam".to_string()));
        assert_eq!(
            mock.send("a@example.com", "hi"),
            Err(MailError::Rejected("spam".to_string()))
        );
        assert_eq!(mock.send("a@example.com", "hi"), Ok(()));
    }

    #[test]
    fn every_test_gets_a_fresh_double() {
        let one = MockMailer::new();
        let two = MockMailer::new();
        one.send("a@example.com", "hi").unwrap();
        one.fail_next_with(MailError::Unreachable);
        // Nothing leaks from one double into another.
        assert_eq!(two.calls(), 0);
        assert!(two.take_sent().is_empty());
        assert_eq!(two.send("b@example.com", "hi"), Ok(()));
    }
}
