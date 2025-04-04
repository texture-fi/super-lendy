use crate::state::curator::CuratorParams;
use borsh::{BorshDeserialize, BorshSerialize};
use texture_common::macros::Instruction;

use crate::state::pool::PoolParams;
use crate::state::position::{BORROW_MEMO_LEN, COLLATERAL_MEMO_LEN};
use crate::state::reserve::{ConfigProposal, ReserveConfig, RewardRules};
use crate::state::texture_cfg::TextureConfigParams;
use crate::SUPER_LENDY_ID;

#[cfg(not(feature = "with-processor"))]
use crate::price_proxy;

#[derive(Instruction, BorshSerialize, BorshDeserialize, Debug)]
#[instruction(
    out_dir = "src/instruction",
    out_mod = "generated",
    program_id = SUPER_LENDY_ID,
    docs_module = ix_docs,
)]
pub enum SuperLendyInstruction {
    // 0
    /// Create TextureConfig account
    ///
    #[doc = ix_docs::create_texture_config!()]
    #[accounts(
        account(
            docs = [
                "Global config account to create. With uninitialized data.",
                "Ownership must be already assigned to SuperLendy.",
            ],
            name = "texture_config",
            flags(writable, signer),
            checks(owner = "self", exempt),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
        account(
            docs = ["Config owner. Will fund Config account."],
            name = "owner",
            flags(writable, signer)
        ),
    )]
    CreateTextureConfig { params: TextureConfigParams },

    // 1
    /// Change GlobalConfig account (except owner)
    ///
    #[doc = ix_docs::alter_texture_config!()]
    #[accounts(
        account(
            docs = ["Global config account to change."],
            name = "texture_config",
            flags(writable),
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
        account(
            docs = ["Global config owner"],
            name = "owner",
            flags(writable, signer),
        ),
    )]
    AlterTextureConfig { params: TextureConfigParams },

    // 2
    /// Create Curator account. Texture config owner must sign.
    ///
    #[doc = ix_docs::create_curator!()]
    #[accounts(
        account(
            docs = ["Curator account to create."],
            name = "curator",
            flags(writable, signer),
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Global Texture config account."],
            name = "texture_config",
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
        account(
            docs = ["Global config owner"],
            name = "global_config_owner",
            flags(writable, signer),
        ),
    )]
    CreateCurator { params: CuratorParams },

    // 3
    /// Create Curator account
    ///
    #[doc = ix_docs::alter_curator!()]
    #[accounts(
        account(
            docs = ["Curator account to change."],
            name = "curator",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Owner of the Curator account."],
            name = "owner",
            flags(signer),
        )
    )]
    AlterCurator { params: CuratorParams },

    // 4
    /// Create Pool account
    ///
    #[doc = ix_docs::create_pool!()]
    #[accounts(
        account(
            docs = [
                "Pool account to create. With uninitialized data.",
                "Ownership must be already assigned to SuperLendy.",
            ],
            name = "pool",
            flags(writable, signer),
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Pools authority configured in `curator` account. Will fund Pool account."],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
    )]
    CreatePool { params: PoolParams },

    // 5
    /// Change existing Pool account
    ///
    #[doc = ix_docs::alter_pool!()]
    #[accounts(
        account(
            docs = ["Pool account to alter"],
            name = "pool",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Pools authority configured in `curator` account. Will fund Pool account."],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
    )]
    AlterPool { params: PoolParams },

    // 6
    /// Creates Reserve.
    ///
    #[doc = ix_docs::create_reserve!()]
    #[accounts(
        account(
            docs = [
                "Reserve account to create. With uninitialized data.",
                "Ownership must be already assigned to SuperLendy.",
            ],
            name = "reserve",
            flags(writable, signer),
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Pool - parent for created Reserve."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = [
                "Authority who can add new reserves in to a pool.",
                "Will fund Reserve account.",
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Liquidity mint of the Reserve"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["Liquidity supply SPL Token wallet. Not initialized. PDA."],
            name = "liquidity_supply",
            flags(writable),
            checks(owner = "system"),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Liquidity provider tokens mint of the Reserve. Not initialized. PDA."],
            name = "lp_mint",
            flags(writable),
            checks(owner = "system"),
            pda_seeds = [reserve, crate::pda::LP_TOKEN_SEED],
        ),
        account(
            docs = ["Collateral supply SPL Token wallet. Not initialized. PDA."],
            name = "collateral_supply",
            flags(writable),
            checks(owner = "system"),
            pda_seeds = [reserve, crate::pda::COLLATERAL_SUPPLY_SEED],
        ),
        account(
            docs = ["Price feed account to get market price for liquidity currency."],
            name = "market_price_feed",
            checks(owner = price_proxy::ID),
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program - classic one"],
            name = "lp_token_program",
            id = spl_token::ID,
        ),
        program(
            docs = ["SPL Token program to manage liquidity tokens. Either classic or 2022"],
            name = "liquidity_token_program",
        ),
        program(
            docs = ["System Program."],
            id = "system",
        ),
    )]
    CreateReserve {
        params: ReserveConfig,
        reserve_type: u8,
    },

    // 7
    /// Change existing reserve.
    ///
    #[doc = ix_docs::alter_reserve!()]
    #[accounts(
        account(
            docs = ["Reserve change."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Pool - parent for created Reserve."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Price feed account to get market price for liquidity currency."],
            name = "market_price_feed",
            checks(owner = price_proxy::ID),
        ),
        account(
            docs = [
            "Authority who can configure reserves."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Global config account"],
            name = "texture_config",
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
    )]
    AlterReserve {
        params: ReserveConfig,
        mode: u8,
        flash_loans_enabled: u8,
    },

    // 8
    /// Accrue interest and update market price of liquidity on a reserve.
    ///
    #[doc = ix_docs::refresh_reserve!()]
    #[accounts(
        account(
            docs = ["Reserve account to refresh."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Price feed account to get market price for liquidity currency."],
            name = "market_price_feed",
            checks(owner = price_proxy::ID),
        ),
        account(
            docs = ["Interest Rate Model account."],
            name = "irm",
            checks(owner = curvy::ID),
        ),
        account(
            docs = ["Global config account"],
            name = "texture_config",
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
    )]
    RefreshReserve,

    // 9
    /// Deposit liquidity in to reserve
    ///
    #[doc = ix_docs::deposit_liquidity!()]
    #[accounts(
        account(
            docs = ["Owner of the source_liquidity_wallet"],
            name = "authority",
            flags(signer),
        ),
        account(
            docs = ["Source SPL Token wallet to transfer liquidity from."],
            name = "source_liquidity_wallet",
            flags(writable),
        ),
        account(
            docs = ["SPL Token wallet to receive LP tokens minted during deposit."],
            name = "destination_lp_wallet",
            flags(writable),
        ),
        account(
            docs = ["Reserve account to deposit to. Must be refreshed beforehand."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Destination SPL Token wallet controlled by contract which will receive the liquidity. PDA."],
            name = "liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Liquidity tokens mint"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["LP tokens mint. PDA."],
            name = "lp_mint",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LP_TOKEN_SEED],
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "lp_token_program",
            id = spl_token::ID,
        ),
        program(
            docs = ["SPL Token program - either classic or 2022"],
            name = "liquidity_token_program",
        ),
    )]
    DepositLiquidity {
        /// amount of liquidity token to deposit
        amount: u64,
    },

    // 10
    /// Withdraw liquidity from reserve
    ///
    #[doc = ix_docs::withdraw_liquidity!()]
    #[accounts(
        account(
            docs = ["Owner of the source_lp_wallet"],
            name = "authority",
            flags(signer),
        ),
        account(
            docs = ["Source SPL Token wallet to transfer LP tokens from."],
            name = "source_lp_wallet",
            flags(writable),
        ),
        account(
            docs = ["SPL Token wallet to receive liquidity."],
            name = "destination_liquidity_wallet",
            flags(writable),
        ),
        account(
            docs = ["Reserve account to withdraw from. Must be refreshed beforehand."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["SPL Token wallet controlled by contract which will give the liquidity. PDA."],
            name = "liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Liquidity tokens mint"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["LP tokens mint. PDA."],
            name = "lp_mint",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LP_TOKEN_SEED],
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "lp_token_program",
            id = spl_token::ID,
        ),
        program(
            docs = ["SPL Token program - either classic or 2022"],
            name = "liquidity_token_program",
        ),
    )]
    /// `lp_amount` - amount of LP tokens to change for liquidity and withdraw it,
    /// When u64::max is passed the contract will use ALL LP tokens from provided wallet.
    WithdrawLiquidity { lp_amount: u64 },

    // 11
    /// Create new user position
    ///
    #[doc = ix_docs::create_position!()]
    #[accounts(
        account(
            docs = ["Position account to initialize. Allocated and owned by SuperLendy. Not initialized yet."],
            name = "position",
            flags(writable, signer),
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Pool the position will belong to."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Owner of the position"],
            name = "owner",
            flags(writable, signer),
        ),
    )]
    CreatePosition {
        /// Position type: POSITION_TYPE_CLASSIC or POSITION_TYPE_TRADING
        position_type: u8,
    },

    // 12
    /// Close user position, delete account and return Rent to the user.
    /// Only "empty" positions can be closed i.e.:
    /// 1. No locked collateral
    /// 2. No borrowings
    /// 3. No accrued rewards
    ///
    #[doc = ix_docs::close_position!()]
    #[accounts(
        account(
            docs = ["Position account to close."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Owner of the position"],
            name = "owner",
            flags(writable, signer),
        ),
    )]
    ClosePosition,

    // 13
    /// Refresh existing user position. Requires refreshed reserves (all deposits and borrowings).
    ///
    #[doc = ix_docs::refresh_position!()]
    #[accounts(
        account(
            docs = ["Position account."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        list(
            docs = [
                "Collateral deposit reserve accounts - refreshed,",
                "all in same order as listed in Position.deposits",
            ],
            name = "deposits",
            checks(owner = "self"),
            count_field = deposit_count,
        ),
        list(
            docs = [
                "Liquidity borrow reserve accounts - refreshed,",
                "all in same order as listed in Position.borrows",
            ],
            name = "borrows",
            checks(owner = "self"),
            count_field = borrow_count,
        ),
    )]
    RefreshPosition {
        #[instruction_builder(internal)]
        deposit_count: u8,
        #[instruction_builder(internal)]
        borrow_count: u8,
    },

    // 14
    /// Lock LP tokens as collateral
    ///
    #[doc = ix_docs::lock_collateral!()]
    #[accounts(
        account(
            docs = ["Position account to lock collateral in."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["User's SPL token wallet which holds LP tokens to be locked as collateral"],
            name = "source_lp_wallet",
            flags(writable),
        ),
        account(
            docs = ["Contract managed SPL token wallet to hold locked LP tokens. PDA."],
            name = "reserve_collateral_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::COLLATERAL_SUPPLY_SEED],
        ),
        account(
            docs = ["Position owner and also authority for source_lp_wallet"],
            name = "owner",
            flags(signer),
        ),
        account(
            docs = ["Reserve account which is the source of LP tokens being deposited. Refreshed."],
            name = "reserve",
            checks(owner = "self"),
        ),
        program(
            docs = ["SPL Token program"],
            name = "lp_token_program",
            id = spl_token::ID,
        ),
    )]
    LockCollateral {
        /// Amount of LP tokens user wants to clock. When u64::MAX is passed - all LPs from provided wallet
        /// will be locked.
        amount: u64,
        /// Arbitrary bytes where caller can store any data along with that Collateral. Later the data
        /// can be read from position.collateral record corresponding to that Collateral. Subsequent calls
        /// of LockCollateral will override that data.
        memo: [u8; COLLATERAL_MEMO_LEN],
    },

    // 15
    /// Unlock and withdraw LP tokens from pool.
    ///
    #[doc = ix_docs::unlock_collateral!()]
    #[accounts(
        account(
            docs = ["Position account to unlock collateral from"],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet which holds locked LP tokens. PDA."],
            name = "reserve_collateral_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::COLLATERAL_SUPPLY_SEED],
        ),
        account(
            docs = ["User's SPL token wallet which will receive unlocked LP tokens"],
            name = "destination_lp_wallet",
            flags(writable),
        ),
        account(
            docs = ["Position owner"],
            name = "owner",
            flags(signer),
        ),
        account(
            docs = ["Reserve account which is the source of LP tokens being deposited. Refreshed."],
            name = "reserve",
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "lp_token_program",
            id = spl_token::ID,
        ),
    )]
    UnlockCollateral {
        /// Amount of LP tokens user wants to unlock.
        /// When u64::MAX is passed - maximum possible amount of LPs from user's position will be unlocked.
        amount: u64,
    },

    // 16
    /// Borrow liquidity from the pool
    ///
    #[doc = ix_docs::borrow!()]
    #[accounts(
        account(
            docs = ["Borrowers Position account. Refreshed."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet which holds liquidity. PDA."],
            name = "reserve_liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["User's SPL token wallet which will receive borrowed liquidity tokens"],
            name = "destination_liquidity_wallet",
            flags(writable),
        ),
        account(
            docs = ["SPL token wallet which will receive loan origination fee. ATA from curator.fee_authority"],
            name = "curator_fee_receiver",
            flags(writable),
        ),
        account(
            docs = ["Position owner who borrow"],
            name = "borrower",
            flags(signer),
        ),
        account(
            docs = ["Reserve account which is the source of LP tokens being deposited. Refreshed."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Pool borrow happens in."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Curator of the pool."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["SPL token wallet which will receive loan origination fee. Must be ATA from GlobalConfig.fees_authority"],
            name = "texture_fee_receiver",
            flags(writable),
        ),
        account(
            docs = ["Global config account"],
            name = "texture_config",
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
        account(
            docs = ["Liquidity tokens mint."],
            name = "liquidity_mint",
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program - either classic or 2022"],
            name = "token_program",
        ),
    )]
    Borrow {
        /// Amount of liquidity to borrow.
        /// u64::MAX - uses 100% of user's borrowing power taking in to account Reserve's liquidity
        /// limitations.
        amount: u64,
        /// Minimum amount of liquidity to receive, if borrowing at 100% of borrowing power
        slippage_limit: u64,
        /// Arbitrary bytes where caller can store any data along with that Borrow. Later the data
        /// can be read from position.borrows record corresponding to that Borrow. Subsequent calls
        /// of Borrow for the same reserve will override that data.
        memo: [u8; BORROW_MEMO_LEN],
    },

    // 17
    /// Repay existing loan.
    ///
    #[doc = ix_docs::repay!()]
    #[accounts(
        account(
            docs = ["Borrowers Position account. Refreshed."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["User's SPL token wallet with liquidity tokens to be used as repayment"],
            name = "source_liquidity_wallet",
            flags(writable),
        ),
        account(
            docs = ["Contract managed SPL token wallet to return liquidity to. PDA."],
            name = "reserve_liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Authority to transfer funds from `source_liquidity_wallet`"],
            name = "user_authority",
            flags(signer),
        ),
        account(
            docs = ["Reserve account which is the source of LP tokens being deposited. Refreshed."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Liquidity tokens mint."],
            name = "liquidity_mint",
        ),
        program(
            docs = ["SPL Token program - either classic or 2022"],
            name = "token_program",
        ),
    )]
    Repay {
        /// amount of principal token to repay. Set to u64::MAX to repay all borrowed amount.
        amount: u64,
    },

    // 18
    /// Writes off bad debt for particular unhealthy Position.
    ///
    #[doc = ix_docs::write_off_bad_debt!()]
    #[accounts(
        account(
            docs = ["Pool to which bad debt position belongs"],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Reserve account to write off bad debt in. Refreshed."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Unhealthy Position account. Refreshed."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = [
            "Authority who can write-off bad debt from the reserve."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
    )]
    WriteOffBadDebt {
        /// principal (borrowed as bad debt) token amount to write off
        amount: u64,
    },

    // 19
    /// Repay borrowed liquidity to a reserve to receive collateral at a
    /// discount from an unhealthy Position. Requires a refreshed
    /// position and reserves.
    ///
    #[doc = ix_docs::liquidate!()]
    #[accounts(
        account(
            docs = ["SPL token wallet to get repayment liquidity from."],
            name = "repayment_source_wallet",
            flags(writable),
        ),
        account(
            docs = ["SPL token wallet to receive LP tokens (released collateral of the liquidated Position)"],
            name = "destination_lp_wallet",
            flags(writable),
        ),
        account(
            docs = ["Reserve account to repay principal tokens owed by unhealthy Position. Refreshed."],
            name = "principal_reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet to return principal liquidity to. PDA."],
            name = "principal_reserve_liquidity_supply",
            flags(writable),
            pda_seeds = [principal_reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Reserve account to repay principal tokens owed by unhealthy Position. Refreshed."],
            name = "collateral_reserve",
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet which holds locked LP tokens. PDA."],
            name = "collateral_reserve_lp_supply",
            flags(writable),
            pda_seeds = [collateral_reserve, crate::pda::COLLATERAL_SUPPLY_SEED],
        ),
        account(
            docs = ["Borrower Position account. Refreshed."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Liquidator's authority which controls `repayment_source_wallet`"],
            name = "liquidator",
            flags(signer),
        ),
        account(
            docs = ["Liquidity tokens mint in principal Reserve."],
            name = "principal_reserve_liquidity_mint",
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program used to manage principal tokens"],
            name = "principal_token_program",
        ),
        program(
            docs = ["SPL Token program used to manage collateral tokens (LPs) - always classic SPL Token"],
            name = "collateral_token_program",
            id = spl_token::ID,
        ),
    )]
    Liquidate {
        /// liquidity amount to repay
        liquidity_amount: u64,
    },

    // 20
    /// Permissionless IX to transfer accrued Curators's performance fees on to ATA of
    /// [Curator.fees_authority]
    ///
    #[doc = ix_docs::claim_curator_performance_fees!()]
    #[accounts(
        account(
            docs = ["Reserve account to claim performance fees from. Refreshed."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet with Reserve's liquidity. PDA."],
            name = "reserve_liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Pool."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["SPL token wallet to receive claimed fees. Must be ATA from curator.fees_authority."],
            name = "fee_receiver",
            flags(writable),
        ),
        account(
            docs = ["Liquidity tokens mint"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
    )]
    ClaimCuratorPerformanceFees,

    // 21
    /// Permissionless IX to transfer accrued Texture's performance fees on to ATA of
    /// [TextureConfig.fees_authority]
    ///
    #[doc = ix_docs::claim_texture_performance_fees!()]
    #[accounts(
        account(
            docs = ["Reserve account to claim performance fees from. Refreshed."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet with Reserve's liquidity. PDA."],
            name = "reserve_liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["SPL token wallet to receive claimed fees. Must be ATA from [TextureConfig.fees_authority]"],
            name = "fee_receiver",
            flags(writable),
        ),
        account(
            docs = ["Global config account"],
            name = "texture_config",
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
        account(
            docs = ["Liquidity tokens mint"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
    )]
    ClaimTexturePerformanceFees,

    // 22
    /// Initialize contract controlled SPL wallet for particular reward token.
    ///
    #[doc = ix_docs::init_reward_supply!()]
    #[accounts(
        account(
            docs = ["Reward supply account to initialize. Uninitialized. PDA"],
            name = "reward_supply",
            flags(writable),
            checks(owner = "system"),
            pda_seeds = [pool, reward_mint, crate::pda::REWARD_SUPPLY_SEED],
        ),
        account(
            docs = ["Reward token mint."],
            name = "reward_mint",
        ),
        account(
            docs = ["Pool to init reward supply for."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = [
            "Authority who can manage pool."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Contract's reward authority. PDA."],
            name = "reward_authority",
            pda_seeds = [pool, crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
        program(
            docs = ["System Program."],
            id = "system",
        ),
    )]
    InitRewardSupply,

    // 23
    /// Set/change reward rule in the existing reserve.
    ///
    #[doc = ix_docs::set_reward_rules!()]
    #[accounts(
        account(
            docs = ["Reserve account to set reward rules for"],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Pool - parent for created Reserve."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = [
            "Authority who can configure reserves."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        list(
            docs = [
            "Reward mint accounts - all in order as in `rules`"
            ],
            name = "reward_mints",
            count_field = mints_count,
        ),
    )]
    SetRewardRules {
        #[instruction_builder(internal)]
        mints_count: u8,
        rules: RewardRules,
    },

    // 24
    /// Claim reward tokens.
    ///
    #[doc = ix_docs::claim_reward!()]
    #[accounts(
        account(
            docs = ["Position account to claim rewords for. Refreshed."],
            name = "position",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet which holds reward tokens. PDA."],
            name = "rewards_supply",
            flags(writable),
            pda_seeds = [pool, reward_mint, crate::pda::REWARD_SUPPLY_SEED],
        ),
        account(
            docs = ["User's SPL token wallet which will receive reward tokens"],
            name = "destination_wallet",
            flags(writable),
        ),
        account(
            docs = ["Position owner"],
            name = "position_owner",
            flags(signer),
        ),
        account(
            docs = ["Pool, position belongs to."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Reward token mint to claim. Determines which reward will be claimed from the Position."],
            name = "reward_mint",
        ),
        account(
            docs = ["Contract's reward authority. PDA."],
            name = "reward_authority",
            pda_seeds = [pool, crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
    )]
    ClaimReward,

    // 25
    /// Withdraw reward tokens from reward supply account
    /// Contract doesn't have DepositReward IX as it can be done by SplToken directly.
    ///
    #[doc = ix_docs::withdraw_reward!()]
    #[accounts(
        account(
            docs = ["Contract managed SPL token wallet which holds reward tokens. PDA."],
            name = "rewards_supply",
            flags(writable),
            pda_seeds = [pool, reward_mint, crate::pda::REWARD_SUPPLY_SEED],
        ),
        account(
            docs = ["SPL token account of reward_mint to receive reward tokens."],
            name = "destination_wallet",
            flags(writable),
        ),
        account(
            docs = ["Pool to withdraw rewards from."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = [
            "Authority who can configure reserves."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Reward token mint to withdraw."],
            name = "reward_mint",
        ),
        account(
            docs = ["Contract's reward authority. PDA."],
            name = "reward_authority",
            pda_seeds = [pool, crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
    )]
    WithdrawReward { amount: u64 },

    // 26
    /// Flash borrow reserve liquidity
    #[doc = ix_docs::flash_borrow!()]
    #[accounts(
        account(
            docs = ["Reserve account to flash-borrow from."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet which holds Reserve's liquidity tokens. PDA."],
            name = "liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["SPL token account to receive flash-borrowed tokens."],
            name = "destination_wallet",
            flags(writable),
        ),
        account(
            docs = ["Liquidity tokens mint"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        account(
            docs = ["Sysvar instructions account"],
            name = "sysvar_instructions",
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
    )]
    FlashBorrow {
        /// Amount of liquidity to flash borrow
        amount: u64,
    },

    // 27
    /// Flash repay reserve liquidity
    #[doc = ix_docs::flash_repay!()]
    #[accounts(
        account(
            docs = ["SPL token account to transfer tokens for repayment from."],
            name = "source_wallet",
            flags(writable),
        ),
        account(
            docs = ["Reserve account to flash-repay to."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Contract managed SPL token wallet which holds Reserve's liquidity tokens. PDA."],
            name = "liquidity_supply",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LIQUIDITY_SUPPLY_SEED],
        ),
        account(
            docs = ["Liquidity tokens mint"],
            name = "liquidity_mint",
        ),
        account(
            docs = ["Authority to transfer funds from source_wallet."],
            name = "user_transfer_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Sysvar instructions account"],
            name = "sysvar_instructions",
        ),
        program(
            docs = ["SPL Token program"],
            name = "token_program",
        ),
    )]
    FlashRepay {
        /// Amount of liquidity to flash repay. Must be the same as in paired FlashBorrow IX.
        amount: u64,
    },

    // 28
    /// Add/replace request for configuration change. Caller must specify exact index of the request.
    /// The content of specified request entry will be reset to the one specified along this IX.
    /// This IX also allow to deactivate already submitted request. To do this one must supply
    /// default (all zeroed) ReserveConfig.
    ///
    #[doc = ix_docs::propose_config!()]
    #[accounts(
        account(
            docs = ["Reserve to propose new config for"],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Pool - parent for created Reserve."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Price feed account to get market price for liquidity currency."],
            name = "market_price_feed",
            checks(owner = price_proxy::ID),
        ),
        account(
            docs = [
            "Authority who can configure reserves."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Global config account"],
            name = "texture_config",
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
    )]
    ProposeConfig { index: u8, proposal: ConfigProposal },

    // 29
    /// Applies (i.e. replaces current Reserve config from the one stored in proposal). New config is
    /// validated in a usual way. Also
    ///
    #[doc = ix_docs::apply_config_proposal!()]
    #[accounts(
        account(
            docs = ["Reserve to apply config proposal for"],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = ["Pool - parent for created Reserve."],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Price feed account to get market price for liquidity currency."],
            name = "market_price_feed",
            checks(owner = price_proxy::ID),
        ),
        account(
            docs = [
            "Authority who can configure reserves."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
    )]
    ApplyConfigProposal { index: u8 },

    // 30
    /// Delete existing reserve. Only reserve with 0 deposits and borrowings can be deleted.
    ///
    #[doc = ix_docs::delete_reserve!()]
    #[accounts(
        account(
            docs = ["Reserve to delete."],
            name = "reserve",
            flags(writable),
            checks(owner = "self"),
        ),
        account(
            docs = [
            "Authority who can configure reserves. He will reserve freed rent."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Pool - Reserve belongs to."],
            name = "pool",
            checks(owner = "self"),
        )
    )]
    DeleteReserve,

    // 31
    /// Change GlobalConfig account owner
    ///
    #[doc = ix_docs::transfer_texture_config_ownership!()]
    #[accounts(
        account(
            docs = ["Global config account."],
            name = "texture_config",
            flags(writable),
            checks(owner = "self"),
            addr = crate::TEXTURE_CONFIG_ID,
        ),
        account(
            docs = ["Current global config owner"],
            name = "owner",
            flags(writable, signer),
        ),
        account(
            docs = ["New global config owner"],
            name = "new_owner",
            flags(writable, signer),
        ),
    )]
    TransferTextureConfigOwnership,

    // 32
    /// When called with `no_error` = false the IX fails but prints contact version in to returned logs
    /// When called with `no_error` = true just do nothing and can be used to increase compute budget for TX.
    #[doc = ix_docs::version!()]
    #[accounts(
        program(
            docs = ["System Program."],
            id = "system",
        ),
    )]
    Version { no_error: bool },

    // 33
    /// Set metadata for LP token of particular Reserve.
    /// When metadata doesn't exist - creates it.
    /// When metadata already exists - updates one.
    #[doc = ix_docs::set_lp_metadata!()]
    #[accounts(
        account(
            docs = ["Reserve to set LP metadata for"],
            name = "reserve",
            checks(owner = "self"),
        ),
        account(
            docs = ["LP tokens mint. PDA."],
            name = "lp_mint",
            flags(writable),
            pda_seeds = [reserve, crate::pda::LP_TOKEN_SEED],
        ),
        account(
            docs = ["Pool - parent for Reserve"],
            name = "pool",
            checks(owner = "self"),
        ),
        account(
            docs = ["Metadata account. PDA."],
            name = "metadata_account",
            flags(writable),
        ),
        account(
            docs = [
            "Authority who can configure reserves."
            ],
            name = "curator_pools_authority",
            flags(writable, signer),
        ),
        account(
            docs = ["Curator account."],
            name = "curator",
            checks(owner = "self", exempt),
        ),
        account(
            docs = ["Contract's authority. PDA."],
            name = "program_authority",
            pda_seeds = [crate::pda::AUTHORITY_SEED],
        ),
        program(
            docs = ["Metaplex token metadata Program."],
            name = "mpl_token_metadata_program",
            id = mpl_token_metadata::ID,
        ),
        program(
            docs = ["System Program."],
            id = "system",
        ),
        account(
            docs = ["Sysvar rent account"],
            name = "sysvar_rent",
        ),
    )]
    SetLpMetadata { metadata: LpTokenMetadata },
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct LpTokenMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}
