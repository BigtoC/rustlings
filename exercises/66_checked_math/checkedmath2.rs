// Module 5 · Checked math — part 2: which way vault share math must round.
//
// A vault is a pool of assets owned pro rata by the holders of its shares: a
// fund, a staking or lending pool, a liquidity pool. If it holds A assets
// and has issued S shares, one share is worth A / S assets. Users trade in
// four ways:
//
//     deposit(assets)   pay exactly `assets`, receive shares
//     mint(shares)      receive exactly `shares`, pay assets
//     withdraw(assets)  receive exactly `assets`, burn shares
//     redeem(shares)    burn exactly `shares`, receive assets
//
// Each conversion is part 1's `mul_div`: `assets * S / A` or `shares * A / S`,
// and the division almost never comes out even. Rounding moves at most one
// unit, but it moves it between the user and every OTHER holder, and an
// attacker can repeat a trade thousands of times. So the rule is: round
// against the user, in favor of the vault. What the user receives rounds
// DOWN; what the user pays rounds UP. Then no trade, however small, lowers the
// value of a share, which the tests check as
//
//     A' / S' >= A / S,   cross-multiplied:   A' * S >= A * S'
//
// (cross-multiplying avoids dividing and rounding in the check itself).
// Ethereum's ERC-4626 vault standard writes exactly this rule down, but it
// has nothing to do with Ethereum: any system that issues claims on a pool
// needs it. Rounding in the user's favor is a recurring finding in
// smart-contract audits, and the attack is literally "repeat the trade until
// the pool is empty".
//
// Rounding down does not protect the depositor, though. In the "inflation"
// (or donation) attack, the first depositor mints 1 share and then sends the
// vault a large amount of assets directly, without minting. Now one share is
// worth, say, 1_001 assets, and the next deposit of 1_000 assets rounds down
// to 0 shares: the victim's assets now belong to the attacker's share. That is
// why `deposit` below refuses a trade that would mint nothing
// (`RoundsToZero`). Production vaults go further with "virtual" shares and
// assets (an offset added to both totals) or by seeding the vault.
//
// How interviewers probe this: "Which way do deposits and withdrawals round,
// and why? Who pays for the rounding? What is the first-depositor attack and
// how do you prevent it?"

/// A rounding direction for `mul_div`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rounding {
    Down,
    Up,
}

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

/// `a * b / d` through the exact 256-bit product, rounded as asked. `None`
/// when `d == 0` or the result does not fit in a `u128`. (Part 1's solution,
/// given.)
fn mul_div(a: u128, b: u128, d: u128, rounding: Rounding) -> Option<u128> {
    let (lo, hi) = a.carrying_mul(b, 0);
    let (quot, rem) = div_wide(hi, lo, d)?;
    match rounding {
        Rounding::Up if rem != 0 => quot.checked_add(1),
        _ => Some(quot),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum VaultError {
    /// The user would receive nothing: no shares for a deposit, no assets
    /// for a redemption, or a mint or withdrawal of zero.
    RoundsToZero,
    /// More shares than exist, or more assets than the vault holds.
    Insufficient,
    /// A result or a new total does not fit in a `u128`, or the vault has
    /// shares but no assets to price them with.
    Overflow,
}

#[derive(Debug, PartialEq, Eq)]
struct Vault {
    total_assets: u128,
    total_shares: u128,
}

impl Vault {
    fn new() -> Self {
        Self::with_totals(0, 0)
    }

    fn with_totals(total_assets: u128, total_shares: u128) -> Self {
        Vault {
            total_assets,
            total_shares,
        }
    }

    /// `assets` expressed in shares at the current price. A vault with no
    /// shares yet trades 1:1.
    fn convert_to_shares(&self, assets: u128, rounding: Rounding) -> Result<u128, VaultError> {
        if self.total_shares == 0 {
            return Ok(assets);
        }
        mul_div(assets, self.total_shares, self.total_assets, rounding).ok_or(VaultError::Overflow)
    }

    /// `shares` expressed in assets at the current price. A vault with no
    /// shares yet trades 1:1.
    fn convert_to_assets(&self, shares: u128, rounding: Rounding) -> Result<u128, VaultError> {
        if self.total_shares == 0 {
            return Ok(shares);
        }
        mul_div(shares, self.total_assets, self.total_shares, rounding).ok_or(VaultError::Overflow)
    }

    // TODO: all four previews round down, and that is wrong for two of
    // them. `minting_one_share_pays_the_full_price` fails (A = 3, S = 2: one
    // share is worth 1.5 assets, but minting it costs 1, and the share price
    // falls from 1.5 to 4/3), `withdrawing_one_asset_burns_a_share` fails (the
    // withdrawal burns no shares at all), and so does the price invariant in
    // `no_trade_lowers_the_share_price`. Requirement: every preview rounds
    // against the user, so that no trade lowers the value of a share. Decide,
    // for each of the four, which way that is. Only the `Rounding` arguments
    // need to change: don't add or subtract units by hand, and don't refuse
    // trades that the vault can price. Until you do, the tests will fail.

    /// Shares minted by depositing exactly `assets`.
    fn preview_deposit(&self, assets: u128) -> Result<u128, VaultError> {
        self.convert_to_shares(assets, Rounding::Down)
    }

    /// Assets the user pays to mint exactly `shares`.
    fn preview_mint(&self, shares: u128) -> Result<u128, VaultError> {
        self.convert_to_assets(shares, Rounding::Down)
    }

    /// Shares burned to withdraw exactly `assets`.
    fn preview_withdraw(&self, assets: u128) -> Result<u128, VaultError> {
        self.convert_to_shares(assets, Rounding::Down)
    }

    /// Assets paid out for redeeming exactly `shares`.
    fn preview_redeem(&self, shares: u128) -> Result<u128, VaultError> {
        self.convert_to_assets(shares, Rounding::Down)
    }

    // The four trades. They price with the previews, refuse a trade in which
    // the user receives nothing, and change the totals only on success.

    /// Returns the shares minted.
    fn deposit(&mut self, assets: u128) -> Result<u128, VaultError> {
        let shares = self.preview_deposit(assets)?;
        if shares == 0 {
            return Err(VaultError::RoundsToZero);
        }
        self.add(assets, shares)?;
        Ok(shares)
    }

    /// Returns the assets paid.
    fn mint(&mut self, shares: u128) -> Result<u128, VaultError> {
        if shares == 0 {
            return Err(VaultError::RoundsToZero);
        }
        let assets = self.preview_mint(shares)?;
        self.add(assets, shares)?;
        Ok(assets)
    }

    /// Returns the shares burned.
    fn withdraw(&mut self, assets: u128) -> Result<u128, VaultError> {
        if assets == 0 {
            return Err(VaultError::RoundsToZero);
        }
        if assets > self.total_assets {
            return Err(VaultError::Insufficient);
        }
        let shares = self.preview_withdraw(assets)?;
        self.remove(assets, shares)?;
        Ok(shares)
    }

    /// Returns the assets paid out.
    fn redeem(&mut self, shares: u128) -> Result<u128, VaultError> {
        if shares > self.total_shares {
            return Err(VaultError::Insufficient);
        }
        let assets = self.preview_redeem(shares)?;
        if assets == 0 {
            return Err(VaultError::RoundsToZero);
        }
        self.remove(assets, shares)?;
        Ok(assets)
    }

    /// The vault earns `assets` of yield, or someone sends it assets
    /// directly: the assets grow and the shares don't, so every share is
    /// worth more.
    fn earn(&mut self, assets: u128) -> Result<(), VaultError> {
        self.add(assets, 0)
    }

    fn add(&mut self, assets: u128, shares: u128) -> Result<(), VaultError> {
        let total_assets = self.total_assets.checked_add(assets);
        let total_shares = self.total_shares.checked_add(shares);
        let (Some(total_assets), Some(total_shares)) = (total_assets, total_shares) else {
            return Err(VaultError::Overflow);
        };
        (self.total_assets, self.total_shares) = (total_assets, total_shares);
        Ok(())
    }

    fn remove(&mut self, assets: u128, shares: u128) -> Result<(), VaultError> {
        let total_assets = self.total_assets.checked_sub(assets);
        let total_shares = self.total_shares.checked_sub(shares);
        let (Some(total_assets), Some(total_shares)) = (total_assets, total_shares) else {
            return Err(VaultError::Insufficient);
        };
        (self.total_assets, self.total_shares) = (total_assets, total_shares);
        Ok(())
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    type Trade = fn(&mut Vault, u128) -> Result<u128, VaultError>;

    const TRADES: [(&str, Trade); 4] = [
        ("deposit", Vault::deposit),
        ("mint", Vault::mint),
        ("withdraw", Vault::withdraw),
        ("redeem", Vault::redeem),
    ];

    // A' / S' >= A / S, cross-multiplied. Only for small totals, so that the
    // products fit.
    fn price_did_not_fall(before: &Vault, after: &Vault) -> bool {
        after.total_assets * before.total_shares >= before.total_assets * after.total_shares
    }

    #[test]
    fn minting_one_share_pays_the_full_price() {
        // A = 3, S = 2: one share is worth 1.5 assets.
        let mut vault = Vault::with_totals(3, 2);
        assert_eq!(
            vault.preview_mint(1),
            Ok(2),
            "1.5 assets must round UP to 2"
        );
        assert_eq!(vault.mint(1), Ok(2));
        assert_eq!(vault, Vault::with_totals(5, 3));
        assert!(price_did_not_fall(&Vault::with_totals(3, 2), &vault));
    }

    #[test]
    fn withdrawing_one_asset_burns_a_share() {
        // A = 3, S = 2: one asset is worth 2/3 of a share.
        let mut vault = Vault::with_totals(3, 2);
        assert_eq!(
            vault.preview_withdraw(1),
            Ok(1),
            "2/3 of a share must round UP to 1, or the asset is free"
        );
        assert_eq!(vault.withdraw(1), Ok(1));
        assert_eq!(vault, Vault::with_totals(2, 1));
    }

    #[test]
    fn deposits_and_redemptions_round_down() {
        let vault = Vault::with_totals(3, 2);
        assert_eq!(
            vault.preview_deposit(2),
            Ok(1),
            "2 assets buy 4/3 shares: round DOWN to 1"
        );
        assert_eq!(
            vault.preview_redeem(1),
            Ok(1),
            "1 share is worth 1.5 assets: round DOWN to 1"
        );
        assert_eq!(vault.preview_deposit(3), Ok(2), "exact, no rounding");
        assert_eq!(vault.preview_redeem(2), Ok(3), "exact, no rounding");
    }

    #[test]
    fn exact_conversions_are_not_rounded() {
        // A = 4, S = 2: one share is worth exactly 2 assets.
        let vault = Vault::with_totals(4, 2);
        assert_eq!(vault.preview_deposit(2), Ok(1));
        assert_eq!(vault.preview_mint(1), Ok(2));
        assert_eq!(vault.preview_withdraw(2), Ok(1));
        assert_eq!(vault.preview_redeem(1), Ok(2));
    }

    #[test]
    fn no_trade_lowers_the_share_price() {
        for assets in 1..=16 {
            for shares in 1..=16 {
                for amount in 1..=16 {
                    let before = Vault::with_totals(assets, shares);
                    for (name, trade) in TRADES {
                        let mut after = Vault::with_totals(assets, shares);
                        let result = trade(&mut after, amount);
                        match name {
                            "mint" => assert!(result.is_ok(), "mint({amount}) must succeed"),
                            "withdraw" if amount <= assets => {
                                assert!(result.is_ok(), "withdraw({amount}) must succeed");
                            }
                            _ => {}
                        }
                        assert!(
                            price_did_not_fall(&before, &after),
                            "{name}({amount}) on A = {assets}, S = {shares} left A = {}, \
                             S = {}: the price of a share fell",
                            after.total_assets,
                            after.total_shares
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn round_trips_never_make_money() {
        for assets in 1..=16 {
            for shares in 1..=16 {
                for amount in 1..=16 {
                    // Deposit, then redeem every share received.
                    let mut vault = Vault::with_totals(assets, shares);
                    if let Ok(minted) = vault.deposit(amount) {
                        let back = vault.redeem(minted).unwrap_or(0);
                        assert!(back <= amount, "deposit({amount}) then redeem gave {back}");
                    }
                    // Mint, then redeem the same shares.
                    let mut vault = Vault::with_totals(assets, shares);
                    let paid = vault.mint(amount).expect("mint must succeed");
                    let back = vault.redeem(amount).unwrap_or(0);
                    assert!(
                        back <= paid,
                        "mint({amount}) cost {paid} but redeemed {back}"
                    );
                }
            }
        }
    }

    #[test]
    fn big_vaults_round_the_same_way() {
        // A = 3e30, S = 2e30: one share is worth 1.5 assets, and every
        // product below is far above u128::MAX.
        let e30 = 10u128.pow(30);
        let vault = Vault::with_totals(3 * e30, 2 * e30);
        let one_and_a_half = 15 * 10u128.pow(29);
        // (e30 + 1) shares are worth 1.5e30 + 1.5 assets.
        assert_eq!(vault.preview_mint(e30 + 1), Ok(one_and_a_half + 2));
        assert_eq!(vault.preview_redeem(e30 + 1), Ok(one_and_a_half + 1));
        // (1.5e30 + 1) assets are worth 1e30 + 2/3 shares.
        assert_eq!(vault.preview_withdraw(one_and_a_half + 1), Ok(e30 + 1));
        assert_eq!(vault.preview_deposit(one_and_a_half + 1), Ok(e30));
    }

    #[test]
    fn an_empty_vault_trades_one_to_one() {
        let mut vault = Vault::new();
        assert_eq!(vault.deposit(100), Ok(100));
        assert_eq!(vault.mint(50), Ok(50));
        assert_eq!(vault, Vault::with_totals(150, 150));
        assert_eq!(vault.redeem(150), Ok(150));
        assert_eq!(vault, Vault::new());
        assert_eq!(vault.mint(7), Ok(7));
    }

    #[test]
    fn a_deposit_worth_less_than_one_share_is_refused() {
        // The inflation attack: mint 1 share, then send the vault 1_000
        // assets directly. One share is now worth 1_001 assets.
        let mut vault = Vault::new();
        assert_eq!(vault.deposit(1), Ok(1));
        assert_eq!(vault.earn(1_000), Ok(()));
        assert_eq!(vault.deposit(1_000), Err(VaultError::RoundsToZero));
        assert_eq!(
            vault,
            Vault::with_totals(1_001, 1),
            "a refused trade changes nothing"
        );
        assert_eq!(vault.deposit(1_001), Ok(1));
        assert_eq!(vault, Vault::with_totals(2_002, 2));
    }

    #[test]
    fn trades_that_give_nothing_are_refused() {
        let mut vault = Vault::with_totals(3, 2);
        for (name, trade) in TRADES {
            assert_eq!(
                trade(&mut vault, 0),
                Err(VaultError::RoundsToZero),
                "{name}(0)"
            );
        }
        // One share of a vault with 1 asset and 2 shares is worth 1/2 asset.
        let mut vault = Vault::with_totals(1, 2);
        assert_eq!(vault.redeem(1), Err(VaultError::RoundsToZero));
        assert_eq!(vault, Vault::with_totals(1, 2));
    }

    #[test]
    fn you_cannot_take_more_than_the_vault_has() {
        let mut vault = Vault::with_totals(10, 10);
        assert_eq!(vault.withdraw(11), Err(VaultError::Insufficient));
        assert_eq!(vault.redeem(11), Err(VaultError::Insufficient));
        assert_eq!(vault, Vault::with_totals(10, 10));
        assert_eq!(vault.withdraw(10), Ok(10));
        assert_eq!(vault, Vault::new());
    }

    #[test]
    fn totals_that_would_overflow_are_refused() {
        let mut vault = Vault::with_totals(u128::MAX, 1);
        assert_eq!(vault.earn(1), Err(VaultError::Overflow));
        assert_eq!(vault.mint(1), Err(VaultError::Overflow));
        assert_eq!(vault, Vault::with_totals(u128::MAX, 1));
        // Shares but no assets: there is no price to trade at.
        let mut vault = Vault::with_totals(0, 5);
        assert_eq!(vault.deposit(1), Err(VaultError::Overflow));
    }
}
