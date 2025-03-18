use crate::error::SuperLendyError;
use crate::LendyResult;
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use solana_program::msg;
use solana_program::pubkey::Pubkey;
use texture_common::account::{PodAccount, PodAccountError};

use crate::state::TEXTURE_CONFIG_DISCRIMINATOR;

static_assertions::const_assert_eq!(TextureConfig::SIZE, std::mem::size_of::<TextureConfig>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<TextureConfig>() % 8);

/// This is global config which allows Texture to manage some Texture related
/// Super Lendy configuration in one place.
/// All pools/reserves take into account this config.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct TextureConfig {
    pub discriminator: [u8; 8],
    pub version: u8,

    /// Vacant to store mode/status flags
    pub _flags: [u8; 3],

    /// Percentage of (any) borrowed amount which will be paid to texture as loan origination fee
    pub borrow_fee_rate_bps: u16,

    /// Percentage of (any) pool's interest which will be paid to Texture as performance fee
    pub performance_fee_rate_bps: u16,

    /// Owner authority who can change this account
    pub owner: Pubkey,

    /// This is main wallet address (SOL holding, system program owned) who allowed to claim
    /// Texture performance fees. Also ATA accounts of this authority are used as fee receivers
    /// for borrow fees.
    pub fees_authority: Pubkey,

    pub reserve_timelock: ReserveTimelock,

    pub _padding: [u8; 32 * 8],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
pub struct TextureConfigParams {
    /// Percentage of (any) borrowed amount which will be paid to texture as loan origination fee
    pub borrow_fee_rate_bps: u16,

    /// Percentage of (any) pool's interest which will be paid to Texture as performance fee
    pub performance_fee_rate_bps: u16,

    /// This is main wallet address (SOL holding, system program owned) who allowed to claim
    /// Texture performance fees.
    pub fees_authority: Pubkey,

    pub reserve_timelock: ReserveTimelock,
}

impl PodAccount for TextureConfig {
    const DISCRIMINATOR: &'static [u8] = TEXTURE_CONFIG_DISCRIMINATOR;

    type Version = u8;

    const VERSION: Self::Version = 1;

    type InitParams = (TextureConfigParams, /*owner*/ Pubkey);

    type InitError = PodAccountError;

    fn discriminator(&self) -> &[u8] {
        &self.discriminator
    }

    fn version(&self) -> Self::Version {
        self.version
    }

    fn init_unckecked(
        &mut self,
        (params, owner_key): Self::InitParams,
    ) -> Result<(), Self::InitError> {
        let Self {
            discriminator,
            version,
            _flags,
            borrow_fee_rate_bps,
            performance_fee_rate_bps,
            owner,
            fees_authority,
            reserve_timelock,
            _padding,
        } = self;

        *discriminator = *TEXTURE_CONFIG_DISCRIMINATOR;
        *version = Self::VERSION;
        *owner = owner_key;
        *fees_authority = params.fees_authority;
        *performance_fee_rate_bps = params.performance_fee_rate_bps;
        *borrow_fee_rate_bps = params.borrow_fee_rate_bps;
        *reserve_timelock = params.reserve_timelock;
        *_padding = Zeroable::zeroed();
        *_flags = Zeroable::zeroed();

        Ok(())
    }
}

impl TextureConfigParams {
    pub fn validate(&self) -> LendyResult<()> {
        if self.borrow_fee_rate_bps > 4000 {
            msg!("Borrow fee rate must be in range [0, 40] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        if self.performance_fee_rate_bps > 4000 {
            msg!("Performance fee rate must be in range [0, 40] %");
            return Err(SuperLendyError::InvalidConfig);
        }

        self.reserve_timelock.validate()?;

        Ok(())
    }
}

/// Holds time lock values for changing Reserve config. All values are in seconds. When several
/// changes done simultaneously - the longest timelock setting will be applied.
/// 0 value means that change of that parameter can be applied immediately.
/// It is field-by-field corresponds to the ReserveConfig structure.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
#[repr(C)]
pub struct ReserveTimelock {
    pub market_price_feed_lock_sec: u32, // u32 gives max 49710 days of delay
    pub irm_lock_sec: u32,
    pub liquidation_bonus_lock_sec: u32,
    pub unhealthy_ltv_lock_sec: u32,
    pub partial_liquidation_factor_lock_sec: u32,
    pub max_total_liquidity_lock_sec: u32,
    pub max_borrow_ltv_lock_sec: u32,
    pub max_borrow_utilization_lock_sec: u32,
    pub price_stale_threshold_lock_sec: u32,
    pub max_withdraw_utilization_lock_sec: u32,
    pub fees_lock_sec: u32,
    pub _padding: u32,
}

const SECONDS_IN_TWO_WEEKS: u32 = 604800 * 2;
impl ReserveTimelock {
    pub fn validate(&self) -> LendyResult<()> {
        if self.market_price_feed_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.irm_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.liquidation_bonus_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.unhealthy_ltv_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.partial_liquidation_factor_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.max_total_liquidity_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.max_borrow_ltv_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.max_borrow_utilization_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.price_stale_threshold_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.max_withdraw_utilization_lock_sec > SECONDS_IN_TWO_WEEKS
            || self.fees_lock_sec > SECONDS_IN_TWO_WEEKS
        {
            msg!(
                "time lock can not be greater than two weeks: {} sec",
                SECONDS_IN_TWO_WEEKS
            );
            return Err(SuperLendyError::InvalidConfig);
        }
        Ok(())
    }
}
