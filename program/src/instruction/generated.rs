#![allow(unexpected_cfgs)]
use super::*;
///[SuperLendyInstruction::CreateTextureConfig] Builder struct
pub struct CreateTextureConfig {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Config owner. Will fund Config account.
    pub owner: solana_program::pubkey::Pubkey,
    pub params: TextureConfigParams,
}
impl CreateTextureConfig {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self { #[cfg(feature = "program-id-manually")] program_id, owner, params } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    crate::TEXTURE_CONFIG_ID,
                    true,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(owner, true)]);
        let ix = SuperLendyInstruction::CreateTextureConfig {
            params,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::AlterTextureConfig] Builder struct
pub struct AlterTextureConfig {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Global config owner
    pub owner: solana_program::pubkey::Pubkey,
    pub params: TextureConfigParams,
}
impl AlterTextureConfig {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self { #[cfg(feature = "program-id-manually")] program_id, owner, params } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(owner, true)]);
        let ix = SuperLendyInstruction::AlterTextureConfig {
            params,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::CreateCurator] Builder struct
pub struct CreateCurator {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Curator account to create.
    pub curator: solana_program::pubkey::Pubkey,
    ///Global config owner
    pub global_config_owner: solana_program::pubkey::Pubkey,
    pub params: CuratorParams,
}
impl CreateCurator {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            curator,
            global_config_owner,
            params,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(curator, true)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(global_config_owner, true),
            ]);
        let ix = SuperLendyInstruction::CreateCurator {
            params,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::AlterCurator] Builder struct
pub struct AlterCurator {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Curator account to change.
    pub curator: solana_program::pubkey::Pubkey,
    ///Owner of the Curator account.
    pub owner: solana_program::pubkey::Pubkey,
    pub params: CuratorParams,
}
impl AlterCurator {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            curator,
            owner,
            params,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(curator, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(owner, true),
            ]);
        let ix = SuperLendyInstruction::AlterCurator {
            params,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::CreatePool] Builder struct
pub struct CreatePool {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Pool account to create. With uninitialized data.
    ///Ownership must be already assigned to SuperLendy.
    pub pool: solana_program::pubkey::Pubkey,
    ///Pools authority configured in `curator` account. Will fund Pool account.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    pub params: PoolParams,
}
impl CreatePool {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            pool,
            curator_pools_authority,
            curator,
            params,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(pool, true)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        let ix = SuperLendyInstruction::CreatePool {
            params,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::AlterPool] Builder struct
pub struct AlterPool {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Pool account to alter
    pub pool: solana_program::pubkey::Pubkey,
    ///Pools authority configured in `curator` account. Will fund Pool account.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    pub params: PoolParams,
}
impl AlterPool {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            pool,
            curator_pools_authority,
            curator,
            params,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(pool, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        let ix = SuperLendyInstruction::AlterPool {
            params,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::CreateReserve] Builder struct
pub struct CreateReserve {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve account to create. With uninitialized data.
    ///Ownership must be already assigned to SuperLendy.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool - parent for created Reserve.
    pub pool: solana_program::pubkey::Pubkey,
    ///Authority who can add new reserves in to a pool.
    ///Will fund Reserve account.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///Liquidity mint of the Reserve
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: solana_program::pubkey::Pubkey,
    ///SPL Token program to manage liquidity tokens. Either classic or 2022
    pub liquidity_token_program: solana_program::pubkey::Pubkey,
    pub params: ReserveConfig,
    pub reserve_type: u8,
}
impl CreateReserve {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            curator_pools_authority,
            curator,
            liquidity_mint,
            market_price_feed,
            liquidity_token_program,
            params,
            reserve_type,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (lp_mint, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LP_TOKEN_SEED.as_ref(),
            ],
            &program_id,
        );
        let (collateral_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::COLLATERAL_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, true)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(liquidity_supply, false),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(lp_mint, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(collateral_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    market_price_feed,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    spl_token::ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_token_program,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::system_program::ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::CreateReserve {
            params,
            reserve_type,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::AlterReserve] Builder struct
pub struct AlterReserve {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve change.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool - parent for created Reserve.
    pub pool: solana_program::pubkey::Pubkey,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    pub params: ReserveConfig,
    pub mode: u8,
    pub flash_loans_enabled: u8,
}
impl AlterReserve {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
            params,
            mode,
            flash_loans_enabled,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    market_price_feed,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::AlterReserve {
            params,
            mode,
            flash_loans_enabled,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::RefreshReserve] Builder struct
pub struct RefreshReserve {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve account to refresh.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: solana_program::pubkey::Pubkey,
    ///Interest Rate Model account.
    pub irm: solana_program::pubkey::Pubkey,
}
impl RefreshReserve {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            market_price_feed,
            irm,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    market_price_feed,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(irm, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::RefreshReserve {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::DepositLiquidity] Builder struct
pub struct DepositLiquidity {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Owner of the source_liquidity_wallet
    pub authority: solana_program::pubkey::Pubkey,
    ///Source SPL Token wallet to transfer liquidity from.
    pub source_liquidity_wallet: solana_program::pubkey::Pubkey,
    ///SPL Token wallet to receive LP tokens minted during deposit.
    pub destination_lp_wallet: solana_program::pubkey::Pubkey,
    ///Reserve account to deposit to. Must be refreshed beforehand.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program - either classic or 2022
    pub liquidity_token_program: solana_program::pubkey::Pubkey,
    /// amount of liquidity token to deposit
    pub amount: u64,
}
impl DepositLiquidity {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            authority,
            source_liquidity_wallet,
            destination_lp_wallet,
            reserve,
            liquidity_mint,
            liquidity_token_program,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (lp_mint, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LP_TOKEN_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(authority, true),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    source_liquidity_wallet,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    destination_lp_wallet,
                    false,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(liquidity_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(lp_mint, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    spl_token::ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::DepositLiquidity {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::WithdrawLiquidity] Builder struct
pub struct WithdrawLiquidity {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Owner of the source_lp_wallet
    pub authority: solana_program::pubkey::Pubkey,
    ///Source SPL Token wallet to transfer LP tokens from.
    pub source_lp_wallet: solana_program::pubkey::Pubkey,
    ///SPL Token wallet to receive liquidity.
    pub destination_liquidity_wallet: solana_program::pubkey::Pubkey,
    ///Reserve account to withdraw from. Must be refreshed beforehand.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program - either classic or 2022
    pub liquidity_token_program: solana_program::pubkey::Pubkey,
    pub lp_amount: u64,
}
impl WithdrawLiquidity {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            authority,
            source_lp_wallet,
            destination_liquidity_wallet,
            reserve,
            liquidity_mint,
            liquidity_token_program,
            lp_amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (lp_mint, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LP_TOKEN_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(authority, true),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(source_lp_wallet, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    destination_liquidity_wallet,
                    false,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(liquidity_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(lp_mint, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    spl_token::ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::WithdrawLiquidity {
            lp_amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::CreatePosition] Builder struct
pub struct CreatePosition {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Position account to initialize. Allocated and owned by SuperLendy. Not initialized yet.
    pub position: solana_program::pubkey::Pubkey,
    ///Pool the position will belong to.
    pub pool: solana_program::pubkey::Pubkey,
    ///Owner of the position
    pub owner: solana_program::pubkey::Pubkey,
    /// Position type: POSITION_TYPE_CLASSIC or POSITION_TYPE_TRADING
    pub position_type: u8,
}
impl CreatePosition {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            pool,
            owner,
            position_type,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(position, true)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(owner, true)]);
        let ix = SuperLendyInstruction::CreatePosition {
            position_type,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::ClosePosition] Builder struct
pub struct ClosePosition {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Position account to close.
    pub position: solana_program::pubkey::Pubkey,
    ///Owner of the position
    pub owner: solana_program::pubkey::Pubkey,
}
impl ClosePosition {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            owner,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts.extend([solana_program::instruction::AccountMeta::new(owner, true)]);
        let ix = SuperLendyInstruction::ClosePosition {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::RefreshPosition] Builder struct
pub struct RefreshPosition {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Position account.
    pub position: solana_program::pubkey::Pubkey,
    ///Collateral deposit reserve accounts - refreshed,
    ///all in same order as listed in Position.deposits
    pub deposits: Vec<solana_program::pubkey::Pubkey>,
    ///Liquidity borrow reserve accounts - refreshed,
    ///all in same order as listed in Position.borrows
    pub borrows: Vec<solana_program::pubkey::Pubkey>,
}
impl RefreshPosition {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            deposits,
            borrows,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut deposit_count;
        #[allow(unused_mut)]
        let mut borrow_count;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend({
                let keys = {
                    deposit_count = deposits
                        .len()
                        .try_into()
                        .expect(
                            concat!("convert ", stringify!(deposits), " accounts length"),
                        );
                    &deposits
                };
                #[allow(clippy::into_iter_on_ref)]
                keys.into_iter()
                    .map(|addr| solana_program::instruction::AccountMeta::new_readonly(
                        *addr,
                        false,
                    ))
            });
        accounts
            .extend({
                let keys = {
                    borrow_count = borrows
                        .len()
                        .try_into()
                        .expect(
                            concat!("convert ", stringify!(borrows), " accounts length"),
                        );
                    &borrows
                };
                #[allow(clippy::into_iter_on_ref)]
                keys.into_iter()
                    .map(|addr| solana_program::instruction::AccountMeta::new_readonly(
                        *addr,
                        false,
                    ))
            });
        let ix = SuperLendyInstruction::RefreshPosition {
            deposit_count,
            borrow_count,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::LockCollateral] Builder struct
pub struct LockCollateral {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Position account to lock collateral in.
    pub position: solana_program::pubkey::Pubkey,
    ///User's SPL token wallet which holds LP tokens to be locked as collateral
    pub source_lp_wallet: solana_program::pubkey::Pubkey,
    ///Position owner and also authority for source_lp_wallet
    pub owner: solana_program::pubkey::Pubkey,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    /// Amount of LP tokens user wants to clock. When u64::MAX is passed - all LPs from provided wallet
    /// will be locked.
    pub amount: u64,
    /// Arbitrary bytes where caller can store any data along with that Collateral. Later the data
    /// can be read from position.collateral record corresponding to that Collateral. Subsequent calls
    /// of LockCollateral will override that data.
    pub memo: [u8; COLLATERAL_MEMO_LEN],
}
impl LockCollateral {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            source_lp_wallet,
            owner,
            reserve,
            amount,
            memo,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reserve_collateral_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::COLLATERAL_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(source_lp_wallet, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    reserve_collateral_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(owner, true),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(reserve, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    spl_token::ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::LockCollateral {
            amount,
            memo,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::UnlockCollateral] Builder struct
pub struct UnlockCollateral {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Position account to unlock collateral from
    pub position: solana_program::pubkey::Pubkey,
    ///User's SPL token wallet which will receive unlocked LP tokens
    pub destination_lp_wallet: solana_program::pubkey::Pubkey,
    ///Position owner
    pub owner: solana_program::pubkey::Pubkey,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    /// Amount of LP tokens user wants to unlock.
    /// When u64::MAX is passed - maximum possible amount of LPs from user's position will be unlocked.
    pub amount: u64,
}
impl UnlockCollateral {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            destination_lp_wallet,
            owner,
            reserve,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reserve_collateral_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::COLLATERAL_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    reserve_collateral_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    destination_lp_wallet,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(owner, true),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(reserve, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    spl_token::ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::UnlockCollateral {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::Borrow] Builder struct
pub struct Borrow {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Borrowers Position account. Refreshed.
    pub position: solana_program::pubkey::Pubkey,
    ///User's SPL token wallet which will receive borrowed liquidity tokens
    pub destination_liquidity_wallet: solana_program::pubkey::Pubkey,
    ///SPL token wallet which will receive loan origination fee. ATA from curator.fee_authority
    pub curator_fee_receiver: solana_program::pubkey::Pubkey,
    ///Position owner who borrow
    pub borrower: solana_program::pubkey::Pubkey,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool borrow happens in.
    pub pool: solana_program::pubkey::Pubkey,
    ///Curator of the pool.
    pub curator: solana_program::pubkey::Pubkey,
    ///SPL token wallet which will receive loan origination fee. Must be ATA from GlobalConfig.fees_authority
    pub texture_fee_receiver: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint.
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program - either classic or 2022
    pub token_program: solana_program::pubkey::Pubkey,
    /// Amount of liquidity to borrow.
    /// u64::MAX - uses 100% of user's borrowing power taking in to account Reserve's liquidity
    /// limitations.
    pub amount: u64,
    /// Minimum amount of liquidity to receive, if borrowing at 100% of borrowing power
    pub slippage_limit: u64,
    /// Arbitrary bytes where caller can store any data along with that Borrow. Later the data
    /// can be read from position.borrows record corresponding to that Borrow. Subsequent calls
    /// of Borrow for the same reserve will override that data.
    pub memo: [u8; BORROW_MEMO_LEN],
}
impl Borrow {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            destination_liquidity_wallet,
            curator_fee_receiver,
            borrower,
            reserve,
            pool,
            curator,
            texture_fee_receiver,
            liquidity_mint,
            token_program,
            amount,
            slippage_limit,
            memo,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reserve_liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    reserve_liquidity_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    destination_liquidity_wallet,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_fee_receiver,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(borrower, true),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    texture_fee_receiver,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::Borrow {
            amount,
            slippage_limit,
            memo,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::Repay] Builder struct
pub struct Repay {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Borrowers Position account. Refreshed.
    pub position: solana_program::pubkey::Pubkey,
    ///User's SPL token wallet with liquidity tokens to be used as repayment
    pub source_liquidity_wallet: solana_program::pubkey::Pubkey,
    ///Authority to transfer funds from `source_liquidity_wallet`
    pub user_authority: solana_program::pubkey::Pubkey,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint.
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program - either classic or 2022
    pub token_program: solana_program::pubkey::Pubkey,
    /// amount of principal token to repay. Set to u64::MAX to repay all borrowed amount.
    pub amount: u64,
}
impl Repay {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            source_liquidity_wallet,
            user_authority,
            reserve,
            liquidity_mint,
            token_program,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reserve_liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    source_liquidity_wallet,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    reserve_liquidity_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    user_authority,
                    true,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::Repay {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::WriteOffBadDebt] Builder struct
pub struct WriteOffBadDebt {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Pool to which bad debt position belongs
    pub pool: solana_program::pubkey::Pubkey,
    ///Reserve account to write off bad debt in. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Unhealthy Position account. Refreshed.
    pub position: solana_program::pubkey::Pubkey,
    ///Authority who can write-off bad debt from the reserve.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    /// principal (borrowed as bad debt) token amount to write off
    pub amount: u64,
}
impl WriteOffBadDebt {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            pool,
            reserve,
            position,
            curator_pools_authority,
            curator,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        let ix = SuperLendyInstruction::WriteOffBadDebt {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::Liquidate] Builder struct
pub struct Liquidate {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///SPL token wallet to get repayment liquidity from.
    pub repayment_source_wallet: solana_program::pubkey::Pubkey,
    ///SPL token wallet to receive LP tokens (released collateral of the liquidated Position)
    pub destination_lp_wallet: solana_program::pubkey::Pubkey,
    ///Reserve account to repay principal tokens owed by unhealthy Position. Refreshed.
    pub principal_reserve: solana_program::pubkey::Pubkey,
    ///Reserve account to repay principal tokens owed by unhealthy Position. Refreshed.
    pub collateral_reserve: solana_program::pubkey::Pubkey,
    ///Borrower Position account. Refreshed.
    pub position: solana_program::pubkey::Pubkey,
    ///Liquidator's authority which controls `repayment_source_wallet`
    pub liquidator: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint in principal Reserve.
    pub principal_reserve_liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program used to manage principal tokens
    pub principal_token_program: solana_program::pubkey::Pubkey,
    /// liquidity amount to repay
    pub liquidity_amount: u64,
}
impl Liquidate {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            repayment_source_wallet,
            destination_lp_wallet,
            principal_reserve,
            collateral_reserve,
            position,
            liquidator,
            principal_reserve_liquidity_mint,
            principal_token_program,
            liquidity_amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (principal_reserve_liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                principal_reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (collateral_reserve_lp_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                collateral_reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::COLLATERAL_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    repayment_source_wallet,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    destination_lp_wallet,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(principal_reserve, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    principal_reserve_liquidity_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    collateral_reserve,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    collateral_reserve_lp_supply,
                    false,
                ),
            ]);
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(liquidator, true),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    principal_reserve_liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    principal_token_program,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    spl_token::ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::Liquidate {
            liquidity_amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::ClaimCuratorPerformanceFees] Builder struct
pub struct ClaimCuratorPerformanceFees {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve account to claim performance fees from. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool.
    pub pool: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///SPL token wallet to receive claimed fees. Must be ATA from curator.fees_authority.
    pub fee_receiver: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
}
impl ClaimCuratorPerformanceFees {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            curator,
            fee_receiver,
            liquidity_mint,
            token_program,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reserve_liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    reserve_liquidity_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(fee_receiver, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::ClaimCuratorPerformanceFees {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::ClaimTexturePerformanceFees] Builder struct
pub struct ClaimTexturePerformanceFees {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve account to claim performance fees from. Refreshed.
    pub reserve: solana_program::pubkey::Pubkey,
    ///SPL token wallet to receive claimed fees. Must be ATA from [TextureConfig.fees_authority]
    pub fee_receiver: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
}
impl ClaimTexturePerformanceFees {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            fee_receiver,
            liquidity_mint,
            token_program,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reserve_liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    reserve_liquidity_supply,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(fee_receiver, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::ClaimTexturePerformanceFees {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::InitRewardSupply] Builder struct
pub struct InitRewardSupply {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reward token mint.
    pub reward_mint: solana_program::pubkey::Pubkey,
    ///Pool to init reward supply for.
    pub pool: solana_program::pubkey::Pubkey,
    ///Authority who can manage pool.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
}
impl InitRewardSupply {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reward_mint,
            pool,
            curator_pools_authority,
            curator,
            token_program,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (reward_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                pool.as_ref(),
                #[allow(clippy::useless_asref)]
                reward_mint.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::REWARD_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (reward_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                pool.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::AUTHORITY_SEED.as_ref(),
            ],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(reward_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    reward_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    reward_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::system_program::ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::InitRewardSupply {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::SetRewardRules] Builder struct
pub struct SetRewardRules {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve account to set reward rules for
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool - parent for created Reserve.
    pub pool: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///Reward mint accounts - all in order as in `rules`
    pub reward_mints: Vec<solana_program::pubkey::Pubkey>,
    pub rules: RewardRules,
}
impl SetRewardRules {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            curator_pools_authority,
            curator,
            reward_mints,
            rules,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut mints_count;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend({
                let keys = {
                    mints_count = reward_mints
                        .len()
                        .try_into()
                        .expect(
                            concat!(
                                "convert ", stringify!(reward_mints), " accounts length"
                            ),
                        );
                    &reward_mints
                };
                #[allow(clippy::into_iter_on_ref)]
                keys.into_iter()
                    .map(|addr| solana_program::instruction::AccountMeta::new_readonly(
                        *addr,
                        false,
                    ))
            });
        let ix = SuperLendyInstruction::SetRewardRules {
            mints_count,
            rules,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::ClaimReward] Builder struct
pub struct ClaimReward {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Position account to claim rewords for. Refreshed.
    pub position: solana_program::pubkey::Pubkey,
    ///User's SPL token wallet which will receive reward tokens
    pub destination_wallet: solana_program::pubkey::Pubkey,
    ///Position owner
    pub position_owner: solana_program::pubkey::Pubkey,
    ///Pool, position belongs to.
    pub pool: solana_program::pubkey::Pubkey,
    ///Reward token mint to claim. Determines which reward will be claimed from the Position.
    pub reward_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
}
impl ClaimReward {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            position,
            destination_wallet,
            position_owner,
            pool,
            reward_mint,
            token_program,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (rewards_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                pool.as_ref(),
                #[allow(clippy::useless_asref)]
                reward_mint.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::REWARD_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (reward_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                pool.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::AUTHORITY_SEED.as_ref(),
            ],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([solana_program::instruction::AccountMeta::new(position, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(rewards_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(destination_wallet, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    position_owner,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    reward_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    reward_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::ClaimReward {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::WithdrawReward] Builder struct
pub struct WithdrawReward {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///SPL token account of reward_mint to receive reward tokens.
    pub destination_wallet: solana_program::pubkey::Pubkey,
    ///Pool to withdraw rewards from.
    pub pool: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///Reward token mint to withdraw.
    pub reward_mint: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
    pub amount: u64,
}
impl WithdrawReward {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            destination_wallet,
            pool,
            curator_pools_authority,
            curator,
            reward_mint,
            token_program,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (rewards_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                pool.as_ref(),
                #[allow(clippy::useless_asref)]
                reward_mint.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::REWARD_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (reward_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                pool.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::AUTHORITY_SEED.as_ref(),
            ],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(rewards_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(destination_wallet, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    reward_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    reward_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::WithdrawReward {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::FlashBorrow] Builder struct
pub struct FlashBorrow {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve account to flash-borrow from.
    pub reserve: solana_program::pubkey::Pubkey,
    ///SPL token account to receive flash-borrowed tokens.
    pub destination_wallet: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///Sysvar instructions account
    pub sysvar_instructions: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
    /// Amount of liquidity to flash borrow
    pub amount: u64,
}
impl FlashBorrow {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            destination_wallet,
            liquidity_mint,
            sysvar_instructions,
            token_program,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(liquidity_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(destination_wallet, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    sysvar_instructions,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::FlashBorrow {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::FlashRepay] Builder struct
pub struct FlashRepay {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///SPL token account to transfer tokens for repayment from.
    pub source_wallet: solana_program::pubkey::Pubkey,
    ///Reserve account to flash-repay to.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Liquidity tokens mint
    pub liquidity_mint: solana_program::pubkey::Pubkey,
    ///Authority to transfer funds from source_wallet.
    pub user_transfer_authority: solana_program::pubkey::Pubkey,
    ///Sysvar instructions account
    pub sysvar_instructions: solana_program::pubkey::Pubkey,
    ///SPL Token program
    pub token_program: solana_program::pubkey::Pubkey,
    /// Amount of liquidity to flash repay. Must be the same as in paired FlashBorrow IX.
    pub amount: u64,
}
impl FlashRepay {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            source_wallet,
            reserve,
            liquidity_mint,
            user_transfer_authority,
            sysvar_instructions,
            token_program,
            amount,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (liquidity_supply, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LIQUIDITY_SUPPLY_SEED.as_ref(),
            ],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(source_wallet, false),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(liquidity_supply, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    liquidity_mint,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    user_transfer_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    sysvar_instructions,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    token_program,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::FlashRepay {
            amount,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::ProposeConfig] Builder struct
pub struct ProposeConfig {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve to propose new config for
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool - parent for created Reserve.
    pub pool: solana_program::pubkey::Pubkey,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    pub index: u8,
    pub proposal: ConfigProposal,
}
impl ProposeConfig {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
            index,
            proposal,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    market_price_feed,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::ProposeConfig {
            index,
            proposal,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::ApplyConfigProposal] Builder struct
pub struct ApplyConfigProposal {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve to apply config proposal for
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool - parent for created Reserve.
    pub pool: solana_program::pubkey::Pubkey,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    pub index: u8,
}
impl ApplyConfigProposal {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
            index,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    market_price_feed,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        let ix = SuperLendyInstruction::ApplyConfigProposal {
            index,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::DeleteReserve] Builder struct
pub struct DeleteReserve {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve to delete.
    pub reserve: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves. He will reserve freed rent.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///Pool - Reserve belongs to.
    pub pool: solana_program::pubkey::Pubkey,
}
impl DeleteReserve {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            curator_pools_authority,
            curator,
            pool,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts.extend([solana_program::instruction::AccountMeta::new(reserve, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        let ix = SuperLendyInstruction::DeleteReserve {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::TransferTextureConfigOwnership] Builder struct
pub struct TransferTextureConfigOwnership {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Current global config owner
    pub owner: solana_program::pubkey::Pubkey,
    ///New global config owner
    pub new_owner: solana_program::pubkey::Pubkey,
}
impl TransferTextureConfigOwnership {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            owner,
            new_owner,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    crate::TEXTURE_CONFIG_ID,
                    false,
                ),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(owner, true)]);
        accounts
            .extend([solana_program::instruction::AccountMeta::new(new_owner, true)]);
        let ix = SuperLendyInstruction::TransferTextureConfigOwnership {
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::Version] Builder struct
pub struct Version {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    pub no_error: bool,
}
impl Version {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self { #[cfg(feature = "program-id-manually")] program_id, no_error } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::system_program::ID,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::Version {
            no_error,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
///[SuperLendyInstruction::SetLpMetadata] Builder struct
pub struct SetLpMetadata {
    #[cfg(feature = "program-id-manually")]
    /// Current program ID
    pub program_id: solana_program::pubkey::Pubkey,
    ///Reserve to set LP metadata for
    pub reserve: solana_program::pubkey::Pubkey,
    ///Pool - parent for Reserve
    pub pool: solana_program::pubkey::Pubkey,
    ///Metadata account. PDA.
    pub metadata_account: solana_program::pubkey::Pubkey,
    ///Authority who can configure reserves.
    pub curator_pools_authority: solana_program::pubkey::Pubkey,
    ///Curator account.
    pub curator: solana_program::pubkey::Pubkey,
    ///Sysvar rent account
    pub sysvar_rent: solana_program::pubkey::Pubkey,
    pub metadata: LpTokenMetadata,
}
impl SetLpMetadata {
    #[track_caller]
    pub fn into_instruction(self) -> solana_program::instruction::Instruction {
        let Self {
            #[cfg(feature = "program-id-manually")]
            program_id,
            reserve,
            pool,
            metadata_account,
            curator_pools_authority,
            curator,
            sysvar_rent,
            metadata,
        } = self;
        #[cfg(not(feature = "program-id-manually"))]
        let program_id = SUPER_LENDY_ID;
        let (lp_mint, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[
                #[allow(clippy::useless_asref)]
                reserve.as_ref(),
                #[allow(clippy::useless_asref)]
                crate::pda::LP_TOKEN_SEED.as_ref(),
            ],
            &program_id,
        );
        let (program_authority, _) = solana_program::pubkey::Pubkey::find_program_address(
            &[#[allow(clippy::useless_asref)] crate::pda::AUTHORITY_SEED.as_ref()],
            &program_id,
        );
        #[allow(unused_mut)]
        let mut accounts = vec![];
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(reserve, false),
            ]);
        accounts.extend([solana_program::instruction::AccountMeta::new(lp_mint, false)]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(pool, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(metadata_account, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new(
                    curator_pools_authority,
                    true,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(curator, false),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    program_authority,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    mpl_token_metadata::ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::system_program::ID,
                    false,
                ),
            ]);
        accounts
            .extend([
                solana_program::instruction::AccountMeta::new_readonly(
                    sysvar_rent,
                    false,
                ),
            ]);
        let ix = SuperLendyInstruction::SetLpMetadata {
            metadata,
        };
        solana_program::instruction::Instruction::new_with_borsh(
            program_id,
            &ix,
            accounts,
        )
    }
}
/// [SuperLendyInstruction::CreateTextureConfig] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct CreateTextureConfigAccountIndexes {
    pub texture_config: usize,
    pub owner: usize,
}
impl CreateTextureConfigAccountIndexes {
    pub const COUNT: usize = 2usize;
    pub const TEXTURE_CONFIG: usize = 0usize;
    pub const OWNER: usize = 1usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            texture_config: iter.next().unwrap(),
            owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for CreateTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for CreateTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for CreateTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for CreateTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::AlterTextureConfig] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct AlterTextureConfigAccountIndexes {
    pub texture_config: usize,
    pub owner: usize,
}
impl AlterTextureConfigAccountIndexes {
    pub const COUNT: usize = 2usize;
    pub const TEXTURE_CONFIG: usize = 0usize;
    pub const OWNER: usize = 1usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            texture_config: iter.next().unwrap(),
            owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for AlterTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for AlterTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for AlterTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for AlterTextureConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::CreateCurator] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct CreateCuratorAccountIndexes {
    pub curator: usize,
    pub texture_config: usize,
    pub global_config_owner: usize,
}
impl CreateCuratorAccountIndexes {
    pub const COUNT: usize = 3usize;
    pub const CURATOR: usize = 0usize;
    pub const TEXTURE_CONFIG: usize = 1usize;
    pub const GLOBAL_CONFIG_OWNER: usize = 2usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            curator: iter.next().unwrap(),
            texture_config: iter.next().unwrap(),
            global_config_owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            global_config_owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for CreateCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for CreateCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for CreateCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for CreateCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::AlterCurator] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct AlterCuratorAccountIndexes {
    pub curator: usize,
    pub owner: usize,
}
impl AlterCuratorAccountIndexes {
    pub const COUNT: usize = 2usize;
    pub const CURATOR: usize = 0usize;
    pub const OWNER: usize = 1usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            curator: iter.next().unwrap(),
            owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for AlterCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for AlterCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for AlterCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for AlterCuratorAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::CreatePool] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct CreatePoolAccountIndexes {
    pub pool: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
}
impl CreatePoolAccountIndexes {
    pub const COUNT: usize = 3usize;
    pub const POOL: usize = 0usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 1usize;
    pub const CURATOR: usize = 2usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            pool: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for CreatePoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for CreatePoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for CreatePoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for CreatePoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::AlterPool] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct AlterPoolAccountIndexes {
    pub pool: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
}
impl AlterPoolAccountIndexes {
    pub const COUNT: usize = 3usize;
    pub const POOL: usize = 0usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 1usize;
    pub const CURATOR: usize = 2usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            pool: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for AlterPoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for AlterPoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for AlterPoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for AlterPoolAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::CreateReserve] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct CreateReserveAccountIndexes {
    pub reserve: usize,
    pub pool: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub liquidity_mint: usize,
    pub liquidity_supply: usize,
    pub lp_mint: usize,
    pub collateral_supply: usize,
    pub market_price_feed: usize,
    pub program_authority: usize,
    pub lp_token_program: usize,
    pub liquidity_token_program: usize,
    pub system_program: usize,
}
impl CreateReserveAccountIndexes {
    pub const COUNT: usize = 13usize;
    pub const RESERVE: usize = 0usize;
    pub const POOL: usize = 1usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 2usize;
    pub const CURATOR: usize = 3usize;
    pub const LIQUIDITY_MINT: usize = 4usize;
    pub const LIQUIDITY_SUPPLY: usize = 5usize;
    pub const LP_MINT: usize = 6usize;
    pub const COLLATERAL_SUPPLY: usize = 7usize;
    pub const MARKET_PRICE_FEED: usize = 8usize;
    pub const PROGRAM_AUTHORITY: usize = 9usize;
    pub const LP_TOKEN_PROGRAM: usize = 10usize;
    pub const LIQUIDITY_TOKEN_PROGRAM: usize = 11usize;
    pub const SYSTEM_PROGRAM: usize = 12usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            liquidity_supply: iter.next().unwrap(),
            lp_mint: iter.next().unwrap(),
            collateral_supply: iter.next().unwrap(),
            market_price_feed: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            lp_token_program: iter.next().unwrap(),
            liquidity_token_program: iter.next().unwrap(),
            system_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            collateral_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            market_price_feed: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            system_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for CreateReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for CreateReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for CreateReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for CreateReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::AlterReserve] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct AlterReserveAccountIndexes {
    pub reserve: usize,
    pub pool: usize,
    pub market_price_feed: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub texture_config: usize,
}
impl AlterReserveAccountIndexes {
    pub const COUNT: usize = 6usize;
    pub const RESERVE: usize = 0usize;
    pub const POOL: usize = 1usize;
    pub const MARKET_PRICE_FEED: usize = 2usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 3usize;
    pub const CURATOR: usize = 4usize;
    pub const TEXTURE_CONFIG: usize = 5usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            market_price_feed: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            texture_config: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            market_price_feed: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for AlterReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for AlterReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for AlterReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for AlterReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::RefreshReserve] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct RefreshReserveAccountIndexes {
    pub reserve: usize,
    pub market_price_feed: usize,
    pub irm: usize,
    pub texture_config: usize,
}
impl RefreshReserveAccountIndexes {
    pub const COUNT: usize = 4usize;
    pub const RESERVE: usize = 0usize;
    pub const MARKET_PRICE_FEED: usize = 1usize;
    pub const IRM: usize = 2usize;
    pub const TEXTURE_CONFIG: usize = 3usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            market_price_feed: iter.next().unwrap(),
            irm: iter.next().unwrap(),
            texture_config: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            market_price_feed: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            irm: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for RefreshReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for RefreshReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for RefreshReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for RefreshReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::DepositLiquidity] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct DepositLiquidityAccountIndexes {
    pub authority: usize,
    pub source_liquidity_wallet: usize,
    pub destination_lp_wallet: usize,
    pub reserve: usize,
    pub liquidity_supply: usize,
    pub liquidity_mint: usize,
    pub lp_mint: usize,
    pub program_authority: usize,
    pub lp_token_program: usize,
    pub liquidity_token_program: usize,
}
impl DepositLiquidityAccountIndexes {
    pub const COUNT: usize = 10usize;
    pub const AUTHORITY: usize = 0usize;
    pub const SOURCE_LIQUIDITY_WALLET: usize = 1usize;
    pub const DESTINATION_LP_WALLET: usize = 2usize;
    pub const RESERVE: usize = 3usize;
    pub const LIQUIDITY_SUPPLY: usize = 4usize;
    pub const LIQUIDITY_MINT: usize = 5usize;
    pub const LP_MINT: usize = 6usize;
    pub const PROGRAM_AUTHORITY: usize = 7usize;
    pub const LP_TOKEN_PROGRAM: usize = 8usize;
    pub const LIQUIDITY_TOKEN_PROGRAM: usize = 9usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            authority: iter.next().unwrap(),
            source_liquidity_wallet: iter.next().unwrap(),
            destination_lp_wallet: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            liquidity_supply: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            lp_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            lp_token_program: iter.next().unwrap(),
            liquidity_token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            source_liquidity_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_lp_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for DepositLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for DepositLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for DepositLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for DepositLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::WithdrawLiquidity] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct WithdrawLiquidityAccountIndexes {
    pub authority: usize,
    pub source_lp_wallet: usize,
    pub destination_liquidity_wallet: usize,
    pub reserve: usize,
    pub liquidity_supply: usize,
    pub liquidity_mint: usize,
    pub lp_mint: usize,
    pub program_authority: usize,
    pub lp_token_program: usize,
    pub liquidity_token_program: usize,
}
impl WithdrawLiquidityAccountIndexes {
    pub const COUNT: usize = 10usize;
    pub const AUTHORITY: usize = 0usize;
    pub const SOURCE_LP_WALLET: usize = 1usize;
    pub const DESTINATION_LIQUIDITY_WALLET: usize = 2usize;
    pub const RESERVE: usize = 3usize;
    pub const LIQUIDITY_SUPPLY: usize = 4usize;
    pub const LIQUIDITY_MINT: usize = 5usize;
    pub const LP_MINT: usize = 6usize;
    pub const PROGRAM_AUTHORITY: usize = 7usize;
    pub const LP_TOKEN_PROGRAM: usize = 8usize;
    pub const LIQUIDITY_TOKEN_PROGRAM: usize = 9usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            authority: iter.next().unwrap(),
            source_lp_wallet: iter.next().unwrap(),
            destination_liquidity_wallet: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            liquidity_supply: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            lp_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            lp_token_program: iter.next().unwrap(),
            liquidity_token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            source_lp_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_liquidity_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for WithdrawLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for WithdrawLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for WithdrawLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for WithdrawLiquidityAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::CreatePosition] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct CreatePositionAccountIndexes {
    pub position: usize,
    pub pool: usize,
    pub owner: usize,
}
impl CreatePositionAccountIndexes {
    pub const COUNT: usize = 3usize;
    pub const POSITION: usize = 0usize;
    pub const POOL: usize = 1usize;
    pub const OWNER: usize = 2usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for CreatePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for CreatePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for CreatePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for CreatePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::ClosePosition] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct ClosePositionAccountIndexes {
    pub position: usize,
    pub owner: usize,
}
impl ClosePositionAccountIndexes {
    pub const COUNT: usize = 2usize;
    pub const POSITION: usize = 0usize;
    pub const OWNER: usize = 1usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for ClosePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for ClosePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for ClosePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for ClosePositionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::RefreshPosition] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct RefreshPositionAccountIndexes {
    pub position: usize,
    pub deposits: Vec<usize>,
    pub borrows: Vec<usize>,
}
impl RefreshPositionAccountIndexes {
    pub const POSITION: usize = 0usize;
    pub fn new_direct_order(deposits_size: usize, borrows_size: usize) -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            deposits: {
                let mut out = vec![];
                for _ in 0..deposits_size {
                    out.push(iter.next().unwrap());
                }
                out
            },
            borrows: {
                let mut out = vec![];
                for _ in 0..borrows_size {
                    out.push(iter.next().unwrap());
                }
                out
            },
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
        deposits_size: usize,
        borrows_size: usize,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            deposits: {
                let mut out = vec![];
                for _ in 0..deposits_size {
                    idx += 1;
                    out.push(iter.next().ok_or(idx - 1)?);
                }
                out
            },
            borrows: {
                let mut out = vec![];
                for _ in 0..borrows_size {
                    idx += 1;
                    out.push(iter.next().ok_or(idx - 1)?);
                }
                out
            },
        })
    }
}
/// [SuperLendyInstruction::LockCollateral] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct LockCollateralAccountIndexes {
    pub position: usize,
    pub source_lp_wallet: usize,
    pub reserve_collateral_supply: usize,
    pub owner: usize,
    pub reserve: usize,
    pub lp_token_program: usize,
}
impl LockCollateralAccountIndexes {
    pub const COUNT: usize = 6usize;
    pub const POSITION: usize = 0usize;
    pub const SOURCE_LP_WALLET: usize = 1usize;
    pub const RESERVE_COLLATERAL_SUPPLY: usize = 2usize;
    pub const OWNER: usize = 3usize;
    pub const RESERVE: usize = 4usize;
    pub const LP_TOKEN_PROGRAM: usize = 5usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            source_lp_wallet: iter.next().unwrap(),
            reserve_collateral_supply: iter.next().unwrap(),
            owner: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            lp_token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            source_lp_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve_collateral_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for LockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for LockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for LockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for LockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::UnlockCollateral] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct UnlockCollateralAccountIndexes {
    pub position: usize,
    pub reserve_collateral_supply: usize,
    pub destination_lp_wallet: usize,
    pub owner: usize,
    pub reserve: usize,
    pub program_authority: usize,
    pub lp_token_program: usize,
}
impl UnlockCollateralAccountIndexes {
    pub const COUNT: usize = 7usize;
    pub const POSITION: usize = 0usize;
    pub const RESERVE_COLLATERAL_SUPPLY: usize = 1usize;
    pub const DESTINATION_LP_WALLET: usize = 2usize;
    pub const OWNER: usize = 3usize;
    pub const RESERVE: usize = 4usize;
    pub const PROGRAM_AUTHORITY: usize = 5usize;
    pub const LP_TOKEN_PROGRAM: usize = 6usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            reserve_collateral_supply: iter.next().unwrap(),
            destination_lp_wallet: iter.next().unwrap(),
            owner: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            lp_token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve_collateral_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_lp_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for UnlockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for UnlockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for UnlockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for UnlockCollateralAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::Borrow] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct BorrowAccountIndexes {
    pub position: usize,
    pub reserve_liquidity_supply: usize,
    pub destination_liquidity_wallet: usize,
    pub curator_fee_receiver: usize,
    pub borrower: usize,
    pub reserve: usize,
    pub pool: usize,
    pub curator: usize,
    pub texture_fee_receiver: usize,
    pub texture_config: usize,
    pub liquidity_mint: usize,
    pub program_authority: usize,
    pub token_program: usize,
}
impl BorrowAccountIndexes {
    pub const COUNT: usize = 13usize;
    pub const POSITION: usize = 0usize;
    pub const RESERVE_LIQUIDITY_SUPPLY: usize = 1usize;
    pub const DESTINATION_LIQUIDITY_WALLET: usize = 2usize;
    pub const CURATOR_FEE_RECEIVER: usize = 3usize;
    pub const BORROWER: usize = 4usize;
    pub const RESERVE: usize = 5usize;
    pub const POOL: usize = 6usize;
    pub const CURATOR: usize = 7usize;
    pub const TEXTURE_FEE_RECEIVER: usize = 8usize;
    pub const TEXTURE_CONFIG: usize = 9usize;
    pub const LIQUIDITY_MINT: usize = 10usize;
    pub const PROGRAM_AUTHORITY: usize = 11usize;
    pub const TOKEN_PROGRAM: usize = 12usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            reserve_liquidity_supply: iter.next().unwrap(),
            destination_liquidity_wallet: iter.next().unwrap(),
            curator_fee_receiver: iter.next().unwrap(),
            borrower: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            texture_fee_receiver: iter.next().unwrap(),
            texture_config: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve_liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_liquidity_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_fee_receiver: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            borrower: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_fee_receiver: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for BorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for BorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for BorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for BorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::Repay] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct RepayAccountIndexes {
    pub position: usize,
    pub source_liquidity_wallet: usize,
    pub reserve_liquidity_supply: usize,
    pub user_authority: usize,
    pub reserve: usize,
    pub liquidity_mint: usize,
    pub token_program: usize,
}
impl RepayAccountIndexes {
    pub const COUNT: usize = 7usize;
    pub const POSITION: usize = 0usize;
    pub const SOURCE_LIQUIDITY_WALLET: usize = 1usize;
    pub const RESERVE_LIQUIDITY_SUPPLY: usize = 2usize;
    pub const USER_AUTHORITY: usize = 3usize;
    pub const RESERVE: usize = 4usize;
    pub const LIQUIDITY_MINT: usize = 5usize;
    pub const TOKEN_PROGRAM: usize = 6usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            source_liquidity_wallet: iter.next().unwrap(),
            reserve_liquidity_supply: iter.next().unwrap(),
            user_authority: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            source_liquidity_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve_liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            user_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for RepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for RepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for RepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for RepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::WriteOffBadDebt] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct WriteOffBadDebtAccountIndexes {
    pub pool: usize,
    pub reserve: usize,
    pub position: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
}
impl WriteOffBadDebtAccountIndexes {
    pub const COUNT: usize = 5usize;
    pub const POOL: usize = 0usize;
    pub const RESERVE: usize = 1usize;
    pub const POSITION: usize = 2usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 3usize;
    pub const CURATOR: usize = 4usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            pool: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            position: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for WriteOffBadDebtAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for WriteOffBadDebtAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for WriteOffBadDebtAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for WriteOffBadDebtAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::Liquidate] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct LiquidateAccountIndexes {
    pub repayment_source_wallet: usize,
    pub destination_lp_wallet: usize,
    pub principal_reserve: usize,
    pub principal_reserve_liquidity_supply: usize,
    pub collateral_reserve: usize,
    pub collateral_reserve_lp_supply: usize,
    pub position: usize,
    pub liquidator: usize,
    pub principal_reserve_liquidity_mint: usize,
    pub program_authority: usize,
    pub principal_token_program: usize,
    pub collateral_token_program: usize,
}
impl LiquidateAccountIndexes {
    pub const COUNT: usize = 12usize;
    pub const REPAYMENT_SOURCE_WALLET: usize = 0usize;
    pub const DESTINATION_LP_WALLET: usize = 1usize;
    pub const PRINCIPAL_RESERVE: usize = 2usize;
    pub const PRINCIPAL_RESERVE_LIQUIDITY_SUPPLY: usize = 3usize;
    pub const COLLATERAL_RESERVE: usize = 4usize;
    pub const COLLATERAL_RESERVE_LP_SUPPLY: usize = 5usize;
    pub const POSITION: usize = 6usize;
    pub const LIQUIDATOR: usize = 7usize;
    pub const PRINCIPAL_RESERVE_LIQUIDITY_MINT: usize = 8usize;
    pub const PROGRAM_AUTHORITY: usize = 9usize;
    pub const PRINCIPAL_TOKEN_PROGRAM: usize = 10usize;
    pub const COLLATERAL_TOKEN_PROGRAM: usize = 11usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            repayment_source_wallet: iter.next().unwrap(),
            destination_lp_wallet: iter.next().unwrap(),
            principal_reserve: iter.next().unwrap(),
            principal_reserve_liquidity_supply: iter.next().unwrap(),
            collateral_reserve: iter.next().unwrap(),
            collateral_reserve_lp_supply: iter.next().unwrap(),
            position: iter.next().unwrap(),
            liquidator: iter.next().unwrap(),
            principal_reserve_liquidity_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            principal_token_program: iter.next().unwrap(),
            collateral_token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            repayment_source_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_lp_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            principal_reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            principal_reserve_liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            collateral_reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            collateral_reserve_lp_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            principal_reserve_liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            principal_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            collateral_token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for LiquidateAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for LiquidateAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for LiquidateAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for LiquidateAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::ClaimCuratorPerformanceFees] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct ClaimCuratorPerformanceFeesAccountIndexes {
    pub reserve: usize,
    pub reserve_liquidity_supply: usize,
    pub pool: usize,
    pub curator: usize,
    pub fee_receiver: usize,
    pub liquidity_mint: usize,
    pub program_authority: usize,
    pub token_program: usize,
}
impl ClaimCuratorPerformanceFeesAccountIndexes {
    pub const COUNT: usize = 8usize;
    pub const RESERVE: usize = 0usize;
    pub const RESERVE_LIQUIDITY_SUPPLY: usize = 1usize;
    pub const POOL: usize = 2usize;
    pub const CURATOR: usize = 3usize;
    pub const FEE_RECEIVER: usize = 4usize;
    pub const LIQUIDITY_MINT: usize = 5usize;
    pub const PROGRAM_AUTHORITY: usize = 6usize;
    pub const TOKEN_PROGRAM: usize = 7usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            reserve_liquidity_supply: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            fee_receiver: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve_liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            fee_receiver: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for ClaimCuratorPerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]>
for ClaimCuratorPerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for ClaimCuratorPerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for ClaimCuratorPerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::ClaimTexturePerformanceFees] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct ClaimTexturePerformanceFeesAccountIndexes {
    pub reserve: usize,
    pub reserve_liquidity_supply: usize,
    pub fee_receiver: usize,
    pub texture_config: usize,
    pub liquidity_mint: usize,
    pub program_authority: usize,
    pub token_program: usize,
}
impl ClaimTexturePerformanceFeesAccountIndexes {
    pub const COUNT: usize = 7usize;
    pub const RESERVE: usize = 0usize;
    pub const RESERVE_LIQUIDITY_SUPPLY: usize = 1usize;
    pub const FEE_RECEIVER: usize = 2usize;
    pub const TEXTURE_CONFIG: usize = 3usize;
    pub const LIQUIDITY_MINT: usize = 4usize;
    pub const PROGRAM_AUTHORITY: usize = 5usize;
    pub const TOKEN_PROGRAM: usize = 6usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            reserve_liquidity_supply: iter.next().unwrap(),
            fee_receiver: iter.next().unwrap(),
            texture_config: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve_liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            fee_receiver: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for ClaimTexturePerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]>
for ClaimTexturePerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for ClaimTexturePerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for ClaimTexturePerformanceFeesAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::InitRewardSupply] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct InitRewardSupplyAccountIndexes {
    pub reward_supply: usize,
    pub reward_mint: usize,
    pub pool: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub reward_authority: usize,
    pub token_program: usize,
    pub system_program: usize,
}
impl InitRewardSupplyAccountIndexes {
    pub const COUNT: usize = 8usize;
    pub const REWARD_SUPPLY: usize = 0usize;
    pub const REWARD_MINT: usize = 1usize;
    pub const POOL: usize = 2usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 3usize;
    pub const CURATOR: usize = 4usize;
    pub const REWARD_AUTHORITY: usize = 5usize;
    pub const TOKEN_PROGRAM: usize = 6usize;
    pub const SYSTEM_PROGRAM: usize = 7usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reward_supply: iter.next().unwrap(),
            reward_mint: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            reward_authority: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
            system_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reward_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            system_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for InitRewardSupplyAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for InitRewardSupplyAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for InitRewardSupplyAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for InitRewardSupplyAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::SetRewardRules] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct SetRewardRulesAccountIndexes {
    pub reserve: usize,
    pub pool: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub reward_mints: Vec<usize>,
}
impl SetRewardRulesAccountIndexes {
    pub const RESERVE: usize = 0usize;
    pub const POOL: usize = 1usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 2usize;
    pub const CURATOR: usize = 3usize;
    pub fn new_direct_order(reward_mints_size: usize) -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            reward_mints: {
                let mut out = vec![];
                for _ in 0..reward_mints_size {
                    out.push(iter.next().unwrap());
                }
                out
            },
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
        reward_mints_size: usize,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_mints: {
                let mut out = vec![];
                for _ in 0..reward_mints_size {
                    idx += 1;
                    out.push(iter.next().ok_or(idx - 1)?);
                }
                out
            },
        })
    }
}
/// [SuperLendyInstruction::ClaimReward] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct ClaimRewardAccountIndexes {
    pub position: usize,
    pub rewards_supply: usize,
    pub destination_wallet: usize,
    pub position_owner: usize,
    pub pool: usize,
    pub reward_mint: usize,
    pub reward_authority: usize,
    pub token_program: usize,
}
impl ClaimRewardAccountIndexes {
    pub const COUNT: usize = 8usize;
    pub const POSITION: usize = 0usize;
    pub const REWARDS_SUPPLY: usize = 1usize;
    pub const DESTINATION_WALLET: usize = 2usize;
    pub const POSITION_OWNER: usize = 3usize;
    pub const POOL: usize = 4usize;
    pub const REWARD_MINT: usize = 5usize;
    pub const REWARD_AUTHORITY: usize = 6usize;
    pub const TOKEN_PROGRAM: usize = 7usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            position: iter.next().unwrap(),
            rewards_supply: iter.next().unwrap(),
            destination_wallet: iter.next().unwrap(),
            position_owner: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            reward_mint: iter.next().unwrap(),
            reward_authority: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            position: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            rewards_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            position_owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for ClaimRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for ClaimRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for ClaimRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for ClaimRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::WithdrawReward] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct WithdrawRewardAccountIndexes {
    pub rewards_supply: usize,
    pub destination_wallet: usize,
    pub pool: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub reward_mint: usize,
    pub reward_authority: usize,
    pub token_program: usize,
}
impl WithdrawRewardAccountIndexes {
    pub const COUNT: usize = 8usize;
    pub const REWARDS_SUPPLY: usize = 0usize;
    pub const DESTINATION_WALLET: usize = 1usize;
    pub const POOL: usize = 2usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 3usize;
    pub const CURATOR: usize = 4usize;
    pub const REWARD_MINT: usize = 5usize;
    pub const REWARD_AUTHORITY: usize = 6usize;
    pub const TOKEN_PROGRAM: usize = 7usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            rewards_supply: iter.next().unwrap(),
            destination_wallet: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            reward_mint: iter.next().unwrap(),
            reward_authority: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            rewards_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reward_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for WithdrawRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for WithdrawRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for WithdrawRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for WithdrawRewardAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::FlashBorrow] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct FlashBorrowAccountIndexes {
    pub reserve: usize,
    pub liquidity_supply: usize,
    pub destination_wallet: usize,
    pub liquidity_mint: usize,
    pub program_authority: usize,
    pub sysvar_instructions: usize,
    pub token_program: usize,
}
impl FlashBorrowAccountIndexes {
    pub const COUNT: usize = 7usize;
    pub const RESERVE: usize = 0usize;
    pub const LIQUIDITY_SUPPLY: usize = 1usize;
    pub const DESTINATION_WALLET: usize = 2usize;
    pub const LIQUIDITY_MINT: usize = 3usize;
    pub const PROGRAM_AUTHORITY: usize = 4usize;
    pub const SYSVAR_INSTRUCTIONS: usize = 5usize;
    pub const TOKEN_PROGRAM: usize = 6usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            liquidity_supply: iter.next().unwrap(),
            destination_wallet: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            sysvar_instructions: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            destination_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            sysvar_instructions: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for FlashBorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for FlashBorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for FlashBorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for FlashBorrowAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::FlashRepay] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct FlashRepayAccountIndexes {
    pub source_wallet: usize,
    pub reserve: usize,
    pub liquidity_supply: usize,
    pub liquidity_mint: usize,
    pub user_transfer_authority: usize,
    pub sysvar_instructions: usize,
    pub token_program: usize,
}
impl FlashRepayAccountIndexes {
    pub const COUNT: usize = 7usize;
    pub const SOURCE_WALLET: usize = 0usize;
    pub const RESERVE: usize = 1usize;
    pub const LIQUIDITY_SUPPLY: usize = 2usize;
    pub const LIQUIDITY_MINT: usize = 3usize;
    pub const USER_TRANSFER_AUTHORITY: usize = 4usize;
    pub const SYSVAR_INSTRUCTIONS: usize = 5usize;
    pub const TOKEN_PROGRAM: usize = 6usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            source_wallet: iter.next().unwrap(),
            reserve: iter.next().unwrap(),
            liquidity_supply: iter.next().unwrap(),
            liquidity_mint: iter.next().unwrap(),
            user_transfer_authority: iter.next().unwrap(),
            sysvar_instructions: iter.next().unwrap(),
            token_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            source_wallet: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_supply: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            liquidity_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            user_transfer_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            sysvar_instructions: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            token_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for FlashRepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for FlashRepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for FlashRepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for FlashRepayAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::ProposeConfig] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct ProposeConfigAccountIndexes {
    pub reserve: usize,
    pub pool: usize,
    pub market_price_feed: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub texture_config: usize,
}
impl ProposeConfigAccountIndexes {
    pub const COUNT: usize = 6usize;
    pub const RESERVE: usize = 0usize;
    pub const POOL: usize = 1usize;
    pub const MARKET_PRICE_FEED: usize = 2usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 3usize;
    pub const CURATOR: usize = 4usize;
    pub const TEXTURE_CONFIG: usize = 5usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            market_price_feed: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            texture_config: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            market_price_feed: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for ProposeConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for ProposeConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for ProposeConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for ProposeConfigAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::ApplyConfigProposal] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct ApplyConfigProposalAccountIndexes {
    pub reserve: usize,
    pub pool: usize,
    pub market_price_feed: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
}
impl ApplyConfigProposalAccountIndexes {
    pub const COUNT: usize = 5usize;
    pub const RESERVE: usize = 0usize;
    pub const POOL: usize = 1usize;
    pub const MARKET_PRICE_FEED: usize = 2usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 3usize;
    pub const CURATOR: usize = 4usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            market_price_feed: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            market_price_feed: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for ApplyConfigProposalAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for ApplyConfigProposalAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for ApplyConfigProposalAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for ApplyConfigProposalAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::DeleteReserve] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct DeleteReserveAccountIndexes {
    pub reserve: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub pool: usize,
}
impl DeleteReserveAccountIndexes {
    pub const COUNT: usize = 4usize;
    pub const RESERVE: usize = 0usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 1usize;
    pub const CURATOR: usize = 2usize;
    pub const POOL: usize = 3usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            pool: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for DeleteReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for DeleteReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for DeleteReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for DeleteReserveAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::TransferTextureConfigOwnership] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct TransferTextureConfigOwnershipAccountIndexes {
    pub texture_config: usize,
    pub owner: usize,
    pub new_owner: usize,
}
impl TransferTextureConfigOwnershipAccountIndexes {
    pub const COUNT: usize = 3usize;
    pub const TEXTURE_CONFIG: usize = 0usize;
    pub const OWNER: usize = 1usize;
    pub const NEW_OWNER: usize = 2usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            texture_config: iter.next().unwrap(),
            owner: iter.next().unwrap(),
            new_owner: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            texture_config: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            new_owner: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for TransferTextureConfigOwnershipAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]>
for TransferTextureConfigOwnershipAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for TransferTextureConfigOwnershipAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for TransferTextureConfigOwnershipAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::Version] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct VersionAccountIndexes {
    pub system_program: usize,
}
impl VersionAccountIndexes {
    pub const COUNT: usize = 1usize;
    pub const SYSTEM_PROGRAM: usize = 0usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            system_program: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            system_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for VersionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for VersionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for VersionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for VersionAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
/// [SuperLendyInstruction::SetLpMetadata] instruction account indexes helper
#[derive(Debug, PartialEq)]
pub struct SetLpMetadataAccountIndexes {
    pub reserve: usize,
    pub lp_mint: usize,
    pub pool: usize,
    pub metadata_account: usize,
    pub curator_pools_authority: usize,
    pub curator: usize,
    pub program_authority: usize,
    pub mpl_token_metadata_program: usize,
    pub system_program: usize,
    pub sysvar_rent: usize,
}
impl SetLpMetadataAccountIndexes {
    pub const COUNT: usize = 10usize;
    pub const RESERVE: usize = 0usize;
    pub const LP_MINT: usize = 1usize;
    pub const POOL: usize = 2usize;
    pub const METADATA_ACCOUNT: usize = 3usize;
    pub const CURATOR_POOLS_AUTHORITY: usize = 4usize;
    pub const CURATOR: usize = 5usize;
    pub const PROGRAM_AUTHORITY: usize = 6usize;
    pub const MPL_TOKEN_METADATA_PROGRAM: usize = 7usize;
    pub const SYSTEM_PROGRAM: usize = 8usize;
    pub const SYSVAR_RENT: usize = 9usize;
    pub fn new_direct_order() -> Self {
        let mut iter = std::iter::repeat(()).enumerate().map(|(idx, ())| idx);
        Self {
            reserve: iter.next().unwrap(),
            lp_mint: iter.next().unwrap(),
            pool: iter.next().unwrap(),
            metadata_account: iter.next().unwrap(),
            curator_pools_authority: iter.next().unwrap(),
            curator: iter.next().unwrap(),
            program_authority: iter.next().unwrap(),
            mpl_token_metadata_program: iter.next().unwrap(),
            system_program: iter.next().unwrap(),
            sysvar_rent: iter.next().unwrap(),
        }
    }
    pub fn try_from_indexes<'a>(
        indexes: impl IntoIterator<Item = &'a u8>,
    ) -> Result<Self, usize> {
        let mut iter = indexes.into_iter().map(|idx| (*idx) as usize);
        let mut idx = 0_usize;
        Ok(Self {
            reserve: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            lp_mint: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            pool: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            metadata_account: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator_pools_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            curator: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            program_authority: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            mpl_token_metadata_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            system_program: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
            sysvar_rent: {
                idx += 1;
                iter.next().ok_or(idx - 1)?
            },
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for SetLpMetadataAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<'a, const N: usize> TryFrom<&'a [u8; N]> for SetLpMetadataAccountIndexes {
    type Error = usize;
    fn try_from(indexes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(indexes)
    }
}
impl<const N: usize> TryFrom<[u8; N]> for SetLpMetadataAccountIndexes {
    type Error = usize;
    fn try_from(indexes: [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
impl TryFrom<Vec<u8>> for SetLpMetadataAccountIndexes {
    type Error = usize;
    fn try_from(indexes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_indexes(&indexes)
    }
}
///[SuperLendyInstruction::CreateTextureConfig] instruction account infos helper
#[derive(Debug)]
pub struct CreateTextureConfigAccounts<'a, 'i> {
    ///Global config account to create. With uninitialized data.
    ///Ownership must be already assigned to SuperLendy.
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
    ///Config owner. Will fund Config account.
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> CreateTextureConfigAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !texture_config.is_writable {
            solana_program::msg!(
                concat!(stringify!(texture_config), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*texture_config.key).into(),
            );
        }
        if !texture_config.is_signer {
            return Err(
                texture_common::error::MissingSignature(*texture_config.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        if !rent.is_exempt(texture_config.lamports(), texture_config.data_len()) {
            solana_program::msg!(
                concat!(stringify!(texture_config), " is not rent exempt")
            );
            return Err(
                texture_common::error::InvalidAccount(*texture_config.key).into(),
            );
        }
        if !owner.is_writable {
            solana_program::msg!(concat!(stringify!(owner), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*owner.key).into());
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        Ok(Self { texture_config, owner })
    }
}
///[SuperLendyInstruction::AlterTextureConfig] instruction account infos helper
#[derive(Debug)]
pub struct AlterTextureConfigAccounts<'a, 'i> {
    ///Global config account to change.
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config owner
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> AlterTextureConfigAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !texture_config.is_writable {
            solana_program::msg!(
                concat!(stringify!(texture_config), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*texture_config.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        if !owner.is_writable {
            solana_program::msg!(concat!(stringify!(owner), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*owner.key).into());
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        Ok(Self { texture_config, owner })
    }
}
///[SuperLendyInstruction::CreateCurator] instruction account infos helper
#[derive(Debug)]
pub struct CreateCuratorAccounts<'a, 'i> {
    ///Curator account to create.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global Texture config account.
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config owner
    pub global_config_owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> CreateCuratorAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let curator = texture_common::utils::next_account_info(iter)?;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        let global_config_owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !curator.is_writable {
            solana_program::msg!(concat!(stringify!(curator), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        if !curator.is_signer {
            return Err(texture_common::error::MissingSignature(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        if !global_config_owner.is_writable {
            solana_program::msg!(
                concat!(stringify!(global_config_owner), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*global_config_owner.key).into(),
            );
        }
        if !global_config_owner.is_signer {
            return Err(
                texture_common::error::MissingSignature(*global_config_owner.key).into(),
            );
        }
        Ok(Self {
            curator,
            texture_config,
            global_config_owner,
        })
    }
}
///[SuperLendyInstruction::AlterCurator] instruction account infos helper
#[derive(Debug)]
pub struct AlterCuratorAccounts<'a, 'i> {
    ///Curator account to change.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Owner of the Curator account.
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> AlterCuratorAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let curator = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !curator.is_writable {
            solana_program::msg!(concat!(stringify!(curator), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        Ok(Self { curator, owner })
    }
}
///[SuperLendyInstruction::CreatePool] instruction account infos helper
#[derive(Debug)]
pub struct CreatePoolAccounts<'a, 'i> {
    ///Pool account to create. With uninitialized data.
    ///Ownership must be already assigned to SuperLendy.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pools authority configured in `curator` account. Will fund Pool account.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> CreatePoolAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !pool.is_writable {
            solana_program::msg!(concat!(stringify!(pool), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*pool.key).into());
        }
        if !pool.is_signer {
            return Err(texture_common::error::MissingSignature(*pool.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !rent.is_exempt(pool.lamports(), pool.data_len()) {
            solana_program::msg!(concat!(stringify!(pool), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*pool.key).into());
        }
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        Ok(Self {
            pool,
            curator_pools_authority,
            curator,
        })
    }
}
///[SuperLendyInstruction::AlterPool] instruction account infos helper
#[derive(Debug)]
pub struct AlterPoolAccounts<'a, 'i> {
    ///Pool account to alter
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pools authority configured in `curator` account. Will fund Pool account.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> AlterPoolAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !pool.is_writable {
            solana_program::msg!(concat!(stringify!(pool), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*pool.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        Ok(Self {
            pool,
            curator_pools_authority,
            curator,
        })
    }
}
///[SuperLendyInstruction::CreateReserve] instruction account infos helper
#[derive(Debug)]
pub struct CreateReserveAccounts<'a, 'i> {
    ///Reserve account to create. With uninitialized data.
    ///Ownership must be already assigned to SuperLendy.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - parent for created Reserve.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can add new reserves in to a pool.
    ///Will fund Reserve account.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity mint of the Reserve
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity supply SPL Token wallet. Not initialized. PDA.
    pub liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity provider tokens mint of the Reserve. Not initialized. PDA.
    pub lp_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Collateral supply SPL Token wallet. Not initialized. PDA.
    pub collateral_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program - classic one
    pub lp_token_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program to manage liquidity tokens. Either classic or 2022
    pub liquidity_token_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///System Program.
    pub system_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> CreateReserveAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let lp_mint = texture_common::utils::next_account_info(iter)?;
        let collateral_supply = texture_common::utils::next_account_info(iter)?;
        let market_price_feed = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let lp_token_program = texture_common::utils::next_account_info(iter)?;
        let liquidity_token_program = texture_common::utils::next_account_info(iter)?;
        let system_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        if !reserve.is_signer {
            return Err(texture_common::error::MissingSignature(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !rent.is_exempt(reserve.lamports(), reserve.data_len()) {
            solana_program::msg!(concat!(stringify!(reserve), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        if !liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*liquidity_supply.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            liquidity_supply.owner,
            &solana_program::system_program::ID,
            concat!(stringify!(liquidity_supply), " owner"),
        )?;
        if !lp_mint.is_writable {
            solana_program::msg!(concat!(stringify!(lp_mint), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*lp_mint.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            lp_mint.owner,
            &solana_program::system_program::ID,
            concat!(stringify!(lp_mint), " owner"),
        )?;
        if !collateral_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(collateral_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*collateral_supply.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            collateral_supply.owner,
            &solana_program::system_program::ID,
            concat!(stringify!(collateral_supply), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            market_price_feed.owner,
            &price_proxy::ID,
            concat!(stringify!(market_price_feed), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            lp_token_program.key,
            &spl_token::ID,
            stringify!(lp_token_program),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            system_program.key,
            &solana_program::system_program::ID,
            stringify!(system_program),
        )?;
        Ok(Self {
            reserve,
            pool,
            curator_pools_authority,
            curator,
            liquidity_mint,
            liquidity_supply,
            lp_mint,
            collateral_supply,
            market_price_feed,
            program_authority,
            lp_token_program,
            liquidity_token_program,
            system_program,
        })
    }
}
///[SuperLendyInstruction::AlterReserve] instruction account infos helper
#[derive(Debug)]
pub struct AlterReserveAccounts<'a, 'i> {
    ///Reserve change.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - parent for created Reserve.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config account
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> AlterReserveAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let market_price_feed = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            market_price_feed.owner,
            &price_proxy::ID,
            concat!(stringify!(market_price_feed), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        Ok(Self {
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
            texture_config,
        })
    }
}
///[SuperLendyInstruction::RefreshReserve] instruction account infos helper
#[derive(Debug)]
pub struct RefreshReserveAccounts<'a, 'i> {
    ///Reserve account to refresh.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: &'a solana_program::account_info::AccountInfo<'i>,
    ///Interest Rate Model account.
    pub irm: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config account
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> RefreshReserveAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let market_price_feed = texture_common::utils::next_account_info(iter)?;
        let irm = texture_common::utils::next_account_info(iter)?;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            market_price_feed.owner,
            &price_proxy::ID,
            concat!(stringify!(market_price_feed), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            irm.owner,
            &curvy::ID,
            concat!(stringify!(irm), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        Ok(Self {
            reserve,
            market_price_feed,
            irm,
            texture_config,
        })
    }
}
///[SuperLendyInstruction::DepositLiquidity] instruction account infos helper
#[derive(Debug)]
pub struct DepositLiquidityAccounts<'a, 'i> {
    ///Owner of the source_liquidity_wallet
    pub authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Source SPL Token wallet to transfer liquidity from.
    pub source_liquidity_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token wallet to receive LP tokens minted during deposit.
    pub destination_lp_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account to deposit to. Must be refreshed beforehand.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Destination SPL Token wallet controlled by contract which will receive the liquidity. PDA.
    pub liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///LP tokens mint. PDA.
    pub lp_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub lp_token_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program - either classic or 2022
    pub liquidity_token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> DepositLiquidityAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let authority = texture_common::utils::next_account_info(iter)?;
        let source_liquidity_wallet = texture_common::utils::next_account_info(iter)?;
        let destination_lp_wallet = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let lp_mint = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let lp_token_program = texture_common::utils::next_account_info(iter)?;
        let liquidity_token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !authority.is_signer {
            return Err(texture_common::error::MissingSignature(*authority.key).into());
        }
        if !source_liquidity_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(source_liquidity_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*source_liquidity_wallet.key)
                    .into(),
            );
        }
        if !destination_lp_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_lp_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_lp_wallet.key).into(),
            );
        }
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*liquidity_supply.key).into(),
            );
        }
        if !lp_mint.is_writable {
            solana_program::msg!(concat!(stringify!(lp_mint), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*lp_mint.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            lp_token_program.key,
            &spl_token::ID,
            stringify!(lp_token_program),
        )?;
        Ok(Self {
            authority,
            source_liquidity_wallet,
            destination_lp_wallet,
            reserve,
            liquidity_supply,
            liquidity_mint,
            lp_mint,
            program_authority,
            lp_token_program,
            liquidity_token_program,
        })
    }
}
///[SuperLendyInstruction::WithdrawLiquidity] instruction account infos helper
#[derive(Debug)]
pub struct WithdrawLiquidityAccounts<'a, 'i> {
    ///Owner of the source_lp_wallet
    pub authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Source SPL Token wallet to transfer LP tokens from.
    pub source_lp_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token wallet to receive liquidity.
    pub destination_liquidity_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account to withdraw from. Must be refreshed beforehand.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token wallet controlled by contract which will give the liquidity. PDA.
    pub liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///LP tokens mint. PDA.
    pub lp_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub lp_token_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program - either classic or 2022
    pub liquidity_token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> WithdrawLiquidityAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let authority = texture_common::utils::next_account_info(iter)?;
        let source_lp_wallet = texture_common::utils::next_account_info(iter)?;
        let destination_liquidity_wallet = texture_common::utils::next_account_info(
            iter,
        )?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let lp_mint = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let lp_token_program = texture_common::utils::next_account_info(iter)?;
        let liquidity_token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !authority.is_signer {
            return Err(texture_common::error::MissingSignature(*authority.key).into());
        }
        if !source_lp_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(source_lp_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*source_lp_wallet.key).into(),
            );
        }
        if !destination_liquidity_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_liquidity_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_liquidity_wallet.key)
                    .into(),
            );
        }
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*liquidity_supply.key).into(),
            );
        }
        if !lp_mint.is_writable {
            solana_program::msg!(concat!(stringify!(lp_mint), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*lp_mint.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            lp_token_program.key,
            &spl_token::ID,
            stringify!(lp_token_program),
        )?;
        Ok(Self {
            authority,
            source_lp_wallet,
            destination_liquidity_wallet,
            reserve,
            liquidity_supply,
            liquidity_mint,
            lp_mint,
            program_authority,
            lp_token_program,
            liquidity_token_program,
        })
    }
}
///[SuperLendyInstruction::CreatePosition] instruction account infos helper
#[derive(Debug)]
pub struct CreatePositionAccounts<'a, 'i> {
    ///Position account to initialize. Allocated and owned by SuperLendy. Not initialized yet.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool the position will belong to.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Owner of the position
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> CreatePositionAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        if !position.is_signer {
            return Err(texture_common::error::MissingSignature(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !rent.is_exempt(position.lamports(), position.data_len()) {
            solana_program::msg!(concat!(stringify!(position), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !owner.is_writable {
            solana_program::msg!(concat!(stringify!(owner), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*owner.key).into());
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        Ok(Self { position, pool, owner })
    }
}
///[SuperLendyInstruction::ClosePosition] instruction account infos helper
#[derive(Debug)]
pub struct ClosePositionAccounts<'a, 'i> {
    ///Position account to close.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Owner of the position
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> ClosePositionAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !owner.is_writable {
            solana_program::msg!(concat!(stringify!(owner), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*owner.key).into());
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        Ok(Self { position, owner })
    }
}
///[SuperLendyInstruction::RefreshPosition] instruction account infos helper
#[derive(Debug)]
pub struct RefreshPositionAccounts<'a, 'i> {
    ///Position account.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Collateral deposit reserve accounts - refreshed,
    ///all in same order as listed in Position.deposits
    pub deposits: Vec<&'a solana_program::account_info::AccountInfo<'i>>,
    ///Liquidity borrow reserve accounts - refreshed,
    ///all in same order as listed in Position.borrows
    pub borrows: Vec<&'a solana_program::account_info::AccountInfo<'i>>,
}
impl<'a, 'i> RefreshPositionAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        deposit_count: usize,
        borrow_count: usize,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let deposits = {
            let mut out = vec![];
            for _ in 0..deposit_count {
                out.push(texture_common::utils::next_account_info(iter)?);
            }
            out
        };
        let borrows = {
            let mut out = vec![];
            for _ in 0..borrow_count {
                out.push(texture_common::utils::next_account_info(iter)?);
            }
            out
        };
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        for acc in &deposits {
            #[allow(clippy::needless_borrow)]
            texture_common::utils::verify_key(
                acc.owner,
                &__self_program_id__,
                concat!(stringify!(acc), " owner"),
            )?;
        }
        for acc in &borrows {
            #[allow(clippy::needless_borrow)]
            texture_common::utils::verify_key(
                acc.owner,
                &__self_program_id__,
                concat!(stringify!(acc), " owner"),
            )?;
        }
        Ok(Self {
            position,
            deposits,
            borrows,
        })
    }
}
///[SuperLendyInstruction::LockCollateral] instruction account infos helper
#[derive(Debug)]
pub struct LockCollateralAccounts<'a, 'i> {
    ///Position account to lock collateral in.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///User's SPL token wallet which holds LP tokens to be locked as collateral
    pub source_lp_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet to hold locked LP tokens. PDA.
    pub reserve_collateral_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Position owner and also authority for source_lp_wallet
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub lp_token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> LockCollateralAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let source_lp_wallet = texture_common::utils::next_account_info(iter)?;
        let reserve_collateral_supply = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let lp_token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !source_lp_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(source_lp_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*source_lp_wallet.key).into(),
            );
        }
        if !reserve_collateral_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(reserve_collateral_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*reserve_collateral_supply.key)
                    .into(),
            );
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            lp_token_program.key,
            &spl_token::ID,
            stringify!(lp_token_program),
        )?;
        Ok(Self {
            position,
            source_lp_wallet,
            reserve_collateral_supply,
            owner,
            reserve,
            lp_token_program,
        })
    }
}
///[SuperLendyInstruction::UnlockCollateral] instruction account infos helper
#[derive(Debug)]
pub struct UnlockCollateralAccounts<'a, 'i> {
    ///Position account to unlock collateral from
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet which holds locked LP tokens. PDA.
    pub reserve_collateral_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///User's SPL token wallet which will receive unlocked LP tokens
    pub destination_lp_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Position owner
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub lp_token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> UnlockCollateralAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let reserve_collateral_supply = texture_common::utils::next_account_info(iter)?;
        let destination_lp_wallet = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let lp_token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !reserve_collateral_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(reserve_collateral_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*reserve_collateral_supply.key)
                    .into(),
            );
        }
        if !destination_lp_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_lp_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_lp_wallet.key).into(),
            );
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            lp_token_program.key,
            &spl_token::ID,
            stringify!(lp_token_program),
        )?;
        Ok(Self {
            position,
            reserve_collateral_supply,
            destination_lp_wallet,
            owner,
            reserve,
            program_authority,
            lp_token_program,
        })
    }
}
///[SuperLendyInstruction::Borrow] instruction account infos helper
#[derive(Debug)]
pub struct BorrowAccounts<'a, 'i> {
    ///Borrowers Position account. Refreshed.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet which holds liquidity. PDA.
    pub reserve_liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///User's SPL token wallet which will receive borrowed liquidity tokens
    pub destination_liquidity_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token wallet which will receive loan origination fee. ATA from curator.fee_authority
    pub curator_fee_receiver: &'a solana_program::account_info::AccountInfo<'i>,
    ///Position owner who borrow
    pub borrower: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool borrow happens in.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator of the pool.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token wallet which will receive loan origination fee. Must be ATA from GlobalConfig.fees_authority
    pub texture_fee_receiver: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config account
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint.
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program - either classic or 2022
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> BorrowAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let reserve_liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let destination_liquidity_wallet = texture_common::utils::next_account_info(
            iter,
        )?;
        let curator_fee_receiver = texture_common::utils::next_account_info(iter)?;
        let borrower = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let texture_fee_receiver = texture_common::utils::next_account_info(iter)?;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !reserve_liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(reserve_liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*reserve_liquidity_supply.key)
                    .into(),
            );
        }
        if !destination_liquidity_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_liquidity_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_liquidity_wallet.key)
                    .into(),
            );
        }
        if !curator_fee_receiver.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_fee_receiver), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_fee_receiver.key).into(),
            );
        }
        if !borrower.is_signer {
            return Err(texture_common::error::MissingSignature(*borrower.key).into());
        }
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        if !texture_fee_receiver.is_writable {
            solana_program::msg!(
                concat!(stringify!(texture_fee_receiver), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*texture_fee_receiver.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        Ok(Self {
            position,
            reserve_liquidity_supply,
            destination_liquidity_wallet,
            curator_fee_receiver,
            borrower,
            reserve,
            pool,
            curator,
            texture_fee_receiver,
            texture_config,
            liquidity_mint,
            program_authority,
            token_program,
        })
    }
}
///[SuperLendyInstruction::Repay] instruction account infos helper
#[derive(Debug)]
pub struct RepayAccounts<'a, 'i> {
    ///Borrowers Position account. Refreshed.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///User's SPL token wallet with liquidity tokens to be used as repayment
    pub source_liquidity_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet to return liquidity to. PDA.
    pub reserve_liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority to transfer funds from `source_liquidity_wallet`
    pub user_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account which is the source of LP tokens being deposited. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint.
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program - either classic or 2022
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> RepayAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let source_liquidity_wallet = texture_common::utils::next_account_info(iter)?;
        let reserve_liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let user_authority = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !source_liquidity_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(source_liquidity_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*source_liquidity_wallet.key)
                    .into(),
            );
        }
        if !reserve_liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(reserve_liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*reserve_liquidity_supply.key)
                    .into(),
            );
        }
        if !user_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*user_authority.key).into(),
            );
        }
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        Ok(Self {
            position,
            source_liquidity_wallet,
            reserve_liquidity_supply,
            user_authority,
            reserve,
            liquidity_mint,
            token_program,
        })
    }
}
///[SuperLendyInstruction::WriteOffBadDebt] instruction account infos helper
#[derive(Debug)]
pub struct WriteOffBadDebtAccounts<'a, 'i> {
    ///Pool to which bad debt position belongs
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account to write off bad debt in. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Unhealthy Position account. Refreshed.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can write-off bad debt from the reserve.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> WriteOffBadDebtAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let pool = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let position = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        Ok(Self {
            pool,
            reserve,
            position,
            curator_pools_authority,
            curator,
        })
    }
}
///[SuperLendyInstruction::Liquidate] instruction account infos helper
#[derive(Debug)]
pub struct LiquidateAccounts<'a, 'i> {
    ///SPL token wallet to get repayment liquidity from.
    pub repayment_source_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token wallet to receive LP tokens (released collateral of the liquidated Position)
    pub destination_lp_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account to repay principal tokens owed by unhealthy Position. Refreshed.
    pub principal_reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet to return principal liquidity to. PDA.
    pub principal_reserve_liquidity_supply: &'a solana_program::account_info::AccountInfo<
        'i,
    >,
    ///Reserve account to repay principal tokens owed by unhealthy Position. Refreshed.
    pub collateral_reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet which holds locked LP tokens. PDA.
    pub collateral_reserve_lp_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Borrower Position account. Refreshed.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidator's authority which controls `repayment_source_wallet`
    pub liquidator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint in principal Reserve.
    pub principal_reserve_liquidity_mint: &'a solana_program::account_info::AccountInfo<
        'i,
    >,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program used to manage principal tokens
    pub principal_token_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program used to manage collateral tokens (LPs) - always classic SPL Token
    pub collateral_token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> LiquidateAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let repayment_source_wallet = texture_common::utils::next_account_info(iter)?;
        let destination_lp_wallet = texture_common::utils::next_account_info(iter)?;
        let principal_reserve = texture_common::utils::next_account_info(iter)?;
        let principal_reserve_liquidity_supply = texture_common::utils::next_account_info(
            iter,
        )?;
        let collateral_reserve = texture_common::utils::next_account_info(iter)?;
        let collateral_reserve_lp_supply = texture_common::utils::next_account_info(
            iter,
        )?;
        let position = texture_common::utils::next_account_info(iter)?;
        let liquidator = texture_common::utils::next_account_info(iter)?;
        let principal_reserve_liquidity_mint = texture_common::utils::next_account_info(
            iter,
        )?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let principal_token_program = texture_common::utils::next_account_info(iter)?;
        let collateral_token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !repayment_source_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(repayment_source_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*repayment_source_wallet.key)
                    .into(),
            );
        }
        if !destination_lp_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_lp_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_lp_wallet.key).into(),
            );
        }
        if !principal_reserve.is_writable {
            solana_program::msg!(
                concat!(stringify!(principal_reserve), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*principal_reserve.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            principal_reserve.owner,
            &__self_program_id__,
            concat!(stringify!(principal_reserve), " owner"),
        )?;
        if !principal_reserve_liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(principal_reserve_liquidity_supply),
                " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(
                        *principal_reserve_liquidity_supply.key,
                    )
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            collateral_reserve.owner,
            &__self_program_id__,
            concat!(stringify!(collateral_reserve), " owner"),
        )?;
        if !collateral_reserve_lp_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(collateral_reserve_lp_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*collateral_reserve_lp_supply.key)
                    .into(),
            );
        }
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !liquidator.is_signer {
            return Err(texture_common::error::MissingSignature(*liquidator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            collateral_token_program.key,
            &spl_token::ID,
            stringify!(collateral_token_program),
        )?;
        Ok(Self {
            repayment_source_wallet,
            destination_lp_wallet,
            principal_reserve,
            principal_reserve_liquidity_supply,
            collateral_reserve,
            collateral_reserve_lp_supply,
            position,
            liquidator,
            principal_reserve_liquidity_mint,
            program_authority,
            principal_token_program,
            collateral_token_program,
        })
    }
}
///[SuperLendyInstruction::ClaimCuratorPerformanceFees] instruction account infos helper
#[derive(Debug)]
pub struct ClaimCuratorPerformanceFeesAccounts<'a, 'i> {
    ///Reserve account to claim performance fees from. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet with Reserve's liquidity. PDA.
    pub reserve_liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token wallet to receive claimed fees. Must be ATA from curator.fees_authority.
    pub fee_receiver: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> ClaimCuratorPerformanceFeesAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let reserve_liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let fee_receiver = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !reserve_liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(reserve_liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*reserve_liquidity_supply.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        if !fee_receiver.is_writable {
            solana_program::msg!(concat!(stringify!(fee_receiver), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*fee_receiver.key).into());
        }
        Ok(Self {
            reserve,
            reserve_liquidity_supply,
            pool,
            curator,
            fee_receiver,
            liquidity_mint,
            program_authority,
            token_program,
        })
    }
}
///[SuperLendyInstruction::ClaimTexturePerformanceFees] instruction account infos helper
#[derive(Debug)]
pub struct ClaimTexturePerformanceFeesAccounts<'a, 'i> {
    ///Reserve account to claim performance fees from. Refreshed.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet with Reserve's liquidity. PDA.
    pub reserve_liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token wallet to receive claimed fees. Must be ATA from [TextureConfig.fees_authority]
    pub fee_receiver: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config account
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> ClaimTexturePerformanceFeesAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let reserve_liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let fee_receiver = texture_common::utils::next_account_info(iter)?;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !reserve_liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(reserve_liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*reserve_liquidity_supply.key)
                    .into(),
            );
        }
        if !fee_receiver.is_writable {
            solana_program::msg!(concat!(stringify!(fee_receiver), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*fee_receiver.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        Ok(Self {
            reserve,
            reserve_liquidity_supply,
            fee_receiver,
            texture_config,
            liquidity_mint,
            program_authority,
            token_program,
        })
    }
}
///[SuperLendyInstruction::InitRewardSupply] instruction account infos helper
#[derive(Debug)]
pub struct InitRewardSupplyAccounts<'a, 'i> {
    ///Reward supply account to initialize. Uninitialized. PDA
    pub reward_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reward token mint.
    pub reward_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool to init reward supply for.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can manage pool.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's reward authority. PDA.
    pub reward_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///System Program.
    pub system_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> InitRewardSupplyAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reward_supply = texture_common::utils::next_account_info(iter)?;
        let reward_mint = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let reward_authority = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        let system_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reward_supply.is_writable {
            solana_program::msg!(concat!(stringify!(reward_supply), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reward_supply.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reward_supply.owner,
            &solana_program::system_program::ID,
            concat!(stringify!(reward_supply), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            system_program.key,
            &solana_program::system_program::ID,
            stringify!(system_program),
        )?;
        Ok(Self {
            reward_supply,
            reward_mint,
            pool,
            curator_pools_authority,
            curator,
            reward_authority,
            token_program,
            system_program,
        })
    }
}
///[SuperLendyInstruction::SetRewardRules] instruction account infos helper
#[derive(Debug)]
pub struct SetRewardRulesAccounts<'a, 'i> {
    ///Reserve account to set reward rules for
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - parent for created Reserve.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reward mint accounts - all in order as in `rules`
    pub reward_mints: Vec<&'a solana_program::account_info::AccountInfo<'i>>,
}
impl<'a, 'i> SetRewardRulesAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        mints_count: usize,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let reward_mints = {
            let mut out = vec![];
            for _ in 0..mints_count {
                out.push(texture_common::utils::next_account_info(iter)?);
            }
            out
        };
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        Ok(Self {
            reserve,
            pool,
            curator_pools_authority,
            curator,
            reward_mints,
        })
    }
}
///[SuperLendyInstruction::ClaimReward] instruction account infos helper
#[derive(Debug)]
pub struct ClaimRewardAccounts<'a, 'i> {
    ///Position account to claim rewords for. Refreshed.
    pub position: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet which holds reward tokens. PDA.
    pub rewards_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///User's SPL token wallet which will receive reward tokens
    pub destination_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Position owner
    pub position_owner: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool, position belongs to.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reward token mint to claim. Determines which reward will be claimed from the Position.
    pub reward_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's reward authority. PDA.
    pub reward_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> ClaimRewardAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let position = texture_common::utils::next_account_info(iter)?;
        let rewards_supply = texture_common::utils::next_account_info(iter)?;
        let destination_wallet = texture_common::utils::next_account_info(iter)?;
        let position_owner = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let reward_mint = texture_common::utils::next_account_info(iter)?;
        let reward_authority = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !position.is_writable {
            solana_program::msg!(concat!(stringify!(position), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*position.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            position.owner,
            &__self_program_id__,
            concat!(stringify!(position), " owner"),
        )?;
        if !rewards_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(rewards_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*rewards_supply.key).into(),
            );
        }
        if !destination_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_wallet.key).into(),
            );
        }
        if !position_owner.is_signer {
            return Err(
                texture_common::error::MissingSignature(*position_owner.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        Ok(Self {
            position,
            rewards_supply,
            destination_wallet,
            position_owner,
            pool,
            reward_mint,
            reward_authority,
            token_program,
        })
    }
}
///[SuperLendyInstruction::WithdrawReward] instruction account infos helper
#[derive(Debug)]
pub struct WithdrawRewardAccounts<'a, 'i> {
    ///Contract managed SPL token wallet which holds reward tokens. PDA.
    pub rewards_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token account of reward_mint to receive reward tokens.
    pub destination_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool to withdraw rewards from.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reward token mint to withdraw.
    pub reward_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's reward authority. PDA.
    pub reward_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> WithdrawRewardAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let rewards_supply = texture_common::utils::next_account_info(iter)?;
        let destination_wallet = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let reward_mint = texture_common::utils::next_account_info(iter)?;
        let reward_authority = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !rewards_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(rewards_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*rewards_supply.key).into(),
            );
        }
        if !destination_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_wallet.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        Ok(Self {
            rewards_supply,
            destination_wallet,
            pool,
            curator_pools_authority,
            curator,
            reward_mint,
            reward_authority,
            token_program,
        })
    }
}
///[SuperLendyInstruction::FlashBorrow] instruction account infos helper
#[derive(Debug)]
pub struct FlashBorrowAccounts<'a, 'i> {
    ///Reserve account to flash-borrow from.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet which holds Reserve's liquidity tokens. PDA.
    pub liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL token account to receive flash-borrowed tokens.
    pub destination_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Sysvar instructions account
    pub sysvar_instructions: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> FlashBorrowAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let destination_wallet = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let sysvar_instructions = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*liquidity_supply.key).into(),
            );
        }
        if !destination_wallet.is_writable {
            solana_program::msg!(
                concat!(stringify!(destination_wallet), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*destination_wallet.key).into(),
            );
        }
        Ok(Self {
            reserve,
            liquidity_supply,
            destination_wallet,
            liquidity_mint,
            program_authority,
            sysvar_instructions,
            token_program,
        })
    }
}
///[SuperLendyInstruction::FlashRepay] instruction account infos helper
#[derive(Debug)]
pub struct FlashRepayAccounts<'a, 'i> {
    ///SPL token account to transfer tokens for repayment from.
    pub source_wallet: &'a solana_program::account_info::AccountInfo<'i>,
    ///Reserve account to flash-repay to.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract managed SPL token wallet which holds Reserve's liquidity tokens. PDA.
    pub liquidity_supply: &'a solana_program::account_info::AccountInfo<'i>,
    ///Liquidity tokens mint
    pub liquidity_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority to transfer funds from source_wallet.
    pub user_transfer_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Sysvar instructions account
    pub sysvar_instructions: &'a solana_program::account_info::AccountInfo<'i>,
    ///SPL Token program
    pub token_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> FlashRepayAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let source_wallet = texture_common::utils::next_account_info(iter)?;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let liquidity_supply = texture_common::utils::next_account_info(iter)?;
        let liquidity_mint = texture_common::utils::next_account_info(iter)?;
        let user_transfer_authority = texture_common::utils::next_account_info(iter)?;
        let sysvar_instructions = texture_common::utils::next_account_info(iter)?;
        let token_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !source_wallet.is_writable {
            solana_program::msg!(concat!(stringify!(source_wallet), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*source_wallet.key).into());
        }
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !liquidity_supply.is_writable {
            solana_program::msg!(
                concat!(stringify!(liquidity_supply), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*liquidity_supply.key).into(),
            );
        }
        if !user_transfer_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(user_transfer_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*user_transfer_authority.key)
                    .into(),
            );
        }
        if !user_transfer_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*user_transfer_authority.key)
                    .into(),
            );
        }
        Ok(Self {
            source_wallet,
            reserve,
            liquidity_supply,
            liquidity_mint,
            user_transfer_authority,
            sysvar_instructions,
            token_program,
        })
    }
}
///[SuperLendyInstruction::ProposeConfig] instruction account infos helper
#[derive(Debug)]
pub struct ProposeConfigAccounts<'a, 'i> {
    ///Reserve to propose new config for
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - parent for created Reserve.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Global config account
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> ProposeConfigAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let market_price_feed = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            market_price_feed.owner,
            &price_proxy::ID,
            concat!(stringify!(market_price_feed), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        Ok(Self {
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
            texture_config,
        })
    }
}
///[SuperLendyInstruction::ApplyConfigProposal] instruction account infos helper
#[derive(Debug)]
pub struct ApplyConfigProposalAccounts<'a, 'i> {
    ///Reserve to apply config proposal for
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - parent for created Reserve.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Price feed account to get market price for liquidity currency.
    pub market_price_feed: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> ApplyConfigProposalAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let market_price_feed = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            market_price_feed.owner,
            &price_proxy::ID,
            concat!(stringify!(market_price_feed), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        Ok(Self {
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
        })
    }
}
///[SuperLendyInstruction::DeleteReserve] instruction account infos helper
#[derive(Debug)]
pub struct DeleteReserveAccounts<'a, 'i> {
    ///Reserve to delete.
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves. He will reserve freed rent.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - Reserve belongs to.
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> DeleteReserveAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        if !reserve.is_writable {
            solana_program::msg!(concat!(stringify!(reserve), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*reserve.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        Ok(Self {
            reserve,
            curator_pools_authority,
            curator,
            pool,
        })
    }
}
///[SuperLendyInstruction::TransferTextureConfigOwnership] instruction account infos helper
#[derive(Debug)]
pub struct TransferTextureConfigOwnershipAccounts<'a, 'i> {
    ///Global config account.
    pub texture_config: &'a solana_program::account_info::AccountInfo<'i>,
    ///Current global config owner
    pub owner: &'a solana_program::account_info::AccountInfo<'i>,
    ///New global config owner
    pub new_owner: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> TransferTextureConfigOwnershipAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let texture_config = texture_common::utils::next_account_info(iter)?;
        let owner = texture_common::utils::next_account_info(iter)?;
        let new_owner = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        if !texture_config.is_writable {
            solana_program::msg!(
                concat!(stringify!(texture_config), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*texture_config.key).into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.owner,
            &__self_program_id__,
            concat!(stringify!(texture_config), " owner"),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            texture_config.key,
            &crate::TEXTURE_CONFIG_ID,
            stringify!(texture_config),
        )?;
        if !owner.is_writable {
            solana_program::msg!(concat!(stringify!(owner), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*owner.key).into());
        }
        if !owner.is_signer {
            return Err(texture_common::error::MissingSignature(*owner.key).into());
        }
        if !new_owner.is_writable {
            solana_program::msg!(concat!(stringify!(new_owner), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*new_owner.key).into());
        }
        if !new_owner.is_signer {
            return Err(texture_common::error::MissingSignature(*new_owner.key).into());
        }
        Ok(Self {
            texture_config,
            owner,
            new_owner,
        })
    }
}
///[SuperLendyInstruction::Version] instruction account infos helper
#[derive(Debug)]
pub struct VersionAccounts<'a, 'i> {
    ///System Program.
    pub system_program: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> VersionAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let system_program = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            system_program.key,
            &solana_program::system_program::ID,
            stringify!(system_program),
        )?;
        Ok(Self { system_program })
    }
}
///[SuperLendyInstruction::SetLpMetadata] instruction account infos helper
#[derive(Debug)]
pub struct SetLpMetadataAccounts<'a, 'i> {
    ///Reserve to set LP metadata for
    pub reserve: &'a solana_program::account_info::AccountInfo<'i>,
    ///LP tokens mint. PDA.
    pub lp_mint: &'a solana_program::account_info::AccountInfo<'i>,
    ///Pool - parent for Reserve
    pub pool: &'a solana_program::account_info::AccountInfo<'i>,
    ///Metadata account. PDA.
    pub metadata_account: &'a solana_program::account_info::AccountInfo<'i>,
    ///Authority who can configure reserves.
    pub curator_pools_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Curator account.
    pub curator: &'a solana_program::account_info::AccountInfo<'i>,
    ///Contract's authority. PDA.
    pub program_authority: &'a solana_program::account_info::AccountInfo<'i>,
    ///Metaplex token metadata Program.
    pub mpl_token_metadata_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///System Program.
    pub system_program: &'a solana_program::account_info::AccountInfo<'i>,
    ///Sysvar rent account
    pub sysvar_rent: &'a solana_program::account_info::AccountInfo<'i>,
}
impl<'a, 'i> SetLpMetadataAccounts<'a, 'i> {
    pub fn from_iter<I>(
        iter: &mut I,
        program_id: &solana_program::pubkey::Pubkey,
    ) -> std::result::Result<Self, texture_common::macros::accounts::AccountParseError>
    where
        I: Iterator<Item = &'a solana_program::account_info::AccountInfo<'i>>,
    {
        let __self_program_id__ = program_id;
        let reserve = texture_common::utils::next_account_info(iter)?;
        let lp_mint = texture_common::utils::next_account_info(iter)?;
        let pool = texture_common::utils::next_account_info(iter)?;
        let metadata_account = texture_common::utils::next_account_info(iter)?;
        let curator_pools_authority = texture_common::utils::next_account_info(iter)?;
        let curator = texture_common::utils::next_account_info(iter)?;
        let program_authority = texture_common::utils::next_account_info(iter)?;
        let mpl_token_metadata_program = texture_common::utils::next_account_info(iter)?;
        let system_program = texture_common::utils::next_account_info(iter)?;
        let sysvar_rent = texture_common::utils::next_account_info(iter)?;
        #[cfg(not(feature = "program-id-manually"))] #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            __self_program_id__,
            &SUPER_LENDY_ID,
            "self_program_id",
        )?;
        let rent = <solana_program::rent::Rent as solana_program::sysvar::Sysvar>::get()
            .expect("rent");
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            reserve.owner,
            &__self_program_id__,
            concat!(stringify!(reserve), " owner"),
        )?;
        if !lp_mint.is_writable {
            solana_program::msg!(concat!(stringify!(lp_mint), " is not writable"));
            return Err(texture_common::error::InvalidAccount(*lp_mint.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            pool.owner,
            &__self_program_id__,
            concat!(stringify!(pool), " owner"),
        )?;
        if !metadata_account.is_writable {
            solana_program::msg!(
                concat!(stringify!(metadata_account), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*metadata_account.key).into(),
            );
        }
        if !curator_pools_authority.is_writable {
            solana_program::msg!(
                concat!(stringify!(curator_pools_authority), " is not writable")
            );
            return Err(
                texture_common::error::InvalidAccount(*curator_pools_authority.key)
                    .into(),
            );
        }
        if !curator_pools_authority.is_signer {
            return Err(
                texture_common::error::MissingSignature(*curator_pools_authority.key)
                    .into(),
            );
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            curator.owner,
            &__self_program_id__,
            concat!(stringify!(curator), " owner"),
        )?;
        if !rent.is_exempt(curator.lamports(), curator.data_len()) {
            solana_program::msg!(concat!(stringify!(curator), " is not rent exempt"));
            return Err(texture_common::error::InvalidAccount(*curator.key).into());
        }
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            mpl_token_metadata_program.key,
            &mpl_token_metadata::ID,
            stringify!(mpl_token_metadata_program),
        )?;
        #[allow(clippy::needless_borrow)]
        texture_common::utils::verify_key(
            system_program.key,
            &solana_program::system_program::ID,
            stringify!(system_program),
        )?;
        Ok(Self {
            reserve,
            lp_mint,
            pool,
            metadata_account,
            curator_pools_authority,
            curator,
            program_authority,
            mpl_token_metadata_program,
            system_program,
            sysvar_rent,
        })
    }
}
pub(crate) mod ix_docs {
    macro_rules! create_texture_config {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable, signer\\]", "</b> ",
            "Global config account to create. With uninitialized data.", "\n",
            "Ownership must be already assigned to SuperLendy.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Config owner. Will fund Config account.", "\n", "\n", " ## Usage", "\n",
            " ", "For create instruction use builder struct [CreateTextureConfig]", " ",
            "(method [into_instruction][CreateTextureConfig::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [CreateTextureConfigAccounts]",
            " ", "(method [from_iter][CreateTextureConfigAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [CreateTextureConfigAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use create_texture_config;
    macro_rules! alter_texture_config {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Global config account to change.", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Global config owner", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [AlterTextureConfig]", " ",
            "(method [into_instruction][AlterTextureConfig::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [AlterTextureConfigAccounts]",
            " ", "(method [from_iter][AlterTextureConfigAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [AlterTextureConfigAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use alter_texture_config;
    macro_rules! create_curator {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable, signer\\]", "</b> ", "Curator account to create.", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Global Texture config account.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ", "Global config owner",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [CreateCurator]", " ",
            "(method [into_instruction][CreateCurator::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [CreateCuratorAccounts]",
            " ", "(method [from_iter][CreateCuratorAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [CreateCuratorAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use create_curator;
    macro_rules! alter_curator {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Curator account to change.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[signer\\]", "</b> ",
            "Owner of the Curator account.", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [AlterCurator]", " ",
            "(method [into_instruction][AlterCurator::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [AlterCuratorAccounts]",
            " ", "(method [from_iter][AlterCuratorAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [AlterCuratorAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use alter_curator;
    macro_rules! create_pool {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable, signer\\]", "</b> ",
            "Pool account to create. With uninitialized data.", "\n",
            "Ownership must be already assigned to SuperLendy.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Pools authority configured in `curator` account. Will fund Pool account.",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Curator account.", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [CreatePool]", " ",
            "(method [into_instruction][CreatePool::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [CreatePoolAccounts]",
            " ", "(method [from_iter][CreatePoolAccounts::from_iter]).", " ", "\n\n",
            " ", "For work with account indexes use struct [CreatePoolAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use create_pool;
    macro_rules! alter_pool {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Pool account to alter", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Pools authority configured in `curator` account. Will fund Pool account.",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Curator account.", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [AlterPool]", " ",
            "(method [into_instruction][AlterPool::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [AlterPoolAccounts]",
            " ", "(method [from_iter][AlterPoolAccounts::from_iter]).", " ", "\n\n", " ",
            "For work with account indexes use struct [AlterPoolAccountIndexes].", "\n",
            }
        };
    }
    pub(crate) use alter_pool;
    macro_rules! create_reserve {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable, signer\\]", "</b> ",
            "Reserve account to create. With uninitialized data.", "\n",
            "Ownership must be already assigned to SuperLendy.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool - parent for created Reserve.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can add new reserves in to a pool.", "\n",
            "Will fund Reserve account.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "4", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity mint of the Reserve", "\n", " ", "\n", "<b><i>", "5",
            "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Liquidity supply SPL Token wallet. Not initialized. PDA.", "\n", " ", "\n",
            "<b><i>", "6", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Liquidity provider tokens mint of the Reserve. Not initialized. PDA.", "\n",
            " ", "\n", "<b><i>", "7", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Collateral supply SPL Token wallet. Not initialized. PDA.", "\n", " ", "\n",
            "<b><i>", "8", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Price feed account to get market price for liquidity currency.", "\n", " ",
            "\n", "<b><i>", "9", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Contract's authority. PDA.", "\n", " ", "\n", "<b><i>", "10",
            "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program - classic one", "\n",
            " ", "\n", "<b><i>", "11", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program to manage liquidity tokens. Either classic or 2022", "\n",
            " ", "\n", "<b><i>", "12", "</i></b>. <b>", "\\[\\]", "</b> ",
            "System Program.", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [CreateReserve]", " ",
            "(method [into_instruction][CreateReserve::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [CreateReserveAccounts]",
            " ", "(method [from_iter][CreateReserveAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [CreateReserveAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use create_reserve;
    macro_rules! alter_reserve {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve change.", "\n", " ", "\n", "<b><i>", "1",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Pool - parent for created Reserve.",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Price feed account to get market price for liquidity currency.", "\n", " ",
            "\n", "<b><i>", "3", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can configure reserves.", "\n", " ", "\n", "<b><i>", "4",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ", "Global config account",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [AlterReserve]", " ",
            "(method [into_instruction][AlterReserve::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [AlterReserveAccounts]",
            " ", "(method [from_iter][AlterReserveAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [AlterReserveAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use alter_reserve;
    macro_rules! refresh_reserve {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve account to refresh.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Price feed account to get market price for liquidity currency.", "\n", " ",
            "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Interest Rate Model account.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Global config account", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [RefreshReserve]", " ",
            "(method [into_instruction][RefreshReserve::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [RefreshReserveAccounts]",
            " ", "(method [from_iter][RefreshReserveAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [RefreshReserveAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use refresh_reserve;
    macro_rules! deposit_liquidity {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[signer\\]", "</b> ", "Owner of the source_liquidity_wallet", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Source SPL Token wallet to transfer liquidity from.", "\n", " ", "\n",
            "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL Token wallet to receive LP tokens minted during deposit.", "\n", " ",
            "\n", "<b><i>", "3", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Reserve account to deposit to. Must be refreshed beforehand.", "\n", " ",
            "\n", "<b><i>", "4", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Destination SPL Token wallet controlled by contract which will receive the liquidity. PDA.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity tokens mint", "\n", " ", "\n", "<b><i>", "6", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "LP tokens mint. PDA.", "\n", " ", "\n", "<b><i>",
            "7", "</i></b>. <b>", "\\[\\]", "</b> ", "Contract's authority. PDA.", "\n",
            " ", "\n", "<b><i>", "8", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program", "\n", " ", "\n", "<b><i>", "9", "</i></b>. <b>",
            "\\[\\]", "</b> ", "SPL Token program - either classic or 2022", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [DepositLiquidity]", " ",
            "(method [into_instruction][DepositLiquidity::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [DepositLiquidityAccounts]",
            " ", "(method [from_iter][DepositLiquidityAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [DepositLiquidityAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use deposit_liquidity;
    macro_rules! withdraw_liquidity {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[signer\\]", "</b> ", "Owner of the source_lp_wallet", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Source SPL Token wallet to transfer LP tokens from.", "\n", " ", "\n",
            "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL Token wallet to receive liquidity.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Reserve account to withdraw from. Must be refreshed beforehand.", "\n", " ",
            "\n", "<b><i>", "4", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL Token wallet controlled by contract which will give the liquidity. PDA.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity tokens mint", "\n", " ", "\n", "<b><i>", "6", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "LP tokens mint. PDA.", "\n", " ", "\n", "<b><i>",
            "7", "</i></b>. <b>", "\\[\\]", "</b> ", "Contract's authority. PDA.", "\n",
            " ", "\n", "<b><i>", "8", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program", "\n", " ", "\n", "<b><i>", "9", "</i></b>. <b>",
            "\\[\\]", "</b> ", "SPL Token program - either classic or 2022", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [WithdrawLiquidity]", " ",
            "(method [into_instruction][WithdrawLiquidity::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [WithdrawLiquidityAccounts]",
            " ", "(method [from_iter][WithdrawLiquidityAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [WithdrawLiquidityAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use withdraw_liquidity;
    macro_rules! create_position {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable, signer\\]", "</b> ",
            "Position account to initialize. Allocated and owned by SuperLendy. Not initialized yet.",
            "\n", " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool the position will belong to.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ", "Owner of the position",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [CreatePosition]", " ",
            "(method [into_instruction][CreatePosition::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [CreatePositionAccounts]",
            " ", "(method [from_iter][CreatePositionAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [CreatePositionAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use create_position;
    macro_rules! close_position {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Position account to close.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Owner of the position", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [ClosePosition]", " ",
            "(method [into_instruction][ClosePosition::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [ClosePositionAccounts]",
            " ", "(method [from_iter][ClosePositionAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [ClosePositionAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use close_position;
    macro_rules! refresh_position {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Position account.", "\n", " ", "\n", "<b><i>",
            "1", " .. ", "1", " + ", "`deposit_count`", "</i></b>. <b>", "\\[\\]",
            "</b> ", "Collateral deposit reserve accounts - refreshed,", "\n",
            "all in same order as listed in Position.deposits", "\n", " ", "\n",
            "<b><i>", "1", " + ", "`deposit_count`", " .. ", "1", " + ",
            "`deposit_count`", " + ", "`borrow_count`", "</i></b>. <b>", "\\[\\]",
            "</b> ", "Liquidity borrow reserve accounts - refreshed,", "\n",
            "all in same order as listed in Position.borrows", "\n", "\n", " ## Usage",
            "\n", " ", "For create instruction use builder struct [RefreshPosition]",
            " ", "(method [into_instruction][RefreshPosition::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [RefreshPositionAccounts]",
            " ", "(method [from_iter][RefreshPositionAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [RefreshPositionAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use refresh_position;
    macro_rules! lock_collateral {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Position account to lock collateral in.", "\n",
            " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "User's SPL token wallet which holds LP tokens to be locked as collateral",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet to hold locked LP tokens. PDA.", "\n",
            " ", "\n", "<b><i>", "3", "</i></b>. <b>", "\\[signer\\]", "</b> ",
            "Position owner and also authority for source_lp_wallet", "\n", " ", "\n",
            "<b><i>", "4", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Reserve account which is the source of LP tokens being deposited. Refreshed.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [LockCollateral]", " ",
            "(method [into_instruction][LockCollateral::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [LockCollateralAccounts]",
            " ", "(method [from_iter][LockCollateralAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [LockCollateralAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use lock_collateral;
    macro_rules! unlock_collateral {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Position account to unlock collateral from",
            "\n", " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds locked LP tokens. PDA.", "\n",
            " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "User's SPL token wallet which will receive unlocked LP tokens", "\n", " ",
            "\n", "<b><i>", "3", "</i></b>. <b>", "\\[signer\\]", "</b> ",
            "Position owner", "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>", "\\[\\]",
            "</b> ",
            "Reserve account which is the source of LP tokens being deposited. Refreshed.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Contract's authority. PDA.", "\n", " ", "\n", "<b><i>", "6",
            "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [UnlockCollateral]", " ",
            "(method [into_instruction][UnlockCollateral::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [UnlockCollateralAccounts]",
            " ", "(method [from_iter][UnlockCollateralAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [UnlockCollateralAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use unlock_collateral;
    macro_rules! borrow {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Borrowers Position account. Refreshed.", "\n",
            " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds liquidity. PDA.", "\n", " ",
            "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "User's SPL token wallet which will receive borrowed liquidity tokens", "\n",
            " ", "\n", "<b><i>", "3", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL token wallet which will receive loan origination fee. ATA from curator.fee_authority",
            "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>", "\\[signer\\]", "</b> ",
            "Position owner who borrow", "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "Reserve account which is the source of LP tokens being deposited. Refreshed.",
            "\n", " ", "\n", "<b><i>", "6", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool borrow happens in.", "\n", " ", "\n", "<b><i>", "7", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Curator of the pool.", "\n", " ", "\n", "<b><i>", "8",
            "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL token wallet which will receive loan origination fee. Must be ATA from GlobalConfig.fees_authority",
            "\n", " ", "\n", "<b><i>", "9", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Global config account", "\n", " ", "\n", "<b><i>", "10", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Liquidity tokens mint.", "\n", " ", "\n", "<b><i>", "11",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Contract's authority. PDA.", "\n", " ",
            "\n", "<b><i>", "12", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program - either classic or 2022", "\n", "\n", " ## Usage", "\n",
            " ", "For create instruction use builder struct [Borrow]", " ",
            "(method [into_instruction][Borrow::into_instruction]).", " ", "\n\n", " ",
            "For parse accounts infos from processor use struct [BorrowAccounts]", " ",
            "(method [from_iter][BorrowAccounts::from_iter]).", " ", "\n\n", " ",
            "For work with account indexes use struct [BorrowAccountIndexes].", "\n", }
        };
    }
    pub(crate) use borrow;
    macro_rules! repay {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Borrowers Position account. Refreshed.", "\n",
            " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "User's SPL token wallet with liquidity tokens to be used as repayment",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet to return liquidity to. PDA.", "\n", " ",
            "\n", "<b><i>", "3", "</i></b>. <b>", "\\[signer\\]", "</b> ",
            "Authority to transfer funds from `source_liquidity_wallet`", "\n", " ",
            "\n", "<b><i>", "4", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Reserve account which is the source of LP tokens being deposited. Refreshed.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity tokens mint.", "\n", " ", "\n", "<b><i>", "6", "</i></b>. <b>",
            "\\[\\]", "</b> ", "SPL Token program - either classic or 2022", "\n", "\n",
            " ## Usage", "\n", " ", "For create instruction use builder struct [Repay]",
            " ", "(method [into_instruction][Repay::into_instruction]).", " ", "\n\n",
            " ", "For parse accounts infos from processor use struct [RepayAccounts]",
            " ", "(method [from_iter][RepayAccounts::from_iter]).", " ", "\n\n", " ",
            "For work with account indexes use struct [RepayAccountIndexes].", "\n", }
        };
    }
    pub(crate) use repay;
    macro_rules! write_off_bad_debt {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Pool to which bad debt position belongs", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Reserve account to write off bad debt in. Refreshed.", "\n", " ", "\n",
            "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Unhealthy Position account. Refreshed.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can write-off bad debt from the reserve.", "\n", " ", "\n",
            "<b><i>", "4", "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n",
            "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [WriteOffBadDebt]", " ",
            "(method [into_instruction][WriteOffBadDebt::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [WriteOffBadDebtAccounts]",
            " ", "(method [from_iter][WriteOffBadDebtAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [WriteOffBadDebtAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use write_off_bad_debt;
    macro_rules! liquidate {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "SPL token wallet to get repayment liquidity from.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL token wallet to receive LP tokens (released collateral of the liquidated Position)",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Reserve account to repay principal tokens owed by unhealthy Position. Refreshed.",
            "\n", " ", "\n", "<b><i>", "3", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet to return principal liquidity to. PDA.",
            "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Reserve account to repay principal tokens owed by unhealthy Position. Refreshed.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds locked LP tokens. PDA.", "\n",
            " ", "\n", "<b><i>", "6", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Borrower Position account. Refreshed.", "\n", " ", "\n", "<b><i>", "7",
            "</i></b>. <b>", "\\[signer\\]", "</b> ",
            "Liquidator's authority which controls `repayment_source_wallet`", "\n", " ",
            "\n", "<b><i>", "8", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity tokens mint in principal Reserve.", "\n", " ", "\n", "<b><i>",
            "9", "</i></b>. <b>", "\\[\\]", "</b> ", "Contract's authority. PDA.", "\n",
            " ", "\n", "<b><i>", "10", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program used to manage principal tokens", "\n", " ", "\n",
            "<b><i>", "11", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program used to manage collateral tokens (LPs) - always classic SPL Token",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [Liquidate]", " ",
            "(method [into_instruction][Liquidate::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [LiquidateAccounts]",
            " ", "(method [from_iter][LiquidateAccounts::from_iter]).", " ", "\n\n", " ",
            "For work with account indexes use struct [LiquidateAccountIndexes].", "\n",
            }
        };
    }
    pub(crate) use liquidate;
    macro_rules! claim_curator_performance_fees {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "Reserve account to claim performance fees from. Refreshed.", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet with Reserve's liquidity. PDA.", "\n",
            " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ", "Pool.", "\n",
            " ", "\n", "<b><i>", "3", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Curator account.", "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "SPL token wallet to receive claimed fees. Must be ATA from curator.fees_authority.",
            "\n", " ", "\n", "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity tokens mint", "\n", " ", "\n", "<b><i>", "6", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Contract's authority. PDA.", "\n", " ", "\n", "<b><i>",
            "7", "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [ClaimCuratorPerformanceFees]",
            " ",
            "(method [into_instruction][ClaimCuratorPerformanceFees::into_instruction]).",
            " ", "\n\n", " ",
            "For parse accounts infos from processor use struct [ClaimCuratorPerformanceFeesAccounts]",
            " ", "(method [from_iter][ClaimCuratorPerformanceFeesAccounts::from_iter]).",
            " ", "\n\n", " ",
            "For work with account indexes use struct [ClaimCuratorPerformanceFeesAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use claim_curator_performance_fees;
    macro_rules! claim_texture_performance_fees {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "Reserve account to claim performance fees from. Refreshed.", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet with Reserve's liquidity. PDA.", "\n",
            " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL token wallet to receive claimed fees. Must be ATA from [TextureConfig.fees_authority]",
            "\n", " ", "\n", "<b><i>", "3", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Global config account", "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Liquidity tokens mint", "\n", " ", "\n", "<b><i>", "5",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Contract's authority. PDA.", "\n", " ",
            "\n", "<b><i>", "6", "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [ClaimTexturePerformanceFees]",
            " ",
            "(method [into_instruction][ClaimTexturePerformanceFees::into_instruction]).",
            " ", "\n\n", " ",
            "For parse accounts infos from processor use struct [ClaimTexturePerformanceFeesAccounts]",
            " ", "(method [from_iter][ClaimTexturePerformanceFeesAccounts::from_iter]).",
            " ", "\n\n", " ",
            "For work with account indexes use struct [ClaimTexturePerformanceFeesAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use claim_texture_performance_fees;
    macro_rules! init_reward_supply {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "Reward supply account to initialize. Uninitialized. PDA", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ", "Reward token mint.",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool to init reward supply for.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can manage pool.", "\n", " ", "\n", "<b><i>", "4",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Contract's reward authority. PDA.", "\n", " ", "\n", "<b><i>", "6",
            "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program", "\n", " ", "\n",
            "<b><i>", "7", "</i></b>. <b>", "\\[\\]", "</b> ", "System Program.", "\n",
            "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [InitRewardSupply]", " ",
            "(method [into_instruction][InitRewardSupply::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [InitRewardSupplyAccounts]",
            " ", "(method [from_iter][InitRewardSupplyAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [InitRewardSupplyAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use init_reward_supply;
    macro_rules! set_reward_rules {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve account to set reward rules for", "\n",
            " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool - parent for created Reserve.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can configure reserves.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "4", " .. ", "4", " + ", "`mints_count`", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Reward mint accounts - all in order as in `rules`", "\n",
            "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [SetRewardRules]", " ",
            "(method [into_instruction][SetRewardRules::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [SetRewardRulesAccounts]",
            " ", "(method [from_iter][SetRewardRulesAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [SetRewardRulesAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use set_reward_rules;
    macro_rules! claim_reward {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "Position account to claim rewords for. Refreshed.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds reward tokens. PDA.", "\n",
            " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "User's SPL token wallet which will receive reward tokens", "\n", " ", "\n",
            "<b><i>", "3", "</i></b>. <b>", "\\[signer\\]", "</b> ", "Position owner",
            "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool, position belongs to.", "\n", " ", "\n", "<b><i>", "5",
            "</i></b>. <b>", "\\[\\]", "</b> ",
            "Reward token mint to claim. Determines which reward will be claimed from the Position.",
            "\n", " ", "\n", "<b><i>", "6", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Contract's reward authority. PDA.", "\n", " ", "\n", "<b><i>", "7",
            "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [ClaimReward]", " ",
            "(method [into_instruction][ClaimReward::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [ClaimRewardAccounts]",
            " ", "(method [from_iter][ClaimRewardAccounts::from_iter]).", " ", "\n\n",
            " ", "For work with account indexes use struct [ClaimRewardAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use claim_reward;
    macro_rules! withdraw_reward {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds reward tokens. PDA.", "\n",
            " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL token account of reward_mint to receive reward tokens.", "\n", " ",
            "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool to withdraw rewards from.", "\n", " ", "\n", "<b><i>", "3",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can configure reserves.", "\n", " ", "\n", "<b><i>", "4",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Reward token mint to withdraw.", "\n", " ", "\n", "<b><i>", "6",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Contract's reward authority. PDA.",
            "\n", " ", "\n", "<b><i>", "7", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [WithdrawReward]", " ",
            "(method [into_instruction][WithdrawReward::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [WithdrawRewardAccounts]",
            " ", "(method [from_iter][WithdrawRewardAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [WithdrawRewardAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use withdraw_reward;
    macro_rules! flash_borrow {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve account to flash-borrow from.", "\n",
            " ", "\n", "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds Reserve's liquidity tokens. PDA.",
            "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "SPL token account to receive flash-borrowed tokens.", "\n", " ", "\n",
            "<b><i>", "3", "</i></b>. <b>", "\\[\\]", "</b> ", "Liquidity tokens mint",
            "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Contract's authority. PDA.", "\n", " ", "\n", "<b><i>", "5",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Sysvar instructions account", "\n", " ",
            "\n", "<b><i>", "6", "</i></b>. <b>", "\\[\\]", "</b> ", "SPL Token program",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [FlashBorrow]", " ",
            "(method [into_instruction][FlashBorrow::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [FlashBorrowAccounts]",
            " ", "(method [from_iter][FlashBorrowAccounts::from_iter]).", " ", "\n\n",
            " ", "For work with account indexes use struct [FlashBorrowAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use flash_borrow;
    macro_rules! flash_repay {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ",
            "SPL token account to transfer tokens for repayment from.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Reserve account to flash-repay to.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "Contract managed SPL token wallet which holds Reserve's liquidity tokens. PDA.",
            "\n", " ", "\n", "<b><i>", "3", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Liquidity tokens mint", "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>",
            "\\[writable, signer\\]", "</b> ",
            "Authority to transfer funds from source_wallet.", "\n", " ", "\n", "<b><i>",
            "5", "</i></b>. <b>", "\\[\\]", "</b> ", "Sysvar instructions account", "\n",
            " ", "\n", "<b><i>", "6", "</i></b>. <b>", "\\[\\]", "</b> ",
            "SPL Token program", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [FlashRepay]", " ",
            "(method [into_instruction][FlashRepay::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [FlashRepayAccounts]",
            " ", "(method [from_iter][FlashRepayAccounts::from_iter]).", " ", "\n\n",
            " ", "For work with account indexes use struct [FlashRepayAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use flash_repay;
    macro_rules! propose_config {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve to propose new config for", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool - parent for created Reserve.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[\\]", "</b> ",
            "Price feed account to get market price for liquidity currency.", "\n", " ",
            "\n", "<b><i>", "3", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can configure reserves.", "\n", " ", "\n", "<b><i>", "4",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "5", "</i></b>. <b>", "\\[\\]", "</b> ", "Global config account",
            "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [ProposeConfig]", " ",
            "(method [into_instruction][ProposeConfig::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [ProposeConfigAccounts]",
            " ", "(method [from_iter][ProposeConfigAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [ProposeConfigAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use propose_config;
    macro_rules! apply_config_proposal {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve to apply config proposal for", "\n", " ",
            "\n", "<b><i>", "1", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Pool - parent for created Reserve.", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[\\]", "</b> ",
            "Price feed account to get market price for liquidity currency.", "\n", " ",
            "\n", "<b><i>", "3", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can configure reserves.", "\n", " ", "\n", "<b><i>", "4",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", "\n",
            " ## Usage", "\n", " ",
            "For create instruction use builder struct [ApplyConfigProposal]", " ",
            "(method [into_instruction][ApplyConfigProposal::into_instruction]).", " ",
            "\n\n", " ",
            "For parse accounts infos from processor use struct [ApplyConfigProposalAccounts]",
            " ", "(method [from_iter][ApplyConfigProposalAccounts::from_iter]).", " ",
            "\n\n", " ",
            "For work with account indexes use struct [ApplyConfigProposalAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use apply_config_proposal;
    macro_rules! delete_reserve {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Reserve to delete.", "\n", " ", "\n", "<b><i>",
            "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Authority who can configure reserves. He will reserve freed rent.", "\n",
            " ", "\n", "<b><i>", "2", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Curator account.", "\n", " ", "\n", "<b><i>", "3", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Pool - Reserve belongs to.", "\n", "\n", " ## Usage",
            "\n", " ", "For create instruction use builder struct [DeleteReserve]", " ",
            "(method [into_instruction][DeleteReserve::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [DeleteReserveAccounts]",
            " ", "(method [from_iter][DeleteReserveAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [DeleteReserveAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use delete_reserve;
    macro_rules! transfer_texture_config_ownership {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[writable\\]", "</b> ", "Global config account.", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "Current global config owner", "\n", " ", "\n", "<b><i>", "2",
            "</i></b>. <b>", "\\[writable, signer\\]", "</b> ",
            "New global config owner", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [TransferTextureConfigOwnership]",
            " ",
            "(method [into_instruction][TransferTextureConfigOwnership::into_instruction]).",
            " ", "\n\n", " ",
            "For parse accounts infos from processor use struct [TransferTextureConfigOwnershipAccounts]",
            " ",
            "(method [from_iter][TransferTextureConfigOwnershipAccounts::from_iter]).",
            " ", "\n\n", " ",
            "For work with account indexes use struct [TransferTextureConfigOwnershipAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use transfer_texture_config_ownership;
    macro_rules! version {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[\\]", "</b> ", "System Program.", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [Version]", " ",
            "(method [into_instruction][Version::into_instruction]).", " ", "\n\n", " ",
            "For parse accounts infos from processor use struct [VersionAccounts]", " ",
            "(method [from_iter][VersionAccounts::from_iter]).", " ", "\n\n", " ",
            "For work with account indexes use struct [VersionAccountIndexes].", "\n", }
        };
    }
    pub(crate) use version;
    macro_rules! set_lp_metadata {
        () => {
            concat! { " ## Accounts", "\n", " ", "\n", "<b><i>", "0", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Reserve to set LP metadata for", "\n", " ", "\n",
            "<b><i>", "1", "</i></b>. <b>", "\\[writable\\]", "</b> ",
            "LP tokens mint. PDA.", "\n", " ", "\n", "<b><i>", "2", "</i></b>. <b>",
            "\\[\\]", "</b> ", "Pool - parent for Reserve", "\n", " ", "\n", "<b><i>",
            "3", "</i></b>. <b>", "\\[writable\\]", "</b> ", "Metadata account. PDA.",
            "\n", " ", "\n", "<b><i>", "4", "</i></b>. <b>", "\\[writable, signer\\]",
            "</b> ", "Authority who can configure reserves.", "\n", " ", "\n", "<b><i>",
            "5", "</i></b>. <b>", "\\[\\]", "</b> ", "Curator account.", "\n", " ", "\n",
            "<b><i>", "6", "</i></b>. <b>", "\\[\\]", "</b> ",
            "Contract's authority. PDA.", "\n", " ", "\n", "<b><i>", "7",
            "</i></b>. <b>", "\\[\\]", "</b> ", "Metaplex token metadata Program.", "\n",
            " ", "\n", "<b><i>", "8", "</i></b>. <b>", "\\[\\]", "</b> ",
            "System Program.", "\n", " ", "\n", "<b><i>", "9", "</i></b>. <b>", "\\[\\]",
            "</b> ", "Sysvar rent account", "\n", "\n", " ## Usage", "\n", " ",
            "For create instruction use builder struct [SetLpMetadata]", " ",
            "(method [into_instruction][SetLpMetadata::into_instruction]).", " ", "\n\n",
            " ",
            "For parse accounts infos from processor use struct [SetLpMetadataAccounts]",
            " ", "(method [from_iter][SetLpMetadataAccounts::from_iter]).", " ", "\n\n",
            " ",
            "For work with account indexes use struct [SetLpMetadataAccountIndexes].",
            "\n", }
        };
    }
    pub(crate) use set_lp_metadata;
}
