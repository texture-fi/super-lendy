use std::cmp::Ordering;
use std::collections::HashSet;

use bytemuck::{Pod, Zeroable};
use solana_program::clock::{Clock, Slot};
use solana_program::{msg, pubkey::Pubkey};
use texture_common::account::{PodAccount, PodAccountError};
use texture_common::math::{
    CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Decimal, MathError, MathResult,
};

use crate::error::SuperLendyError;
use crate::state::last_update::LastUpdate;
use crate::state::reserve::{RewardRule, RewardRules};
use crate::state::POSITION_DISCRIMINATOR;
use crate::{LendyResult, MAX_AMOUNT};

static_assertions::const_assert_eq!(Position::SIZE, std::mem::size_of::<Position>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<Position>() % 8);
static_assertions::const_assert_eq!(0, std::mem::size_of::<DepositedCollateral>() % 16);
static_assertions::const_assert_eq!(0, std::mem::size_of::<BorrowedLiquidity>() % 16);

pub const MAX_DEPOSITS: usize = 10;
pub const MAX_BORROWS: usize = 10;
pub const MAX_REWARDS: usize = 10;

/// Classic Borrow/Lend position
pub const POSITION_TYPE_CLASSIC: u8 = 0;
/// Long/Short representation
pub const POSITION_TYPE_LONG_SHORT: u8 = 1;
/// Long/Short representation
pub const POSITION_TYPE_LST_LEVERAGE: u8 = 2;

/// User `position` in the system. Shows his deposits and borrows.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Position {
    pub discriminator: [u8; 8],
    pub version: u8,

    /// Position types (Classic borrow-lend vs. trading) are not differentiated by the contract - all
    /// positions are processed same way. This field makes difference in UI interpretation.
    pub position_type: u8,

    /// Vacant to store mode/status flags
    pub _flags: [u8; 6],

    /// Last update to collateral, liquidity, or their market values
    pub last_update: LastUpdate,
    /// Pool address (user have this position in that particular pool)
    pub pool: Pubkey,
    /// Owner authority which can borrow liquidity
    pub owner: Pubkey,
    /// Deposited collateral, unique by deposit reserve address
    pub collateral: [DepositedCollateral; MAX_DEPOSITS],
    /// Borrowed liquidity for the position, unique by borrow reserve address
    pub borrows: [BorrowedLiquidity; MAX_BORROWS],
    /// Tracks rewards for the position
    pub rewards: Rewards,
    /// Market value of deposits
    pub deposited_value: i128,
    /// Market value of borrows
    pub borrowed_value: i128,
    /// The maximum borrow value allowed for this position. Borrowing is not allowed for the position
    /// which has that value equal or above. Calculated as weighted average through all deposited
    /// collaterals.
    pub allowed_borrow_value: i128,
    /// The dangerous borrow value at which partial liquidations of the position become possible.
    pub partly_unhealthy_borrow_value: i128,
    /// Very dangerous borrow value at which position can be liquidated at once.
    pub fully_unhealthy_borrow_value: i128,

    pub _padding: [u8; 256],
}

impl PodAccount for Position {
    const DISCRIMINATOR: &'static [u8] = POSITION_DISCRIMINATOR;

    type Version = u8;

    const VERSION: Self::Version = 1;

    type InitParams = InitPositionParams;

    type InitError = PodAccountError;

    fn discriminator(&self) -> &[u8] {
        &self.discriminator
    }

    fn version(&self) -> Self::Version {
        self.version
    }

    fn init_unckecked(&mut self, params: Self::InitParams) -> Result<(), Self::InitError> {
        let Self {
            discriminator,
            version,
            position_type,
            _flags,
            last_update,
            pool,
            owner,
            collateral: deposits,
            borrows,
            rewards,
            deposited_value,
            borrowed_value,
            allowed_borrow_value,
            partly_unhealthy_borrow_value,
            fully_unhealthy_borrow_value,
            _padding,
        } = self;

        *discriminator = *POSITION_DISCRIMINATOR;
        *version = Self::VERSION;
        *position_type = params.position_type;
        *last_update = LastUpdate::new(0, 0);
        *owner = params.owner;
        *pool = params.pool;
        *deposits = Zeroable::zeroed();
        *borrows = Zeroable::zeroed();
        *deposited_value = Decimal::ZERO.into_bits().unwrap();
        *borrowed_value = Decimal::ZERO.into_bits().unwrap();
        *allowed_borrow_value = Decimal::ZERO.into_bits().unwrap();
        *partly_unhealthy_borrow_value = Decimal::ZERO.into_bits().unwrap();
        *fully_unhealthy_borrow_value = Decimal::ZERO.into_bits().unwrap();
        *rewards = Zeroable::zeroed();
        *_padding = Zeroable::zeroed();
        *_flags = Zeroable::zeroed();

        Ok(())
    }
}

impl Position {
    // Constructor. Mainly for testing purposes.
    pub fn new(
        pool: Pubkey,
        owner: Pubkey,
        deposits: [DepositedCollateral; MAX_DEPOSITS],
        borrows: [BorrowedLiquidity; MAX_BORROWS],
    ) -> Self {
        Position {
            discriminator: *POSITION_DISCRIMINATOR,
            version: 0,
            position_type: POSITION_TYPE_CLASSIC,
            _flags: Zeroable::zeroed(),
            last_update: LastUpdate {
                slot: 0,
                timestamp: 0,
                stale: 0,
                _padding: Zeroable::zeroed(),
            },
            pool,
            owner,
            collateral: deposits,
            borrows,
            rewards: Zeroable::zeroed(),
            deposited_value: Decimal::ZERO.into_bits().unwrap(),
            borrowed_value: Decimal::ZERO.into_bits().unwrap(),
            allowed_borrow_value: Decimal::ZERO.into_bits().unwrap(),
            partly_unhealthy_borrow_value: Decimal::ZERO.into_bits().unwrap(),
            fully_unhealthy_borrow_value: Decimal::ZERO.into_bits().unwrap(),
            _padding: Zeroable::zeroed(),
        }
    }

    /// Functions to read and set Decimal amounts which are i128 inside
    pub fn deposited_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.deposited_value).map_err(From::from)
    }

    pub fn set_deposited_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.deposited_value = value.into_bits()?;
        Ok(())
    }

    pub fn borrowed_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.borrowed_value).map_err(From::from)
    }

    pub fn set_borrowed_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.borrowed_value = value.into_bits()?;
        Ok(())
    }

    pub fn allowed_borrow_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.allowed_borrow_value).map_err(From::from)
    }

    pub fn set_allowed_borrow_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.allowed_borrow_value = value.into_bits()?;
        Ok(())
    }

    pub fn partly_unhealthy_borrow_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.partly_unhealthy_borrow_value).map_err(From::from)
    }

    pub fn set_partly_unhealthy_borrow_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.partly_unhealthy_borrow_value = value.into_bits()?;
        Ok(())
    }

    pub fn fully_unhealthy_borrow_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.fully_unhealthy_borrow_value).map_err(From::from)
    }

    pub fn set_fully_unhealthy_borrow_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.fully_unhealthy_borrow_value = value.into_bits()?;
        Ok(())
    }

    /// Calculate the current ratio of borrowed value to deposited value
    pub fn ltv(&self) -> LendyResult<Decimal> {
        Ok(ltv(self.borrowed_value()?, self.deposited_value()?)?)
    }

    /// Repay liquidity
    pub fn repay(&mut self, settle_amount: Decimal, borrowing_index: usize) -> LendyResult<()> {
        if borrowing_index >= MAX_BORROWS {
            msg!(
                "liquidity_index {} beyond limit {}",
                borrowing_index,
                MAX_BORROWS
            );
            return Err(SuperLendyError::Internal(String::from(
                "liquidity_index to big during repay",
            )));
        }
        let borrowed_liquidity = &mut self.borrows[borrowing_index];

        borrowed_liquidity.repay(settle_amount)?;

        Ok(())
    }

    /// Withdraw collateral
    pub fn withdraw(&mut self, withdraw_amount: u64, collateral_index: usize) -> LendyResult<()> {
        if collateral_index >= MAX_DEPOSITS {
            msg!(
                "collateral_index {} beyond limit {}",
                collateral_index,
                MAX_DEPOSITS
            );
            return Err(SuperLendyError::Internal(String::from(
                "liquidity_index to big during repay",
            )));
        }
        let collateral = &mut self.collateral[collateral_index];

        collateral.withdraw(withdraw_amount)?;

        Ok(())
    }

    /// Calculate the maximum collateral value that can be withdrawn
    pub fn max_withdraw_value(&self, withdraw_collateral_ltv: Decimal) -> LendyResult<Decimal> {
        Ok(max_withdraw_value(
            self.allowed_borrow_value()?,
            self.borrowed_value()?,
            withdraw_collateral_ltv,
        )?)
    }

    /// Calculate the maximum liquidity value that can be borrowed
    pub fn remaining_borrow_value(&self) -> LendyResult<Decimal> {
        Ok(remaining_borrow_value(
            self.allowed_borrow_value()?,
            self.borrowed_value()?,
        )?)
    }

    /// Calculate the maximum liquidation amount for a given borrowed liquidity
    /// `borrowed_liquidity` - borrowed liquidity of this position to calculate liquidation amount against
    /// `liquidation_close_factor` - portion (aka percentage in decimal form) of the borrowed amount
    /// which can be liquidated this time.
    pub fn max_liquidation_amount(
        &self,
        borrowed_liquidity: &BorrowedLiquidity,
        liquidation_close_factor: Decimal,
    ) -> LendyResult<Decimal> {
        // When
        let max_liquidation_value = self
            .borrowed_value()?
            .checked_mul(liquidation_close_factor)?
            .min(borrowed_liquidity.market_value()?);
        let max_liquidation_pct =
            max_liquidation_value.checked_div(borrowed_liquidity.market_value()?)?;
        borrowed_liquidity
            .borrowed_amount()?
            .checked_mul(max_liquidation_pct)
            .map_err(From::from)
    }

    /// Find collateral by deposit reserve
    pub fn find_collateral(
        &self,
        deposit_reserve: Pubkey,
    ) -> LendyResult<(&DepositedCollateral, /* collateral index */ usize)> {
        let collateral_index = self
            ._find_collateral_index(deposit_reserve)
            .ok_or(SuperLendyError::DepositedCollateralNotFound)?;
        Ok((&self.collateral[collateral_index], collateral_index))
    }

    /// Find or add collateral by deposit reserve
    pub fn find_or_add_collateral(
        &mut self,
        deposit_reserve: Pubkey,
    ) -> LendyResult<&mut DepositedCollateral> {
        if let Some(collateral_index) = self._find_collateral_index(deposit_reserve) {
            return Ok(&mut self.collateral[collateral_index]);
        }

        // Already initialized deposit is not found. Thus initialize new record.
        for deposit in self.collateral.iter_mut() {
            if deposit.deposited_amount == 0 {
                // i.e. its never been initialized or fully withdrawn
                let new_deposit = DepositedCollateral::new(deposit_reserve);
                *deposit = new_deposit;
                return Ok(deposit);
            }
        }

        msg!("max limit {} for deposits reached", MAX_DEPOSITS);

        Err(SuperLendyError::ResourceExhausted)
    }

    fn _find_collateral_index(&self, deposit_reserve: Pubkey) -> Option<usize> {
        self.collateral
            .iter()
            .position(|collateral| collateral.deposit_reserve == deposit_reserve)
    }

    /// Find borrowed liquidity record by borrow reserve
    pub fn find_borrowed_liquidity(
        &self,
        borrow_reserve: Pubkey,
    ) -> LendyResult<(&BorrowedLiquidity, usize)> {
        let liquidity_index = self
            ._find_borrowed_liquidity(borrow_reserve)
            .ok_or(SuperLendyError::BorrowedLiquidityNotFound)?;
        Ok((&self.borrows[liquidity_index], liquidity_index))
    }

    pub fn find_borrowed_liquidity_mut(
        &mut self,
        borrow_reserve: Pubkey,
    ) -> LendyResult<&mut BorrowedLiquidity> {
        let liquidity_index = self
            ._find_borrowed_liquidity(borrow_reserve)
            .ok_or(SuperLendyError::BorrowedLiquidityNotFound)?;
        Ok(&mut self.borrows[liquidity_index])
    }

    /// Find or add liquidity by borrow reserve
    pub fn find_or_add_borrowed_liquidity(
        &mut self,
        borrow_reserve: Pubkey,
        cumulative_borrow_rate: Decimal,
    ) -> LendyResult<&mut BorrowedLiquidity> {
        // Try to find existing record by borrow_reserve
        if let Some(liquidity_index) = self._find_borrowed_liquidity(borrow_reserve) {
            return Ok(&mut self.borrows[liquidity_index]);
        }

        // No record found by borrow_reserve. Therefore, try to find first record with no borrowed amount.
        // This could be uninitialized record or currently not used.
        for borrowing in self.borrows.iter_mut() {
            if borrowing.borrowed_amount()? == Decimal::ZERO {
                let borrowed_liquidity =
                    BorrowedLiquidity::new(borrow_reserve, cumulative_borrow_rate);
                *borrowing = borrowed_liquidity;
                return Ok(borrowing);
            }
        }

        msg!("max limit {} for borrows reached", MAX_BORROWS);

        Err(SuperLendyError::ResourceExhausted)
    }

    pub fn have_any_borrowings(&self) -> bool {
        for borrowing in self.borrows.iter() {
            if borrowing.borrowed_amount().unwrap_or(Decimal::ZERO) != Decimal::ZERO {
                return true;
            }
        }
        false
    }

    pub fn have_any_deposits(&self) -> bool {
        for deposit in self.collateral.iter() {
            if deposit.deposited_amount != 0 {
                return true;
            }
        }
        false
    }

    fn _find_borrowed_liquidity(&self, borrow_reserve: Pubkey) -> Option<usize> {
        self.borrows
            .iter()
            .position(|liquidity| liquidity.borrow_reserve == borrow_reserve)
    }

    pub fn is_stale(&self, clock: &Clock) -> LendyResult<bool> {
        // Position is stale if it was updated longer then 1 slot ago i.e. to be not stale (by time)
        // it should be updated in current slot.
        self.last_update.is_stale_by_slot(clock.slot, 1)
    }

    pub fn mark_stale(&mut self) {
        self.last_update.mark_stale();
    }

    /// Checks is position can be safely closed.
    /// Returns None - when position can be safely closed.
    /// Returns Some(reason why it can NOT be closed) - when  position can NOT be safely closed.
    pub fn closable(&self) -> Option</* reason why it can NOT be closed */ String> {
        for (idx, deposited_collateral) in self.collateral.iter().enumerate() {
            if deposited_collateral.deposited_amount > 0 {
                return Some(format!(
                    "locked collateral exists. idx {} amount {}",
                    idx, deposited_collateral.deposited_amount
                ));
            }
        }

        for (idx, borrow) in self.borrows.iter().enumerate() {
            if borrow.borrowed_amount().expect("borrowed amount") > Decimal::ZERO {
                return Some(format!(
                    "borrow exists. idx {} amount {}",
                    idx,
                    borrow.borrowed_amount().expect("borrowed amount")
                ));
            }
        }

        for (idx, reward) in self.rewards.rewards.iter().enumerate() {
            if reward.accrued_amount > 0 {
                return Some(format!(
                    "unclaimed reward exists. idx {} amount {}",
                    idx, reward.accrued_amount
                ));
            }
        }

        None
    }
}

/// For position initialization
pub struct InitPositionParams {
    pub position_type: u8,
    /// Lending market address
    pub pool: Pubkey,
    /// Owner authority which can borrow liquidity
    pub owner: Pubkey,
}

pub const COLLATERAL_MEMO_LEN: usize = 24;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct DepositedCollateral {
    /// Reserve collateral is deposited to
    pub deposit_reserve: Pubkey,
    /// Collateral market value in quote currency at the moment of collateral lock
    pub entry_market_value: i128,
    /// Collateral market value in quote currency
    pub market_value: i128,
    /// Amount of collateral deposited
    pub deposited_amount: u64,
    /// Memo is arbitrary bytes which can be specified by User during LockCollateral
    /// Subsequent LockCollateral will override this field. Can be used by User to store info valuable
    /// for him e.g. leverage.
    pub memo: [u8; COLLATERAL_MEMO_LEN],
}

impl DepositedCollateral {
    pub fn new(deposit_reserve: Pubkey) -> Self {
        Self {
            deposit_reserve,
            deposited_amount: 0,
            entry_market_value: Decimal::ZERO.into_bits().unwrap(),
            market_value: Decimal::ZERO.into_bits().unwrap(),
            memo: Zeroable::zeroed(),
        }
    }

    pub fn market_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.market_value).map_err(From::from)
    }

    pub fn set_market_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.market_value = value.into_bits()?;
        Ok(())
    }

    pub fn entry_market_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.entry_market_value).map_err(From::from)
    }

    pub fn set_entry_market_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.entry_market_value = value.into_bits()?;
        Ok(())
    }

    /// Increase deposited collateral
    /// `collateral_amount` - amount of LP tokens to be locked as collateral
    /// `collateral_market_price` - market price of LP tokens in quote currency at the time of deposit
    pub fn deposit(
        &mut self,
        collateral_amount: u64,
        collateral_market_price: Decimal,
        decimals: u8,
    ) -> LendyResult<()> {
        // New deposited amount is just sum
        self.deposited_amount =
            self.deposited_amount
                .checked_add(collateral_amount)
                .ok_or(MathError(format!(
                    "deposit(): checked_add {} + {}",
                    self.deposited_amount, collateral_amount
                )))?;

        let new_deposit_value = Decimal::from_lamports(collateral_amount, decimals)?
            .checked_mul(collateral_market_price)?;

        // When more collateral added we calculate new entry_market_value as a sum of values.
        self.set_entry_market_value(self.entry_market_value()?.checked_add(new_deposit_value)?)?;

        Ok(())
    }

    /// Decrease deposited collateral
    pub fn withdraw(&mut self, collateral_amount: u64) -> LendyResult<()> {
        let new_deposited_amount =
            self.deposited_amount
                .checked_sub(collateral_amount)
                .ok_or(MathError(format!(
                    "deposit(): checked_sub {} + {}",
                    self.deposited_amount, collateral_amount
                )))?;

        // When collateral is unlocked we proportionally decrease entry_market_value.
        // E.g. self.deposited_amount = 100, collateral_amount = 40,
        // this gives remaining_factor = 1 - 40/100 = 0.6
        let remaining_factor = Decimal::ONE.checked_sub(
            Decimal::from_i128_with_scale(collateral_amount as i128, 0)?.checked_div(
                Decimal::from_i128_with_scale(self.deposited_amount as i128, 0)?,
            )?,
        )?;

        self.deposited_amount = new_deposited_amount;
        self.set_entry_market_value(self.entry_market_value()?.checked_mul(remaining_factor)?)?;

        Ok(())
    }

    /// PnL of the `long` position if that collateral deposit is viewed as trading position.
    pub fn pnl(&self) -> LendyResult<Decimal> {
        self.market_value()?
            .checked_sub(self.entry_market_value()?)
            .map_err(From::from)
    }
}

pub const BORROW_MEMO_LEN: usize = 32;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct BorrowedLiquidity {
    /// Reserve liquidity is borrowed from
    pub borrow_reserve: Pubkey,
    /// Borrow rate used for calculating interest
    pub cumulative_borrow_rate: i128,
    /// Amount of liquidity borrowed plus interest. When this amount is 0 then such record can be reused for new borrow.
    pub borrowed_amount: i128,
    /// Liquidity market value in quote currency
    pub market_value: i128,
    /// Liquidity market value in quote currency at the moment its deposit
    pub entry_market_value: i128,
    /// Arbitrary data User can supply along with Borrow
    pub memo: [u8; BORROW_MEMO_LEN],
}

impl BorrowedLiquidity {
    /// Functions to read and set Decimal amounts which are i128 inside
    pub fn borrowed_amount(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.borrowed_amount).map_err(From::from)
    }

    pub fn set_borrowed_amount(&mut self, value: Decimal) -> LendyResult<()> {
        self.borrowed_amount = value.into_bits()?;
        Ok(())
    }

    pub fn cumulative_borrow_rate(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.cumulative_borrow_rate).map_err(From::from)
    }

    pub fn set_cumulative_borrow_rate(&mut self, value: Decimal) -> LendyResult<()> {
        self.cumulative_borrow_rate = value.into_bits()?;
        Ok(())
    }

    pub fn market_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.market_value).map_err(From::from)
    }

    pub fn set_market_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.market_value = value.into_bits()?;
        Ok(())
    }

    pub fn entry_market_value(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.entry_market_value).map_err(From::from)
    }

    pub fn set_entry_market_value(&mut self, value: Decimal) -> LendyResult<()> {
        self.entry_market_value = value.into_bits()?;
        Ok(())
    }

    /// Create new position liquidity
    pub fn new(borrow_reserve: Pubkey, cumulative_borrow_rate: Decimal) -> Self {
        Self {
            borrow_reserve,
            cumulative_borrow_rate: cumulative_borrow_rate.into_bits().unwrap(),
            borrowed_amount: Decimal::ZERO.into_bits().unwrap(),
            market_value: Decimal::ZERO.into_bits().unwrap(),
            entry_market_value: Decimal::ZERO.into_bits().unwrap(),
            memo: Zeroable::zeroed(),
        }
    }

    /// Decrease borrowed liquidity
    pub fn repay(&mut self, settle_amount: Decimal) -> LendyResult<()> {
        let new_borrowed_amount = self
            .borrowed_amount()?
            .checked_sub(settle_amount)?
            .max(Decimal::ZERO);

        // On decrease of borrowed amount - proportionally decrease entry_market_value
        let remaining_factor =
            Decimal::ONE.checked_sub(settle_amount.checked_div(self.borrowed_amount()?)?)?;

        self.set_borrowed_amount(new_borrowed_amount)?;
        self.set_entry_market_value(self.entry_market_value()?.checked_mul(remaining_factor)?)?;

        Ok(())
    }

    /// Increase borrowed liquidity
    /// `borrow_amount` - amount to be borrowed in WAD representation.
    pub fn borrow(&mut self, borrow_amount: Decimal, market_price: Decimal) -> LendyResult<()> {
        self.set_borrowed_amount(self.borrowed_amount()?.checked_add(borrow_amount)?)?;

        let borrowed_value = borrow_amount.checked_mul(market_price)?;

        // When borrowing more - add borrowed value to the entry_market_value
        self.set_entry_market_value(self.entry_market_value()?.checked_add(borrowed_value)?)?;

        Ok(())
    }

    /// Accrue interest
    /// `cumulative_borrow_rate_wads` - comes from Reserve(s) position borrowed from.
    pub fn accrue_interest(&mut self, cumulative_borrow_rate_wads: Decimal) -> LendyResult<()> {
        match cumulative_borrow_rate_wads.cmp(&self.cumulative_borrow_rate()?) {
            Ordering::Less => {
                msg!("Interest rate cannot be negative");
                return Err(SuperLendyError::Internal(String::from(
                    "negative interest rate",
                )));
            }
            Ordering::Equal => {}
            Ordering::Greater => {
                let compounded_interest_rate =
                    cumulative_borrow_rate_wads.checked_div(self.cumulative_borrow_rate()?)?;

                self.set_borrowed_amount(
                    self.borrowed_amount()?
                        .checked_mul(compounded_interest_rate)?,
                )?;
                self.set_cumulative_borrow_rate(cumulative_borrow_rate_wads)?;
            }
        }

        Ok(())
    }

    /// Decreases Reserve's borrowed amount by a specified `amount`.
    /// `amount` - is in lamports form
    pub fn write_off_bad_debt(&mut self, amount: u64, decimals: u8) -> LendyResult<()> {
        if amount == MAX_AMOUNT {
            self.set_borrowed_amount(Decimal::ZERO)?;
        } else {
            self.set_borrowed_amount(
                self.borrowed_amount()?
                    .checked_sub(Decimal::from_lamports(amount, decimals)?)?
                    .max(Decimal::ZERO),
            )?;
        }

        Ok(())
    }

    /// PnL of the `short` position if that borrowing is viewed as trading position.
    /// When entry market value is more than current market value - PnL is positive. Because
    /// it is `short` position.
    pub fn pnl(&self) -> LendyResult<Decimal> {
        self.entry_market_value()?
            .checked_sub(self.market_value()?)
            .map_err(From::from)
    }
}

/// Reward record tracks interaction of user's position with ALL Reward Rules with given
/// reward_mint. Rewards from all such rules accumulated on one Rewards record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Reward {
    /// Reward token counted in this record
    /// Default value marks record as unused
    pub reward_mint: Pubkey,

    /// Slot when `accrued_amount` was calculated
    pub accrued_slot: Slot,
    _padding: u64,

    /// This is total amount or Rewards token accrued at the time of `accrued_slot`
    /// Let it be Decimal with WAD representation. This will allow to accrue very small (e.g. 10^-9 of
    /// lamport) amounts.
    pub accrued_amount: i128,
}

impl Reward {
    pub fn is_vacant(&self) -> bool {
        self.reward_mint == Pubkey::default()
    }

    pub fn accrued_amount(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.accrued_amount).map_err(From::from)
    }

    pub fn set_accrued_amount(&mut self, value: Decimal) -> LendyResult<()> {
        self.accrued_amount = value.into_bits()?;
        Ok(())
    }

    /// Calculates increase of reward amount.
    /// This function does NOT update accrued_slot leaving open possibility to accrue more Rules to
    /// the same reward record. After all rules are processed reward record should be updated with
    /// proper accrued_slot.
    ///
    /// Calculated amount is written right in to self but also this time accrued amount is returned.
    ///
    /// `rule` - particular reward rule to apply
    /// `current_slot` - current Solana slot
    /// `reward_base` - this is either amount deposited to a particular Reserve or amount borrowed. WAD
    pub fn accrue(
        &mut self,
        rule: &RewardRule,
        current_slot: Slot,
        reward_base: Decimal,
    ) -> LendyResult<Decimal> {
        if rule.reward_mint != self.reward_mint {
            msg!(
                "rule.reward_mint {}   self.reward_mint {}",
                rule.reward_mint,
                self.reward_mint
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if current_slot < self.accrued_slot {
            msg!(
                "can't accrue rewards as current_slot {} less then accrued_slot {} ",
                current_slot,
                self.accrued_slot
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let slots_since_last_accrual = current_slot - self.accrued_slot;

        let rewards_increase =
            reward_base
                .checked_mul(rule.rate()?)?
                .checked_mul(Decimal::from_i128_with_scale(
                    slots_since_last_accrual as i128,
                    0,
                )?)?;

        // `reward_base` and `rewards_decimal` are in WAD (i.e. human readable decimal) in token
        // which Reserve operates on.
        // For example let reserve's liquidity token be wSOL with decimals = 9. And reward token is
        // USDC wuth decimals = 6.
        // Reward rule says "accrue 0.001 reward tokens (USDC) on each deposited token (SOL) per each slot".
        // When user holds 1 SOL deposit for 1000 slots it will result in 1 USDC of reward.
        // Rule and math here operates without knowledge about liquidity and rewards mint decimals because
        // we use WAD decimals in calculation and to store the result.
        // When user will be claiming accrued rewards its will be quite important to convert WAD amount in
        // to "lamports" with proper decimals.

        if rewards_increase != Decimal::ZERO {
            let new_amount = self.accrued_amount()?.checked_add(rewards_increase)?;

            self.set_accrued_amount(new_amount)?;

            msg!(
                "rewards for rule {}: rewards_increase {} for slots {}. balance {}",
                String::from_utf8_lossy(&rule.name),
                rewards_increase,
                slots_since_last_accrual,
                new_amount
            );
        }

        Ok(rewards_increase)
    }

    /// Claim ALL previously accrued rewards
    /// `decimals` - mint decimals of the reward token
    /// Returns 'lamports" amount to be transferred to the User.
    pub fn claim(&mut self, decimals: u8) -> LendyResult<u64> {
        let decimal_amount_to_claim = self.accrued_amount()?;

        self.set_accrued_amount(Decimal::ZERO)?;
        self.reward_mint = Pubkey::default(); // Mark record as unused
        self.accrued_slot = 0;

        Ok(decimal_amount_to_claim.to_lamports_floor(decimals)?)
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Rewards {
    pub rewards: [Reward; MAX_REWARDS],
}

impl Rewards {
    /// Accrues rewards for one particular `deposit` or `borrow` record.
    /// `reason` - encodes rewarded situation. REWARD_FOR_BORROW or REWARD_FOR_LIQUIDITY
    /// `amount` - locked collateral or borrowed amount to use as base for rewards accrual. WAD.
    /// `reward_rules` - rules from deserialized Reserve account deposit made to
    /// `current_slot` - current Solana slot
    ///
    /// Returns set of reward record indexes, touched by this call
    ///
    /// This function:
    /// 1. Search for new rules which can be applied to the Position and init new `reward` records
    ///    for them
    /// 2. Accrues reward tokens for existing `reward` records
    pub fn accrue_rewards(
        &mut self,
        reason: u8, // reward reason: REWARD_FOR_BORROW or REWARD_FOR_LIQUIDITY
        amount: Decimal,
        reward_rules: &RewardRules,
        current_slot: Slot,
    ) -> LendyResult<HashSet<usize>> {
        let mut touched_reward_records = HashSet::new();

        for rule in reward_rules.rules {
            if rule.reason != reason {
                continue;
            }

            match self.find_reward(&rule.reward_mint) {
                None => {
                    // Try to allocate new `rewards` record. Because we just found Reward Rule and
                    // have no previous accrual with timestamp - we can't calculate reward this time.
                    match self.find_unused_reward() {
                        None => {
                            msg!(
                                "warning: no free space for rewards for rule {} reward_mint {}",
                                String::from_utf8_lossy(&rule.name),
                                rule.reward_mint
                            );
                            // do not return error. because rewards are not so important to fail the RefreshPosition IX.
                        }
                        Some(new_record) => {
                            new_record.reward_mint = rule.reward_mint;
                            new_record.accrued_slot = current_slot;
                            new_record.accrued_amount = 0;
                        }
                    }
                }
                Some((index, reward_record)) => {
                    reward_record.accrue(&rule, current_slot, amount)?;
                    touched_reward_records.insert(index);
                }
            }
        }

        Ok(touched_reward_records)
    }

    pub fn find_reward(&mut self, reward_mint: &Pubkey) -> Option<(usize, &mut Reward)> {
        self.rewards
            .iter_mut()
            .enumerate()
            .find(|(_index, reward)| &reward.reward_mint == reward_mint)
    }

    // Unused record is either:
    // 1. Default initialized with default (zero) pubkey.
    // 2. With non default reward_mint AND zero accrued amount (i.e. fully claimed)
    pub fn find_unused_reward(&mut self) -> Option<&mut Reward> {
        self.rewards.iter_mut().find(|reward| reward.is_vacant())
    }

    pub fn set_accrued_slot(&mut self, current_slot: Slot, indexes: HashSet<usize>) {
        for idx in indexes {
            self.rewards[idx].accrued_slot = current_slot;
        }
    }
}

pub fn remaining_borrow_value(
    allowed_borrow_value: Decimal,
    borrowed_value: Decimal,
) -> MathResult<Decimal> {
    Ok(allowed_borrow_value
        .checked_sub(borrowed_value)?
        .max(Decimal::ZERO))
}

pub fn max_withdraw_value(
    allowed_borrow_value: Decimal,
    borrowed_value: Decimal,
    withdraw_collateral_ltv: Decimal,
) -> MathResult<Decimal> {
    if allowed_borrow_value <= borrowed_value {
        return Ok(Decimal::ZERO);
    }
    allowed_borrow_value
        .checked_sub(borrowed_value)?
        .checked_div(withdraw_collateral_ltv)
}

pub fn ltv(borrowed_value: Decimal, deposited_value: Decimal) -> MathResult<Decimal> {
    borrowed_value.checked_div(deposited_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::reserve::{REWARD_FOR_BORROW, REWARD_FOR_LIQUIDITY};

    fn test_rules() -> RewardRules {
        let mut rules = RewardRules {
            rules: Zeroable::zeroed(),
        };

        let reward_mint1 = Pubkey::new_unique();
        let reward_mint2 = Pubkey::new_unique();

        // Rate is 0.5
        let mut rule = rules.rules[0];
        rule.reward_mint = reward_mint1;
        rule.name = *b"Borrow1";
        rule.reason = REWARD_FOR_BORROW;
        rule.start_slot = 0;
        rule.set_rate(Decimal::from_i128_with_scale(5, 1).unwrap())
            .unwrap();
        rules.rules[0] = rule;

        // Rate is 0.01
        let mut rule = rules.rules[1];
        rule.reward_mint = reward_mint1;
        rule.name = *b"Liquid1";
        rule.reason = REWARD_FOR_LIQUIDITY;
        rule.start_slot = 0;
        rule.set_rate(Decimal::from_i128_with_scale(1, 2).unwrap())
            .unwrap();
        rules.rules[1] = rule;

        // Collateral lock rewarded by two different reward tokens. Rate is 0.02
        let mut rule = rules.rules[3];
        rule.reward_mint = reward_mint2;
        rule.name = *b"Liquid2";
        rule.reason = REWARD_FOR_LIQUIDITY;
        rule.start_slot = 0;
        rule.set_rate(Decimal::from_i128_with_scale(2, 2).unwrap())
            .unwrap();
        rules.rules[3] = rule;

        rules
    }

    #[test]
    fn accrue_rewards() {
        let rules = test_rules();

        let mut rewards = Rewards {
            rewards: Zeroable::zeroed(),
        };

        // ======================= Emulate first call of RefreshPosition ==========================
        // First accruals calls allocates new reward record
        rewards
            .accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(), // 1 SOL supplied
                &rules,
                100,
            )
            .unwrap();
        rewards
            .accrue_rewards(
                REWARD_FOR_BORROW,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(), // 1 SOL borrowed
                &rules,
                100,
            )
            .unwrap();

        assert_eq!(rewards.rewards[0].accrued_amount().unwrap(), Decimal::ZERO);
        assert_eq!(rewards.rewards[1].accrued_amount().unwrap(), Decimal::ZERO);

        assert!(!rewards.rewards[0].is_vacant());
        assert!(!rewards.rewards[1].is_vacant());

        let touched_records = rewards
            .accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(), // Another 100 slots of 1 SOL liquidity provision
                &rules,
                200,
            )
            .unwrap();

        // This is accruals for reward_mint1
        // 1_000_000_000 deposited and hold for 100 slots. Contract accrues 0.01 reward token per deposited token per slot.
        // Thus 1_000_000_000 * 100 * 0.01 = 1_000_000_000 i.e. it accrues deposited amount every 100 slots.
        // 1 SOL of deposit produces 1 SOL of rewards every 100 slots.
        assert_eq!(
            rewards.rewards[0].accrued_amount().unwrap(),
            Decimal::from_lamports(1_000_000_000, 9).unwrap()
        );

        // This is accruals for reward_mint2
        assert_eq!(
            rewards.rewards[1].accrued_amount().unwrap(),
            Decimal::from_lamports(2_000_000_000, 9).unwrap()
        );

        // Let's see what will happen if Reward token has decimals = 6 (e.g. USDC)
        // 1 SOL of deposit produces 1 USDC of rewards every 100 slots.
        assert_eq!(
            rewards.rewards[0].accrued_amount().unwrap(),
            Decimal::from_lamports(1_000_000, 6).unwrap()
        );
        assert_eq!(
            rewards.rewards[1].accrued_amount().unwrap(),
            Decimal::from_lamports(2_000_000, 6).unwrap()
        );
        // So disregarding of reward mint decimals our rule gives 1 reward token per one deposited token each 100 slots.

        // Increase borrowed amount and go another 100 slots.
        rewards
            .accrue_rewards(
                REWARD_FOR_BORROW,
                Decimal::from_lamports(2_000_000_000, 9).unwrap(),
                &rules,
                200,
            )
            .unwrap();

        // Reward for borrowing uses the same reward token as one of deposit rules. Thus rewards comes
        // to record with 0 index.
        // Borrow rewards are 2_000_000_000 * 0.5 * 100 = 100_000_000_000
        // Plus 1_000_000_000 already being here from liquidity provision.
        assert_eq!(
            rewards.rewards[0].accrued_amount().unwrap(),
            Decimal::from_lamports(101_000_000_000, 9).unwrap()
        );
        assert_eq!(
            rewards.rewards[1].accrued_amount().unwrap(),
            Decimal::from_lamports(2_000_000, 6).unwrap()
        ); // unchanged

        // We've processed all so update accrued slot.
        rewards.set_accrued_slot(200, touched_records);
        assert_eq!(rewards.rewards[0].accrued_slot, 200);
        assert_eq!(rewards.rewards[1].accrued_slot, 200);

        // ================ Emulate second call of RefreshPosition + 100 slots ====================
        println!("================ Emulate second call of RefreshPosition + 100 slots");

        let mut touched_records = rewards
            .accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(),
                &rules,
                300,
            )
            .unwrap();
        let r = rewards
            .accrue_rewards(
                REWARD_FOR_BORROW,
                Decimal::from_lamports(2_000_000_000, 9).unwrap(),
                &rules,
                300,
            )
            .unwrap();
        touched_records.extend(r);

        assert_eq!(touched_records.len(), 2);
        // Basically twice more as another 100 slots passed
        assert_eq!(
            rewards.rewards[0].accrued_amount().unwrap(),
            Decimal::from_lamports(101_000_000_000 * 2, 9).unwrap()
        );
        assert_eq!(
            rewards.rewards[1].accrued_amount().unwrap(),
            Decimal::from_lamports(2_000_000_000 * 2, 9).unwrap()
        );
    }

    #[test]
    fn reuse_reward_records() {
        let rules = test_rules();

        let mut rewards = Rewards {
            rewards: Zeroable::zeroed(),
        };

        rewards
            .accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(),
                &rules,
                100,
            )
            .unwrap();
        rewards
            .accrue_rewards(
                REWARD_FOR_BORROW,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(),
                &rules,
                100,
            )
            .unwrap();
        let mut touched_records = rewards
            .accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(),
                &rules,
                200,
            )
            .unwrap();
        let r = rewards
            .accrue_rewards(
                REWARD_FOR_BORROW,
                Decimal::from_lamports(2_000_000_000, 9).unwrap(),
                &rules,
                200,
            )
            .unwrap();
        touched_records.extend(r);

        assert_eq!(touched_records.len(), 2); // two records for different reward mints
        assert_eq!(
            rewards.rewards[0].accrued_amount().unwrap(),
            Decimal::from_lamports(101_000_000_000, 9).unwrap()
        );
        assert_eq!(
            rewards.rewards[1].accrued_amount().unwrap(),
            Decimal::from_lamports(2_000_000_000, 9).unwrap()
        );
        rewards.set_accrued_slot(200, touched_records);

        // Now assume that Reward1 was claimed and borrow was repaid.
        rewards.rewards[0].claim(9).unwrap();

        assert!(rewards.rewards[0].is_vacant());

        // After 100 slots from prev accrual we call it again. Locked liquidity still 1 SOL
        rewards
            .accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(1_000_000_000, 9).unwrap(),
                &rules,
                300,
            )
            .unwrap();

        // reward[0] should be reused for same reward mint. Because record was cleared during claim() first
        // accrue_rewards() just allocates new record but doesn't accrue anything,
        // RefreshPosition MUST be called right after ClaimRewards to allocate `reward` record again.
        assert_eq!(rewards.rewards[0].accrued_amount().unwrap(), Decimal::ZERO);
        assert!(!rewards.rewards[0].is_vacant());
    }

    // Tests that entry_market_value correctly increased and decreased during deposit/withdraw
    // for DepositedCollateral records.
    #[test]
    fn collateral_entry_market_value() {
        let mut collateral = DepositedCollateral {
            deposit_reserve: Default::default(),
            entry_market_value: Decimal::ZERO.into_bits().unwrap(),
            market_value: Decimal::ZERO.into_bits().unwrap(),
            deposited_amount: 0,
            memo: Zeroable::zeroed(),
        };

        // Assume that LP token have decimals = 9. Deposit 100 tokens each worth 10$.
        let lp_amount_lamports = 100_000_000_000;
        collateral
            .deposit(
                lp_amount_lamports,
                Decimal::from_i128_with_scale(10, 0).unwrap(),
                9,
            )
            .unwrap();

        assert_eq!(collateral.deposited_amount, lp_amount_lamports);
        assert_eq!(
            collateral.entry_market_value().unwrap(),
            Decimal::from_i128_with_scale(lp_amount_lamports as i128 * 10, 9).unwrap()
        );

        // Now assume that someone called RefreshPosition and thus set current market_value which
        // by 20 bigger than entry_market_value
        collateral
            .set_market_value(Decimal::from_lamports(lp_amount_lamports * 10 + 20, 9).unwrap())
            .unwrap();

        assert_eq!(
            collateral.pnl().unwrap(),
            Decimal::from_i128_with_scale(20, 9).unwrap()
        ); // PnL is +20 lamports

        // Now assume that someone called RefreshPosition and thus set current market_value which
        // by 20 less than entry_market_value
        collateral
            .set_market_value(Decimal::from_lamports(lp_amount_lamports * 10 - 20, 9).unwrap())
            .unwrap();
        assert_eq!(
            collateral.pnl().unwrap(),
            Decimal::from_i128_with_scale(-20, 9).unwrap()
        ); // PnL is -20 lamports

        // Now deposit 10 LPs but with 15$ each. This gives 150$ of value.
        collateral
            .deposit(
                10_000_000_000,
                Decimal::from_i128_with_scale(15, 0).unwrap(),
                9,
            )
            .unwrap();
        assert_eq!(
            collateral.deposited_amount,
            lp_amount_lamports + 10_000_000_000
        );

        // Deposited values simply added to entry_market_value
        assert_eq!(
            collateral.entry_market_value().unwrap(),
            Decimal::from_i128_with_scale(1_150_000_000_000, 9).unwrap()
        );

        // Now withdraw 10% of deposited collateral
        collateral.withdraw(11_000_000_000).unwrap();

        // entry_market_value must be reduced by 10% also
        assert_eq!(
            collateral.entry_market_value().unwrap(),
            Decimal::from_i128_with_scale(1_150_000_000_000 * 90 / 100, 9).unwrap()
        );
    }

    #[test]
    fn liquidity_entry_market_value() {
        let mut liquidity = BorrowedLiquidity {
            borrow_reserve: Default::default(),
            cumulative_borrow_rate: Decimal::ZERO.into_bits().unwrap(),
            entry_market_value: Decimal::ZERO.into_bits().unwrap(),
            market_value: Decimal::ZERO.into_bits().unwrap(),
            borrowed_amount: Decimal::ZERO.into_bits().unwrap(),
            memo: Zeroable::zeroed(),
        };

        liquidity
            .borrow(
                Decimal::from_lamports(100, 9).unwrap(),
                Decimal::from_i128_with_scale(10, 0).unwrap(),
            )
            .unwrap();

        assert_eq!(
            liquidity.borrowed_amount().unwrap(),
            Decimal::from_lamports(100, 9).unwrap()
        );
        assert_eq!(
            liquidity.entry_market_value().unwrap(),
            Decimal::from_i128_with_scale(100 * 10, 9).unwrap()
        );

        // Now assume that someone called RefreshPosition and thus set current market_value which
        // by 20 bigger than entry_market_value.
        liquidity
            .set_market_value(Decimal::from_i128_with_scale(100 * 10 + 20, 9).unwrap())
            .unwrap();

        assert_eq!(
            liquidity.pnl().unwrap(),
            Decimal::from_i128_with_scale(-20, 9).unwrap()
        ); // PnL is -20 because we are `short`

        // Now assume that someone called RefreshPosition and thus set current market_value which
        // by 20 less than entry_market_value
        liquidity
            .set_market_value(Decimal::from_i128_with_scale(100 * 10 - 20, 9).unwrap())
            .unwrap();
        assert_eq!(
            liquidity.pnl().unwrap(),
            Decimal::from_lamports(20, 9).unwrap()
        ); // PnL is +20

        // Now borrow 10 tokens more. Each worth 15$
        liquidity
            .borrow(
                Decimal::from_lamports(10, 9).unwrap(),
                Decimal::from_i128_with_scale(15, 0).unwrap(),
            )
            .unwrap();
        assert_eq!(
            liquidity.borrowed_amount().unwrap(),
            Decimal::from_lamports(110, 9).unwrap()
        );

        assert_eq!(
            liquidity.entry_market_value().unwrap(),
            Decimal::from_i128_with_scale(1150, 9).unwrap()
        );

        // Now repay 10% of borrowed amount
        liquidity
            .repay(Decimal::from_lamports(11, 9).unwrap())
            .unwrap();

        // entry_market_value must be reduced by 10% also
        assert_eq!(
            liquidity.entry_market_value().unwrap(),
            Decimal::from_i128_with_scale(1150 * 90 / 100, 9).unwrap()
        );
    }

    #[test]
    fn unlock_collateral() {
        let mut collateral = DepositedCollateral {
            deposit_reserve: Default::default(),
            entry_market_value: Decimal::ZERO.into_bits().unwrap(),
            market_value: Decimal::ZERO.into_bits().unwrap(),
            deposited_amount: 0,
            memo: Zeroable::zeroed(),
        };

        // 125 tokens deposited each worth 10$ - value 1250
        collateral
            .deposit(125, Decimal::from_i128_with_scale(10, 0).unwrap(), 0)
            .unwrap();

        let mut liquidity = BorrowedLiquidity {
            borrow_reserve: Default::default(),
            cumulative_borrow_rate: Decimal::ZERO.into_bits().unwrap(),
            entry_market_value: Decimal::ZERO.into_bits().unwrap(),
            market_value: Decimal::ZERO.into_bits().unwrap(),
            borrowed_amount: Decimal::ZERO.into_bits().unwrap(),
            memo: Zeroable::zeroed(),
        };

        // borrowed value 100
        liquidity
            .borrow(
                Decimal::from_lamports(10, 9).unwrap(),
                Decimal::from_i128_with_scale(10, 9).unwrap(),
            )
            .unwrap();

        let mut position = Position {
            discriminator: Zeroable::zeroed(),
            version: 0,
            position_type: 0,
            _flags: Zeroable::zeroed(),
            last_update: Zeroable::zeroed(),
            pool: Default::default(),
            owner: Default::default(),
            collateral: Zeroable::zeroed(),
            borrows: Zeroable::zeroed(),
            rewards: Zeroable::zeroed(),
            deposited_value: Decimal::from_lamports(1250, 9)
                .unwrap()
                .into_bits()
                .unwrap(),
            borrowed_value: Decimal::from_i128_with_scale(100, 9)
                .unwrap()
                .into_bits()
                .unwrap(),
            allowed_borrow_value: Decimal::from_i128_with_scale(1000, 9)
                .unwrap()
                .into_bits()
                .unwrap(), // max_borrow_ltv was 80%
            partly_unhealthy_borrow_value: Decimal::from_i128_with_scale(850, 9)
                .unwrap()
                .into_bits()
                .unwrap(),
            fully_unhealthy_borrow_value: Decimal::from_i128_with_scale(900, 9)
                .unwrap()
                .into_bits()
                .unwrap(),
            _padding: Zeroable::zeroed(),
        };

        position.collateral[0] = collateral;
        position.borrows[0] = liquidity;

        // Withdraw from the reserve with max_borrow_ltv = 80%.
        // After the withdraw user position must have LTV = 80%. Contract will allow to withdraw all
        // collateral up to LTV = 80%.
        let max_withdraw = position
            .max_withdraw_value(Decimal::from_basis_points(8000).unwrap())
            .unwrap();

        // Withdrawing 1125 will bring position's LTV to 80%
        assert_eq!(max_withdraw, Decimal::from_lamports(1125, 9).unwrap());
    }
}
