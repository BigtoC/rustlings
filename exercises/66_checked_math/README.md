# Module 5 · Checked Math: Overflow-Safe `mul_div`, Rounding Direction, Fixed Point and Determinism

> The arithmetic questions of blockchain and DeFi interviews, and of any
> backend that moves money. It follows `31_debugging/debugging2` (overflow
> panics in debug builds and wraps in release builds) and builds on
> `23_conversions` (`as`, `FromStr`, `TryFrom`) and `35_error_design` (error
> enums). All **std**, **100% safe**, **stable** Rust, edition 2024.
>
> The `// TODO` comments name the failing test and the requirements but
> **not** the fix, as in an interview. Read what the tests say first. Press
> `h` when you want the full answer.

## Core Ideas

- **Overflow is a choice you make on purpose.** `31_debugging/debugging2`
  covers the default (a panic in debug builds, wrapping in release builds)
  and the `checked_*` / `saturating_*` / `wrapping_*` methods. Money code
  often turns the checks on for release builds too, with
  `overflow-checks = true` in the profile (the CosmWasm and Anchor project
  templates do). And integer division and remainder by zero panic in every
  build.
- **`a * b / d` needs the full product.** Two `u128` factors never need more
  than 256 bits. `u128::carrying_mul(b, 0)` (stable since 1.91, though not yet
  usable in a `const fn`) returns that product as `(low, high)`; the quotient
  fits in 128 bits exactly when `high < d`. `widening_mul` is still unstable.
  Dividing first loses the remainder, and `f64` has only 53 bits of mantissa.
- **Round against the user.** Every integer division rounds, and the unit it
  moves goes either to the user or to everyone else in the pool. Round what
  the user receives down and what the user pays up, and no trade can lower
  the value of a share.
- **Fixed point instead of floats.** Store an integer count of the smallest
  unit (`Decimal(n)` means `n / 10^18`). Addition is exact; multiplication
  and division go through `mul_div`. Parse and print without `f64`, with one
  canonical string per value.
- **Consensus code must be deterministic.** No `HashMap` iteration order, no
  floats, no unspecified hash functions, no truncating casts: the same state
  must produce the same bytes and the same digest on every machine and with
  every compiler.

## Rounding Direction in a Vault

A vault holds `A` assets and has issued `S` shares, so one share is worth
`A / S`. Each trade fixes one side and converts the other:

| Trade      | The user gives   | The user gets    | Computed         | Rounds |
| ---------- | ---------------- | ---------------- | ---------------- | ------ |
| `deposit`  | exactly `assets` | shares           | `assets * S / A` | down   |
| `mint`     | assets           | exactly `shares` | `shares * A / S` | up     |
| `withdraw` | shares (burned)  | exactly `assets` | `assets * S / A` | up     |
| `redeem`   | exactly `shares` | assets           | `shares * A / S` | down   |

With that table, `A' * S >= A * S'` after every trade. Rounding down does
not protect the depositor from a share that is worth a lot: in the
first-depositor ("inflation" or donation) attack, the attacker mints one
share, sends the vault assets directly, and the victim's deposit rounds down
to zero shares. Refuse trades that mint nothing, and use virtual shares and
assets (an offset in both totals) or seed the vault.

## What `as` Does

`as` never fails and never panics, in any build:

| Cast                                 | Result                                            |
| ------------------------------------ | ------------------------------------------------- |
| integer to a narrower integer        | keeps the low bits: `(2^64 + 5) as u64 == 5`      |
| signed to unsigned of the same width | reinterprets the bits: `-1i32 as u32 == u32::MAX` |
| float to integer                     | rounds toward zero and saturates; NaN becomes 0   |
| integer to float                     | rounds to the nearest float                       |

Use `u64::try_from(x)` when the value might not fit, and `u128::from(x)`
when it always does. std has no `From<usize> for u64`, because `usize` is
not guaranteed to be at most 64 bits wide.

## Sources of Nondeterminism

| Source                        | Why it differs between nodes                                                              | Deterministic replacement                        |
| ----------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `HashMap` iteration           | the order is arbitrary, and `RandomState` seeds every map differently                     | `BTreeMap`, or sort the entries before use       |
| `f64` money math              | inexact (0.29, 1e30); `powf` and `exp` have "non-deterministic" precision                 | integers with an explicit rounding, `mul_div`    |
| `DefaultHasher`, `Hash` impls | the algorithm "should not be relied upon over releases"; `Hash` feeds native-endian bytes | a specified hash over a byte encoding you define |
| `as` casts                    | silently truncate or saturate, so different values encode the same                        | `try_from`, and every field at its full width    |

## Exercise Path

1. **checkedmath1** — `mul_div_floor` and `mul_div_ceil` start as
   `checked_mul` plus the textbook `(x + d - 1) / d`, so 500 million tokens
   times a price of 1.5 returns `None`, a zero divisor panics, and the
   ceiling overflows near `u128::MAX`. Compute the exact 256-bit product,
   divide it with the given `div_wide`, and round up only on a remainder.
2. **checkedmath2** — Vault share math. All four previews round down, so
   minting a share at a price of 1.5 costs 1 asset and a withdrawal can burn
   no shares at all. Pick the rounding direction of each preview so that no
   trade lowers the share price.
3. **checkedmath3** — A `Decimal(u128)` with 18 decimals whose `FromStr`,
   `Display`, `checked_mul` and `checked_div` are `todo!()`. Parse strictly
   (no `f64`, no sign, at most 18 decimals, overflow reported as overflow),
   print one canonical form, and multiply and divide through `mul_div`, so
   that `1.1 * 3 == 3.3` exactly. The `todo!()`s panic, and Clippy's `todo`
   lint (forbidden in this course) rejects any that are left.
4. **checkedmath4** — A ledger's `state_digest` walks a `HashMap`, feeds
   `Hash` impls to `DefaultHasher` and casts balances with `as u64`, and its
   interest uses `f64`. Iterate in name order, hash a documented byte
   encoding with the given FNV-1a, pay interest in basis points with
   all-or-nothing integer math, and export with `u64::try_from`.

Related: `34_iterators/iter1` ends an iterator with `checked_add`, and
`44_trait_contracts/contracts1` shows the bytes a `Hash` impl feeds a hasher.
checkedmath1's randomized comparison against an exact oracle is a
hand-rolled property test. The planned `api-surface-lab` deep-dive lab (its
`integration_and_proptest` part) writes such tests with proptest, which also
shrinks a failing input to a minimal counterexample.
Next, `67_code_review/review1` hides an `amount * fee / 10_000` overflow, a
`balance as u32` truncation and a `HashMap`-ordered statement in a pull
request, without telling you where.

## Further Reading

- [Overflow (The Reference)](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow) and [numeric casts (The Reference)](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast)
- [`overflow-checks` (The Cargo Book)](https://doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks)
- [`u128::carrying_mul`](https://doc.rust-lang.org/std/primitive.u128.html#method.carrying_mul), [`u128::checked_mul`](https://doc.rust-lang.org/std/primitive.u128.html#method.checked_mul) and [`u128::div_ceil`](https://doc.rust-lang.org/std/primitive.u128.html#method.div_ceil)
- Remco Bloemen, [Mathemagic finale: muldiv](https://xn--2-umb.com/21/muldiv/), the 512-bit `mulDiv` behind Uniswap v3's [`FullMath.sol`](https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/FullMath.sol)
- [ERC-4626: Tokenized Vaults](https://eips.ethereum.org/EIPS/eip-4626), whose Security Considerations state the rounding rule, and OpenZeppelin on [the inflation attack and the virtual offset](https://docs.openzeppelin.com/contracts/5.x/erc4626)
- [`cosmwasm_std::Decimal`](https://docs.rs/cosmwasm-std/latest/cosmwasm_std/struct.Decimal.html), a production 18-decimal fixed-point type
- [What Every Computer Scientist Should Know About Floating-Point Arithmetic](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html), and the "Unspecified precision" note on [`f64::powf`](https://doc.rust-lang.org/std/primitive.f64.html#method.powf)
- [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html), [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html), [`DefaultHasher`](https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html) and the [portability of `Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html#portability)
- [RFC 9923: The FNV Non-Cryptographic Hash Algorithm](https://www.rfc-editor.org/rfc/rfc9923.html)
- [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) and [`TryFromIntError`](https://doc.rust-lang.org/std/num/struct.TryFromIntError.html)
