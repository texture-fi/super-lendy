//! State types

use solana_program::clock::{DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY};

pub mod curator;
pub mod last_update;
pub mod pool;
pub mod position;
pub mod reserve;
pub mod texture_cfg;

pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

/// Scale of precision
pub const SCALE: u32 = 18;
/// Identity
pub const WAD: u64 = 1_000_000_000_000_000_000;
/// Half of identity
pub const HALF_WAD: u64 = 500_000_000_000_000_000;
/// Scale for percentages
pub const PERCENT_SCALER: u64 = 10_000_000_000_000_000;
pub const INITIAL_COLLATERAL_RATIO: u64 = 1;
const INITIAL_COLLATERAL_RATE: u64 = INITIAL_COLLATERAL_RATIO * WAD;

pub const TEXTURE_CONFIG_DISCRIMINATOR: &[u8; 8] = b"TXT__CFG";

pub const POOL_DISCRIMINATOR: &[u8; 8] = b"POOL____";
pub const RESERVE_DISCRIMINATOR: &[u8; 8] = b"RESERVE_";
pub const POSITION_DISCRIMINATOR: &[u8; 8] = b"POSITION";
pub const CURATOR_DISCRIMINATOR: &[u8; 8] = b"CURATOR_";

/// Reward rule identifier
/// Can hold UUID for example
pub type RuleId = [u8; 16];
