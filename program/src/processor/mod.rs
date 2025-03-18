use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use spl_token_2022::extension::StateWithExtensions;
use texture_common::account::PodAccount;
use texture_common::error;
use texture_common::utils::verify_key;
use tinyvec::ArrayVec;

use crate::error::SuperLendyError;
use crate::error::SuperLendyError::{InvalidKey, OperationCanNotBePerformed};
use crate::instruction::{
    AlterPoolAccounts, AlterTextureConfigAccounts, CreatePoolAccounts, CreateTextureConfigAccounts,
    SuperLendyInstruction, TransferTextureConfigOwnershipAccounts,
};
use crate::state::curator::Curator;
use crate::state::pool::{Pool, PoolParams};
use crate::state::texture_cfg::{TextureConfig, TextureConfigParams};
use crate::{LendyResult, SUPER_LENDY_ID};

mod curator;
mod position;
mod reserve;
mod rewards;

pub type SeedVec<'a> = ArrayVec<[&'a [u8]; 5]>;
macro_rules! seedvec {
    ($($seed:expr),*) => {{
        let mut seed_vec = $crate::processor::SeedVec::new();
        $( seed_vec.push($seed); )*
        seed_vec
    }};
}

pub(crate) use seedvec;

pub struct Processor<'a, 'b> {
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'b>],
}

impl<'a, 'b> Processor<'a, 'b> {
    pub fn new(program_id: &'a Pubkey, accounts: &'a [AccountInfo<'b>]) -> Self {
        Self {
            program_id,
            accounts,
        }
    }

    pub fn process_instruction(&self, input: &[u8]) -> LendyResult<()> {
        // Two process functions - is to overcome stack size limitation for IX arguments

        match self.process_instruction_1(input) {
            Ok(result) => Ok(result),
            Err(SuperLendyError::Unimplemented) => self.process_instruction_2(input),
            Err(err) => Err(err),
        }
    }

    fn process_instruction_1(&self, input: &[u8]) -> LendyResult<()> {
        match SuperLendyInstruction::try_from_slice(input).map_err(SuperLendyError::from)? {
            SuperLendyInstruction::CreateTextureConfig { params } => {
                self.create_texture_config(params)
            }
            SuperLendyInstruction::AlterTextureConfig { params } => {
                self.alter_texture_config(params)
            }
            SuperLendyInstruction::TransferTextureConfigOwnership => {
                self.transfer_texture_config_ownership()
            }
            SuperLendyInstruction::CreateCurator { params } => self.create_curator(params),
            SuperLendyInstruction::AlterCurator { params } => self.alter_curator(params),
            SuperLendyInstruction::CreatePool { params } => self.create_pool(params),
            SuperLendyInstruction::AlterPool { params } => self.alter_pool(params),
            SuperLendyInstruction::CreateReserve {
                params,
                reserve_type,
            } => self.create_reserve(params, reserve_type),
            SuperLendyInstruction::AlterReserve {
                params,
                mode,
                flash_loans_enabled,
            } => self.alter_reserve(params, mode, flash_loans_enabled),
            SuperLendyInstruction::RefreshReserve => self.refresh_reserve(),
            SuperLendyInstruction::DeleteReserve => self.delete_reserve(),
            SuperLendyInstruction::DepositLiquidity { amount } => self.deposit_liquidity(amount),
            SuperLendyInstruction::WithdrawLiquidity { lp_amount } => {
                self.withdraw_liquidity(lp_amount)
            }
            SuperLendyInstruction::CreatePosition { position_type } => {
                self.create_position(position_type)
            }
            SuperLendyInstruction::ClosePosition => self.close_position(),
            SuperLendyInstruction::RefreshPosition {
                deposit_count,
                borrow_count,
            } => self.refresh_position(deposit_count as usize, borrow_count as usize),
            SuperLendyInstruction::LockCollateral { amount, memo } => {
                self.lock_collateral(amount, memo)
            }
            SuperLendyInstruction::UnlockCollateral { amount } => self.unlock_collateral(amount),
            _ => Err(SuperLendyError::Unimplemented),
        }
    }
    fn process_instruction_2(&self, input: &[u8]) -> LendyResult<()> {
        match SuperLendyInstruction::try_from_slice(input).map_err(SuperLendyError::from)? {
            SuperLendyInstruction::Borrow {
                amount,
                slippage_limit,
                memo,
            } => self.borrow(amount, slippage_limit, memo),
            SuperLendyInstruction::Repay { amount } => self.repay(amount),
            SuperLendyInstruction::Liquidate { liquidity_amount } => {
                self.liquidate(liquidity_amount)
            }
            SuperLendyInstruction::WriteOffBadDebt { amount } => self.write_off_bad_debt(amount),
            SuperLendyInstruction::ClaimCuratorPerformanceFees => {
                self.claim_curator_performance_fees()
            }
            SuperLendyInstruction::ClaimTexturePerformanceFees => {
                self.claim_texture_performance_fees()
            }
            SuperLendyInstruction::SetRewardRules { mints_count, rules } => {
                self.set_reward_rules(mints_count as usize, rules)
            }
            SuperLendyInstruction::InitRewardSupply => self.init_reward_supply(),
            SuperLendyInstruction::ClaimReward => self.claim_reward(),
            SuperLendyInstruction::WithdrawReward { amount } => self.withdraw_reward(amount),
            SuperLendyInstruction::FlashBorrow { amount } => self.flash_borrow(amount),
            SuperLendyInstruction::FlashRepay { amount } => self.flash_repay(amount),
            SuperLendyInstruction::ProposeConfig { index, proposal } => {
                self.propose_config(index, proposal)
            }
            SuperLendyInstruction::ApplyConfigProposal { index } => {
                self.apply_config_proposal(index)
            }
            SuperLendyInstruction::Version { no_error } => self.version(no_error),
            _ => {
                msg!("Unrecognized IX");
                Err(SuperLendyError::Unimplemented)
            }
        }
    }

    #[inline(never)]
    pub(super) fn create_texture_config(&self, params: TextureConfigParams) -> LendyResult<()> {
        msg!("create_texture_config ix: {:?}", params);

        let CreateTextureConfigAccounts {
            texture_config,
            owner,
        } = CreateTextureConfigAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        if params.performance_fee_rate_bps >= 5000 {
            msg!("performance_fee_rate_bps must be in range [0, 50) %");
            return Err(SuperLendyError::InvalidConfig);
        }

        params.validate()?;

        // Initialize internal structure of the account.
        // Account itself should be already crated (rent exempt) and assigned to SuperLendy
        let mut cfg_data = texture_config.data.borrow_mut();
        TextureConfig::init_bytes(cfg_data.as_mut(), (params, *owner.key))?;

        Ok(())
    }

    #[inline(never)]
    pub(super) fn alter_texture_config(&self, params: TextureConfigParams) -> LendyResult<()> {
        msg!("alter_texture_config ix: {:?}", params);

        let AlterTextureConfigAccounts {
            texture_config,
            owner,
        } = AlterTextureConfigAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let mut cfg_data = texture_config.data.borrow_mut();
        let unpacked_cfg = TextureConfig::try_from_bytes_mut(cfg_data.as_mut())?;

        verify_key(owner.key, &unpacked_cfg.owner, "global config owner")?;

        params.validate()?;

        unpacked_cfg.fees_authority = params.fees_authority;
        unpacked_cfg.performance_fee_rate_bps = params.performance_fee_rate_bps;
        unpacked_cfg.borrow_fee_rate_bps = params.borrow_fee_rate_bps;
        unpacked_cfg.reserve_timelock = params.reserve_timelock;

        Ok(())
    }

    #[inline(never)]
    pub(super) fn transfer_texture_config_ownership(&self) -> LendyResult<()> {
        msg!("transfer_texture_config_ownership ix");

        let TransferTextureConfigOwnershipAccounts {
            texture_config,
            owner,
            new_owner,
        } = TransferTextureConfigOwnershipAccounts::from_iter(
            &mut self.accounts.iter(),
            self.program_id,
        )?;

        let mut cfg_data = texture_config.data.borrow_mut();
        let unpacked_cfg = TextureConfig::try_from_bytes_mut(cfg_data.as_mut())?;

        verify_key(owner.key, &unpacked_cfg.owner, "global config owner")?;

        // in accounts() macro we've checked that new_owner is signer thus we are transferring authority
        // to valid address who can sign.
        unpacked_cfg.owner = *new_owner.key;

        Ok(())
    }

    #[inline(never)]
    pub(super) fn create_pool(&self, params: PoolParams) -> LendyResult<()> {
        msg!("create_pool ix: {:?}", params);

        let CreatePoolAccounts {
            pool,
            curator_pools_authority,
            curator,
        } = CreatePoolAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        // Pools can be created by `pools_authority` from Curator account
        let curator_data = curator.data.borrow();
        let unpacked_curator = Curator::try_from_bytes(curator_data.as_ref())?;

        verify_key(
            curator_pools_authority.key,
            &unpacked_curator.pools_authority,
            "curator_pools_authority",
        )?;

        // Initialize internal structure of the pool account.
        // Account itself should be already crated (rent exempt) and assigned to SuperLendy
        let mut pool_data = pool.data.borrow_mut();
        Pool::init_bytes(pool_data.as_mut(), (params, *curator.key))?;

        Ok(())
    }

    #[inline(never)]
    pub(super) fn alter_pool(&self, params: PoolParams) -> LendyResult<()> {
        msg!("alter_pool ix: {:?}", params);

        let AlterPoolAccounts {
            pool,
            curator_pools_authority,
            curator,
        } = AlterPoolAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        // Pools can be changed by `pools_authority` from Curator account
        let curator_data = curator.data.borrow();
        let unpacked_curator = Curator::try_from_bytes(curator_data.as_ref())?;

        verify_key(
            curator_pools_authority.key,
            &unpacked_curator.pools_authority,
            "curator_pools_authority",
        )?;

        let mut pool_data = pool.data.borrow_mut();
        let unpacked_pool = Pool::try_from_bytes_mut(pool_data.as_mut())?;

        verify_key(curator.key, &unpacked_pool.curator, "curator")?;

        unpacked_pool.name = params.name;
        unpacked_pool.market_price_currency_symbol = params.market_price_currency_symbol;
        unpacked_pool.visible = params.visible;

        Ok(())
    }

    #[inline(never)]
    pub(super) fn version(&self, no_error: bool) -> LendyResult<()> {
        if no_error {
            Ok(())
        } else {
            msg!(
                "SuperLendy contract {}",
                env!("CARGO_PKG_VERSION").to_string()
            );
            Err(OperationCanNotBePerformed)
        }
    }
}

/// Checks that Curator account as in the Pool and checks that curator_pools_authority
/// is correct.
pub fn verify_curator(
    pool: &AccountInfo<'_>,
    curator: &AccountInfo<'_>,
    curator_pools_authority: &AccountInfo<'_>,
) -> LendyResult<()> {
    let pool_data = pool.data.borrow();
    let unpacked_pool = Pool::try_from_bytes(&pool_data)?;

    let curator_data = curator.data.borrow();
    let unpacked_curator = Curator::try_from_bytes(&curator_data)?;

    verify_key(&unpacked_pool.curator, curator.key, "curator vs pool")?;
    verify_key(
        curator_pools_authority.key,
        &unpacked_curator.pools_authority,
        "curator_pools_authority vs curator.pools_authority",
    )?;

    Ok(())
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &SUPER_LENDY_ID {
        msg!("IX in not for Super Lendy but for {}", program_id);
        return Err(ProgramError::IncorrectProgramId);
    }

    Processor::new(program_id, accounts)
        .process_instruction(instruction_data)
        .map_err(|err| {
            msg!("Error: {}", err);
            err.into()
        })
}

/// Transfers `amount` lamports from `from_account` (must be program owned)
/// to another `to_account`. The `to_account` can be owned by anyone else.
pub fn transfer_lamports(
    from_account: &AccountInfo<'_>,
    to_account: &AccountInfo<'_>,
    amount: u64,
) -> LendyResult<()> {
    if **from_account
        .try_borrow_lamports()
        .map_err(|_| SuperLendyError::OperationCanNotBePerformed)?
        < amount
    {
        return Err(SuperLendyError::OperationCanNotBePerformed);
    }

    **from_account
        .try_borrow_mut_lamports()
        .map_err(|_| SuperLendyError::OperationCanNotBePerformed)? -= amount;
    **to_account
        .try_borrow_mut_lamports()
        .map_err(|_| SuperLendyError::OperationCanNotBePerformed)? += amount;

    msg!(
        "transfer_lamports {} from {} to {}",
        amount,
        from_account.key,
        to_account.key
    );

    Ok(())
}

/// SuperLendy can work only with two predefined Token programs
pub fn verify_token_program(token_program: &AccountInfo<'_>) -> LendyResult<()> {
    if token_program.key != &spl_token::id() && token_program.key != &spl_token_2022::id() {
        Err(InvalidKey(error::InvalidKey {
            key_type: "Token program",
            actual: *token_program.key,
            expected: spl_token::id(),
        }))
    } else {
        Ok(())
    }
}

/// Unpacks Mint account of either Token or Token2022 and returns it's `decimals` field
pub fn mint_decimals(mint: &AccountInfo<'_>) -> LendyResult<u8> {
    if mint.owner == &spl_token::id() {
        let unpacked_liquidity_mint = spl_token::state::Mint::unpack(&mint.data.borrow())
            .map_err(|err| SuperLendyError::AccountUnpackError(*mint.key, err))?;
        Ok(unpacked_liquidity_mint.decimals)
    } else if mint.owner == &spl_token_2022::id() {
        let mint_data = mint.data.borrow();
        let unpacked_liquidity_mint =
            StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)
                .map_err(|err| SuperLendyError::AccountUnpackError(*mint.key, err))?;
        Ok(unpacked_liquidity_mint.base.decimals)
    } else {
        Err(InvalidKey(error::InvalidKey {
            key_type: "Token program",
            actual: *mint.owner,
            expected: spl_token::id(),
        }))
    }
}

/// Get mint address of either Token or Token2022 account.
pub fn spl_token_mint(spl_token_account: &AccountInfo<'_>) -> LendyResult<Pubkey> {
    if spl_token_account.owner == &spl_token::id() {
        let acc_data = spl_token_account.data.borrow();
        let unpacked_wallet = spl_token::state::Account::unpack(&acc_data)
            .map_err(|err| SuperLendyError::AccountUnpackError(*spl_token_account.key, err))?;
        Ok(unpacked_wallet.mint)
    } else if spl_token_account.owner == &spl_token_2022::id() {
        let acc_data = spl_token_account.data.borrow();
        let unpacked_wallet =
            StateWithExtensions::<spl_token_2022::state::Account>::unpack(&acc_data)
                .map_err(|err| SuperLendyError::AccountUnpackError(*spl_token_account.key, err))?;
        Ok(unpacked_wallet.base.mint)
    } else {
        Err(InvalidKey(error::InvalidKey {
            key_type: "Token program",
            actual: *spl_token_account.owner,
            expected: spl_token::id(),
        }))
    }
}
