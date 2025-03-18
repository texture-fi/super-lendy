use crate::state::POOL_DISCRIMINATOR;
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;
use texture_common::account::{PodAccount, PodAccountError};

pub const POOL_NAME_MAX_LEN: usize = 128;
pub const CURRENCY_SYMBOL_MAX_LEN: usize = 16;

static_assertions::const_assert_eq!(Pool::SIZE, std::mem::size_of::<Pool>());
static_assertions::const_assert_eq!(0, std::mem::size_of::<Pool>() % 8);

/// This is multi-currency lend\borrow pool. This is grouping entity for Reserves.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Pool {
    pub discriminator: [u8; 8],
    pub version: u8,

    /// When Pool is created it is in non-visible state by default (0). After the pool is fully configured
    /// (e.g. all planned reserves are created) Curator toggles this field to 1 via AlterPool IX.
    /// When Pool is not visible - it is not shown in UI.
    pub visible: u8,

    /// Vacant to store mode/status flags
    pub _flags: [u8; 6],

    /// Address of Curator account this pool belongs to. `pools_authority` from the Curator account has rights
    /// to add and configure new Reserves in the Pool.
    pub curator: Pubkey,

    /// Pool name to show in UI
    pub name: [u8; POOL_NAME_MAX_LEN],

    /// Human-readable symbol of the currency used in all Reserves of that pool
    /// to express market prices and values.
    pub market_price_currency_symbol: [u8; CURRENCY_SYMBOL_MAX_LEN],

    pub _padding: [u8; 30 * 8],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
pub struct PoolParams {
    /// Name of the Market
    pub name: [u8; POOL_NAME_MAX_LEN],

    /// Market price currency symbol
    pub market_price_currency_symbol: [u8; CURRENCY_SYMBOL_MAX_LEN],

    pub visible: u8, // 0 or 1
}

impl PodAccount for Pool {
    const DISCRIMINATOR: &'static [u8] = POOL_DISCRIMINATOR;

    type Version = u8;

    const VERSION: Self::Version = 1;

    type InitParams = (/*params:*/ PoolParams, /*owner:*/ Pubkey);

    type InitError = PodAccountError;

    fn discriminator(&self) -> &[u8] {
        &self.discriminator
    }

    fn version(&self) -> Self::Version {
        self.version
    }

    fn init_unckecked(
        &mut self,
        (params, curator_key): Self::InitParams,
    ) -> Result<(), Self::InitError> {
        let Self {
            discriminator,
            version,
            visible,
            _flags,
            curator,
            name,
            market_price_currency_symbol,
            _padding,
        } = self;

        *discriminator = *POOL_DISCRIMINATOR;
        *version = Self::VERSION;
        *name = params.name;
        *market_price_currency_symbol = params.market_price_currency_symbol;
        *curator = curator_key;
        *visible = 0;
        *_padding = Zeroable::zeroed();
        *_flags = Zeroable::zeroed();

        Ok(())
    }
}
