// Module 5 · Checked math — part 4: a deterministic ledger and state digest.
//
// A blockchain node is a replicated state machine. Every validator runs the
// same transactions on its own copy of the state, and they check that they
// agree by comparing a digest of the result (an "app hash" or state root).
// That digest has to be the same on every machine, in every run and with
// every build: one differing bit is a consensus failure, and the chain halts
// or forks. The same rule holds for anything that replays a log and compares
// checksums (event sourcing, deterministic simulation, lockstep multiplayer
// games). The four classic ways Rust code breaks it are all in `Ledger`:
//
//   - `HashMap` iteration order. The docs call it "arbitrary", and with the
//     default `RandomState` it differs from one map to the next, even for
//     equal contents. A `BTreeMap` iterates in key order.
//   - Floats. `+`, `-`, `*` and `/` on `f64` are correctly rounded, so they
//     ARE reproducible (NaN bit patterns aside), but they are not exact: a
//     53-bit mantissa can't hold most integers above 2^53, and 0.29 is not a
//     binary fraction. And the std docs say the precision of `powf`, `exp`
//     and friends "is non-deterministic": it varies by platform and Rust
//     version. Money math is integer math: here, basis points (1 bp is
//     0.01%) and part 1's `mul_div_floor`, with an explicit rounding
//     direction.
//   - Unspecified hashes. For `DefaultHasher`, "the internal algorithm is
//     not specified, and so it and its hashes should not be relied upon over
//     releases", and the `Hash` impls that feed it are not portable either
//     (a `u128` hashes its native-endian bytes). A consensus digest is a
//     specified function (FNV-1a here; SHA-256 or BLAKE3 in production) of a
//     byte encoding that YOU define: the order, width, endianness and length
//     prefix of every field.
//   - Truncating casts. `as` between integers never fails: `u128 as u64`
//     keeps the low 64 bits, in debug and release builds alike (and a float
//     to integer `as` saturates, with NaN becoming 0). Two balances that
//     agree in their low 64 bits then encode, and hash, the same.
//     `u64::try_from(x)` returns an `Err` instead.
//
// None of these show up in a test that runs once on one machine. That is why
// the tests below build the same state in different orders and compare it
// against golden digests.
//
// How interviewers probe this: "Why no floats or HashMap iteration in
// consensus state? Is float addition nondeterministic? What does `as` do on
// overflow? How do you hash a struct so that the hash is stable across
// machines and compiler versions?"

use std::collections::BTreeMap;

/// One basis point is 1/10_000.
const BPS_PER_UNIT: u128 = 10_000;

/// Divides the 256-bit number `hi * 2^128 + lo` by `d`. Returns
/// `(quotient, remainder)`, or `None` when `d == 0` or the quotient does not
/// fit in a `u128`. (From part 1, given.)
fn div_wide(hi: u128, lo: u128, d: u128) -> Option<(u128, u128)> {
    if d == 0 || hi >= d {
        return None;
    }
    if hi == 0 {
        return Some((lo / d, lo % d));
    }
    let (mut quot, mut rem) = (0u128, hi);
    for i in (0..128).rev() {
        let carry = rem >> 127;
        rem = (rem << 1) | ((lo >> i) & 1);
        quot <<= 1;
        if carry == 1 || rem >= d {
            rem = rem.wrapping_sub(d);
            quot |= 1;
        }
    }
    Some((quot, rem))
}

/// `a * b / d` through the exact 256-bit product, rounded down. `None` when
/// `d == 0` or the result does not fit in a `u128`. (Part 1's solution,
/// given.)
fn mul_div_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    let (lo, hi) = a.carrying_mul(b, 0);
    div_wide(hi, lo, d).map(|(quot, _)| quot)
}

/// 64-bit FNV-1a: tiny and fully specified, so every node computes the same
/// value from the same bytes. It is not collision-resistant (a real chain
/// would use SHA-256 or BLAKE3 from a crate), but the lesson is the same.
/// Given, and deliberately NOT a `std::hash::Hasher`, so that no `Hash` impl
/// can feed it bytes you did not choose.
struct Fnv1a(u64);

impl Fnv1a {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Fnv1a(Self::OFFSET_BASIS)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LedgerError {
    /// A balance would not fit in a `u128`.
    Overflow,
    /// This account's balance does not fit in the API's `u64` amounts.
    TooLargeForApi { account: String },
}

#[derive(Debug, Default)]
struct Ledger {
    // A `BTreeMap` iterates in key order, whatever order the accounts were
    // opened in, so every node walks the same sequence. (Sorting the entries
    // at each use would work too, but fixing the type fixes every use at
    // once, including the ones written next year.)
    balances: BTreeMap<String, u128>,
}

impl Ledger {
    fn new() -> Self {
        Self::default()
    }

    /// Adds `amount` to `account`, opening the account if needed.
    fn credit(&mut self, account: &str, amount: u128) -> Result<(), LedgerError> {
        let balance = self.balances.entry(account.to_string()).or_insert(0);
        *balance = balance.checked_add(amount).ok_or(LedgerError::Overflow)?;
        Ok(())
    }

    fn balance(&self, account: &str) -> u128 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    /// Pays `rate_bps` basis points of interest on every balance: each one
    /// grows by `balance * rate_bps / 10_000`, rounded down (the ledger pays,
    /// so it rounds in its own favor). All or nothing: if any new balance
    /// would not fit in a `u128`, return `Err(LedgerError::Overflow)` and
    /// change no balance at all.
    fn accrue_interest(&mut self, rate_bps: u32) -> Result<(), LedgerError> {
        // Integers all the way: `mul_div_floor` keeps the exact 256-bit
        // product and rounds down, and `checked_add` catches a new balance
        // above `u128::MAX`. The first pass only computes; the second
        // commits, so an `Err` leaves every balance as it was. (The two
        // passes see the same order because the map is not touched in
        // between.)
        let rate = u128::from(rate_bps);
        let new_balances = self
            .balances
            .values()
            .map(|&balance| {
                let interest = mul_div_floor(balance, rate, BPS_PER_UNIT)?;
                balance.checked_add(interest)
            })
            .collect::<Option<Vec<u128>>>()
            .ok_or(LedgerError::Overflow)?;
        for (balance, new_balance) in self.balances.values_mut().zip(new_balances) {
            *balance = new_balance;
        }
        Ok(())
    }

    /// The consensus digest: FNV-1a (the `Fnv1a` above) over this exact
    /// byte string, built from every account in ascending order of name
    /// (`str` order, which is byte order):
    ///
    /// ```text
    /// name length   8 bytes, a little-endian u64
    /// name          its UTF-8 bytes
    /// balance       16 bytes, a little-endian u128
    /// ```
    ///
    /// Nothing else: no separators, no account count, no `Hash` impls. An
    /// empty ledger hashes no bytes at all.
    fn state_digest(&self) -> u64 {
        // Every byte is chosen here, in a fixed order, with fixed widths and
        // endianness: nothing depends on the platform, the Rust version or
        // the map's history. The length prefix keeps the encoding
        // unambiguous, and all 16 bytes of the balance go in. `usize` has no
        // `From` impl into `u64` (std leaves room for wider targets), hence
        // `try_from`; it cannot fail on any target Rust supports.
        let mut fnv = Fnv1a::new();
        for (name, balance) in &self.balances {
            let len = u64::try_from(name.len()).expect("a name longer than u64::MAX bytes");
            fnv.write(&len.to_le_bytes());
            fnv.write(name.as_bytes());
            fnv.write(&balance.to_le_bytes());
        }
        fnv.finish()
    }

    /// The balances for the public API, whose amounts are `u64` (protobuf,
    /// for one, has no 128-bit integer type), in ascending order of name. A
    /// balance above `u64::MAX` must not be cut down to fit: return
    /// `Err(LedgerError::TooLargeForApi { account })` for the first such
    /// account in name order.
    fn export(&self) -> Result<Vec<(String, u64)>, LedgerError> {
        // `u64::try_from` fails instead of truncating. Collecting into a
        // `Result` stops at the first `Err`, and the map yields the accounts
        // in name order, so that `Err` names the first offender.
        self.balances
            .iter()
            .map(|(name, &balance)| {
                let amount = u64::try_from(balance).map_err(|_| LedgerError::TooLargeForApi {
                    account: name.clone(),
                })?;
                Ok((name.clone(), amount))
            })
            .collect()
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    const E18: u128 = 1_000_000_000_000_000_000;

    // Twelve accounts, several of them above `u64::MAX`. If the accounts
    // came out in a random order, the chance of getting name order by luck
    // would be 1 in 12!, about 2e-9.
    const ACCOUNTS: [(&str, u128); 12] = [
        ("alice", 1_250 * E18),
        ("bob", 7),
        ("carol", 3 * E18 / 2),
        ("dave", 18_446_744_073_709_551_616),
        ("erin", 0),
        ("frank", 42 * E18),
        ("grace", 999_999_999_999),
        ("heidi", 1_000_000_000_000 * E18),
        ("ivan", 1),
        ("judy", 123_456_789 * E18 + 987_654_321),
        ("mallory", u128::MAX / 3),
        ("niaj", 5_000),
    ];
    const FORWARD: [usize; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    const REVERSED: [usize; 12] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    const SHUFFLED: [usize; 12] = [7, 2, 11, 0, 5, 9, 3, 10, 1, 6, 4, 8];

    // The documented encoding of ACCOUNTS, hashed with FNV-1a (and checked
    // against an independent implementation), before and after 250 bps of
    // interest.
    const GOLDEN_DIGEST: u64 = 0xda20_7b78_fb15_77f8;
    const GOLDEN_DIGEST_AFTER_INTEREST: u64 = 0x2e76_20e2_94a9_c336;

    fn ledger_from(order: &[usize]) -> Ledger {
        let mut ledger = Ledger::new();
        for &i in order {
            let (name, balance) = ACCOUNTS[i];
            ledger.credit(name, balance).unwrap();
        }
        ledger
    }

    fn single(balance: u128) -> Ledger {
        let mut ledger = Ledger::new();
        ledger.credit("a", balance).unwrap();
        ledger
    }

    #[test]
    fn fnv1a_matches_the_published_test_vectors() {
        let vectors = [
            ("", 0xcbf2_9ce4_8422_2325),
            ("a", 0xaf63_dc4c_8601_ec8c),
            ("foobar", 0x8594_4171_f739_67e8),
        ];
        for (input, hash) in vectors {
            let mut fnv = Fnv1a::new();
            fnv.write(input.as_bytes());
            assert_eq!(fnv.finish(), hash, "{input:?}");
        }
    }

    #[test]
    fn the_digest_hashes_exactly_the_documented_bytes() {
        assert_eq!(
            Ledger::new().state_digest(),
            Fnv1a::new().finish(),
            "an empty ledger hashes no bytes"
        );
        let mut ledger = Ledger::new();
        ledger.credit("b", 1).unwrap();
        ledger.credit("ab", 258).unwrap();
        let mut bytes: Vec<u8> = Vec::new();
        // "ab" sorts before "b".
        bytes.extend([2, 0, 0, 0, 0, 0, 0, 0]); // the length of "ab", a u64
        bytes.extend(*b"ab");
        bytes.extend([2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // 258, a u128
        bytes.extend([1, 0, 0, 0, 0, 0, 0, 0]); // the length of "b"
        bytes.extend(*b"b");
        bytes.extend([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // 1
        let mut fnv = Fnv1a::new();
        fnv.write(&bytes);
        assert_eq!(ledger.state_digest(), fnv.finish());
    }

    #[test]
    fn the_digest_matches_the_golden_value() {
        assert_eq!(ledger_from(&FORWARD).state_digest(), GOLDEN_DIGEST);
    }

    #[test]
    fn insertion_order_does_not_change_the_digest() {
        for order in [FORWARD, REVERSED, SHUFFLED] {
            let digest = ledger_from(&order).state_digest();
            assert_eq!(
                digest, GOLDEN_DIGEST,
                "accounts opened in the order {order:?}"
            );
        }
        // The same balances, with alice's paid in two credits, one first and
        // one last.
        let mut ledger = Ledger::new();
        ledger.credit("alice", 250 * E18).unwrap();
        for i in REVERSED {
            let (name, balance) = ACCOUNTS[i];
            let rest = if name == "alice" {
                balance - 250 * E18
            } else {
                balance
            };
            ledger.credit(name, rest).unwrap();
        }
        assert_eq!(ledger.state_digest(), GOLDEN_DIGEST);
    }

    #[test]
    fn the_digest_sees_every_bit_of_a_balance() {
        let pairs = [(1, 1 + (1 << 64)), (5, 5 + (1 << 100)), (0, 1 << 127)];
        for (low, high) in pairs {
            assert_ne!(
                single(low).state_digest(),
                single(high).state_digest(),
                "{low} and {high} differ only above bit 64"
            );
        }
    }

    #[test]
    fn interest_is_exact_on_small_balances() {
        // (balance, rate in bps, the new balance)
        let cases = [
            (100, 2_900, 129),
            (10_000, 1, 10_001),
            (9_999, 1, 9_999),
            (3, 5_000, 4),
            (1, 10_000, 2),
            (1, 30_000, 4),
            (0, 10_000, 0),
            (7, 0, 7),
        ];
        for (balance, rate, expected) in cases {
            let mut ledger = single(balance);
            assert_eq!(ledger.accrue_interest(rate), Ok(()));
            assert_eq!(ledger.balance("a"), expected, "{balance} at {rate} bps");
        }
    }

    #[test]
    fn interest_is_exact_on_a_1e30_balance() {
        let mut ledger = single(10u128.pow(30));
        ledger.accrue_interest(7).unwrap();
        // 1e30 * 7 / 10_000 is 7e26, exactly.
        assert_eq!(ledger.balance("a"), 10u128.pow(30) + 7 * 10u128.pow(26));
    }

    #[test]
    fn interest_never_overflows_on_the_way() {
        // `balance * 3` is above u128::MAX; the new balance is not.
        let balance = u128::MAX / 2;
        let mut ledger = single(balance);
        assert_eq!(ledger.accrue_interest(3), Ok(()));
        // floor(balance * 3 / 10_000), split so that every piece fits.
        let interest = balance / 10_000 * 3 + balance % 10_000 * 3 / 10_000;
        assert_eq!(ledger.balance("a"), balance + interest);
    }

    #[test]
    fn interest_that_would_overflow_changes_nothing() {
        let mut ledger = Ledger::new();
        ledger.credit("alice", 100).unwrap();
        ledger.credit("zed", u128::MAX - 5).unwrap();
        assert_eq!(ledger.accrue_interest(100), Err(LedgerError::Overflow));
        assert_eq!(
            ledger.balance("alice"),
            100,
            "no one is paid when the update as a whole fails"
        );
        assert_eq!(ledger.balance("zed"), u128::MAX - 5);
        // Here the interest alone is above u128::MAX: an `Err`, not a panic.
        let mut ledger = single(u128::MAX / 2);
        assert_eq!(ledger.accrue_interest(u32::MAX), Err(LedgerError::Overflow));
        assert_eq!(ledger.balance("a"), u128::MAX / 2);
    }

    #[test]
    fn interest_is_part_of_the_state() {
        let mut ledger = ledger_from(&SHUFFLED);
        ledger.accrue_interest(250).unwrap();
        assert_eq!(ledger.balance("heidi"), 1_025_000_000_000 * E18);
        assert_eq!(
            ledger.balance("bob"),
            7,
            "0.175 units of interest round down"
        );
        assert_eq!(ledger.state_digest(), GOLDEN_DIGEST_AFTER_INTEREST);
    }

    #[test]
    fn export_lists_accounts_in_name_order() {
        // Every balance fits in a u64 here: each account holds its name's
        // length.
        let amount = |name: &str| u64::try_from(name.len()).unwrap();
        let mut ledger = Ledger::new();
        for i in SHUFFLED {
            let (name, _) = ACCOUNTS[i];
            ledger.credit(name, u128::from(amount(name))).unwrap();
        }
        let expected: Vec<(String, u64)> = ACCOUNTS
            .iter()
            .map(|&(name, _)| (name.to_string(), amount(name)))
            .collect();
        assert_eq!(ledger.export(), Ok(expected));
    }

    #[test]
    fn export_refuses_to_truncate() {
        let mut ledger = Ledger::new();
        ledger.credit("zed", u128::MAX).unwrap();
        ledger.credit("bob", 7).unwrap();
        ledger.credit("whale", (1 << 64) + 5).unwrap();
        ledger.credit("max", u128::from(u64::MAX)).unwrap();
        assert_eq!(
            ledger.export(),
            Err(LedgerError::TooLargeForApi {
                account: "whale".to_string()
            })
        );
        // Six of the twelve ACCOUNTS are above u64::MAX. The error names the
        // first of them in name order, whatever order the accounts were
        // opened in. Every ledger here is a new map, so an `Err` that depends
        // on a map's iteration order can't pass 30 times by luck.
        for _ in 0..10 {
            for order in [FORWARD, REVERSED, SHUFFLED] {
                assert_eq!(
                    ledger_from(&order).export(),
                    Err(LedgerError::TooLargeForApi {
                        account: "alice".to_string()
                    }),
                    "accounts opened in the order {order:?}"
                );
            }
        }
    }

    #[test]
    fn export_accepts_everything_up_to_u64_max() {
        let mut ledger = single(u128::from(u64::MAX));
        assert_eq!(ledger.export(), Ok(vec![("a".to_string(), u64::MAX)]));
        ledger.credit("a", 1).unwrap();
        assert_eq!(
            ledger.export(),
            Err(LedgerError::TooLargeForApi {
                account: "a".to_string()
            })
        );
    }
}
