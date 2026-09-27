//! `Email` from the outside, with no serde involved: these properties run
//! under every feature set, `--no-default-features` included.

mod common;

use config_lab::Email;
use proptest::prelude::*;
use proptest::sample::Index;

/// An accepted address must already be in normal form: a lowercase domain,
/// and checking it a second time gives the same `Email` back.
fn assert_normalized(email: Email) -> Result<(), TestCaseError> {
    prop_assert_eq!(email.domain(), email.domain().to_ascii_lowercase());
    prop_assert_eq!(Email::try_from(email.to_string()), Ok(email));
    Ok(())
}

proptest! {
    #![proptest_config(common::proptest_config())]

    /// Everything the generator makes is accepted. Tighten a rule in
    /// `Email::try_from` and this fails with a shrunk, minimal address.
    #[test]
    fn generated_addresses_are_accepted(s in common::address()) {
        match Email::try_from(s.clone()) {
            Ok(email) => assert_normalized(email)?,
            Err(e) => prop_assert!(false, "rejected {:?}: {}", s, e),
        }
    }

    /// Near misses: a valid address with one arbitrary character inserted.
    /// Some are still valid, most are not; none may panic.
    #[test]
    fn near_misses_are_rejected_or_normalized(
        s in common::address(),
        c in any::<char>(),
        at in any::<Index>(),
    ) {
        let mut s = s;
        // `address()` is ASCII, so every index is a char boundary.
        s.insert(at.index(s.len() + 1), c);
        if let Ok(email) = Email::try_from(s) {
            assert_normalized(email)?;
        }
    }

    /// Arbitrary Unicode never panics the validator.
    #[test]
    fn any_string_is_rejected_or_normalized(s in any::<String>()) {
        if let Ok(email) = Email::try_from(s) {
            assert_normalized(email)?;
        }
    }
}
