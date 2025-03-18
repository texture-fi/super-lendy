use std::cmp::{max, Ordering};
use std::fmt::{Display, Error, Formatter};

use bitflags::bitflags;
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use curvy::state::curve::Curve;
use curvy_utils::calc_y_with_params;
use solana_program::account_info::AccountInfo;
use solana_program::clock::{Clock, UnixTimestamp};
use solana_program::program_pack::Pack;
use solana_program::{clock::Slot, msg, pubkey::Pubkey};
use spl_token_2022::extension::StateWithExtensions;
use texture_common::account::{PodAccount, PodAccountError};
use texture_common::math::{
    CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Decimal, MathError, MathResult,
};

use crate::error::SuperLendyError;
use crate::state::last_update::LastUpdate;
use crate::state::position::{BorrowedLiquidity, DepositedCollateral, Position};
use crate::state::texture_cfg::ReserveTimelock;
use crate::state::{INITIAL_COLLATERAL_RATE, RESERVE_DISCRIMINATOR, SCALE, SLOTS_PER_YEAR};
use crate::{LendyResult, MAX_AMOUNT};

static_assertions::const_assert_eq!(Reserve::SIZE, std::mem::size_of::<Reserve>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<Reserve>() % 16);
static_assertions::const_assert_eq!(0, std::mem::size_of::<ReserveLiquidity>() % 16);
static_assertions::const_assert_eq!(0, std::mem::size_of::<ReserveCollateral>() % 16);
static_assertions::const_assert_eq!(0, std::mem::size_of::<ReserveConfig>() % 16);
static_assertions::const_assert_eq!(0, std::mem::size_of::<ReserveFeesConfig>() % 16);
static_assertions::const_assert_eq!(0, std::mem::size_of::<RewardRules>() % 16);

/// Important note about amounts representation in this contact.
/// When some fn parameter or variable has _amount suffix and u64 type - this "lamports" representation.
/// This is the same value as could be found in spl_token::Account.amount field. We call this "lamports"
/// but of course this could be smallest (in terms of SPL Token) fractions of other tokens (not necessarily wSOL).
///
/// Second amount representation is fixed point decimal with 18 scale (digits past delimiter). In structs
/// such values are i128. In the code these are Decimal type.
/// When i128 is deserialized as Decimal it will look as human-readable number with 18 digits after dot.
/// For example 12.123456789123456789 - means 12 full tokens plus some fractional part :-) - just
/// to be clear.
/// Max practical value of Decimal with 18 scale is 79228162514.264337593543950335.
/// Thus when storing token amounts for tokens with decimals=9 (e.g. wSOL) it is possible to
/// represent ~79B amounts with precision of 10^-9 lamport.
/// Decimal type have associated functions to construct values out of lamport values and vice versa.

/// RESERVE TYPES. Can only be set once - during reserve creation.

/// Fully functional Reserve
pub const RESERVE_TYPE_NORMAL: u8 = 0;

/// Marks special Reserve used to hold `protected` collateral. Such reserve is not used for
/// borrowing i.e. deposited funds seat steady in the Reserve and not given to borrowers.
/// This also means that deposited funds doesn't produce any gains.
/// Reserve of that kind is used by Borrowers who don't like any lending risks but just want
/// to borrow something against that collateral.
/// This field can only be set once - during reserve creation. Thus, Borrowers deposited there
/// can be sure that their collateral is safe.
pub const RESERVE_TYPE_PROTECTED_COLLATERAL: u8 = 1;

/// Marks Reserve that can NOT be used as collateral.
/// This means that one could:
/// 1. Deposit liquidity and get LP tokens
/// 2. Borrow from this Reserve thus making yield for depositors
/// But one can NOT:
/// 1. Lock LP tokens as collateral in this Reserve. Thus deposits in such reserve will not serve as
/// collateral.
pub const RESERVE_TYPE_NOT_A_COLLATERAL: u8 = 2;

// RESERVE MODES. Can be set during `AlterReserve` ix.

/// Fully functional Reserve
pub const RESERVE_MODE_NORMAL: u8 = 0;

/// Switches off borrow. Borrow can be enabled later on. Can be used by Curator under tough market
/// conditions to suspend further borrowings. Liquidity withdrawals by Lenders still possible.
/// This means that one could:
/// 1. Deposit liquidity and get LP tokens.
/// 2. Lock/Unlock LP tokens as collateral in this Reserve.
/// 3. Withdraw from this Reserve.
/// 4. Unlock LP tokens in this Reserve.
/// But one can NOT:
/// 1. Borrow from this Reserve.
pub const RESERVE_MODE_BORROW_DISABLED: u8 = 1;

/// Disables Borrow, WithdrawLiquidity and UnlockCollateral operations. Should be used on emergency
/// market conditions when Curator wants to retain all liquidity as it is and temporary suspend
/// all liquidity decreasing operations.
/// This means that one could:
/// 1. Deposit liquidity and get LP tokens.
/// 2. Lock LP tokens as collateral in this Reserve.
/// But one can NOT:
/// 1. Borrow from this Reserve.
/// 2. Withdraw from this Reserve.
/// 3. Unlock LP tokens in this Reserve.
pub const RESERVE_MODE_RETAIN_LIQUIDITY: u8 = 2;

/// Reserve is a part on the Pool which manages all aspect of one currency (token) i.e. supply,
/// LP tokens, interest, LP exchange rate, oracles and more.
/// Reserves (token in it) can be used as principal currency only or both as principal and collateral.
/// It must be specified during Reserve creation.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Reserve {
    pub discriminator: [u8; 8],
    pub version: u8,

    /// Operation type of this reserve. RESERVE_TYPE_NORMAL, etc. See all constants/types above.
    pub reserve_type: u8,

    /// Operation mode of this reserve. RESERVE_MODE_NORMAL, etc. See all constants/modes above.
    pub mode: u8,
    pub flash_loans_enabled: u8,

    /// Vacant to store mode/status flags
    pub _flags: [u8; 4],

    /// Last slot when supply and rates updated
    pub last_update: LastUpdate,
    /// Pool that reserve belongs to
    pub pool: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Collateral
    pub collateral: ReserveCollateral,
    /// Reserve configuration values
    pub config: ReserveConfig,
    /// Reward rules
    pub reward_rules: RewardRules,
    /// Configuration changes list (it is not a queue - just indexed list of proposed changes)
    pub proposed_configs: ProposedConfigs,

    // For future use
    pub _padding: [u8; 256],
}

impl PodAccount for Reserve {
    const DISCRIMINATOR: &'static [u8] = RESERVE_DISCRIMINATOR;

    type Version = u8;

    const VERSION: Self::Version = 1;

    type InitParams = (ReserveParams, LastUpdate);

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
            reserve_type,
            mode,
            flash_loans_enabled,
            _flags,
            last_update,
            pool,
            liquidity,
            collateral,
            config,
            reward_rules,
            proposed_configs: pending_config,
            _padding,
        } = self;

        *discriminator = *RESERVE_DISCRIMINATOR;
        *version = Self::VERSION;
        *last_update = params.1;
        *reserve_type = params.0.reserve_type;
        *mode = params.0.mode;
        *pool = params.0.pool;
        *liquidity = params.0.liquidity;
        *collateral = params.0.collateral;
        *config = params.0.config;
        *flash_loans_enabled = params.0.flash_loans_enabled;

        *reward_rules = Zeroable::zeroed();
        *pending_config = Zeroable::zeroed();
        *_padding = Zeroable::zeroed();
        *_flags = Zeroable::zeroed();

        Ok(())
    }
}

impl Reserve {
    /// Record deposited liquidity and return amount of LP tokens to mint
    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> LendyResult<u64> {
        let lp_amount = self.lp_exchange_rate()?.liquidity_to_lp(liquidity_amount)?;
        self.liquidity.deposit(liquidity_amount)?;
        self.collateral.mint(lp_amount)?;

        // Check for resulting total liquidity threshold.
        if self
            .liquidity
            .total_liquidity()?
            .to_lamports_round(self.liquidity.mint_decimals)?
            > self.config.max_total_liquidity
        {
            msg!(
                "Deposit results in {} total liquidity which is greater then threshold {}",
                self.liquidity.total_liquidity()?,
                self.config.max_total_liquidity
            );
            return Err(SuperLendyError::ResourceExhausted);
        }

        Ok(lp_amount)
    }

    /// Record redeemed LPs and return amount of liquidity to withdraw
    pub fn withdraw_liquidity(&mut self, lp_amount: u64) -> LendyResult<u64> {
        let collateral_exchange_rate = self.lp_exchange_rate()?;
        let liquidity_amount = collateral_exchange_rate.lp_to_liquidity(lp_amount)?;

        self.collateral.burn(lp_amount)?;
        self.liquidity.withdraw(liquidity_amount)?;

        Ok(liquidity_amount)
    }

    /// Calculate the current borrow rate.
    pub fn current_borrow_rate(&self, curve: &Curve) -> LendyResult<Decimal> {
        Ok(current_borrow_rate(
            &curve.y[..curve.y_count as usize],
            curve.decimals,
            curve.x_step,
            Decimal::from_i128_with_scale(curve.x0 as i128, 0)?,
            self.liquidity.utilization_rate()?,
        )?)
    }

    /// Collateral exchange rate
    pub fn lp_exchange_rate(&self) -> LendyResult<LpExchangeRate> {
        self.collateral.exchange_rate(
            self.liquidity.total_liquidity()?,
            self.liquidity.mint_decimals,
        )
    }

    /// Calculates value of one LP token in reserve's quote currency (e.g. USD)
    pub fn lp_market_price(&self) -> LendyResult<Decimal> {
        // Reserve's knows:
        // 1. market price of its Liquidity token
        // 2. How many LPs corresponds to one Liquidity token

        Ok(lp_market_price(
            self.liquidity.total_liquidity()?,
            Decimal::from_lamports(
                self.collateral.lp_total_supply,
                self.liquidity.mint_decimals,
            )?,
            self.liquidity.market_price()?,
        )?)
    }

    /// Maximum liquidity amount Reserve will allow to withdraw
    pub fn max_withdraw_liquidity_amount(&self) -> LendyResult<Decimal> {
        Ok(max_withdraw_liquidity_amount(
            Decimal::from_lamports(
                self.liquidity.available_amount,
                self.liquidity.mint_decimals,
            )?,
            self.liquidity.borrowed_amount()?,
            Decimal::from_basis_points(self.config.max_withdraw_utilization_bps as u32)?,
        )?)
    }

    /// Maximum LP amount Reserve will serve as an argument of WithdrawLiquidity. This LP amount
    /// corresponds to the max_withdraw_liquidity_amount()
    pub fn max_withdraw_lp_amount(&self) -> LendyResult<u64> {
        Ok(self.lp_exchange_rate()?.liquidity_to_lp(
            self.max_withdraw_liquidity_amount()?
                .to_lamports_floor(self.liquidity.mint_decimals)?,
        )?)
    }

    /// Amount of liquidity can be borrowed from the Reserve
    /// There are two limiting factors:
    /// 1. available liquidity
    /// 2. Reserve's max_borrow_utilization
    pub fn max_borrow_amount(&self) -> LendyResult<Decimal> {
        self.liquidity
            .max_borrow_amount(self.config.max_borrow_utilization_bps)
    }

    /// Value of liquidity can be borrowed from the Reserve
    pub fn max_borrow_value(&self) -> LendyResult<Decimal> {
        Ok(self
            .liquidity
            .max_borrow_amount(self.config.max_borrow_utilization_bps)?
            .checked_mul(self.liquidity.market_price()?)?)
    }

    /// Update borrow rate and accrue interest
    pub fn accrue_interest(
        &mut self,
        current_slot: Slot,
        texture_performance_fee_rate_bps: u16,
        irm: &Curve,
    ) -> LendyResult<()> {
        let slots_elapsed = self.last_update.slots_elapsed(current_slot)?;
        if slots_elapsed > 0 {
            let current_borrow_rate = self.current_borrow_rate(irm)?;
            self.liquidity.compound_interest(
                current_borrow_rate,
                self.config.fees.curator_performance_fee_rate_bps,
                texture_performance_fee_rate_bps,
                slots_elapsed,
            )?;
            self.liquidity.set_borrow_rate(current_borrow_rate).ok();
        }
        Ok(())
    }

    /// Borrow liquidity up to a maximum market value
    /// `amount_to_borrow` - amount in Lamports (or equivalent for other tokens) user wants to borrow
    /// `max_borrow_value` - limit in value (usually $) to borrow i.e. we ask the function "please,
    /// calculate borrow amount to result in borrow value equal to one provided as parameter".
    /// `texture_borrow_fee_rate_bps` - origination borrow fee rate for Texture
    pub fn calculate_borrow(
        &self,
        amount_to_borrow: u64,
        max_borrow_value: Decimal,
        texture_borrow_fee_rate_bps: u16,
    ) -> LendyResult<CalculateBorrowResult> {
        if amount_to_borrow == MAX_AMOUNT {
            let borrow_amount = max_borrow_value
                .checked_div(self.liquidity.market_price()?)?
                .min(Decimal::from_lamports(
                    self.liquidity.available_amount,
                    self.liquidity.mint_decimals,
                )?);

            let (curator_borrow_fee, texture_borrow_fee) = self.config.fees.calculate_borrow_fees(
                borrow_amount,
                self.liquidity.mint_decimals,
                texture_borrow_fee_rate_bps,
                FeeCalculation::Inclusive,
            )?;
            let receive_amount = borrow_amount
                .to_lamports_floor(self.liquidity.mint_decimals)?
                .checked_sub(curator_borrow_fee)
                .ok_or(MathError(format!(
                    "calculate_borrow(): checked_sub curator_borrow_fee {}",
                    curator_borrow_fee
                )))?
                .checked_sub(texture_borrow_fee)
                .ok_or(MathError(format!(
                    "calculate_borrow(): checked_sub texture_borrow_fee {}",
                    texture_borrow_fee
                )))?;

            Ok(CalculateBorrowResult {
                borrow_amount,
                receive_amount,
                curator_borrow_fee,
                texture_borrow_fee,
            })
        } else {
            // This is amount in "lamports" or equivalent smallest fractional part of the token
            let receive_amount = amount_to_borrow;

            let borrow_amount_wad =
                Decimal::from_lamports(receive_amount, self.liquidity.mint_decimals)?;

            let (curator_borrow_fee, texture_borrow_fee) = self.config.fees.calculate_borrow_fees(
                borrow_amount_wad,
                self.liquidity.mint_decimals,
                texture_borrow_fee_rate_bps,
                FeeCalculation::Exclusive,
            )?;

            let borrow_amount_wad = borrow_amount_wad
                .checked_add(Decimal::from_lamports(
                    curator_borrow_fee,
                    self.liquidity.mint_decimals,
                )?)?
                .checked_add(Decimal::from_lamports(
                    texture_borrow_fee,
                    self.liquidity.mint_decimals,
                )?)?;
            let borrow_value = borrow_amount_wad.checked_mul(self.liquidity.market_price()?)?;
            if borrow_value > max_borrow_value {
                msg!("Borrow value cannot exceed maximum borrow value");
                return Err(SuperLendyError::BorrowTooLarge);
            }

            Ok(CalculateBorrowResult {
                borrow_amount: borrow_amount_wad,
                receive_amount,
                curator_borrow_fee,
                texture_borrow_fee,
            })
        }
    }

    /// Repay liquidity up to the borrowed amount
    pub fn calculate_repay(
        &self,
        amount_to_repay: u64,
        borrowed_amount: Decimal,
    ) -> LendyResult<CalculateRepayResult> {
        let settle_amount = if amount_to_repay == MAX_AMOUNT {
            borrowed_amount
        } else {
            Decimal::from_lamports(amount_to_repay, self.liquidity.mint_decimals)?
                .min(borrowed_amount)
        };
        let repay_amount = settle_amount.to_lamports_ceil(self.liquidity.mint_decimals)?;

        Ok(CalculateRepayResult {
            settle_amount,
            repay_amount,
        })
    }

    /// Liquidate some or all of an unhealthy position
    /// There are two liquidation levels:
    /// 1. When reserve.config.partly_unhealthy_ltv >= positions.LTV > reserve.config.fully_unhealthy_ltv - in this case we
    /// allow maximum reserve.config.partial_liquidation_factor_bps of the collateral to be liquidated.
    /// 2. When reserve.config.fully_unhealthy_ltv <= positions.LTV - liquidator can liquidate as much of the position
    /// as he wants (may do full liquidation at once).
    pub fn calculate_liquidation(
        &self,
        amount_to_liquidate: u64,
        position: &Position,
        borrowed_liquidity: &BorrowedLiquidity,
        collateral: &DepositedCollateral,
        principal_mint_decimals: u8,
    ) -> Result<CalculateLiquidationResult, SuperLendyError> {
        let position_ltv = position.ltv()?;
        let partly_unhealthy_ltv =
            Decimal::from_basis_points(self.config.partly_unhealthy_ltv_bps as u32)?;
        let fully_unhealthy_ltv =
            Decimal::from_basis_points(self.config.fully_unhealthy_ltv_bps as u32)?;

        // Check position can be partially liquidated
        if position_ltv < partly_unhealthy_ltv {
            return Err(SuperLendyError::AttemptToLiquidateHealthyPosition(
                position_ltv,
                partly_unhealthy_ltv,
            ));
        }

        let bonus_rate = Decimal::from_basis_points(self.config.liquidation_bonus_bps as u32)?
            .checked_add(Decimal::ONE)?;

        let reserve_max_liquidation_amount = borrowed_liquidity.borrowed_amount()?;

        let liquidation_close_factor = if position_ltv >= fully_unhealthy_ltv {
            Decimal::ONE // Allow to liquidate full borrowed amount at once
        } else {
            Decimal::from_basis_points(self.config.partial_liquidation_factor_bps as u32)?
        };

        let position_max_liquidation_amount =
            position.max_liquidation_amount(borrowed_liquidity, liquidation_close_factor)?;

        // Basically two entities have their own limits regarding liquidation amount:
        // 1. Reserve - can not allow liquidation of more than it is borrowed from it.
        // 2. Position - can not allow liquidation of more than borrowed (full liquidation) or
        //    more than liquidation_factor (partial liquidation).
        // Take smallest from the two.
        let max_liquidation_amount =
            reserve_max_liquidation_amount.min(position_max_liquidation_amount);

        let liquidation_amount = if amount_to_liquidate == MAX_AMOUNT {
            // Allow to liquidate as much principal as Reserve & Position allow
            max_liquidation_amount
        } else {
            let amount_to_liquidate_wad =
                Decimal::from_lamports(amount_to_liquidate, principal_mint_decimals)?;

            if amount_to_liquidate_wad > max_liquidation_amount {
                msg!("Attempt to liquidate exact principal amount {} which is more than max allowed {}", amount_to_liquidate_wad, max_liquidation_amount);
                return Err(SuperLendyError::InvalidAmount);
            }

            amount_to_liquidate_wad
        };

        let liquidation_pct =
            liquidation_amount.checked_div(borrowed_liquidity.borrowed_amount()?)?;
        let liquidation_value = borrowed_liquidity
            .market_value()?
            .checked_mul(liquidation_pct)?
            .checked_mul(bonus_rate)?;

        // calculate settle_amount and withdraw_amount, repay_amount is settle_amount rounded
        let settle_amount;
        let repay_amount;
        let withdraw_amount;

        match liquidation_value.cmp(&collateral.market_value()?) {
            Ordering::Greater => {
                let repay_pct = collateral.market_value()?.checked_div(liquidation_value)?;
                settle_amount = liquidation_amount.checked_mul(repay_pct)?;
                repay_amount = settle_amount.to_lamports_ceil(principal_mint_decimals)?;
                withdraw_amount = collateral.deposited_amount;
            }
            Ordering::Equal => {
                settle_amount = liquidation_amount;
                repay_amount = settle_amount.to_lamports_ceil(principal_mint_decimals)?;
                withdraw_amount = collateral.deposited_amount;
            }
            Ordering::Less => {
                let withdraw_pct = liquidation_value.checked_div(collateral.market_value()?)?;
                settle_amount = liquidation_amount;
                repay_amount = settle_amount.to_lamports_floor(principal_mint_decimals)?;
                withdraw_amount = Decimal::from_lamports(
                    collateral.deposited_amount,
                    self.liquidity.mint_decimals,
                )?
                .checked_mul(withdraw_pct)?
                .to_lamports_floor(self.liquidity.mint_decimals)?;
            }
        }

        Ok(CalculateLiquidationResult {
            settle_amount,
            repay_amount,
            withdraw_amount,
        })
    }

    pub fn is_stale(&self, clock: &Clock) -> LendyResult<bool> {
        self.last_update.is_stale(
            clock.unix_timestamp,
            self.config.price_stale_threshold_sec as u64,
        )
    }

    pub fn mark_stale(&mut self) {
        self.last_update.mark_stale();
    }
}

/// For reserve initialization
#[derive(Debug, Clone, Copy)]
pub struct ReserveParams {
    /// Enables\disables borrowing from that Reserve
    pub reserve_type: u8,
    /// Partly\Fully disables withdrawing from that Reserve
    pub mode: u8,
    /// Enables\disables flash loans for that Reserve.
    pub flash_loans_enabled: u8,
    /// Pool address
    pub pool: Pubkey,
    /// Reserve liquidity which can be borrowed
    pub liquidity: ReserveLiquidity,
    /// Reserve collateral
    pub collateral: ReserveCollateral,
    /// Reserve configuration values
    pub config: ReserveConfig,
}

/// Calculate borrow result
#[derive(Debug)]
pub struct CalculateBorrowResult {
    /// Total amount of borrow including fees
    pub borrow_amount: Decimal,
    /// Borrow amount portion of total amount
    pub receive_amount: u64,
    /// Loan origination fee
    pub curator_borrow_fee: u64,
    /// Host fee portion of origination fee
    pub texture_borrow_fee: u64,
}

/// Calculate repay result
#[derive(Debug)]
pub struct CalculateRepayResult {
    /// Amount of liquidity that is settled from the position.
    pub settle_amount: Decimal,
    /// Amount that will be repaid as u64
    pub repay_amount: u64,
}

/// Calculate liquidation result
#[derive(Debug)]
pub struct CalculateLiquidationResult {
    /// Amount of liquidity that is settled from the position. It includes
    /// the amount of loan that was defaulted if collateral is depleted.
    pub settle_amount: Decimal,
    /// Amount that will be repaid as u64
    pub repay_amount: u64,
    /// Amount of collateral to withdraw in exchange for repay amount
    pub withdraw_amount: u64,
}

/// Reserve liquidity
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ReserveLiquidity {
    /// Reserve liquidity mint address
    pub mint: Pubkey,
    /// Reserve liquidity borrowed. In WAD representation.
    pub borrowed_amount: i128,
    /// Reserve liquidity cumulative borrow rate
    pub cumulative_borrow_rate: i128,
    /// Reserve liquidity market price in quote currency
    pub market_price: i128,
    /// Performance fee amount which can be claimed by Curator. WAD
    pub curator_performance_fee: i128,
    /// Performance fee amount which can be claimed by Texture. WAD
    pub texture_performance_fee: i128,
    /// This is last time calculated Borrow Rate. It is NOT used by the contract for any purposes.
    /// Instead contract calculates fresh value every time RefreshReserve is called. The field is
    /// here to simplify things for off-chain apps interested in that value.
    pub borrow_rate: i128,

    /// Reserve liquidity available. Measured in smallest token unit (e.g. lamports).
    pub available_amount: u64,
    pub _padding: u64,

    /// Reserve liquidity mint decimals
    pub mint_decimals: u8,

    pub _padding1: [u8; 15 + 32 * 2],
}

impl ReserveLiquidity {
    pub fn new(mint: Pubkey, mint_decimals: u8) -> Self {
        Self {
            mint,
            available_amount: 0,
            borrowed_amount: Decimal::ZERO.into_bits().unwrap(),
            curator_performance_fee: Decimal::ZERO.into_bits().unwrap(),
            texture_performance_fee: Decimal::ZERO.into_bits().unwrap(),
            cumulative_borrow_rate: Decimal::ONE.into_bits().unwrap(),
            market_price: Decimal::ZERO.into_bits().unwrap(),
            mint_decimals,
            _padding1: Zeroable::zeroed(),
            _padding: 0,
            borrow_rate: Decimal::ZERO.into_bits().unwrap(),
        }
    }

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

    pub fn market_price(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.market_price).map_err(From::from)
    }

    pub fn set_market_price(&mut self, value: Decimal) -> LendyResult<()> {
        self.market_price = value.into_bits()?;
        Ok(())
    }

    pub fn curator_performance_fee(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.curator_performance_fee).map_err(From::from)
    }

    pub fn set_curator_performance_fee(&mut self, value: Decimal) -> LendyResult<()> {
        self.curator_performance_fee = value.into_bits()?;
        Ok(())
    }

    pub fn texture_performance_fee(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.texture_performance_fee).map_err(From::from)
    }

    pub fn set_texture_performance_fee(&mut self, value: Decimal) -> LendyResult<()> {
        self.texture_performance_fee = value.into_bits()?;
        Ok(())
    }

    pub fn borrow_rate(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.borrow_rate).map_err(From::from)
    }

    pub fn set_borrow_rate(&mut self, value: Decimal) -> LendyResult<()> {
        self.borrow_rate = value.into_bits()?;
        Ok(())
    }

    /// Calculate the total reserve supply including active loans
    pub fn total_liquidity(&self) -> LendyResult<Decimal> {
        Ok(total_liquidity(
            Decimal::from_lamports(self.available_amount, self.mint_decimals)?,
            self.borrowed_amount()?,
        )?)
    }

    /// Add liquidity to available amount
    pub fn deposit(&mut self, liquidity_amount: u64) -> LendyResult<()> {
        self.available_amount =
            self.available_amount
                .checked_add(liquidity_amount)
                .ok_or(MathError(format!(
                    "deposit(): checked_add {} + {}",
                    self.available_amount, liquidity_amount
                )))?;
        Ok(())
    }

    /// Remove liquidity from available amount
    pub fn withdraw(&mut self, liquidity_amount: u64) -> LendyResult<()> {
        if liquidity_amount > self.available_amount {
            msg!("Withdraw amount cannot exceed available amount");
            return Err(SuperLendyError::InvalidAmount);
        }
        self.available_amount =
            self.available_amount
                .checked_sub(liquidity_amount)
                .ok_or(MathError(format!(
                    "withdraw(): checked_sub {} + {}",
                    self.available_amount, liquidity_amount
                )))?;
        Ok(())
    }

    /// Subtract borrow amount from available liquidity and add to borrows
    /// `borrow_amount` - amount to borrow in WAD form - this is for Reserve's internal accounting
    /// `borrowed_lamports` - borrowed amount but in Lamports - exactly the number contract will transfer out
    pub fn borrow(&mut self, borrow_amount: Decimal, borrowed_lamports: u64) -> LendyResult<()> {
        if borrowed_lamports > self.available_amount {
            msg!(
                "Borrow amount {} cannot exceed available amount {}",
                borrowed_lamports,
                self.available_amount
            );
            return Err(SuperLendyError::InvalidAmount);
        }

        self.available_amount = self.available_amount.checked_sub(borrowed_lamports).ok_or(
            SuperLendyError::MathError(MathError(format!(
                "borrow(): checked_sub {} + {}",
                self.available_amount, borrowed_lamports
            ))),
        )?;

        self.set_borrowed_amount(self.borrowed_amount()?.checked_add(borrow_amount)?)?;

        Ok(())
    }

    /// Add repay amount to available liquidity and subtract settle amount from
    /// total borrows
    pub fn repay(&mut self, repay_amount: u64, settle_amount: Decimal) -> LendyResult<()> {
        self.available_amount =
            self.available_amount
                .checked_add(repay_amount)
                .ok_or(SuperLendyError::MathError(MathError(format!(
                    "repay(): checked_add {} + {}",
                    self.available_amount, repay_amount
                ))))?;

        // During last repay in the Reserve it is possible that settle amount is slightly bigger
        // (due to rounding) then reserve's borrowed amount. Thus don't go negative...
        let new_borrowed_amount = self
            .borrowed_amount()?
            .checked_sub(settle_amount)?
            .max(Decimal::ZERO);

        self.set_borrowed_amount(new_borrowed_amount)?;

        Ok(())
    }

    /// Calculate the liquidity utilization rate of the reserve
    pub fn utilization_rate(&self) -> LendyResult<Decimal> {
        Ok(liquidity_utilization_rate(
            self.total_liquidity()?,
            self.borrowed_amount()?,
        )?)
    }

    /// Amount of liquidity can be borrowed from the Reserve
    /// There are two limiting factors:
    /// 1. available liquidity
    /// 2. Reserve's max_borrow_utilization
    pub fn max_borrow_amount(&self, max_borrow_utilization_bps: u16) -> LendyResult<Decimal> {
        max_borrow_amount(
            self.available_amount,
            self.mint_decimals,
            self.borrowed_amount()?,
            max_borrow_utilization_bps,
        )
    }

    pub fn claim_curator_performance_fee(&mut self) -> LendyResult<u64> {
        let claimable_amount = self
            .curator_performance_fee()?
            .to_lamports_floor(self.mint_decimals)?;
        let remaining_amount = self
            .curator_performance_fee()?
            .checked_sub(Decimal::from_lamports(
                claimable_amount,
                self.mint_decimals,
            )?)?
            .max(Decimal::ZERO);
        self.set_curator_performance_fee(remaining_amount)?;

        Ok(claimable_amount)
    }

    pub fn claim_texture_performance_fee(&mut self) -> LendyResult<u64> {
        let claimable_amount = self
            .texture_performance_fee()?
            .to_lamports_floor(self.mint_decimals)?;
        let remaining_amount = self
            .texture_performance_fee()?
            .checked_sub(Decimal::from_lamports(
                claimable_amount,
                self.mint_decimals,
            )?)?
            .max(Decimal::ZERO);
        self.set_texture_performance_fee(remaining_amount)?;

        Ok(claimable_amount)
    }

    /// Compound current borrow rate over elapsed slots
    fn compound_interest(
        &mut self,
        current_borrow_rate: Decimal,
        curator_performance_fee_rate_bps: u16,
        texture_performance_fee_rate_bps: u16,
        slots_elapsed: u64,
    ) -> LendyResult<()> {
        let slot_interest_rate = current_borrow_rate
            .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0)?)?;
        let compounded_interest_rate = Decimal::ONE
            .checked_add(slot_interest_rate)?
            .checked_pow(slots_elapsed)?;

        // All decimals in that contract stored with 18 precision. And cumulative_borrow_rate_wads do so. Above calculated
        // `compounded_interest_rate` may have greater precision. We'll scale it to 18 to store same value in the sate as
        // will be used to calc `new_borrowed_amount_wads`.
        let compounded_interest_rate = Decimal::from_bits(compounded_interest_rate.into_bits()?)?;

        self.set_cumulative_borrow_rate(
            self.cumulative_borrow_rate()?
                .checked_mul(compounded_interest_rate)?,
        )?;

        let new_borrowed_amount_wads = self
            .borrowed_amount()?
            .checked_mul(compounded_interest_rate)?;

        // Performance fee - is a part of the interest. Accrued performance fees must result in slower
        // growth of reserve's borrowed_amount (thus slowing down growth of LP tokens exchange rate) but it should not
        // take any effect on cumulative_borrow_rate. The later is used to get interest from Borrowers.
        // This means that reserve.borrowed_amount will be less (if any performance fee accrued) then
        // the sum of position.borrowed_amount from all positions borrowed from the Reserve.
        // Just to repeat... reserve.borrowed_amount = liquidity_given_to_borrowers + interest.

        // This is absolute increase of the reserve during elapsed slots. It is base for performance fee.
        let interest_for_elapsed_slots =
            new_borrowed_amount_wads.checked_sub(self.borrowed_amount()?)?;

        if interest_for_elapsed_slots != Decimal::ZERO && curator_performance_fee_rate_bps != 0 {
            let fee_for_elapsed_slots = interest_for_elapsed_slots.checked_mul(
                Decimal::from_basis_points(curator_performance_fee_rate_bps as u32)?,
            )?;
            self.set_curator_performance_fee(
                self.curator_performance_fee()?
                    .checked_add(fee_for_elapsed_slots)?,
            )?;
        }

        if interest_for_elapsed_slots != Decimal::ZERO && texture_performance_fee_rate_bps != 0 {
            let fee_for_elapsed_slots = interest_for_elapsed_slots.checked_mul(
                Decimal::from_basis_points(texture_performance_fee_rate_bps as u32)?,
            )?;
            self.set_texture_performance_fee(
                self.texture_performance_fee()?
                    .checked_add(fee_for_elapsed_slots)?,
            )?;
        }

        self.set_borrowed_amount(new_borrowed_amount_wads)?;

        Ok(())
    }

    pub fn write_off_bad_debt(&mut self, amount: u64) -> LendyResult<()> {
        if amount == MAX_AMOUNT {
            self.set_borrowed_amount(Decimal::ZERO)?;
        } else {
            self.set_borrowed_amount(
                self.borrowed_amount()?
                    .checked_sub(Decimal::from_lamports(amount, self.mint_decimals)?)?
                    .max(Decimal::ZERO),
            )?;
        }

        Ok(())
    }
}

/// Reserve collateral
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ReserveCollateral {
    /// Reserve LP tokens supply, used for exchange rate
    pub lp_total_supply: u64,
    pub _padding: u64,
}

impl Default for ReserveCollateral {
    fn default() -> Self {
        Self::new()
    }
}

impl ReserveCollateral {
    /// Create a new reserve collateral
    pub fn new() -> Self {
        Self {
            lp_total_supply: 0,
            _padding: 0,
        }
    }

    /// Add LP to total supply
    pub fn mint(&mut self, collateral_amount: u64) -> LendyResult<()> {
        self.lp_total_supply =
            self.lp_total_supply
                .checked_add(collateral_amount)
                .ok_or(MathError(format!(
                    "mint(): checked_add {} + {}",
                    self.lp_total_supply, collateral_amount
                )))?;
        Ok(())
    }

    /// Remove LP from total supply
    pub fn burn(&mut self, collateral_amount: u64) -> LendyResult<()> {
        self.lp_total_supply =
            self.lp_total_supply
                .checked_sub(collateral_amount)
                .ok_or(MathError(format!(
                    "burn(): checked_sub {} + {}",
                    self.lp_total_supply, collateral_amount
                )))?;
        Ok(())
    }

    /// Return the current LP exchange rate.
    fn exchange_rate(&self, total_liquidity: Decimal, decimals: u8) -> LendyResult<LpExchangeRate> {
        Ok(lp_exchange_rate(
            total_liquidity,
            Decimal::from_lamports(self.lp_total_supply, decimals)?,
        )?)
    }
}

/// Collateral exchange rate
#[derive(Clone, Copy, Debug)]
pub struct LpExchangeRate(pub Decimal);

impl LpExchangeRate {
    /// Convert reserve LP tokens amount to liquidity
    pub fn lp_to_liquidity(&self, lp_amount: u64) -> MathResult<u64> {
        self.decimal_lp_to_liquidity(Decimal::from_i128_with_scale(lp_amount as i128, 0)?)?
            .floor() // This is just a math with amounts in lamport from
    }

    /// Convert reserve LP to liquidity
    pub fn decimal_lp_to_liquidity(&self, lp_amount: Decimal) -> MathResult<Decimal> {
        lp_amount.checked_div(self.0)
    }

    /// Convert reserve liquidity to LP
    pub fn liquidity_to_lp(&self, liquidity_amount: u64) -> MathResult<u64> {
        self.decimal_liquidity_to_lp(Decimal::from_i128_with_scale(liquidity_amount as i128, 0)?)?
            .floor() // This is just a math with amounts in lamport from
    }

    /// Convert reserve liquidity to LP
    pub fn decimal_liquidity_to_lp(&self, liquidity_amount: Decimal) -> MathResult<Decimal> {
        liquidity_amount.checked_mul(self.0)
    }
}

impl From<LpExchangeRate> for Decimal {
    fn from(exchange_rate: LpExchangeRate) -> Self {
        exchange_rate.0
    }
}

/// Reserve configuration. This part of the Reserve can be changed via AlterReserve IX.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
#[repr(C)]
pub struct ReserveConfig {
    /// `Price proxy` price account for
    pub market_price_feed: Pubkey,
    /// Interest rate model (IRM) account
    pub irm: Pubkey,
    /// Bonus a liquidator gets when repaying part of an unhealthy position,
    /// as a basis points - bps (0.01%)
    pub liquidation_bonus_bps: u16,
    /// LTV at which partial liquidation starts (if there is only one collateral deposit). When there
    /// are several various deposits then each will give its own value-weighted part in to total
    /// partly_unhealthy_ltv of the position.
    pub partly_unhealthy_ltv_bps: u16,
    /// LTV as above but full liquidation is allowed as one operation
    pub fully_unhealthy_ltv_bps: u16,
    /// Collateral percentage (in basis points) which can be liquidated in one operation
    pub partial_liquidation_factor_bps: u16,
    /// Maximum total liquidity (available + borrowed) this Reserve could contain. DepositLiquidity
    /// operation will fail if resulting TotalLiquidity will be more then specified.
    pub max_total_liquidity: u64,

    /// When this Reserve is used as Collateral then Curator can define LTV till which Position can
    /// borrow. The more unstable and less liquid currency from this Reserve - the smaller setting
    /// should be applied.
    /// When there is only one collateral deposited for the Position then total position's max_borrow_ltv
    /// will be equal to that setting of single collateral Reserve. But when there are several collateral
    /// deposits each will give its own value-weighted part in to total max_borrow_ltv.
    /// For example...
    /// Collateral Deposit 1: amount = 100, usd_value = 1000, max_borrow_ltv = 0.7
    /// Collateral Deposit 2: amount = 200, usd_value = 10, max_borrow_ltv = 0.5
    /// Total allowed borrow value will be 1000*0.7 + 10*0.5 = 705 USD. Which equivalent to
    /// total position's LTV = 705 / 1010 = 0.698
    pub max_borrow_ltv_bps: u16,

    /// Maximum utilization until which Reserve allows borrowing. Measured in bps. Borrow operation
    /// will fail if resulting utilization will be higher than specified.
    pub max_borrow_utilization_bps: u16,

    /// Market price freshness threshold. When price (timestamp in PriceFeed account) is older than
    /// current Solana time for more than `stale_threshold_sec` - such price will not be accepted.
    pub price_stale_threshold_sec: u32,

    /// Maximum utilization until which Reserve allows WithdrawLiquidity operation. Measured in bps.
    /// The setting should be used to keep some available_liquidity in the Reserve to support
    /// liquidations.
    pub max_withdraw_utilization_bps: u16,
    pub _padding: [u8; 6],

    /// Program owner fees assessed, separate from gains due to interest accrual
    pub fees: ReserveFeesConfig,
}

impl ReserveConfig {
    /// Validate the reserve configs, when initializing or modifying the reserve
    /// configs
    pub fn validate(&self) -> LendyResult<()> {
        if self.liquidation_bonus_bps > 5_000 {
            msg!("Liquidation bonus must be in range [0, 50] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.partly_unhealthy_ltv_bps < 1_000 || self.partly_unhealthy_ltv_bps > 10_000 {
            msg!("partly_unhealthy_ltv must be in range [10, 100] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.fully_unhealthy_ltv_bps <= self.partly_unhealthy_ltv_bps
            || self.fully_unhealthy_ltv_bps > 10_000
        {
            msg!("fully_unhealthy_ltv must be in range (partly_unhealthy_ltv, 100]");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.max_borrow_ltv_bps < 500 || self.max_borrow_ltv_bps >= self.partly_unhealthy_ltv_bps
        {
            msg!(
                "max_borrow_ltv_bps must be in range [5, partly_unhealthy_ltv) % i.e. [5, {}) %",
                self.partly_unhealthy_ltv_bps / 100
            );
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.fees.curator_borrow_fee_rate_bps >= 200 {
            msg!("curator_borrow_fee must be in range [0, 2] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.fees.curator_performance_fee_rate_bps > 3000 {
            msg!("curator_performance_fee_rate_bps must be in range [0, 30] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.max_borrow_utilization_bps > 10_000 {
            msg!("max_borrow_utilization_bps must be in range [0, 100] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.max_total_liquidity == 0 {
            msg!("max_total_liquidity can't be zero");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.price_stale_threshold_sec == 0 {
            msg!("price_stale_threshold_sec can't be zero");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.max_withdraw_utilization_bps > 10_000 {
            msg!("max_withdraw_utilization_bps must be in range [0, 100] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        Ok(())
    }

    pub fn can_be_applied_now(
        &self,
        proposed_config: &ReserveConfig,
        reserve_timelock: &ReserveTimelock,
    ) -> bool {
        if self.market_price_feed != proposed_config.market_price_feed
            && reserve_timelock.market_price_feed_lock_sec != 0
        {
            return false;
        }
        if self.irm != proposed_config.irm && reserve_timelock.irm_lock_sec != 0 {
            return false;
        }

        if self.liquidation_bonus_bps != proposed_config.liquidation_bonus_bps
            && reserve_timelock.liquidation_bonus_lock_sec != 0
        {
            return false;
        }

        if self.partly_unhealthy_ltv_bps != proposed_config.partly_unhealthy_ltv_bps
            && reserve_timelock.unhealthy_ltv_lock_sec != 0
        {
            return false;
        }

        if self.fully_unhealthy_ltv_bps != proposed_config.fully_unhealthy_ltv_bps
            && reserve_timelock.unhealthy_ltv_lock_sec != 0
        {
            return false;
        }

        if self.partial_liquidation_factor_bps != proposed_config.partial_liquidation_factor_bps
            && reserve_timelock.partial_liquidation_factor_lock_sec != 0
        {
            return false;
        }

        if self.max_total_liquidity != proposed_config.max_total_liquidity
            && reserve_timelock.max_total_liquidity_lock_sec != 0
        {
            return false;
        }

        if self.max_borrow_ltv_bps != proposed_config.max_borrow_ltv_bps
            && reserve_timelock.max_borrow_ltv_lock_sec != 0
        {
            return false;
        }

        if self.max_borrow_utilization_bps != proposed_config.max_borrow_utilization_bps
            && reserve_timelock.max_borrow_utilization_lock_sec != 0
        {
            return false;
        }

        if self.price_stale_threshold_sec != proposed_config.price_stale_threshold_sec
            && reserve_timelock.price_stale_threshold_lock_sec != 0
        {
            return false;
        }

        if self.max_withdraw_utilization_bps != proposed_config.max_withdraw_utilization_bps
            && reserve_timelock.max_withdraw_utilization_lock_sec != 0
        {
            return false;
        }

        if self.fees != proposed_config.fees && reserve_timelock.fees_lock_sec != 0 {
            return false;
        }

        true
    }

    /// Changes `self` in a way described in `proposal`
    pub fn apply_proposal(&mut self, proposal: ConfigProposal) -> LendyResult<()> {
        let change_map = ConfigFields::from_bits(proposal.change_map).ok_or(
            SuperLendyError::Internal("proposal change_map interpretation".to_string()),
        )?;

        if change_map.is_empty() {
            msg!("proposal contains no changes and can not be applied");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if change_map.contains(ConfigFields::MARKET_PRICE_FEED) {
            msg!(
                "apply MARKET_PRICE_FEED. Old value {} new value {}",
                self.market_price_feed,
                proposal.config.market_price_feed
            );
            self.market_price_feed = proposal.config.market_price_feed;
        }

        if change_map.contains(ConfigFields::IRM) {
            msg!(
                "apply IRM. Old value {} new value {}",
                self.irm,
                proposal.config.irm
            );
            self.irm = proposal.config.irm;
        }

        if change_map.contains(ConfigFields::LIQUIDATION_BONUS) {
            msg!(
                "apply LIQUIDATION_BONUS. Old value {} new value {}",
                self.liquidation_bonus_bps,
                proposal.config.liquidation_bonus_bps
            );
            self.liquidation_bonus_bps = proposal.config.liquidation_bonus_bps;
        }

        if change_map.contains(ConfigFields::PARTLY_UNHEALTHY_LTV) {
            msg!(
                "apply PARTLY_UNHEALTHY_LTV. Old value {} new value {}",
                self.partly_unhealthy_ltv_bps,
                proposal.config.partly_unhealthy_ltv_bps
            );
            self.partly_unhealthy_ltv_bps = proposal.config.partly_unhealthy_ltv_bps;
        }

        if change_map.contains(ConfigFields::FULLY_UNHEALTHY_LTV) {
            msg!(
                "apply FULLY_UNHEALTHY_LTV. Old value {} new value {}",
                self.fully_unhealthy_ltv_bps,
                proposal.config.fully_unhealthy_ltv_bps
            );
            self.fully_unhealthy_ltv_bps = proposal.config.fully_unhealthy_ltv_bps;
        }

        if change_map.contains(ConfigFields::PARTIAL_LIQUIDATION_FACTOR) {
            msg!(
                "apply PARTIAL_LIQUIDATION_FACTOR. Old value {} new value {}",
                self.partial_liquidation_factor_bps,
                proposal.config.partial_liquidation_factor_bps
            );
            self.partial_liquidation_factor_bps = proposal.config.partial_liquidation_factor_bps;
        }

        if change_map.contains(ConfigFields::MAX_TOTAL_LIQUIDITY) {
            msg!(
                "apply MAX_TOTAL_LIQUIDITY. Old value {} new value {}",
                self.max_total_liquidity,
                proposal.config.max_total_liquidity
            );
            self.max_total_liquidity = proposal.config.max_total_liquidity;
        }

        if change_map.contains(ConfigFields::MAX_BORROW_LTV) {
            msg!(
                "apply MAX_BORROW_LTV. Old value {} new value {}",
                self.max_borrow_ltv_bps,
                proposal.config.max_borrow_ltv_bps
            );
            self.max_borrow_ltv_bps = proposal.config.max_borrow_ltv_bps;
        }

        if change_map.contains(ConfigFields::MAX_BORROW_UTILIZATION) {
            msg!(
                "apply MAX_BORROW_UTILIZATION. Old value {} new value {}",
                self.max_borrow_utilization_bps,
                proposal.config.max_borrow_utilization_bps
            );
            self.max_borrow_utilization_bps = proposal.config.max_borrow_utilization_bps;
        }

        if change_map.contains(ConfigFields::PRICE_STALE_THRESHOLD) {
            msg!(
                "apply PRICE_STALE_THRESHOLD. Old value {} new value {}",
                self.price_stale_threshold_sec,
                proposal.config.price_stale_threshold_sec
            );
            self.price_stale_threshold_sec = proposal.config.price_stale_threshold_sec;
        }

        if change_map.contains(ConfigFields::MAX_WITHDRAW_UTILIZATION) {
            msg!(
                "apply MAX_WITHDRAW_UTILIZATION. Old value {} new value {}",
                self.max_withdraw_utilization_bps,
                proposal.config.max_withdraw_utilization_bps
            );
            self.max_withdraw_utilization_bps = proposal.config.max_withdraw_utilization_bps;
        }

        if change_map.contains(ConfigFields::CURATOR_BORROW_FEE_RATE) {
            msg!(
                "apply CURATOR_BORROW_FEE_RATE. Old value {} new value {}",
                self.fees.curator_borrow_fee_rate_bps,
                proposal.config.fees.curator_borrow_fee_rate_bps
            );
            self.fees.curator_borrow_fee_rate_bps =
                proposal.config.fees.curator_borrow_fee_rate_bps;
        }

        if change_map.contains(ConfigFields::CURATOR_PERFORMANCE_FEE_RATE) {
            msg!(
                "apply CURATOR_PERFORMANCE_FEE_RATE. Old value {} new value {}",
                self.fees.curator_performance_fee_rate_bps,
                proposal.config.fees.curator_performance_fee_rate_bps
            );
            self.fees.curator_performance_fee_rate_bps =
                proposal.config.fees.curator_performance_fee_rate_bps;
        }

        Ok(())
    }
}

/// Proposed (time locked) config changes works as follows:
/// 1. Each Reserve has proposed_configs array - holds proposed changes.
/// 2. Each proposed change is a new config which will be applied on to current config once brewed.
///    Applied - means that changed values from proposed config will be copied in to Reserve's
///    config. Rest of the Reserve's config stay unchanged.
/// 3. Brew duration is determined based on largest brew time among all changed parameters.
pub const MAX_CONFIG_PROPOSALS: usize = 4;

bitflags! {
    pub struct ConfigFields: u64 {
        const MARKET_PRICE_FEED            = 0b0000000000000001;
        const IRM                          = 0b0000000000000010;
        const LIQUIDATION_BONUS            = 0b0000000000000100;
        const PARTLY_UNHEALTHY_LTV         = 0b0000000000001000;
        const FULLY_UNHEALTHY_LTV          = 0b0000000000010000;
        const PARTIAL_LIQUIDATION_FACTOR   = 0b0000000000100000;
        const MAX_TOTAL_LIQUIDITY          = 0b0000000001000000;
        const MAX_BORROW_LTV               = 0b0000000010000000;
        const MAX_BORROW_UTILIZATION       = 0b0000000100000000;
        const PRICE_STALE_THRESHOLD        = 0b0000001000000000;
        const MAX_WITHDRAW_UTILIZATION     = 0b0000010000000000;
        const CURATOR_BORROW_FEE_RATE      = 0b0000100000000000;
        const CURATOR_PERFORMANCE_FEE_RATE = 0b0001000000000000;
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable, BorshDeserialize, BorshSerialize, PartialEq)]
#[repr(C)]
pub struct ConfigProposal {
    /// Solana time this proposed change can be applied to Reserve. 0 indicates unused entry.
    pub can_be_applied_at: UnixTimestamp,
    /// This is bitmap showing which parameters in `config` represent proposed change. There could
    /// be just one field changed or several or all.
    pub change_map: u64,
    /// Used as storage for changed values. At maximum whole config can be changed via single request.
    pub config: ReserveConfig,
}

impl ConfigProposal {
    /// Returns max time lock value for that proposal - simply max lock among all changed fields.
    pub fn max_time_lock(self, reserve_timelock: &ReserveTimelock) -> LendyResult<UnixTimestamp> {
        let mut max_time_lock: u32 = 0;

        let change_map = ConfigFields::from_bits(self.change_map).ok_or(
            SuperLendyError::Internal("proposal change_map interpretation".to_string()),
        )?;

        if change_map.contains(ConfigFields::MARKET_PRICE_FEED) {
            max_time_lock = max(reserve_timelock.market_price_feed_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::IRM) {
            max_time_lock = max(reserve_timelock.irm_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::LIQUIDATION_BONUS) {
            max_time_lock = max(reserve_timelock.liquidation_bonus_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::PARTLY_UNHEALTHY_LTV) {
            max_time_lock = max(reserve_timelock.unhealthy_ltv_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::FULLY_UNHEALTHY_LTV) {
            max_time_lock = max(reserve_timelock.unhealthy_ltv_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::PARTIAL_LIQUIDATION_FACTOR) {
            max_time_lock = max(
                reserve_timelock.partial_liquidation_factor_lock_sec,
                max_time_lock,
            );
        }

        if change_map.contains(ConfigFields::MAX_TOTAL_LIQUIDITY) {
            max_time_lock = max(reserve_timelock.max_total_liquidity_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::MAX_BORROW_LTV) {
            max_time_lock = max(reserve_timelock.max_borrow_ltv_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::MAX_BORROW_UTILIZATION) {
            max_time_lock = max(
                reserve_timelock.max_borrow_utilization_lock_sec,
                max_time_lock,
            );
        }

        if change_map.contains(ConfigFields::PRICE_STALE_THRESHOLD) {
            max_time_lock = max(
                reserve_timelock.price_stale_threshold_lock_sec,
                max_time_lock,
            );
        }

        if change_map.contains(ConfigFields::MAX_WITHDRAW_UTILIZATION) {
            max_time_lock = max(
                reserve_timelock.max_withdraw_utilization_lock_sec,
                max_time_lock,
            );
        }

        if change_map.contains(ConfigFields::CURATOR_BORROW_FEE_RATE) {
            max_time_lock = max(reserve_timelock.fees_lock_sec, max_time_lock);
        }

        if change_map.contains(ConfigFields::CURATOR_PERFORMANCE_FEE_RATE) {
            max_time_lock = max(reserve_timelock.fees_lock_sec, max_time_lock);
        }

        Ok(max_time_lock as UnixTimestamp)
    }
}

impl Display for ConfigProposal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let change_map = ConfigFields::from_bits(self.change_map).ok_or(Error)?;

        if change_map.contains(ConfigFields::MARKET_PRICE_FEED) {
            write!(f, " market_price_feed: {}", self.config.market_price_feed)?;
        }

        if change_map.contains(ConfigFields::IRM) {
            write!(f, " irm: {}", self.config.irm)?;
        }

        if change_map.contains(ConfigFields::LIQUIDATION_BONUS) {
            write!(
                f,
                " liquidation_bonus_bps: {}",
                self.config.liquidation_bonus_bps
            )?;
        }

        if change_map.contains(ConfigFields::PARTLY_UNHEALTHY_LTV) {
            write!(
                f,
                " partly_unhealthy_ltv_bps: {}",
                self.config.partly_unhealthy_ltv_bps
            )?;
        }

        if change_map.contains(ConfigFields::FULLY_UNHEALTHY_LTV) {
            write!(
                f,
                " fully_unhealthy_ltv_bps: {}",
                self.config.fully_unhealthy_ltv_bps
            )?;
        }

        if change_map.contains(ConfigFields::PARTIAL_LIQUIDATION_FACTOR) {
            write!(
                f,
                " partial_liquidation_factor_bps: {}",
                self.config.partial_liquidation_factor_bps
            )?;
        }

        if change_map.contains(ConfigFields::MAX_TOTAL_LIQUIDITY) {
            write!(
                f,
                " max_total_liquidity: {}",
                self.config.max_total_liquidity
            )?;
        }

        if change_map.contains(ConfigFields::MAX_BORROW_LTV) {
            write!(f, " max_borrow_ltv_bps: {}", self.config.max_borrow_ltv_bps)?;
        }

        if change_map.contains(ConfigFields::MAX_BORROW_UTILIZATION) {
            write!(
                f,
                " max_borrow_utilization_bps: {}",
                self.config.max_borrow_utilization_bps
            )?;
        }

        if change_map.contains(ConfigFields::PRICE_STALE_THRESHOLD) {
            write!(
                f,
                " price_stale_threshold_sec: {}",
                self.config.price_stale_threshold_sec
            )?;
        }

        if change_map.contains(ConfigFields::MAX_WITHDRAW_UTILIZATION) {
            write!(
                f,
                " max_withdraw_utilization_bps: {}",
                self.config.max_withdraw_utilization_bps
            )?;
        }

        if change_map.contains(ConfigFields::CURATOR_BORROW_FEE_RATE) {
            write!(
                f,
                " curator_borrow_fee_rate_bps: {}",
                self.config.fees.curator_borrow_fee_rate_bps
            )?;
        }

        if change_map.contains(ConfigFields::CURATOR_PERFORMANCE_FEE_RATE) {
            write!(
                f,
                " curator_performance_fee_rate_bps: {}",
                self.config.fees.curator_performance_fee_rate_bps
            )?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ProposedConfigs(pub [ConfigProposal; MAX_CONFIG_PROPOSALS]);

/// These are possible values for `reason` in reward rule.
pub const NO_REWARD: u8 = 0;
pub const REWARD_FOR_LIQUIDITY: u8 = 1;
pub const REWARD_FOR_BORROW: u8 = 2;

pub const REWARD_RULE_NAME_MAX_LEN: usize = 7;

/// How rewards logic works...
/// There are reward rules in each reserve. There can be MANY rules for a particular reward token.
/// New rules may be added and changed. Also rules can be deactivated.
/// In the user's position there are Rewards records - each corresponding to a particular reward
/// token. This means that all rules with same reward token from all reserves (in given pool) User
/// works with accumulate rewards on to one `reward` record. That simplifies reward claiming and
/// saves space in Position account.
///
/// In `Reward` records contract tracks how much rewards user earned.
/// 1. When RefreshPosition is called contract look for new rules which
///    can be applied to user's position. It does so by scanning all rules in all reserves user works with.
///    If there is rule which position qualifies for but no `Reward` record - such record is added.
/// 2. Also, during RefreshPosition contract evaluates all existing
///    `Reward` records and accrues rewards using all rules position qualifies.
/// 3. When user claims some reward - that `Reward` record becomes available for use by other rule.
///    It could happen (during process described in p. 1) that such record will be used for same
///    rule or may be for others.
///
/// Reward rule - describes when and how to reward user
#[derive(
    BorshSerialize, BorshDeserialize, Default, Clone, Copy, Debug, Eq, PartialEq, Pod, Zeroable,
)]
#[repr(C)]
pub struct RewardRule {
    /// Reward token mint. This is the key to match Rules and
    pub reward_mint: Pubkey,
    /// Human-readable rule name. It's here because anyway need to align this struct on 16 :-)
    pub name: [u8; REWARD_RULE_NAME_MAX_LEN],
    /// Encodes rewardable situation. Either holding of deposit or holding debt.
    /// Value NO_REWARD indicates inactive rule.
    pub reason: u8,
    /// Solana slot from which this rule takes effect. Also it is slot when rule was created.
    pub start_slot: Slot,
    /// Amount of reward tokens to accrue per slot per deposited/borrowed liquidity token
    pub rate: i128,
}

impl RewardRule {
    /// Functions to read and set Decimal amounts which are i128 inside
    pub fn rate(&self) -> LendyResult<Decimal> {
        Decimal::from_bits(self.rate).map_err(From::from)
    }

    pub fn set_rate(&mut self, value: Decimal) -> LendyResult<()> {
        self.rate = value.into_bits()?;
        Ok(())
    }

    pub fn verify(&self, reward_mint_info: &AccountInfo<'_>) -> LendyResult<()> {
        // reward_mint_info must be owned by Token or Token2022 programs. Just do not want to add
        // dependency on token_2022 so let it be string address defined here.
        if *reward_mint_info.owner != spl_token::id()
            && *reward_mint_info.owner != spl_token_2022::id()
        {
            msg!(
                "Unrecognized owner {} of reward token mint {}",
                reward_mint_info.owner,
                reward_mint_info.key
            );
            return Err(SuperLendyError::InvalidConfig);
        }

        if *reward_mint_info.owner == spl_token::id()
            && spl_token::state::Mint::unpack(&reward_mint_info.data.borrow()).is_err()
        {
            msg!(
                "Provided reward token mint {} can not be unpacked as Mint",
                reward_mint_info.key
            );
            return Err(SuperLendyError::InvalidConfig);
        }

        if *reward_mint_info.owner == spl_token_2022::id() {
            if let Err(err) = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(
                &reward_mint_info.data.borrow(),
            ) {
                msg!(
                    "Provided reward token mint {} can not be unpacked as Mint 2022: {}",
                    reward_mint_info.key,
                    err
                );
                return Err(SuperLendyError::InvalidConfig);
            }
        }

        if self.reward_mint != *reward_mint_info.key {
            msg!(
                "reward mint account provided {} doesn't match rule's mint {}",
                reward_mint_info.key,
                self.reward_mint
            );
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.reason > REWARD_FOR_BORROW {
            msg!("invalid `reason` value {}", self.reason);
            return Err(SuperLendyError::InvalidConfig);
        }

        let rate_decimal = Decimal::from_bits(self.rate)?;

        if rate_decimal == Decimal::ZERO {
            msg!("reward rate can't be zero");
            return Err(SuperLendyError::InvalidConfig);
        }

        // Reward rates are usually much smaller than 1. It is clearly input mistake if we
        // see value bigger than 100.
        if rate_decimal == Decimal::from_i128_with_scale(100_i128, 0)? {
            msg!("reward rate can't be greater than 100");
            return Err(SuperLendyError::InvalidConfig);
        }

        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.reason != NO_REWARD
    }
}

pub const MAX_REWARD_RULES: usize = 8;

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct RewardRules {
    pub rules: [RewardRule; MAX_REWARD_RULES],
}

impl RewardRules {
    pub fn find_rules_by_reason(&mut self, reason: u8) -> Vec<&mut RewardRule> {
        let mut found_rules = Vec::new();
        for rule in self.rules.iter_mut() {
            if rule.reason == reason {
                found_rules.push(rule)
            }
        }

        found_rules
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
#[repr(C)]
pub struct ReserveFeesConfig {
    /// Fee assessed on `Borrow`, expressed as a basis points. Value 1 means 0.01%.
    /// Thus u16 gives 655.35% max
    /// Goes to pool Curator firm
    pub curator_borrow_fee_rate_bps: u16,
    /// Part of pool yield which goes to pool Curator firm
    pub curator_performance_fee_rate_bps: u16,

    pub _padding: [u8; 12],
}

impl ReserveFeesConfig {
    /// Calculate the Curator and Texture fees on borrow
    /// Function accepts `borrow_amount` in WAD representation. And outputs Texture and Curator fees
    /// also in WAD representation.
    pub fn calculate_borrow_fees(
        &self,
        borrow_amount: Decimal,
        decimals: u8,
        texture_borrow_fee_bps: u16,
        fee_calculation: FeeCalculation,
    ) -> LendyResult<(u64, u64)> {
        self.calculate_fees(
            borrow_amount,
            decimals,
            texture_borrow_fee_bps,
            fee_calculation,
        )
    }

    fn calculate_fees(
        &self,
        amount: Decimal,
        decimals: u8,
        texture_borrow_fee_bps: u16,
        fee_calculation: FeeCalculation,
    ) -> LendyResult<(/*curator fee*/ u64, /*texture fee */ u64)> {
        if amount == Decimal::ZERO {
            return Ok((0, 0));
        }

        let curator_fee_rate = Decimal::from_basis_points(self.curator_borrow_fee_rate_bps as u32)?;
        let texture_fee_rate = Decimal::from_basis_points(texture_borrow_fee_bps as u32)?;

        let (fee_rate, minimum_total_fee) =
            if curator_fee_rate != Decimal::ZERO && texture_fee_rate != Decimal::ZERO {
                (
                    curator_fee_rate.checked_add(texture_fee_rate)?,
                    Decimal::from_lamports(2, decimals)?,
                )
            } else if texture_fee_rate != Decimal::ZERO {
                (texture_fee_rate, Decimal::from_lamports(1, decimals)?)
            } else if curator_fee_rate != Decimal::ZERO {
                (curator_fee_rate, Decimal::from_lamports(1, decimals)?)
            } else {
                return Ok((0, 0));
            };

        let borrow_fee_amount = match fee_calculation {
            // Calculate fee to be added to borrow: fee = amount * rate
            FeeCalculation::Exclusive => amount.checked_mul(fee_rate)?,
            // Calculate fee to be subtracted from borrow: fee = amount * (rate / (rate + 1))
            FeeCalculation::Inclusive => {
                let borrow_fee_rate = fee_rate.checked_div(fee_rate.checked_add(Decimal::ONE)?)?;
                amount.checked_mul(borrow_fee_rate)?
            }
        };

        // Would like to take at least minimum fees (1 lamport per Texture and Curator)
        let borrow_fee_amount = borrow_fee_amount.max(minimum_total_fee);

        if borrow_fee_amount >= amount {
            msg!("Borrow amount is too small to receive liquidity after fees");
            return Err(SuperLendyError::InvalidAmount);
        }

        let total_borrow_fee = borrow_fee_amount.to_lamports_round(decimals)?;

        if curator_fee_rate != Decimal::ZERO && texture_fee_rate != Decimal::ZERO {
            // Split the fee between Curator anf Texture. We did combine fee math above for correct
            // Inclusive type calculation.
            let fees_relation = curator_fee_rate.checked_div(texture_fee_rate)?;
            let texture_fee_decimal = Decimal::from_lamports(total_borrow_fee, decimals)?
                .checked_div(fees_relation.checked_add(Decimal::ONE)?)?;
            let texture_fee = texture_fee_decimal.to_lamports_round(decimals)?;

            let curator_fee = if total_borrow_fee >= texture_fee {
                total_borrow_fee - texture_fee
            } else {
                msg!(
                    "total_borrow_fee {} less then texture_fee {}",
                    total_borrow_fee,
                    texture_fee
                );
                return Err(SuperLendyError::from(MathError(String::from(
                    "curator_fee calc",
                ))));
            };

            Ok((curator_fee, texture_fee))
        } else if texture_fee_rate != Decimal::ZERO {
            Ok((0, total_borrow_fee))
        } else if curator_fee_rate != Decimal::ZERO {
            Ok((total_borrow_fee, 0))
        } else {
            Ok((0, 0))
        }
    }
}

/// Calculate fees exlusive or inclusive of an amount
pub enum FeeCalculation {
    /// Fee added to amount: fee = rate * amount
    Exclusive,
    /// Fee included in amount: fee = (rate / (1 + rate)) * amount
    Inclusive,
}

pub fn current_borrow_rate(
    curve_y: &[u32],
    curve_decimals: u8,
    curve_x_step: u32,
    curve_x0: Decimal,
    utilization_rate: Decimal,
) -> MathResult<Decimal> {
    calc_y_with_params(
        curve_y,
        curve_decimals,
        curve_x_step,
        curve_x0,
        utilization_rate,
    )
}

pub fn current_deposit_rate(
    curve_y: &[u32],
    curve_decimals: u8,
    curve_x_step: u32,
    curve_x0: Decimal,
    utilization_rate: Decimal,
    texture_performance_fee_rate_bps: u16,
    curator_performance_fee_rate_bps: u16,
) -> MathResult<Decimal> {
    let current_borrow_rate = current_borrow_rate(
        curve_y,
        curve_decimals,
        curve_x_step,
        curve_x0,
        utilization_rate,
    )?;
    let deposit_interest_rate_gross = current_borrow_rate.checked_mul(utilization_rate)?;
    let net_fees_rate = Decimal::ONE
        .checked_sub(Decimal::from_basis_points(
            texture_performance_fee_rate_bps as u32,
        )?)?
        .checked_sub(Decimal::from_basis_points(
            curator_performance_fee_rate_bps as u32,
        )?)?;

    deposit_interest_rate_gross.checked_mul(net_fees_rate)
}

/// Calculates total liquidity for given Reserve. This is simpy a sum of:
/// `available_amount` - money on still present on Reserve's liquidity supply. WAD.
/// `borrowed_amount` - money Borrowers owe to the Reserve. WAD.
pub fn total_liquidity(available_amount: Decimal, borrowed_amount: Decimal) -> MathResult<Decimal> {
    available_amount
        .checked_add(borrowed_amount)
        .map_err(From::from)
}

pub fn liquidity_utilization_rate(
    total_liquidity: Decimal,
    borrowed_amount_wads: Decimal,
) -> MathResult<Decimal> {
    if total_liquidity == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }
    borrowed_amount_wads.checked_div(total_liquidity)
}

/// Calculates maximum liquidity amount which can be withdrawn from the Reserve without hitting
/// its max_withdraw_utilization threshold.
pub fn max_withdraw_liquidity_amount(
    available_liquidity: Decimal,
    borrowed_amount_wads: Decimal,
    max_withdraw_utilization: Decimal,
) -> MathResult<Decimal> {
    if available_liquidity == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    if max_withdraw_utilization == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let available_amount_to_keep = borrowed_amount_wads
        .checked_div(max_withdraw_utilization)?
        .checked_sub(borrowed_amount_wads)?;

    if available_amount_to_keep > available_liquidity {
        // Means that max_withdraw_utilization was already reached and Reserve can't afford any withdraw
        return Ok(Decimal::ZERO);
    }

    available_liquidity.checked_sub(available_amount_to_keep)
}

/// Amount of liquidity can be borrowed from the Reserve
/// There are two limiting factors:
/// 1. available liquidity
/// 2. Reserve's max_borrow_utilization
pub fn max_borrow_amount(
    available_amount: u64,
    decimals: u8,
    borrowed_amount: Decimal,
    max_borrow_utilization_bps: u16,
) -> LendyResult<Decimal> {
    if max_borrow_utilization_bps == 0 {
        // Reserve can not give any money
        return Ok(Decimal::ZERO);
    }

    if max_borrow_utilization_bps == 10_000 {
        // Reserve can give all available amount
        return Ok(Decimal::from_lamports(available_amount, decimals)?);
    }

    let max_borrow_utilization = Decimal::from_basis_points(max_borrow_utilization_bps as u32)?;

    let remaining_borrow_amount = max_borrow_utilization
        .checked_mul(Decimal::from_lamports(available_amount, decimals)?)?
        .checked_add(max_borrow_utilization.checked_mul(borrowed_amount)?)?
        .checked_sub(borrowed_amount)?;

    if remaining_borrow_amount <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    Ok(Decimal::from_lamports(available_amount, decimals)?.min(remaining_borrow_amount))
}

/// Calculates LP exchange rate.
/// `total_liquidity` - total liquidity in given Reserve. WAD.
/// `lp_total_supply` - total LP tokens minted by that Reserve. WAD.
pub fn lp_exchange_rate(
    total_liquidity: Decimal,
    lp_total_supply: Decimal,
) -> MathResult<LpExchangeRate> {
    let rate = if lp_total_supply == Decimal::ZERO || total_liquidity == Decimal::ZERO {
        Decimal::from_i128_with_scale(INITIAL_COLLATERAL_RATE as i128, SCALE)?
    } else {
        lp_total_supply.checked_div(total_liquidity)?
    };

    Ok(LpExchangeRate(rate))
}

/// Calculates LP price based on price of underling assets.
/// `total_liquidity` - total liquidity from given Reserve. WAD
/// `lp_total_supply` - total number of minted LP tokens for that Reserve. WAD.
/// `liquidity_market_price` - market price of the liquidity token from the Reserve.
pub fn lp_market_price(
    total_liquidity: Decimal,
    lp_total_supply: Decimal,
    liquidity_market_price: Decimal,
) -> MathResult<Decimal> {
    let exchange_rate = lp_exchange_rate(total_liquidity, lp_total_supply)?;
    let liquidity_in_one_lp = exchange_rate.decimal_lp_to_liquidity(Decimal::ONE)?;

    liquidity_in_one_lp.checked_mul(liquidity_market_price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::position::{MAX_BORROWS, MAX_DEPOSITS};
    use assert_matches::assert_matches;

    fn test_reserve(available_amount: u64) -> Reserve {
        Reserve {
            discriminator: *RESERVE_DISCRIMINATOR,
            version: 1,
            reserve_type: RESERVE_TYPE_NORMAL,
            mode: RESERVE_MODE_NORMAL,
            flash_loans_enabled: 0,
            _flags: Zeroable::zeroed(),
            last_update: LastUpdate {
                slot: 0,
                timestamp: 0,
                stale: 0,
                _padding: Zeroable::zeroed(),
            },
            pool: Default::default(),
            liquidity: ReserveLiquidity {
                mint: Default::default(),
                available_amount,
                borrowed_amount: 0,
                cumulative_borrow_rate: Decimal::ONE.into_bits().unwrap(),
                market_price: Decimal::from_i128_with_scale(345, 0)
                    .unwrap()
                    .into_bits()
                    .unwrap(),
                curator_performance_fee: Decimal::from_basis_points(300)
                    .unwrap()
                    .into_bits()
                    .unwrap(),
                texture_performance_fee: Decimal::from_basis_points(500)
                    .unwrap()
                    .into_bits()
                    .unwrap(),
                mint_decimals: 9,
                _padding1: Zeroable::zeroed(),
                _padding: 0,
                borrow_rate: 0,
            },
            collateral: ReserveCollateral {
                lp_total_supply: available_amount, // LP exchange rate = 1
                _padding: 0,
            },
            config: ReserveConfig {
                market_price_feed: Default::default(),
                irm: Default::default(),
                liquidation_bonus_bps: 10,
                max_borrow_ltv_bps: 8000,
                partly_unhealthy_ltv_bps: 8500,
                fully_unhealthy_ltv_bps: 9000,
                partial_liquidation_factor_bps: 2000,
                _padding: Zeroable::zeroed(),
                fees: ReserveFeesConfig {
                    curator_borrow_fee_rate_bps: 300, // 3%
                    curator_performance_fee_rate_bps: 0,
                    _padding: Zeroable::zeroed(),
                },
                max_total_liquidity: u64::MAX,
                max_borrow_utilization_bps: 8000,
                price_stale_threshold_sec: 1,
                max_withdraw_utilization_bps: 9500,
            },
            reward_rules: Zeroable::zeroed(),
            proposed_configs: ProposedConfigs::zeroed(),
            _padding: Zeroable::zeroed(),
        }
    }

    #[test]
    fn calc_lp_market_price() {
        let mut test_reserve = test_reserve(1000);

        // Borrowed amount is 1000
        test_reserve
            .liquidity
            .set_borrowed_amount(Decimal::from_lamports(1000, 9).unwrap())
            .unwrap();

        test_reserve
            .liquidity
            .set_market_price(Decimal::from_i128_with_scale(250, 0).unwrap())
            .unwrap();

        // There are twice less LP tokens then liquidity tokens.
        test_reserve.collateral.lp_total_supply = 1000;

        let total_liquidity = test_reserve.liquidity.total_liquidity().unwrap();

        assert_eq!(total_liquidity, Decimal::from_lamports(2000, 9).unwrap());

        let lp_market_price = test_reserve.lp_market_price().unwrap();

        // Because there twice less LP tokens each of them worth 2 liquidity tokens. One liquidity
        // token have value of 250. Thus one LP costs 500.
        assert_eq!(
            lp_market_price,
            Decimal::from_i128_with_scale(500, 0).unwrap()
        );
    }

    #[test]
    fn compound_interest() {
        let mut test_reserve = test_reserve(1000);
        // Borrowed amount is 1000
        test_reserve
            .liquidity
            .set_borrowed_amount(Decimal::from_lamports(1000, 9).unwrap())
            .unwrap();

        let current_borrow_rate = Decimal::from_basis_points(2000).expect("current_borrow_rate"); // 20%
        let curator_performance_fee_rate_bps = 0;
        let texture_performance_fee_rate_bps = 0;
        let slots_elapsed = SLOTS_PER_YEAR;

        test_reserve
            .liquidity
            .compound_interest(
                current_borrow_rate,
                curator_performance_fee_rate_bps,
                texture_performance_fee_rate_bps,
                slots_elapsed,
            )
            .expect("interest");

        // Because one Solana year is passed and borrow rate was 20% - borrowed amount in the
        // reserve must increase approx. by 22% due to interest compounding
        assert_eq!(
            test_reserve
                .liquidity
                .borrowed_amount()
                .unwrap()
                .to_lamports_round(9)
                .unwrap(),
            1221
        );

        assert_eq!(
            test_reserve.liquidity.cumulative_borrow_rate().unwrap(),
            Decimal::from_i128_with_scale(1221402757772865561, 18).unwrap()
        );

        // Same reserve with same rates BUT with curator and Texture performance fees turned on.
        let mut test_reserve1 = crate::state::reserve::tests::test_reserve(1000);
        // Borrowed amount is 1000
        test_reserve1
            .liquidity
            .set_borrowed_amount(Decimal::from_lamports(1000, 9).unwrap())
            .unwrap();

        let current_borrow_rate = Decimal::from_basis_points(2000).expect("current_borrow_rate"); // 20%
        let curator_performance_fee_rate_bps = 2000; // 20%
        let texture_performance_fee_rate_bps = 3000; // 30%
        let slots_elapsed = SLOTS_PER_YEAR;

        test_reserve1
            .liquidity
            .compound_interest(
                current_borrow_rate,
                curator_performance_fee_rate_bps,
                texture_performance_fee_rate_bps,
                slots_elapsed,
            )
            .expect("interest");

        // Interest for the period is ~21 USDC. All interest goes in to borrowed_amount
        assert_eq!(
            test_reserve1
                .liquidity
                .borrowed_amount()
                .unwrap()
                .to_lamports_round(9)
                .unwrap(),
            1221
        );

        // Cumulative borrow rate stays the same as in situation without perf. fees.
        assert_eq!(
            test_reserve.liquidity.cumulative_borrow_rate().unwrap(),
            Decimal::from_i128_with_scale(1221402757772865561, 18).unwrap()
        );
    }
    #[test]
    fn calc_borrow() {
        let liquidity_available_amount = 1_000_000_000;

        let test_reserve = test_reserve(liquidity_available_amount);

        let amount_to_borrow = 1_000; // amount of money used from Reserve (principal + fee)
        let max_borrow_value = Decimal::from_i128_with_scale(100000, 0).unwrap();
        let texture_borrow_fee_bps = 200; // 2%
        let borrow_result = test_reserve
            .calculate_borrow(amount_to_borrow, max_borrow_value, texture_borrow_fee_bps)
            .unwrap();

        // When there is enough borrowing power and available amount then borrow amount is principal + fees
        assert_eq!(
            borrow_result.borrow_amount,
            Decimal::from_lamports(1050, 9).unwrap()
        ); // 1000 principal + 30 curator fee + 20 texture fee
        assert_eq!(borrow_result.receive_amount, amount_to_borrow);
        assert_eq!(borrow_result.curator_borrow_fee, 30);
        assert_eq!(borrow_result.texture_borrow_fee, 20);

        // Now borrow max allowed
        let borrow_result = test_reserve
            .calculate_borrow(u64::MAX, max_borrow_value, texture_borrow_fee_bps)
            .unwrap();

        // When there is NOT enough borrowing power or available amount then fees are included in to the borrowing limit.
        assert_eq!(
            borrow_result.borrow_amount,
            Decimal::from_lamports(liquidity_available_amount, 9).unwrap()
        ); // limited by available amount in Reserve
        assert_eq!(
            borrow_result.receive_amount,
            liquidity_available_amount - 28571429 - 19047619
        ); // minus fees
        assert_eq!(borrow_result.curator_borrow_fee, 28571429); // 3% of receive_amount
        assert_eq!(borrow_result.texture_borrow_fee, 19047619); // 2% of receive_amount

        let two_percents = borrow_result.receive_amount * 2 / 100;
        assert_eq!(two_percents, 19047619);
    }

    #[test]
    fn calc_repay() {
        let liquidity_available_amount = 1_000_000_000;

        let test_reserve = test_reserve(liquidity_available_amount);

        let amount_to_repay = 1000;
        let borrowed_amount = Decimal::from_lamports(100000, 9).unwrap();
        let repay_result = test_reserve
            .calculate_repay(amount_to_repay, borrowed_amount)
            .unwrap();

        assert_eq!(
            repay_result.settle_amount,
            Decimal::from_lamports(amount_to_repay, 9).unwrap()
        );
        assert_eq!(repay_result.repay_amount, amount_to_repay);

        let repay_result = test_reserve
            .calculate_repay(u64::MAX, borrowed_amount)
            .unwrap();

        assert_eq!(repay_result.settle_amount, borrowed_amount);
        assert_eq!(repay_result.repay_amount, 100000);
    }

    #[test]
    fn calc_max_withdraw_amount() {
        let liquidity_available_amount = 1_000_000_000; // 1 SOL

        let mut test_reserve = test_reserve(liquidity_available_amount);

        // Borrowed amount equals to available. Thus utilization is 50%
        test_reserve
            .liquidity
            .set_borrowed_amount(Decimal::from_lamports(liquidity_available_amount, 9).unwrap())
            .unwrap();
        test_reserve.config.max_withdraw_utilization_bps = 4000; // 40%

        let amount = test_reserve.max_withdraw_liquidity_amount().unwrap();

        assert_eq!(amount, Decimal::ZERO);

        test_reserve.config.max_withdraw_utilization_bps = 5000; // 50%

        let amount = test_reserve.max_withdraw_liquidity_amount().unwrap();

        // At max_withdraw_utilization_bps=50% Reserve will allow 0 withdraw
        assert_eq!(amount, Decimal::ZERO);

        test_reserve.config.max_withdraw_utilization_bps = 5001; // 50.01%

        let amount = test_reserve.max_withdraw_liquidity_amount().unwrap();

        // At max_withdraw_utilization_bps=50.01% Reserve will allow small withdraw of ~399920 lamports which
        // is 399920 / 1_000_000_000 * 100 = 0.04% of the available liquidity.
        // If we'll withdraw 399920 then:
        // available_amount = 1_000_000_000 - 399920 = 999600080
        // utilization = borrowed_amount / total_amount = 1_000_000_000 / (1_000_000_000 + 999600080) = 0,5001     - magic!
        assert_eq!(amount.to_lamports_round(9).unwrap(), 399920);

        test_reserve.config.max_withdraw_utilization_bps = 10000; // 100%

        let amount = test_reserve.max_withdraw_liquidity_amount().unwrap();

        // At max_withdraw_utilization_bps=100% Reserve will allow to withdraw all available liquidity.
        assert_eq!(
            amount,
            Decimal::from_lamports(liquidity_available_amount, 9).unwrap()
        );
    }

    #[test]
    fn try_liquidate_healthy_position() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        // Situation: user's LTV is 80% and partly_unhealthy_ltv 84% thus position is healthy.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(1_000_000, 9).unwrap(); // Other users borrowed some amount too
        let reserve_borrowed_value = reserve_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let partial_liquidation_factor_bps = 2000; // 20%

        let user_borrowed_amount = Decimal::from_lamports(1000, 9).unwrap();
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000, 6).unwrap();
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 8400; // 84%
        let fully_unhealthy_ltv_bps = 9000; // 90%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = 100;

        let mut reserve = test_reserve(1_000_000_000);

        // Reserve's borrowed amount is greater then borrowed by test user.
        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(reserve_borrowed_value)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.round().unwrap();

        let calc_result = reserve.calculate_liquidation(
            amount_to_liquidate,
            &position,
            &borrowed_liquidity,
            &collateral,
            reserve.liquidity.mint_decimals,
        );

        assert_matches!(
            calc_result,
            Err(SuperLendyError::AttemptToLiquidateHealthyPosition(
                _ltv,
                _partly_unhealthy_ltv
            ))
        );
    }

    // Normal partial liquidation with amount specified by Liquidator and not bounded by Reserve.
    #[test]
    fn partial_liquidation() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(100_000_000_000, 9).unwrap(); // Other users borrowed some amount too
        let partial_liquidation_factor_bps = 2000; // 20%
        let liquidation_bonus_bps = 10; // 0.1 %

        let user_borrowed_amount = Decimal::from_lamports(1_000_000_000, 9).unwrap(); // 1 SOL
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000_000, 6).unwrap(); // 100 USDC
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 7000; // 70%
        let fully_unhealthy_ltv_bps = 9000; // 90%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = 100_000_000; // 0.1 SOL

        let mut reserve = test_reserve(1_000_000_000_000);

        // Reserve's borrowed amount is greater then borrowed by test user.
        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(principal_market_price)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;
        reserve.config.liquidation_bonus_bps = liquidation_bonus_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.to_lamports_round(6).unwrap();

        let calc_result = reserve
            .calculate_liquidation(
                amount_to_liquidate,
                &position,
                &borrowed_liquidity,
                &collateral,
                reserve.liquidity.mint_decimals,
            )
            .unwrap();

        assert_eq!(
            position.ltv().unwrap(),
            Decimal::from_basis_points(8000).unwrap()
        );

        // Amount of principal which Liquidator repays
        // Because Liquidator asked for 100, user borrowed 1000 and partial liquidation factor is 20% then
        // we can execute exact the quantity Liquidator asks for.
        assert_eq!(calc_result.repay_amount, amount_to_liquidate);

        // Amount of collateral tokens Liquidator receives. Principal price is 80, collateral 1. Means
        // that 1 borrowed token worth 80 LP tokens (not interest accruals yet). Thus "base" collateral
        // amount for Liquidator is 8000. Plus 0.1 % of bonus - 8.
        assert_eq!(calc_result.withdraw_amount, 8_008_000);
    }

    // User's position meets criteria for partial liquidation. Liquidator asks for max possible amount,
    #[test]
    fn partial_liquidation_max_allowed() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(100_000_000_000, 9).unwrap(); // Other users borrowed some amount too
        let partial_liquidation_factor_bps = 2000; // 20%
        let liquidation_bonus_bps = 10; // 0.1 %

        let user_borrowed_amount = Decimal::from_lamports(1_000_000_000, 9).unwrap();
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000_000, 6).unwrap();
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 7000; // 70%
        let fully_unhealthy_ltv_bps = 9000; // 90%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = u64::MAX;

        let mut reserve = test_reserve(1_000_000_000_000);

        // Reserve's borrowed amount is greater then borrowed by test user.
        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(principal_market_price)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;
        reserve.config.liquidation_bonus_bps = liquidation_bonus_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.to_lamports_round(6).unwrap();

        let calc_result = reserve
            .calculate_liquidation(
                amount_to_liquidate,
                &position,
                &borrowed_liquidity,
                &collateral,
                reserve.liquidity.mint_decimals,
            )
            .unwrap();

        assert_eq!(
            position.ltv().unwrap(),
            Decimal::from_basis_points(8000).unwrap()
        );

        // Amount of principal which Liquidator repays
        // Because Liquidator asked for maximum contract can give and liquidation_factor is 20% i.e. 0.2 SOL
        assert_eq!(calc_result.repay_amount, 200_000_000);

        // Amount of collateral tokens Liquidator receives. Principal price is 80, collateral 1. Means
        // that 1 borrowed token worth 80 LP tokens (not interest accruals yet). Thus "base" collateral
        // amount for Liquidator is 16 USDC i.e. 16_000_000. Plus 0.1 % of bonus - 16.
        assert_eq!(calc_result.withdraw_amount, 16_016_000);
    }

    // User's position meets criteria for partial liquidation. Liquidator asks for fixed amount
    // which is bigger then partial_liquidation_factor allows.
    // Error: cannot liquidate bigger then allowed. Use MAX_AMOUNT instead.
    #[test]
    fn partial_liquidation_with_too_big_amount() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(10_000_000_000, 9).unwrap(); // Other users borrowed some amount too
        let partial_liquidation_factor_bps = 2000; // 20%
        let liquidation_bonus_bps = 10; // 0.1 %

        let user_borrowed_amount = Decimal::from_lamports(1_000_000_000, 9).unwrap();
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000_000, 6).unwrap();
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 7000; // 70%
        let fully_unhealthy_ltv_bps = 9000; // 90%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = 300_000_000; // BIGGER then allowed! Max allowed is 200_000_000.

        let mut reserve = test_reserve(1_000_000_000_000);

        // Reserve's borrowed amount is greater then borrowed by test user.
        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(principal_market_price)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;
        reserve.config.liquidation_bonus_bps = liquidation_bonus_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.round().unwrap();

        let position_ltv = position.ltv().unwrap();

        assert_eq!(position_ltv, Decimal::from_i128_with_scale(8, 1).unwrap()); // 0.8

        let calc_result = reserve.calculate_liquidation(
            amount_to_liquidate,
            &position,
            &borrowed_liquidity,
            &collateral,
            reserve.liquidity.mint_decimals,
        );
        assert!(calc_result.is_err());
    }

    // User's position meets criteria for FULL liquidation. Liquidator asks for fixed amount
    // which is less then total position's borrowing.
    #[test]
    fn full_liquidation_with_fixed_amount() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(10_000_000_000, 9).unwrap(); // 10 SOL - Other users borrowed some amount too
        let partial_liquidation_factor_bps = 2000; // 20%
        let liquidation_bonus_bps = 10; // 0.1 %

        let user_borrowed_amount = Decimal::from_lamports(1_000_000_000, 9).unwrap(); // 1 SOL
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000_000, 6).unwrap(); // 100 USDC
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 7000; // 70%
        let fully_unhealthy_ltv_bps = 7500; // 75%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = 800_000_000; // 0.8 SOL

        let mut reserve = test_reserve(100_000_000_000);

        // Reserve's borrowed amount is greater then borrowed by test user.
        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(principal_market_price)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;
        reserve.config.liquidation_bonus_bps = liquidation_bonus_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.to_lamports_round(6).unwrap();

        let calc_result = reserve
            .calculate_liquidation(
                amount_to_liquidate,
                &position,
                &borrowed_liquidity,
                &collateral,
                reserve.liquidity.mint_decimals,
            )
            .unwrap();

        assert_eq!(
            position.ltv().unwrap(),
            Decimal::from_basis_points(8000).unwrap()
        );

        // Amount of principal which Liquidator repays. As was specified by Liquidator.
        assert_eq!(calc_result.repay_amount, 800_000_000);

        // Amount of collateral tokens Liquidator receives. Principal price is 80, collateral 1. Means
        // that 1 borrowed token worth 80 LP tokens (not interest accruals yet). Thus "base" collateral
        // amount for Liquidator is 64_000_000 (64$). Plus 0.1 % of bonus - 64_000.
        assert_eq!(calc_result.withdraw_amount, 64_064_000);
    }

    // User's position meets criteria for FULL liquidation. Liquidator asks for max possible amount
    #[test]
    fn full_liquidation() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(10_000_000_000, 9).unwrap(); // 10 SOL - Other users borrowed some amount too
        let partial_liquidation_factor_bps = 2000; // 20%
        let liquidation_bonus_bps = 10; // 0.1 %

        let user_borrowed_amount = Decimal::from_lamports(1_000_000_000, 9).unwrap(); // 1 SOL
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000_000, 6).unwrap(); // 100 USDC
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 7000; // 70%
        let fully_unhealthy_ltv_bps = 7500; // 75%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = u64::MAX;

        let mut reserve = test_reserve(100_000_000_000);

        // Reserve's borrowed amount is greater then borrowed by test user.
        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(principal_market_price)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;
        reserve.config.liquidation_bonus_bps = liquidation_bonus_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.to_lamports_round(6).unwrap();

        let calc_result = reserve
            .calculate_liquidation(
                amount_to_liquidate,
                &position,
                &borrowed_liquidity,
                &collateral,
                reserve.liquidity.mint_decimals,
            )
            .unwrap();

        assert_eq!(
            position.ltv().unwrap(),
            Decimal::from_basis_points(8000).unwrap()
        );

        // Amount of principal (SOL) which Liquidator repays. Maximum contract allows - full borrowed amount.
        assert_eq!(calc_result.repay_amount, 1_000_000_000); // 1 SOL - as was borrowed

        // Amount of collateral (USDC) tokens Liquidator receives. It should be 80$ + 0.1% bonus = 80.08$ i.e. 80_080_000
        assert_eq!(calc_result.withdraw_amount, 80_080_000);
    }

    // User's position meets criteria for FULL liquidation. Moreover - there is already bad debt.
    // Liquidator asks for max possible amount.
    // Contract should allow as much of borrowed amount as mush collateral is there taking in to
    // account liquidation_bonus.
    #[test]
    fn bad_debt_position_liquidation() {
        // Below are amounts and prices used as input to setup all objects in this test. Edit this
        // section if you want to simulate other situations.
        let principal_market_price = Decimal::from_i128_with_scale(80, 0).unwrap(); // Assume principal is SOL
        let collateral_market_price = Decimal::from_i128_with_scale(1, 0).unwrap(); // Assume collateral is USDC
        let reserve_borrowed_amount = Decimal::from_lamports(10_000_000_000, 9).unwrap(); // 10 SOL - Other users borrowed some amount too
        let partial_liquidation_factor_bps = 2000; // 20%
        let liquidation_bonus_bps = 1000; // 10 %

        let user_borrowed_amount = Decimal::from_lamports(2_000_000_000, 9).unwrap(); // 2 SOL
        let user_borrowed_value = user_borrowed_amount
            .checked_mul(principal_market_price)
            .unwrap();
        let user_collateral_amount = Decimal::from_lamports(100_000_000, 6).unwrap(); // 100 USDC
        let user_collateral_value = user_collateral_amount
            .checked_mul(collateral_market_price)
            .unwrap();
        let partly_unhealthy_ltv_bps = 7000; // 70%
        let fully_unhealthy_ltv_bps = 7500; // 75%
        let partly_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(partly_unhealthy_ltv_bps).unwrap())
            .unwrap();
        let fully_unhealthy_borrow_value = user_borrowed_value
            .checked_mul(Decimal::from_basis_points(fully_unhealthy_ltv_bps).unwrap())
            .unwrap();

        let amount_to_liquidate = u64::MAX;

        let mut reserve = test_reserve(100_000_000_000);

        reserve
            .liquidity
            .set_borrowed_amount(reserve_borrowed_amount)
            .unwrap();
        reserve
            .liquidity
            .set_market_price(principal_market_price)
            .unwrap();
        reserve.config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps as u16;
        reserve.config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps as u16;
        reserve.config.partial_liquidation_factor_bps = partial_liquidation_factor_bps;
        reserve.config.liquidation_bonus_bps = liquidation_bonus_bps;

        // Its OK to have uninitialized deposits and borrows as calculate_liquidation() doesn't look
        // in to these arrays inside Position. It uses deposit and borrow entries passed as input params.
        let deposits: [DepositedCollateral; MAX_DEPOSITS] = Zeroable::zeroed();
        let borrows: [BorrowedLiquidity; MAX_BORROWS] = Zeroable::zeroed();

        let mut position = Position::new(Default::default(), Default::default(), deposits, borrows);

        position.set_deposited_value(user_collateral_value).unwrap();
        position.set_borrowed_value(user_borrowed_value).unwrap();
        position
            .set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)
            .unwrap();
        position
            .set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)
            .unwrap();

        let mut borrowed_liquidity = BorrowedLiquidity::new(Pubkey::new_unique(), Decimal::ONE);
        borrowed_liquidity
            .set_borrowed_amount(user_borrowed_amount)
            .unwrap();
        borrowed_liquidity
            .set_market_value(user_borrowed_value)
            .unwrap();

        let mut collateral = DepositedCollateral::new(Pubkey::new_unique());
        collateral.set_market_value(user_collateral_value).unwrap();
        collateral.deposited_amount = user_collateral_amount.round().unwrap();

        println!("borrowed_value {}", position.borrowed_value().unwrap());
        println!("deposited_value {}", position.deposited_value().unwrap());

        assert_eq!(
            position.ltv().unwrap(),
            Decimal::from_basis_points(16000).unwrap()
        ); // Position's LTV is 160%

        // Principal Reserve state:
        // borrowed_amount = 1_000_000
        // market_price = 80$
        // Position:
        // LTV = 160 %
        // collateral_amount = 100_000
        // collateral_value = 100_000
        // borrowed_amount = 2000
        // borrowed_value = 2000*80 = 160_000

        let calc_result = reserve
            .calculate_liquidation(
                amount_to_liquidate,
                &position,
                &borrowed_liquidity,
                &collateral,
                reserve.liquidity.mint_decimals,
            )
            .unwrap();

        assert_eq!(
            position.ltv().unwrap(),
            Decimal::from_basis_points(16000).unwrap()
        ); // Position's LTV is 160%

        // Amount of collateral which Liquidator repays.
        // This is the biggest repay amount which results in all collateral being transferred to
        // Liquidator plus his bonus.
        assert_eq!(calc_result.repay_amount, 1136363637);

        // Liquidator receives all users collateral.
        assert_eq!(
            calc_result.withdraw_amount,
            user_collateral_amount.round().unwrap()
        );
    }

    #[test]
    fn max_borrow_amount() {
        // max_borrow_utilization_bps = 80%
        let initial_deposit = 1_000_000;

        let mut reserve = test_reserve(initial_deposit);

        let max_borrow_amount = reserve.max_borrow_amount().unwrap();

        // Reserve can
        assert_eq!(
            max_borrow_amount,
            Decimal::from_lamports(800_000, 9).unwrap()
        );

        // Borrow half of the available amount. Brings utilization to 50%
        reserve
            .liquidity
            .borrow(Decimal::from_lamports(500_000, 9).unwrap(), 500_000)
            .unwrap();

        let max_borrow_amount = reserve.max_borrow_amount().unwrap();

        // Reserve can only give 300_000 in order to not rise utilization above 80%
        assert_eq!(
            max_borrow_amount,
            Decimal::from_lamports(300_000, 9).unwrap()
        );

        // Borrow 300_000 amount. Brings utilization to 80% - no more borrows allowed.
        reserve
            .liquidity
            .borrow(Decimal::from_lamports(300_000, 9).unwrap(), 300_000)
            .unwrap();

        let max_borrow_amount = reserve.max_borrow_amount().unwrap();

        // Reserve can give nothing as utilization is 80%
        assert_eq!(max_borrow_amount, Decimal::ZERO);

        // Set borrowed_amount little bit higher - simulate accrued interest.
        reserve
            .liquidity
            .set_borrowed_amount(Decimal::from_lamports(900_000, 9).unwrap())
            .unwrap();

        let max_borrow_amount = reserve.max_borrow_amount().unwrap();

        // Reserve can give nothing
        assert_eq!(max_borrow_amount, Decimal::ZERO);
    }

    #[test]
    fn huge_deposits() {
        // max_borrow_utilization_bps = 80%
        let initial_deposit = 1_000_000;

        // Test reserve have decimals = 9
        let mut reserve = test_reserve(initial_deposit);

        reserve
            .deposit_liquidity(18_000_000_000_000_000_000)
            .unwrap();
        // 18_446_744_073_709_551_615 u64::MAX

        // Prepare test reserve with decimals = 6.
        let mut reserve = test_reserve(initial_deposit);
        reserve.liquidity.mint_decimals = 6;
        reserve.config.max_total_liquidity = u64::MAX;
        reserve.deposit_liquidity(18_000_000_000_000_000).unwrap(); // 18_000_000 billions

        // Prepare test reserve with decimals = 5. BONK
        let mut reserve = test_reserve(initial_deposit);
        reserve.liquidity.mint_decimals = 5;
        reserve.config.max_total_liquidity = u64::MAX;
        reserve.deposit_liquidity(1_800_000_000_000_000).unwrap(); // 18000000000 billions
    }
}
