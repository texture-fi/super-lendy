pub mod error;
pub mod instruction;
#[cfg(feature = "with-processor")]
pub mod processor;
pub mod state;

pub mod pda;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Symbolize max available amount (to borrow, liquidate, etc.). Recognized as special amount value
// by many IXes and internal functions. This is useful when User want's to operate on whole token amount.
pub const MAX_AMOUNT: u64 = u64::MAX;

pub use lender_id::ID as SUPER_LENDY_ID;
mod lender_id {
    solana_program::declare_id!("sUperbZBsdZa4s7pWPKQaQ2fRTesjKxupxagZ8FSgVi");
}

pub use texture_config_id::ID as TEXTURE_CONFIG_ID;
mod texture_config_id {
    solana_program::declare_id!("gLoBanTpd5VuvyCpYjvYNudFREwLqFy418fGuuXUJfX");
}

#[cfg(not(feature = "with-processor"))]
pub(crate) mod price_proxy {
    solana_program::declare_id!("priceEvKXX3KERsitDpmvujXfPFYesmEspw4kiC3ryF");
}

pub type LendyResult<T> = std::result::Result<T, error::SuperLendyError>;
