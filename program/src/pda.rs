//! Helper functions for finding derived addresses to entities.
use solana_program::pubkey::Pubkey;

use crate::SUPER_LENDY_ID;

pub const LP_TOKEN_SEED: &[u8] = b"LP_TOKEN";
pub const LIQUIDITY_SUPPLY_SEED: &[u8] = b"LIQUIDITY_SUPPLY";
pub const COLLATERAL_SUPPLY_SEED: &[u8] = b"COLLATERAL_SUPPLY";
pub const POSITION_SEED: &[u8] = b"POSITION";

pub const AUTHORITY_SEED: &[u8] = b"AUTHORITY";

pub const REWARD_SUPPLY_SEED: &[u8] = b"REWARD_SUPPLY";

/// LP token mints are unique for each Reserve
pub fn find_lp_token_mint(reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[reserve.as_ref(), LP_TOKEN_SEED], &SUPER_LENDY_ID)
}

/// Liquidity supply - is the wallet where Reserve stores its liquidity tokens. It is unique for each Reserve.
pub fn find_liquidity_supply(reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[reserve.as_ref(), LIQUIDITY_SUPPLY_SEED], &SUPER_LENDY_ID)
}

/// Collateral supply - is the wallet where Reserve stores deposited LP tokens of the corresponding
/// Reserve's LP mint.
pub fn find_collateral_supply(reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[reserve.as_ref(), COLLATERAL_SUPPLY_SEED], &SUPER_LENDY_ID)
}

// Program authority is the owner of SPL Token wallets used to fund offers.
pub fn find_program_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[AUTHORITY_SEED], &SUPER_LENDY_ID)
}

/// Reward token supply - is the wallet where Rewards tokens stored by the contract.
/// Reward wallets are global for the Pool.
pub fn find_reward_supply(pool: &Pubkey, reward_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[pool.as_ref(), reward_mint.as_ref(), REWARD_SUPPLY_SEED],
        &SUPER_LENDY_ID,
    )
}

/// Program (contract) authority used in rewards operations (e.g. to transfer rewards to users).
pub fn find_rewards_program_authority(pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[pool.as_ref(), AUTHORITY_SEED], &SUPER_LENDY_ID)
}
