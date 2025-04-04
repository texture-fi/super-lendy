use borsh::io::Error as BorshIoError;
use bytemuck::PodCastError;
use solana_program::program_error::ProgramError;
use solana_program::program_error::ProgramError::Custom;
use solana_program::pubkey::{Pubkey, PubkeyError};
use solana_program::system_instruction::SystemError;
use spl_token::error::TokenError;
use texture_common::account;
use texture_common::error;
use texture_common::math::{Decimal, MathError};
use texture_common::remote::RemoteError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerializeError {
    #[error("borsh: {0}")]
    Borsh(#[from] BorshIoError),
    #[error("not enough data")]
    NotEnoughData,
    #[error("not enough space")]
    NotEnoughSpace,
    #[error("too much space")]
    TooMuchSpace,
    #[error("version mismatch: {actual} != {expected}")]
    VersionMismatch { expected: u8, actual: u8 },
    #[error("bytemuck: {0}")]
    Bytemuck(#[from] PodCastError),
    #[error("math: {0}")]
    Math(#[from] MathError),
    #[error("reinitialization attempt")]
    Reinit,
    #[error("uninitialized data")]
    Uninit,
    #[error("invalid data")]
    Invalid,
}

#[derive(Debug, Error)]
pub enum SuperLendyError {
    #[error("math error: {0}")]
    MathError(#[from] MathError),

    #[error("borsh error: {0}")]
    Borsh(#[from] BorshIoError),

    #[error("serialize error: {0}")]
    Serialize(#[from] SerializeError),

    #[error("pod account: {0}")]
    PodAccount(#[from] account::PodAccountError),

    #[error("pod account: {0}")]
    PodAccountExt(#[from] account::PodAccountErrorWithHeader),

    #[error(transparent)]
    InvalidKey(#[from] error::InvalidKey),

    #[error(transparent)]
    InvalidAccount(#[from] error::InvalidAccount),

    #[error(transparent)]
    NotEnoughAccountKeys(#[from] error::NotEnoughAccountKeys),

    #[error(transparent)]
    MissingSignature(#[from] error::MissingSignature),

    #[error("unimplemented")]
    Unimplemented,

    #[error("uninintialized account: {0}")]
    UninitializedAccount(Pubkey),

    #[error("address creation error: {0}")]
    AddressCreation(#[from] PubkeyError),

    #[error("error unpaking account {0} with error {1}")]
    AccountUnpackError(Pubkey, ProgramError),

    #[error("invalid config")]
    InvalidConfig,

    #[error("invalid amount")]
    InvalidAmount,

    #[error("internal logic error: {0}")]
    Internal(String),

    #[error("deserialized account contains unexpected values")]
    InvalidAccountData,

    #[error("requested operation can not be performed")]
    OperationCanNotBePerformed,

    #[error("provided deposited liquidity was not found for position")]
    DepositedCollateralNotFound,

    #[error("provided borrowed liquidity was not found for position")]
    BorrowedLiquidityNotFound,

    #[error("invalid realloc")]
    InvalidRealloc,

    #[error("owner specified doesn't match expected one")]
    OwnerMismatch,

    #[error("market price age {0} sec reached stale threshold {1} sec")]
    StaleMarketPrice(/*actual age*/ i64, /*threshold*/ u32),

    #[error("reserve was not refreshed ")]
    StaleReserve,

    #[error("borrow value is too large")]
    BorrowTooLarge,

    #[error("limited internal resource was exhausted")]
    ResourceExhausted,

    #[error("position was not refreshed ")]
    StalePosition,

    #[error("liquidation results in untransferrable amounts")]
    LiquidationTooSmall,

    #[error("position is healthy as its LTV {0} is less then threshold {1}")]
    AttemptToLiquidateHealthyPosition(
        /*position_ltv*/ Decimal,
        /*partly_unhealthy_ltv*/ Decimal,
    ),
    #[error("error while extracting IXes from Sysvar {0}")]
    SysvarError(ProgramError),

    #[error("metaplex error: {0}")]
    MetaplexError(ProgramError),

    // NaN
    #[error("spl-token error: {0}")]
    SplToken(#[from] RemoteError<TokenError>),

    #[error("system program error: {0}")]
    SystemProgram(#[from] RemoteError<SystemError>),
}

texture_common::from_account_parse_error!(SuperLendyError);

impl From<SuperLendyError> for ProgramError {
    fn from(error: SuperLendyError) -> Self {
        match error {
            SuperLendyError::MathError(..) => Custom(0),
            SuperLendyError::Borsh(..) => Custom(1),
            SuperLendyError::Serialize(..) => Custom(2),
            SuperLendyError::PodAccount(..) | SuperLendyError::PodAccountExt(..) => Custom(3),
            SuperLendyError::InvalidKey { .. } => Custom(4),
            SuperLendyError::InvalidAccount(..) => Custom(5),
            SuperLendyError::NotEnoughAccountKeys(..) => Custom(6),
            SuperLendyError::MissingSignature(..) => Custom(7),
            SuperLendyError::Unimplemented => Custom(8),
            SuperLendyError::UninitializedAccount(..) => Custom(9),
            SuperLendyError::AddressCreation(..) => Custom(10),
            SuperLendyError::AccountUnpackError(..) => Custom(11),
            SuperLendyError::InvalidConfig => Custom(12),
            SuperLendyError::InvalidAmount => Custom(13),
            SuperLendyError::Internal(..) => Custom(14),
            SuperLendyError::InvalidAccountData => Custom(15),
            SuperLendyError::OperationCanNotBePerformed => Custom(16),
            SuperLendyError::DepositedCollateralNotFound => Custom(17),
            SuperLendyError::InvalidRealloc => Custom(18),
            SuperLendyError::OwnerMismatch => Custom(19),
            SuperLendyError::StaleMarketPrice(..) => Custom(20),
            SuperLendyError::StaleReserve => Custom(21),
            SuperLendyError::BorrowTooLarge => Custom(22),
            SuperLendyError::ResourceExhausted => Custom(23),
            SuperLendyError::BorrowedLiquidityNotFound => Custom(24),
            SuperLendyError::StalePosition => Custom(25),
            SuperLendyError::LiquidationTooSmall => Custom(26),
            SuperLendyError::AttemptToLiquidateHealthyPosition(..) => Custom(27),
            SuperLendyError::SysvarError(..) => Custom(28),
            SuperLendyError::MetaplexError(..) => Custom(29),

            SuperLendyError::SplToken(err) => err.into(),
            SuperLendyError::SystemProgram(RemoteError::Unrecognized(err)) => err,
            SuperLendyError::SystemProgram(RemoteError::Recognized(err)) => Custom(err as u32),
        }
    }
}

texture_common::convert_remote_err!(
    system_err,
    texture_common::remote::system::SystemError,
    SuperLendyError
);

texture_common::convert_remote_err!(
    token_err,
    texture_common::remote::token::TokenError,
    SuperLendyError
);
