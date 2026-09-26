// Module 5 · Code review — part 1: review a ledger pull request with seven planted bugs (it compiles; the tests fail).
//
// Many Rust interview loops have a review round next to (or instead of) the
// live-coding round. The interviewer shares a pull request of about 150 lines
// and asks: "What would you flag before approving this, and why?" The code
// compiles, and that is the point. In safe code, the compiler proves memory
// safety and the absence of data races. It does not prove that the code does
// what its comments promise. Arithmetic, conversions, run-time checks,
// output order and plain logic are still yours to review.
//
// This file is that pull request: a small single-threaded `Ledger` with
// accounts, a transfer fee, per-account transfer limits, observers, a
// history, a statement, an export for a legacy app and a text console. The
// comment above each item is its spec. QA ran the tests at the bottom of the
// file. The happy-path tests pass, and seven bug reports fail. Each report is
// named after what the user saw, not after the cause, just like a real
// ticket. Nothing else in the file tells you where the bugs are.
//
// How to work, and how interviewers grade it:
//   1. Review first, without running anything. Read each function against its
//      comment. For every line you would comment on, note the input that
//      breaks it and what happens then. The reason is what gets scored: "this
//      lookup is slow" is weak; "this scans all 10k customers for each of 10k
//      orders, 100 million comparisons where a `HashMap` would make 10k
//      lookups" is strong.
//   2. Then run the tests. Every failure (a panic message or an `assert_eq!`
//      diff) is evidence. Match it to a line you flagged, or find the line you
//      missed. A panic in a test tells you what happened; the failing input
//      tells you why.
//   3. Fix each bug where it lives, in the smallest change that meets the
//      spec.
//
// A strong answer also ranks the findings (a crash that user input can
// trigger, or a silently wrong number, blocks the merge; a style nit does
// not), says which test or lint would have caught each bug, and does not
// rewrite code that is fine. Expect the follow-up questions "what happens in
// a release build?" and "how would you have caught this in CI?".

use std::cell::RefCell;
use std::collections::BTreeMap;

// The review, most severe first. Each finding is marked `Review (n)` at its
// fix below.
//   1. `transfer` calls the observers while its `RefMut` on the books is
//      still alive, so an observer that reads the ledger panics with "RefCell
//      already mutably borrowed", after the money has moved. Blocker.
//   2. `legacy_balance` narrows with `as u32`, which keeps the low 32 bits:
//      5 billion becomes 705_032_704, and nobody is told. Blocker: a silently
//      wrong number.
//   3. The fee `amount * fee_bps` overflows `u64` for amounts above
//      `u64::MAX / fee_bps` (about 6.1e17 at 30 bps): a panic in a debug
//      build, and in release a wrapped, far too small fee. Blocker.
//   4. `&memo[..16]` cuts at BYTE 16, which panics when that byte is inside a
//      character. `len() > 16` counts bytes too, so short non-ASCII memos
//      were cut as well. Memos are user input: blocker.
//   5. `parse().unwrap()` on console input: one typo crashes the console,
//      which its own spec rules out. Blocker.
//   6. `amount >= limit` refuses a transfer of exactly the limit, which the
//      spec allows. Major: users hit it on day one.
//   7. `statement` iterates a `HashMap`, whose order is arbitrary and changes
//      from run to run, while the spec promises name order. Major.
// How CI could have caught them: a boundary test for each (the limit itself,
// `u32::MAX` and one more, the largest amount, a non-ASCII memo, a typo, a
// dozen accounts, an observer that reads), plus clippy's opt-in lints
// `cast_possible_truncation` (2), `arithmetic_side_effects` (3),
// `string_slice` (4) and `unwrap_used` (5). No lint sees 1: a `RefCell`
// borrow is checked at run time, so only a test with a reentrant observer
// finds it.

// Everything a ledger operation can fail with.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LedgerError {
    DuplicateAccount(String),
    UnknownAccount(String),
    SameAccount,
    InsufficientFunds { needed: u64, available: u64 },
    OverLimit { limit: u64, amount: u64 },
    Overflow,
    // The balance does not fit in the legacy app's 32-bit field.
    TooLargeForLegacy { balance: u64 },
    // The word that should have been an amount (a `u64`) but is not one.
    BadAmount(String),
    // The whole line, when a console command has the wrong shape.
    Malformed(String),
    UnknownCommand(String),
}

struct Account {
    balance: u64,
    // The most that one transfer may take out of this account (`None`: no
    // limit).
    limit: Option<u64>,
}

// One entry of the transfer history.
struct Record {
    from: String,
    to: String,
    amount: u64,
    fee: u64,
    memo: String,
}

// What observers are told after each successful transfer.
struct Transfer<'a> {
    from: &'a str,
    to: &'a str,
    amount: u64,
    fee: u64,
}

type Observer = Box<dyn Fn(&Ledger, &Transfer<'_>)>;

#[derive(Default)]
struct Books {
    // Review (7): a `BTreeMap` iterates in key order, so every listing of the
    // accounts comes out sorted by name, today and after the next refactor.
    accounts: BTreeMap<String, Account>,
    fees: u64,
    history: Vec<Record>,
}

// A single-threaded ledger. The app shares it (through an `Rc`) between
// several screens, so every operation takes `&self`, and the books live in a
// `RefCell`.
struct Ledger {
    fee_bps: u64,
    books: RefCell<Books>,
    observers: Vec<Observer>,
}

fn unknown(name: &str) -> LedgerError {
    LedgerError::UnknownAccount(name.to_string())
}

// Review (5): console input is untrusted, so a word that is not a `u64`
// ("12x", "-5", or one past `u64::MAX`) must become an error value, not a
// panic. `map_err` turns the `ParseIntError` into the error the spec names,
// and `?` returns it. `unwrap_or(0)` would be worse than the panic: it would
// silently deposit or transfer 0.
fn parse_amount(word: &str) -> Result<u64, LedgerError> {
    word.parse()
        .map_err(|_| LedgerError::BadAmount(word.to_string()))
}

impl Ledger {
    // A ledger that keeps `fee_bps` basis points (hundredths of a percent) of
    // every transfer as its fee, rounded down.
    fn new(fee_bps: u64) -> Self {
        assert!(fee_bps <= 10_000, "a fee above 100% is a config error");
        Ledger {
            fee_bps,
            books: RefCell::default(),
            observers: Vec::new(),
        }
    }

    // `observer` is called after every successful transfer, with the ledger
    // (so that it can look at balances) and the transfer.
    fn subscribe(&mut self, observer: impl Fn(&Ledger, &Transfer<'_>) + 'static) {
        self.observers.push(Box::new(observer));
    }

    fn open(&self, name: &str, limit: Option<u64>) -> Result<(), LedgerError> {
        let mut books = self.books.borrow_mut();
        if books.accounts.contains_key(name) {
            return Err(LedgerError::DuplicateAccount(name.to_string()));
        }
        let account = Account { balance: 0, limit };
        books.accounts.insert(name.to_string(), account);
        Ok(())
    }

    fn deposit(&self, name: &str, amount: u64) -> Result<(), LedgerError> {
        let mut books = self.books.borrow_mut();
        let account = books.accounts.get_mut(name).ok_or_else(|| unknown(name))?;
        account.balance = account
            .balance
            .checked_add(amount)
            .ok_or(LedgerError::Overflow)?;
        Ok(())
    }

    fn balance(&self, name: &str) -> Option<u64> {
        let books = self.books.borrow();
        books.accounts.get(name).map(|account| account.balance)
    }

    // The fees collected so far.
    fn fees(&self) -> u64 {
        self.books.borrow().fees
    }

    // Moves `amount` out of `from`: `to` receives `amount` minus the fee, and
    // the ledger keeps the fee. One transfer may move up to the sender's
    // limit, the limit itself included. On any error nothing changes. After a
    // successful transfer, every observer is called, and it already sees the
    // new balances. Returns the fee.
    fn transfer(&self, from: &str, to: &str, amount: u64, memo: &str) -> Result<u64, LedgerError> {
        if from == to {
            return Err(LedgerError::SameAccount);
        }
        let mut books = self.books.borrow_mut();
        let receiver = books.accounts.get(to).ok_or_else(|| unknown(to))?.balance;
        let sender = books.accounts.get(from).ok_or_else(|| unknown(from))?;
        // Review (6): the limit itself is allowed, so only an amount ABOVE it
        // is refused. (`amount >= limit + 1` says the same thing, but it
        // overflows when the limit is `u64::MAX`.)
        if let Some(limit) = sender.limit
            && amount > limit
        {
            return Err(LedgerError::OverLimit { limit, amount });
        }
        if sender.balance < amount {
            let available = sender.balance;
            return Err(LedgerError::InsufficientFunds {
                needed: amount,
                available,
            });
        }
        // Review (3): in `u128` the product cannot overflow (it is below
        // 2^64 * 10_000 < 2^78), and the division is still exact floor
        // division. The quotient is at most `amount`, because
        // `fee_bps <= 10_000`, so it always fits back into a `u64`:
        // `try_from` states that instead of hiding a truncation behind `as`.
        let fee = u128::from(amount) * u128::from(self.fee_bps) / 10_000;
        let fee = u64::try_from(fee).expect("the fee is at most `amount`");
        let credited = receiver
            .checked_add(amount - fee)
            .ok_or(LedgerError::Overflow)?;
        let fees = books.fees.checked_add(fee).ok_or(LedgerError::Overflow)?;

        books.accounts.get_mut(from).expect("checked above").balance -= amount;
        books.accounts.get_mut(to).expect("checked above").balance = credited;
        books.fees = fees;
        books.history.push(Record {
            from: from.to_string(),
            to: to.to_string(),
            amount,
            fee,
            memo: memo.to_string(),
        });
        // Review (1): the books are updated, so release the `RefMut` BEFORE
        // calling code we don't control. An observer may call back into the
        // ledger (`balance`, `statement`, ...), and each of those borrows the
        // books again. With the guard still alive that is "RefCell already
        // mutably borrowed". Notifying before the update would not help
        // either: the spec says observers see the new balances.
        drop(books);

        let transfer = Transfer {
            from,
            to,
            amount,
            fee,
        };
        for observer in &self.observers {
            observer(self, &transfer);
        }
        Ok(fee)
    }

    // One line per transfer, oldest first: `alice -> bob: 1000 (fee 3) Rent`.
    // A memo longer than 16 characters shows its first 16 characters and
    // "...".
    fn history(&self) -> Vec<String> {
        let books = self.books.borrow();
        let mut lines = Vec::new();
        for record in &books.history {
            // Review (4): `len()` and `[..16]` count BYTES, and a `str` may
            // only be sliced where a character starts. `char_indices().nth(16)`
            // finds the byte offset of the 17th character, if there is one:
            // that is exactly where to cut, and it walks at most 17
            // characters.
            let memo = match record.memo.char_indices().nth(16) {
                Some((cut, _)) => format!("{}...", &record.memo[..cut]),
                None => record.memo.clone(),
            };
            let (from, to) = (&record.from, &record.to);
            lines.push(format!(
                "{from} -> {to}: {} (fee {}) {memo}",
                record.amount, record.fee
            ));
        }
        lines
    }

    // One line per account, sorted by account name: `alice: 1000`.
    fn statement(&self) -> Vec<String> {
        let books = self.books.borrow();
        books
            .accounts
            .iter()
            .map(|(name, account)| format!("{name}: {}", account.balance))
            .collect()
    }

    // The balance for the old mobile app, whose API has a 32-bit balance
    // field. A balance that does not fit is a `TooLargeForLegacy` error.
    fn legacy_balance(&self, name: &str) -> Result<u32, LedgerError> {
        let balance = self.balance(name).ok_or_else(|| unknown(name))?;
        // Review (2): `as u32` keeps the low 32 bits and never fails.
        // `u32::try_from` fails exactly when the value does not fit, and the
        // error becomes the one the spec asks for.
        u32::try_from(balance).map_err(|_| LedgerError::TooLargeForLegacy { balance })
    }

    // Runs one line typed into the admin console:
    //     deposit <name> <amount>
    //     transfer <from> <to> <amount> [memo ...]
    // People make typos here, so bad input must come back as an error
    // (`UnknownCommand`, `Malformed` or `BadAmount`), never crash the console.
    fn apply(&self, line: &str) -> Result<(), LedgerError> {
        let mut words = line.split_whitespace();
        let malformed = || LedgerError::Malformed(line.to_string());
        match words.next() {
            Some("deposit") => {
                let (Some(name), Some(amount), None) = (words.next(), words.next(), words.next())
                else {
                    return Err(malformed());
                };
                self.deposit(name, parse_amount(amount)?)
            }
            Some("transfer") => {
                let (Some(from), Some(to), Some(amount)) =
                    (words.next(), words.next(), words.next())
                else {
                    return Err(malformed());
                };
                let memo = words.collect::<Vec<_>>().join(" ");
                self.transfer(from, to, parse_amount(amount)?, &memo)
                    .map(|_fee| ())
            }
            Some(other) => Err(LedgerError::UnknownCommand(other.to_string())),
            None => Err(malformed()),
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    // A ledger with a 0.3% fee (30 basis points) and the given accounts, all
    // without a transfer limit.
    fn ledger_with(accounts: &[(&str, u64)]) -> Ledger {
        let ledger = Ledger::new(30);
        for &(name, balance) in accounts {
            ledger.open(name, None).unwrap();
            ledger.deposit(name, balance).unwrap();
        }
        ledger
    }

    // ---- Happy path: these pass already and must keep passing ----

    #[test]
    fn transfer_moves_money_and_keeps_the_fee() {
        let ledger = ledger_with(&[("alice", 10_000), ("bob", 0)]);
        assert_eq!(ledger.transfer("alice", "bob", 1_000, "rent"), Ok(3));
        assert_eq!(ledger.balance("alice"), Some(9_000));
        assert_eq!(ledger.balance("bob"), Some(997));
        assert_eq!(ledger.fees(), 3);
        assert_eq!(ledger.history(), ["alice -> bob: 1000 (fee 3) rent"]);
    }

    #[test]
    fn fee_is_rounded_down() {
        let ledger = ledger_with(&[("alice", 100_000), ("bob", 0)]);
        // 0.3% of 9_999 is 29.997, of 333 is 0.999, of 399 is 1.197, and of
        // 10_000 exactly 30.
        assert_eq!(ledger.transfer("alice", "bob", 9_999, "a"), Ok(29));
        assert_eq!(ledger.transfer("alice", "bob", 333, "b"), Ok(0));
        assert_eq!(ledger.transfer("alice", "bob", 399, "c"), Ok(1));
        assert_eq!(ledger.transfer("alice", "bob", 10_000, "d"), Ok(30));
        assert_eq!(ledger.fees(), 60);
        let sent = 9_999 + 333 + 399 + 10_000;
        assert_eq!(ledger.balance("alice"), Some(100_000 - sent));
        assert_eq!(ledger.balance("bob"), Some(sent - 60));
    }

    #[test]
    fn failed_operations_change_nothing() {
        let ledger = ledger_with(&[("alice", 1_000), ("bob", 50)]);
        ledger.open("carol", Some(500)).unwrap();
        ledger.deposit("carol", 2_000).unwrap();
        let before = [
            ledger.balance("alice"),
            ledger.balance("bob"),
            ledger.balance("carol"),
        ];

        let err = ledger.transfer("alice", "nobody", 10, "x");
        assert_eq!(err, Err(LedgerError::UnknownAccount("nobody".to_string())));
        let err = ledger.transfer("nobody", "alice", 10, "x");
        assert_eq!(err, Err(LedgerError::UnknownAccount("nobody".to_string())));
        let err = ledger.transfer("alice", "alice", 10, "x");
        assert_eq!(err, Err(LedgerError::SameAccount));
        let err = ledger.transfer("bob", "alice", 51, "x");
        let short = LedgerError::InsufficientFunds {
            needed: 51,
            available: 50,
        };
        assert_eq!(err, Err(short));
        let err = ledger.transfer("carol", "alice", 501, "x");
        let over = LedgerError::OverLimit {
            limit: 500,
            amount: 501,
        };
        assert_eq!(err, Err(over));
        assert_eq!(
            ledger.open("bob", None),
            Err(LedgerError::DuplicateAccount("bob".to_string()))
        );
        assert_eq!(
            ledger.deposit("alice", u64::MAX),
            Err(LedgerError::Overflow)
        );

        let after = [
            ledger.balance("alice"),
            ledger.balance("bob"),
            ledger.balance("carol"),
        ];
        assert_eq!(after, before);
        assert_eq!(ledger.fees(), 0);
        assert!(ledger.history().is_empty());
    }

    #[test]
    fn short_and_plain_ascii_memos_show_up_in_the_history() {
        let ledger = ledger_with(&[("alice", 10_000), ("bob", 0)]);
        ledger.transfer("alice", "bob", 100, "Coffee").unwrap();
        ledger
            .transfer("alice", "bob", 100, "sixteen chars ok")
            .unwrap();
        ledger
            .transfer("alice", "bob", 100, "Monthly rent for the apartment")
            .unwrap();
        ledger.transfer("alice", "bob", 100, "Café").unwrap();
        assert_eq!(
            ledger.history(),
            [
                "alice -> bob: 100 (fee 0) Coffee",
                "alice -> bob: 100 (fee 0) sixteen chars ok",
                "alice -> bob: 100 (fee 0) Monthly rent for...",
                "alice -> bob: 100 (fee 0) Café",
            ]
        );
    }

    #[test]
    fn statement_of_a_single_account() {
        let ledger = ledger_with(&[("alice", 1_234)]);
        assert_eq!(ledger.statement(), ["alice: 1234"]);
    }

    #[test]
    fn legacy_balance_of_a_normal_account() {
        let ledger = ledger_with(&[("alice", 1_234)]);
        assert_eq!(ledger.legacy_balance("alice"), Ok(1_234));
        let err = ledger.legacy_balance("nobody");
        assert_eq!(err, Err(LedgerError::UnknownAccount("nobody".to_string())));
    }

    #[test]
    fn admin_console_runs_well_formed_commands() {
        let ledger = ledger_with(&[("alice", 0), ("bob", 0)]);
        assert_eq!(ledger.apply("deposit alice 10000"), Ok(()));
        assert_eq!(
            ledger.apply("  transfer  alice bob 1000   rent  for May "),
            Ok(())
        );
        assert_eq!(ledger.balance("alice"), Some(9_000));
        assert_eq!(ledger.balance("bob"), Some(997));
        assert_eq!(
            ledger.history(),
            ["alice -> bob: 1000 (fee 3) rent for May"]
        );

        let err = ledger.apply("withdraw alice 5");
        assert_eq!(
            err,
            Err(LedgerError::UnknownCommand("withdraw".to_string()))
        );
        for line in [
            "",
            "   ",
            "deposit alice",
            "deposit alice 5 6",
            "transfer alice bob",
        ] {
            assert_eq!(
                ledger.apply(line),
                Err(LedgerError::Malformed(line.to_string())),
                "input: {line:?}"
            );
        }
        let err = ledger.apply("transfer alice carol 5");
        assert_eq!(err, Err(LedgerError::UnknownAccount("carol".to_string())));
        assert_eq!(ledger.balance("alice"), Some(9_000));
    }

    #[test]
    fn observers_hear_about_every_successful_transfer() {
        let mut ledger = ledger_with(&[("alice", 10_000), ("bob", 0)]);
        let heard = Rc::new(RefCell::new(Vec::new()));
        for id in [1, 2] {
            let heard = Rc::clone(&heard);
            ledger.subscribe(move |_ledger, t| {
                let entry = (id, t.from.to_string(), t.to.to_string(), t.amount, t.fee);
                heard.borrow_mut().push(entry);
            });
        }
        ledger.transfer("alice", "bob", 1_000, "rent").unwrap();
        // A failed transfer notifies nobody.
        ledger
            .transfer("bob", "alice", 5_000, "too much")
            .unwrap_err();
        ledger.transfer("bob", "alice", 100, "back").unwrap();
        let alice = || "alice".to_string();
        let bob = || "bob".to_string();
        assert_eq!(
            *heard.borrow(),
            [
                (1, alice(), bob(), 1_000, 3),
                (2, alice(), bob(), 1_000, 3),
                (1, bob(), alice(), 100, 0),
                (2, bob(), alice(), 100, 0),
            ]
        );
    }

    // ---- Bug reports from QA: these fail until the bugs are fixed ----

    // "I moved the proceeds of a house sale and the app crashed."
    #[test]
    fn large_transfer_does_not_crash_and_charges_the_exact_fee() {
        let amount = 12_345_678_901_234_567_399;
        let ledger = ledger_with(&[("alice", amount), ("bob", 0)]);
        // 0.3% of 12_345_678_901_234_567_399 is 37_037_036_703_703_702.197.
        let fee = 37_037_036_703_703_702;
        assert_eq!(ledger.transfer("alice", "bob", amount, "house"), Ok(fee));
        assert_eq!(ledger.balance("alice"), Some(0));
        assert_eq!(ledger.balance("bob"), Some(amount - fee));
        assert_eq!(ledger.fees(), fee);

        // The largest possible transfer. 0.3% of `u64::MAX` is
        // 55_340_232_221_128_654.845.
        let ledger = ledger_with(&[("alice", u64::MAX), ("bob", 0)]);
        let fee = 55_340_232_221_128_654;
        assert_eq!(ledger.transfer("alice", "bob", u64::MAX, "all"), Ok(fee));
        assert_eq!(ledger.balance("bob"), Some(u64::MAX - fee));
    }

    // "Opening the history after paying for concert tickets crashes the app."
    #[test]
    fn history_with_a_long_non_english_memo_does_not_crash() {
        let ledger = ledger_with(&[("alice", 10_000), ("bob", 0)]);
        let memos = [
            // 33 characters: the first 16, then "...".
            ("Konzertkarten für Jürgen und Anna", "Konzertkarten fü..."),
            // 10 characters (20 bytes): short enough to show in full.
            ("ÄÖÜäöüßÄÖÜ", "ÄÖÜäöüßÄÖÜ"),
            // Exactly 16 characters (19 bytes): shown in full.
            ("Grüße aus Zürich", "Grüße aus Zürich"),
            // 17 characters (68 bytes): the first 16, then "...".
            (
                "🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉",
                "🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉...",
            ),
        ];
        for (memo, _) in memos {
            ledger.transfer("alice", "bob", 10, memo).unwrap();
        }
        let history = ledger.history();
        assert_eq!(history.len(), memos.len());
        for (line, (memo, shown)) in history.iter().zip(memos) {
            assert_eq!(
                *line,
                format!("alice -> bob: 10 (fee 0) {shown}"),
                "memo: {memo:?}"
            );
        }
    }

    // "The old mobile app says I have 705_032_704, but I have 5 billion."
    #[test]
    fn legacy_app_never_shows_a_wrong_balance() {
        let max = u64::from(u32::MAX);
        let ledger = ledger_with(&[
            ("fits", max),
            ("just_over", max + 1),
            ("rich", 5_000_000_000),
        ]);
        assert_eq!(ledger.legacy_balance("fits"), Ok(u32::MAX));
        assert_eq!(
            ledger.legacy_balance("just_over"),
            Err(LedgerError::TooLargeForLegacy { balance: max + 1 })
        );
        assert_eq!(
            ledger.legacy_balance("rich"),
            Err(LedgerError::TooLargeForLegacy {
                balance: 5_000_000_000
            })
        );
    }

    // "The statement comes out in a different order every time."
    #[test]
    fn statement_is_sorted_by_account_name() {
        let names = [
            "mallory", "bob2", "carol", "alice", "bob", "zed", "dave", "erin", "frank", "bob-jr",
            "grace", "heidi",
        ];
        let ledger = Ledger::new(30);
        for (i, name) in (1..).zip(names) {
            ledger.open(name, None).unwrap();
            ledger.deposit(name, i * 100).unwrap();
        }
        // Sorted by NAME: "bob" comes before "bob-jr" and "bob2".
        let expected = [
            "alice: 400",
            "bob: 500",
            "bob-jr: 1000",
            "bob2: 200",
            "carol: 300",
            "dave: 700",
            "erin: 800",
            "frank: 900",
            "grace: 1100",
            "heidi: 1200",
            "mallory: 100",
            "zed: 600",
        ];
        assert_eq!(ledger.statement(), expected);
    }

    // "Since we added the audit log, every transfer crashes."
    #[test]
    fn observer_can_read_balances_during_a_transfer() {
        let mut ledger = ledger_with(&[("alice", 10_000), ("bob", 0)]);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let audit = Rc::clone(&seen);
        ledger.subscribe(move |ledger, t| {
            let entry = (t.amount, ledger.balance(t.from), ledger.balance(t.to));
            audit.borrow_mut().push(entry);
        });
        assert_eq!(ledger.transfer("alice", "bob", 1_000, "rent"), Ok(3));
        assert_eq!(ledger.transfer("bob", "alice", 97, "change"), Ok(0));
        // The audit log saw the balances AFTER each transfer.
        assert_eq!(
            *seen.borrow(),
            [
                (1_000, Some(9_000), Some(997)),
                (97, Some(900), Some(9_097))
            ]
        );
    }

    // "My limit is 500, and a transfer of 500 is refused."
    #[test]
    fn transfer_of_exactly_the_limit_goes_through() {
        let ledger = ledger_with(&[("bob", 0)]);
        ledger.open("alice", Some(500)).unwrap();
        ledger.deposit("alice", 1_000).unwrap();
        assert_eq!(ledger.transfer("alice", "bob", 500, "max"), Ok(1));
        assert_eq!(ledger.balance("alice"), Some(500));
        assert_eq!(ledger.balance("bob"), Some(499));

        // The highest limit there is: every amount is within it.
        ledger.open("carol", Some(u64::MAX)).unwrap();
        ledger.deposit("carol", 1_000).unwrap();
        assert_eq!(ledger.transfer("carol", "bob", 1_000, "all"), Ok(3));
        assert_eq!(ledger.balance("bob"), Some(499 + 997));
    }

    // "I typed 12x instead of 12 into the admin console and it crashed."
    #[test]
    fn typo_in_an_admin_command_is_an_error_not_a_crash() {
        let ledger = ledger_with(&[("alice", 100), ("bob", 0)]);
        let bad = |word: &str| Err(LedgerError::BadAmount(word.to_string()));
        assert_eq!(ledger.apply("deposit alice 12x"), bad("12x"));
        assert_eq!(ledger.apply("deposit alice -5"), bad("-5"));
        // `u64::MAX + 1`
        assert_eq!(
            ledger.apply("deposit alice 18446744073709551616"),
            bad("18446744073709551616")
        );
        assert_eq!(ledger.apply("transfer alice bob ten coffee"), bad("ten"));
        assert_eq!(ledger.balance("alice"), Some(100));
        assert_eq!(ledger.balance("bob"), Some(0));
    }
}
