use std::str::FromStr;

use borsh::BorshDeserialize;
use bytemuck::Zeroable;
use curvy::state::curve::Curve;
use mpl_token_metadata::instructions::{
    CreateMetadataAccountV3CpiBuilder, UpdateMetadataAccountV2CpiBuilder,
};
use mpl_token_metadata::types::DataV2;
use price_proxy::state::price_feed::{PriceFeed, QuoteSymbol};
use price_proxy::state::texture_account::PodAccount;
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_program};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, PodStateWithExtensions};
use spl_token_2022::pod::PodMint;
use spl_token_2022::state::Account;
use texture_common::math::Decimal;
use texture_common::remote::system::SystemProgram;
use texture_common::remote::token::SplToken;
use texture_common::utils::verify_key;

use crate::error::SuperLendyError;
use crate::error::SuperLendyError::OperationCanNotBePerformed;
use crate::instruction::{
    AlterReserveAccounts, ApplyConfigProposalAccounts, ClaimCuratorPerformanceFeesAccounts,
    ClaimTexturePerformanceFeesAccounts, CreateReserveAccounts, DeleteReserveAccounts,
    DepositLiquidityAccounts, FlashBorrowAccounts, FlashRepayAccounts, LpTokenMetadata,
    ProposeConfigAccounts, RefreshReserveAccounts, SetLpMetadataAccounts, SuperLendyInstruction,
    WithdrawLiquidityAccounts,
};
use crate::pda::{
    find_collateral_supply, find_liquidity_supply, find_lp_token_mint, find_metadata,
    find_program_authority,
};
use crate::processor::{
    mint_decimals, seedvec, spl_token_mint, verify_curator, verify_token_program, Processor,
    SeedVec,
};
use crate::state::curator::Curator;
use crate::state::last_update::LastUpdate;
use crate::state::pool::Pool;
use crate::state::reserve::{
    ConfigFields, ConfigProposal, Reserve, ReserveCollateral, ReserveConfig, ReserveLiquidity,
    ReserveParams, MAX_CONFIG_PROPOSALS, RESERVE_MODE_NORMAL, RESERVE_MODE_RETAIN_LIQUIDITY,
};
use crate::state::texture_cfg::TextureConfig;
use crate::{pda, LendyResult, MAX_AMOUNT};

impl<'a, 'b> Processor<'a, 'b> {
    pub fn check_price_feed(
        &self,
        market_price_feed: &'a AccountInfo<'b>,
        pool: &'a AccountInfo<'b>,
    ) -> LendyResult<()> {
        let price_feed_data = market_price_feed.data.borrow();
        let unpacked_price_feed = PriceFeed::try_from_bytes(&price_feed_data)?;
        let feed_quote_symbol = unpacked_price_feed.quote_symbol();

        let pool_data = pool.data.borrow();
        let unpacked_pool = Pool::try_from_bytes(&pool_data)?;
        let market_price_currency_symbol =
            String::from_utf8_lossy(&unpacked_pool.market_price_currency_symbol).to_string();
        let market_price_currency_symbol = market_price_currency_symbol.trim_matches(char::from(0));
        let pool_quote_symbol =
            QuoteSymbol::from_str(market_price_currency_symbol).map_err(|err| {
                msg!("pool currency symbol from str: {} symbol", err);
                SuperLendyError::OperationCanNotBePerformed
            })?;

        if pool_quote_symbol != feed_quote_symbol {
            msg!(
                "market price symbol mismatch: symbol from Pool {} is {}, symbol from Feed {} is {}",
                pool.key,
                pool_quote_symbol,
                market_price_feed.key,
                feed_quote_symbol
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        Ok(())
    }

    /// Creates Reserve. The most important property of the reserve is `liquidity_mint`. It can not
    /// be changed thereafter.
    /// LP tokens are ALWAYS of classic token standard (not 2022) and inherits decimals from liquidity
    /// tokens.
    #[inline(never)]
    pub fn create_reserve(&self, config: ReserveConfig, reserve_type: u8) -> LendyResult<()> {
        msg!("create_reserve ix: {:?}", config);

        let CreateReserveAccounts {
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
        } = CreateReserveAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(liquidity_token_program)?;

        let liquidity_decimals = mint_decimals(liquidity_mint)?;

        verify_curator(pool, curator, curator_pools_authority)?;

        verify_key(
            market_price_feed.key,
            &config.market_price_feed,
            "market price feed",
        )?;

        self.check_price_feed(market_price_feed, pool)?;

        let (expected_lp_mint, lp_mint_bump) = find_lp_token_mint(reserve.key);
        verify_key(lp_mint.key, &expected_lp_mint, "LP mint")?;

        let (expected_liquidity_supply, liquidity_supply_bump) = find_liquidity_supply(reserve.key);
        verify_key(
            liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        let (expected_collateral_supply, collateral_supply_bump) =
            find_collateral_supply(reserve.key);
        verify_key(
            collateral_supply.key,
            &expected_collateral_supply,
            "collateral supply",
        )?;

        let (expected_authority, _authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        config.validate()?;

        // liquidity_supply and collateral_supply - are PDAs which needs to be inited by the contract
        let rent = Rent::get().expect("No Rent");
        let system_program = SystemProgram::new(system_program);
        let create_account =
            |account_info: &AccountInfo<'b>, owner: &Pubkey, length, seeds: SeedVec<'_>, bump| {
                let init_lamports = rent.minimum_balance(length);

                let nonce = [bump];
                let mut seeds: SeedVec<'_> = seeds;
                seeds.push(&nonce);

                system_program
                    .create_account(
                        curator_pools_authority,
                        account_info,
                        length as u64,
                        init_lamports,
                        owner,
                    )
                    .signed(&[&seeds])
                    .map_err(SuperLendyError::from)
            };

        let liquidity_supply_data_len = if liquidity_mint.owner == &spl_token_2022::id() {
            let mint_data = liquidity_mint.data.borrow();
            let state = PodStateWithExtensions::<PodMint>::unpack(&mint_data).map_err(|_| {
                msg!("unpack Token2022 mint failed");
                OperationCanNotBePerformed
            })?;

            let mint_extensions = state.get_extension_types().map_err(|_| {
                msg!("get_extension_types for Token2022 failed");
                OperationCanNotBePerformed
            })?;

            let required_extensions =
                ExtensionType::get_required_init_account_extensions(&mint_extensions);

            ExtensionType::try_calculate_account_len::<Account>(&required_extensions).map_err(
                |_| {
                    msg!("try_calculate_account_len failed for Token2022 account");
                    OperationCanNotBePerformed
                },
            )?
        } else if liquidity_mint.owner == &spl_token::id() {
            spl_token::state::Account::LEN
        } else {
            msg!("unrecognized liquidity mint owner {}", liquidity_mint.owner);
            return Err(OperationCanNotBePerformed);
        };

        let reserve_key_bytes = reserve.key.to_bytes();
        let seeds = seedvec![&reserve_key_bytes, pda::LIQUIDITY_SUPPLY_SEED];
        create_account(
            liquidity_supply,
            liquidity_token_program.key,
            liquidity_supply_data_len,
            seeds,
            liquidity_supply_bump,
        )?;

        let spl_token_for_liquidity = SplToken::new(liquidity_token_program);
        spl_token_for_liquidity
            .init_account3(liquidity_supply, liquidity_mint, program_authority)?
            .call()?;

        let spl_token_for_lp = SplToken::new(lp_token_program);
        // Init LP mint - always classic SPL tokens
        let seeds = seedvec![&reserve_key_bytes, pda::LP_TOKEN_SEED];
        create_account(
            lp_mint,
            lp_token_program.key,
            spl_token::state::Mint::LEN,
            seeds,
            lp_mint_bump,
        )?;

        spl_token_for_lp
            .init_mint2(lp_mint, program_authority, liquidity_decimals)?
            .call()?;

        let seeds = seedvec![&reserve_key_bytes, pda::COLLATERAL_SUPPLY_SEED];
        create_account(
            collateral_supply,
            lp_token_program.key,
            spl_token::state::Account::LEN,
            seeds,
            collateral_supply_bump,
        )?;

        spl_token_for_lp
            .init_account3(collateral_supply, lp_mint, program_authority)?
            .call()?;

        let liquidity = ReserveLiquidity::new(*liquidity_mint.key, liquidity_decimals);

        let reserve_params = ReserveParams {
            reserve_type,
            mode: RESERVE_MODE_NORMAL,
            pool: *pool.key,
            liquidity,
            collateral: ReserveCollateral {
                lp_total_supply: 0,
                _padding: 0,
            },
            config,
            flash_loans_enabled: 0, // Flash loans disabled by default
        };

        let clock = Clock::get().expect("no clock");
        let last_update = LastUpdate::new(clock.slot, clock.unix_timestamp);

        let mut reserve_data = reserve.data.borrow_mut();
        Reserve::init_bytes(reserve_data.as_mut(), (reserve_params, last_update))?;

        Ok(())
    }

    #[inline(never)]
    pub fn alter_reserve(
        &self,
        proposed_config: ReserveConfig,
        mode: u8,
        flash_loans_enabled: u8,
    ) -> LendyResult<()> {
        msg!("alter_reserve ix: {:?}", proposed_config);

        let AlterReserveAccounts {
            reserve,
            pool,
            curator_pools_authority,
            curator,
            market_price_feed,
            texture_config,
        } = AlterReserveAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        self.validate_config_change_accounts(
            pool,
            reserve,
            market_price_feed,
            curator,
            curator_pools_authority,
        )?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        let texture_config_data = texture_config.data.borrow();
        let unpacked_texture_config = TextureConfig::try_from_bytes(texture_config_data.as_ref())?;

        proposed_config.validate()?;

        if !unpacked_reserve
            .config
            .can_be_applied_now(&proposed_config, &unpacked_texture_config.reserve_timelock)
        {
            msg!("supplied config has time locked changes. Use ProposeConfig IX instead.");
            return Err(OperationCanNotBePerformed);
        }

        unpacked_reserve.config = proposed_config;
        unpacked_reserve.mode = mode;
        unpacked_reserve.flash_loans_enabled = flash_loans_enabled;

        Ok(())
    }

    #[inline(never)]
    pub fn delete_reserve(&self) -> LendyResult<()> {
        msg!("delete_reserve ix");

        let DeleteReserveAccounts {
            reserve,
            curator_pools_authority,
            curator,
            pool,
        } = DeleteReserveAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_curator(pool, curator, curator_pools_authority)?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;
        verify_key(pool.key, &unpacked_reserve.pool, "pool vs. reserve.pool")?;

        if unpacked_reserve.liquidity.total_liquidity()? != Decimal::ZERO {
            msg!(
                "reserve can't be deleted because its total liquidity is not 0 but {}",
                unpacked_reserve.liquidity.total_liquidity()?
            );
            return Err(OperationCanNotBePerformed);
        }

        let balance = {
            let lamports_data = reserve.lamports.borrow();
            **lamports_data
        };

        crate::processor::transfer_lamports(reserve, curator_pools_authority, balance)?;

        Ok(())
    }

    #[inline(never)]
    pub fn refresh_reserve(&self) -> LendyResult<()> {
        msg!("refresh_reserve ix");

        let RefreshReserveAccounts {
            reserve,
            market_price_feed,
            irm,
            texture_config,
        } = RefreshReserveAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            market_price_feed.key,
            &unpacked_reserve.config.market_price_feed,
            "market price feed",
        )?;

        verify_key(irm.key, &unpacked_reserve.config.irm, "irm")?;

        let irm_data = irm.data.borrow();
        let unpacked_irm = Curve::try_from_bytes(irm_data.as_ref())?;

        let price_feed_data = market_price_feed.data.borrow();
        let unpacked_price_feed = PriceFeed::try_from_bytes(&price_feed_data)?;

        let texture_config_data = texture_config.data.borrow();
        let unpacked_texture_config = TextureConfig::try_from_bytes(&texture_config_data)?;

        let clock = Clock::get().expect("no clock");

        // Check that price is fresh enough
        if unpacked_price_feed.update_slot > clock.slot {
            msg!(
                "Price feed update slot {} is in future compared to current Solana slot {}",
                unpacked_price_feed.update_slot,
                clock.slot
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if clock.unix_timestamp < unpacked_price_feed.update_timestamp {
            msg!(
                "Invalid update_timestamp {} in price feed account. Current Solana time {}",
                unpacked_price_feed.update_timestamp,
                clock.unix_timestamp
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if clock.unix_timestamp - unpacked_price_feed.update_timestamp
            > unpacked_reserve.config.price_stale_threshold_sec as i64
        {
            msg!(
                "Market price age {} sec while threshold is {} sec    clock.unix_timestamp {}   update_timestamp {}",
                clock.unix_timestamp - unpacked_price_feed.update_timestamp,
                unpacked_reserve.config.price_stale_threshold_sec,
                clock.unix_timestamp,
                unpacked_price_feed.update_timestamp
            );
            return Err(SuperLendyError::StaleMarketPrice(
                clock.unix_timestamp - unpacked_price_feed.update_timestamp,
                unpacked_reserve.config.price_stale_threshold_sec,
            ));
        }

        if unpacked_price_feed.try_price()? == Decimal::ZERO {
            msg!(
                "invalid zero market price in feed {}",
                market_price_feed.key
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        unpacked_reserve
            .liquidity
            .set_market_price(unpacked_price_feed.try_price()?)?;

        unpacked_reserve.accrue_interest(
            clock.slot,
            unpacked_texture_config.performance_fee_rate_bps,
            unpacked_irm,
        )?;
        unpacked_reserve
            .last_update
            .update(clock.slot, clock.unix_timestamp);

        msg!("Reserve {} updated in slot {}", reserve.key, clock.slot);

        Ok(())
    }

    /// `amount` - either exact amount or u64::MAX.
    /// When u64::MAX passed - all tokens from specified wallet will be deposited.
    #[inline(never)]
    pub fn deposit_liquidity(&self, amount: u64) -> LendyResult<()> {
        msg!("deposit_liquidity ix: {}", amount);

        let DepositLiquidityAccounts {
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
        } = DepositLiquidityAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        if amount == 0 {
            msg!("amount to deposit must be non zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        verify_token_program(liquidity_token_program)?;

        if source_liquidity_wallet.key == liquidity_supply.key {
            msg!("Source liquidity wallet can not be same as reserve's liquidity supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_lp_mint, _lp_mint_bump) = find_lp_token_mint(reserve.key);
        verify_key(lp_mint.key, &expected_lp_mint, "LP mint")?;

        // Check lp_wallet.mint & liquidity_wallet.mint
        {
            let lp_user_wallet_mint = spl_token_mint(destination_lp_wallet)?;
            verify_key(&lp_user_wallet_mint, &expected_lp_mint, "lp_wallet.mint")?;

            let liquidity_user_wallet_mint = spl_token_mint(source_liquidity_wallet)?;
            verify_key(
                &liquidity_user_wallet_mint,
                liquidity_mint.key,
                "liquidity_wallet.mint",
            )?;
        }

        let (expected_liquidity_supply, _liquidity_supply_bump) =
            find_liquidity_supply(reserve.key);
        verify_key(
            liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;

        let clock = Clock::get().expect("no clock");

        // Reserve must be fresh to ensure that borrowed amount includes latest interest. When calculating
        // LP tokens amount for given liquidity amount we provide a "share" in total liquidity in the pool
        // which consist from available tokens, borrowed tokens and interest.
        if unpacked_reserve.is_stale(&clock)? {
            msg!("update reserve and try again");
            return Err(SuperLendyError::StaleReserve);
        }

        let amount = if amount == MAX_AMOUNT {
            // deposit all tokens from user's wallet
            let unpacked_user_wallet =
                spl_token::state::Account::unpack(&source_liquidity_wallet.data.borrow()).map_err(
                    |err| SuperLendyError::AccountUnpackError(*source_liquidity_wallet.key, err),
                )?;

            unpacked_user_wallet.amount
        } else {
            amount
        };

        let lp_amount = unpacked_reserve.deposit_liquidity(amount)?;
        unpacked_reserve.mark_stale();

        if lp_amount == 0 {
            msg!("deposit is too small and results in zero LP tokens to mint");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        msg!(
            "amount {},  mint_decimals {},  lp_amount {}",
            amount,
            unpacked_reserve.liquidity.mint_decimals,
            lp_amount
        );
        let liquidity_spl_token = SplToken::new(liquidity_token_program);
        liquidity_spl_token
            .transfer(
                source_liquidity_wallet,
                Some(liquidity_mint),
                liquidity_supply,
                authority,
                amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .call()?;

        let lp_spl_token = SplToken::new(lp_token_program);
        lp_spl_token
            .mint_to(lp_mint, program_authority, destination_lp_wallet, lp_amount)?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn withdraw_liquidity(&self, lp_amount: u64) -> LendyResult<()> {
        msg!("withdraw_liquidity ix: {}", lp_amount);

        let WithdrawLiquidityAccounts {
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
        } = WithdrawLiquidityAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(liquidity_token_program)?;

        if lp_amount == 0 {
            msg!("LP amount to withdraw must be non zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if destination_liquidity_wallet.key == liquidity_supply.key {
            msg!("Destination liquidity wallet can not be same as reserve's liquidity supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_lp_mint, _lp_mint_bump) = find_lp_token_mint(reserve.key);
        verify_key(lp_mint.key, &expected_lp_mint, "LP mint")?;

        // Check lp_wallet.mint & liquidity_wallet.mint
        {
            let lp_user_wallet_mint = spl_token_mint(source_lp_wallet)?;
            verify_key(&lp_user_wallet_mint, &expected_lp_mint, "lp_wallet.mint")?;

            let liquidity_user_wallet_mint = spl_token_mint(destination_liquidity_wallet)?;
            verify_key(
                &liquidity_user_wallet_mint,
                liquidity_mint.key,
                "liquidity_wallet.mint",
            )?;
        }

        let (expected_liquidity_supply, _liquidity_supply_bump) =
            find_liquidity_supply(reserve.key);
        verify_key(
            liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        if unpacked_reserve.mode == RESERVE_MODE_RETAIN_LIQUIDITY {
            msg!("reserve do not allow withdrawing");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;

        let clock = Clock::get().expect("no clock");

        // Reserve must be fresh to ensure that borrowed amount includes latest interest.
        if unpacked_reserve.is_stale(&clock)? {
            msg!("update reserve and try again");
            return Err(SuperLendyError::StaleReserve);
        }

        let max_withdraw_lp_amount = unpacked_reserve.max_withdraw_lp_amount()?;

        let lp_amount = if lp_amount == u64::MAX {
            // User wants to withdraw maximum liquidity amount possible.
            // Limitations to consider:
            // 1. Amount of LP tokens user has
            // 2. Withdraw utilization limit of the Reserve
            let unpacked_lp_user_wallet = spl_token::state::Account::unpack(
                &source_lp_wallet.data.borrow(),
            )
            .map_err(|err| SuperLendyError::AccountUnpackError(*source_lp_wallet.key, err))?;

            let user_lp_balance = unpacked_lp_user_wallet.amount;

            msg!("full withdraw. user_lp_balance {}.  reserve's max_withdraw_lp_amount {}  utilization_rate {}  max_withdraw_utilization {}",
                user_lp_balance,
                max_withdraw_lp_amount,
                unpacked_reserve.liquidity.utilization_rate()?,
                unpacked_reserve.config.max_withdraw_utilization_bps
            );

            user_lp_balance.min(max_withdraw_lp_amount)
        } else {
            lp_amount
        };

        let liquidity_amount = unpacked_reserve.withdraw_liquidity(lp_amount)?;

        let utilization_after_withdraw = unpacked_reserve.liquidity.utilization_rate()?;
        let max_withdraw_utilization = texture_common::math::Decimal::from_basis_points(
            unpacked_reserve.config.max_withdraw_utilization_bps as u32,
        )?;
        if utilization_after_withdraw > max_withdraw_utilization {
            msg!("withdraw operation results in utilization {} which is higher than threshold {}. Max allowed withdraw LP amount {}",
                utilization_after_withdraw, max_withdraw_utilization, max_withdraw_lp_amount);
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        unpacked_reserve.mark_stale();

        if lp_amount == 0 {
            msg!("lp_amount to burn is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let lp_spl_token = SplToken::new(lp_token_program);
        lp_spl_token
            .burn(source_lp_wallet, lp_mint, authority, lp_amount)?
            .call()?;

        if liquidity_amount == 0 {
            msg!("liquidity_amount to transfer is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let liquidity_spl_token = SplToken::new(liquidity_token_program);
        liquidity_spl_token
            .transfer(
                liquidity_supply,
                Some(liquidity_mint),
                destination_liquidity_wallet,
                program_authority,
                liquidity_amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn claim_curator_performance_fees(&self) -> LendyResult<()> {
        msg!("claim_curator_performance_fees ix");

        let ClaimCuratorPerformanceFeesAccounts {
            reserve,
            reserve_liquidity_supply,
            pool,
            curator,
            fee_receiver,
            liquidity_mint,
            program_authority,
            token_program,
        } = ClaimCuratorPerformanceFeesAccounts::from_iter(
            &mut self.accounts.iter(),
            self.program_id,
        )?;

        verify_token_program(token_program)?;

        if fee_receiver.key == reserve_liquidity_supply.key {
            msg!("Fee receiver wallet can not be same as reserve's liquidity supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_liquidity_supply, _liquidity_supply_bump) =
            find_liquidity_supply(reserve.key);
        verify_key(
            reserve_liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        let pool_data = pool.data.borrow();
        let unpacked_pool = Pool::try_from_bytes(&pool_data)?;

        let curator_data = curator.data.borrow();
        let unpacked_curator = Curator::try_from_bytes(&curator_data)?;

        verify_key(
            &unpacked_pool.curator,
            curator.key,
            "pool.curator vs. curator",
        )?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;
        verify_key(pool.key, &unpacked_reserve.pool, "pool vs. reserve.pool")?;

        // Curator's fee receiver should be the ATA from curator.fee_authority
        let expected_curator_fee_receiver = get_associated_token_address_with_program_id(
            &unpacked_curator.fees_authority,
            &unpacked_reserve.liquidity.mint,
            token_program.key,
        );

        verify_key(
            fee_receiver.key,
            &expected_curator_fee_receiver,
            "fee_receiver",
        )?;

        let clock = Clock::get().expect("no clock");

        // Reserve must be fresh to ensure that all fees are accrued.
        if unpacked_reserve.is_stale(&clock)? {
            msg!("update reserve and try again");
            return Err(SuperLendyError::StaleReserve);
        }

        let fee_amount = unpacked_reserve.liquidity.claim_curator_performance_fee()?;

        if fee_amount == 0 {
            msg!("fee amount is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let spl_token = SplToken::new(token_program);

        spl_token
            .transfer(
                reserve_liquidity_supply,
                Some(liquidity_mint),
                fee_receiver,
                program_authority,
                fee_amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn claim_texture_performance_fees(&self) -> LendyResult<()> {
        msg!("claim_texture_performance_fees ix");

        let ClaimTexturePerformanceFeesAccounts {
            reserve,
            reserve_liquidity_supply,
            fee_receiver,
            texture_config,
            liquidity_mint,
            program_authority,
            token_program,
        } = ClaimTexturePerformanceFeesAccounts::from_iter(
            &mut self.accounts.iter(),
            self.program_id,
        )?;

        verify_token_program(token_program)?;

        if fee_receiver.key == reserve_liquidity_supply.key {
            msg!("Fee receiver wallet can not be same as reserve's liquidity supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_liquidity_supply, _liquidity_supply_bump) =
            find_liquidity_supply(reserve.key);
        verify_key(
            reserve_liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        let cfg_data = texture_config.data.borrow();
        let unpacked_cfg = TextureConfig::try_from_bytes(cfg_data.as_ref())?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        // Provided `fee_receiver` must be ATA from Texture's fees_authority
        let expected_fees_receiver = get_associated_token_address_with_program_id(
            &unpacked_cfg.fees_authority,
            &unpacked_reserve.liquidity.mint,
            token_program.key,
        );

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;
        verify_key(fee_receiver.key, &expected_fees_receiver, "fee_receiver")?;

        let clock = Clock::get().expect("no clock");

        // Reserve must be fresh to ensure that all fees are accrued.
        if unpacked_reserve.is_stale(&clock)? {
            msg!("update reserve and try again");
            return Err(SuperLendyError::StaleReserve);
        }

        let fee_amount = unpacked_reserve.liquidity.claim_texture_performance_fee()?;

        if fee_amount == 0 {
            msg!("fee amount is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let spl_token = SplToken::new(token_program);

        spl_token
            .transfer(
                reserve_liquidity_supply,
                Some(liquidity_mint),
                fee_receiver,
                program_authority,
                fee_amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn flash_borrow(&self, amount: u64) -> LendyResult<()> {
        msg!("flash_borrow ix");

        let FlashBorrowAccounts {
            reserve,
            liquidity_supply,
            destination_wallet,
            liquidity_mint,
            program_authority,
            sysvar_instructions,
            token_program,
        } = FlashBorrowAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        if amount == 0 {
            msg!("Liquidity amount provided cannot be zero");
            return Err(SuperLendyError::InvalidAmount);
        }

        verify_key(
            sysvar_instructions.key,
            &solana_program::sysvar::instructions::id(),
            "sysvar_instructions",
        )?;

        let (expected_liquidity_supply, _liquidity_supply_bump) =
            find_liquidity_supply(reserve.key);
        verify_key(
            liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        if liquidity_supply.key == destination_wallet.key {
            msg!("liquidity supply cannot be used as destination");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;

        if unpacked_reserve.flash_loans_enabled == 0 {
            msg!("Flash borrow disabled for that reserve.");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let clock = Clock::get().expect("no clock");

        // Reserve must be fresh to ensure that borrow_rate will not be updated between FlashBorrow and
        // FlashRepay. It is important as otherwise there could be huge interests accrued in the Reserve.
        if unpacked_reserve.is_stale(&clock)? {
            msg!("update reserve and try again");
            return Err(SuperLendyError::StaleReserve);
        }

        // Make sure this isn't a cpi call
        let current_index = load_current_index_checked(sysvar_instructions)
            .map_err(SuperLendyError::SysvarError)? as usize;
        let current_ixn = load_instruction_at_checked(current_index, sysvar_instructions)
            .map_err(SuperLendyError::SysvarError)?;
        if current_ixn.program_id != *self.program_id {
            msg!(
                "Cpi call found: flash borrow from {} does not match SuperLendy program_id {} ",
                current_ixn.program_id,
                self.program_id
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        // Loop through instructions, looking for an equivalent repay to this borrow.
        // Always start by looking at next instruction after current_index.
        let mut i = current_index;
        loop {
            i += 1;

            // Get the next instruction, die if there are no more.
            if let Ok(ixn) = load_instruction_at_checked(i, sysvar_instructions) {
                // In order to validate the repay we need to:
                //
                // 1. Ensure the instruction can be unpacked into a SuperLendyInstruction
                // 2. Ensure the instruction is for this program
                // 3. Ensure that the reserve for the repay matches the borrow
                // 4. Ensure that there are no flash borrows between this borrow and the repay for the same reserve
                // 5. Ensure that the repay amount matches the borrow amount
                //
                // If all of these conditions are not met, the flash borrow fails.

                if ixn.program_id != *self.program_id {
                    // If the instruction is not from this program
                    // then we don't care about it, so continue.
                    continue;
                }

                // Attempt to unpack this instruction into a SuperLendyInstruction.
                let unpacked = match SuperLendyInstruction::try_from_slice(ixn.data.as_slice()) {
                    Ok(unpacked) => unpacked,
                    Err(err) => {
                        // If the instruction addressed to SuperLendy but can not be unpacked
                        // throw an error.
                        msg!(
                            "IX at {} addressed to {} but can't be unpacked {}",
                            i,
                            ixn.program_id,
                            err
                        );
                        return Err(SuperLendyError::OperationCanNotBePerformed);
                    }
                };

                match unpacked {
                    SuperLendyInstruction::FlashRepay {
                        amount: repay_liquidity_amount,
                    } => {
                        if ixn.accounts[1].pubkey != *reserve.key {
                            // If the instruction is not for this reserve
                            // then we don't care about it, so continue.
                            msg!("Repay is not for this reserve");
                            continue;
                        }

                        // If borrow amount matches repay amount then
                        // we have successfully verified that the flash
                        // borrow will be repaid. If it does not – then
                        // return an error as this borrow is invalid.
                        if repay_liquidity_amount == amount {
                            break;
                        } else {
                            msg!("Liquidity amount for flash repay doesn't match borrow");
                            return Err(SuperLendyError::OperationCanNotBePerformed);
                        }
                    }
                    SuperLendyInstruction::FlashBorrow { .. } => {
                        if ixn.accounts[0].pubkey != *reserve.key {
                            // If the instruction is not for this reserve
                            // then we don't care about it, so continue.
                            continue;
                        }

                        // Throw an error if we encounter another flash borrow before
                        // encountering a valid flash repay.
                        msg!("Multiple sequential flash borrows not allowed");
                        return Err(SuperLendyError::OperationCanNotBePerformed);
                    }
                    _ => {
                        // Other SuperLendy IXes
                        continue;
                    }
                }
            } else {
                // If no more instructions and a valid repay was never found then
                // return an error.
                msg!("No matching flash repay found");
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }
        }

        unpacked_reserve.liquidity.borrow(
            Decimal::from_lamports(amount, unpacked_reserve.liquidity.mint_decimals)?,
            amount,
        )?;

        // This IX requires the Reserve to be refreshed.
        // We don't mark Reserve as stale after FlashBorrow to avoid needless RefreshReserve IXes
        // after the FlashBorrow and before some other reserve manipulating SuperLendy IXes.
        // We know that before FalshBorrow reserve was refreshed. This happened in the same TX
        // (same slot) as that FlashBorrow handler called. Thus any
        // subsequent RefreshReserve in this TX anyway will not do any useful work
        // (because slot wasn't changed yet).

        let spl_token = SplToken::new(token_program);

        spl_token
            .transfer(
                liquidity_supply,
                Some(liquidity_mint),
                destination_wallet,
                program_authority,
                amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn flash_repay(&self, amount: u64) -> LendyResult<()> {
        msg!("flash_repay ix");

        let FlashRepayAccounts {
            source_wallet,
            reserve,
            liquidity_supply,
            liquidity_mint,
            user_transfer_authority,
            sysvar_instructions,
            token_program,
        } = FlashRepayAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        if amount == 0 {
            msg!("Liquidity amount provided cannot be zero");
            return Err(SuperLendyError::InvalidAmount);
        }

        verify_key(
            sysvar_instructions.key,
            &solana_program::sysvar::instructions::id(),
            "sysvar_instructions",
        )?;

        let (expected_liquidity_supply, _liquidity_supply_bump) =
            find_liquidity_supply(reserve.key);
        verify_key(
            liquidity_supply.key,
            &expected_liquidity_supply,
            "liquidity supply",
        )?;

        if liquidity_supply.key == source_wallet.key {
            msg!("Reserve liquidity supply cannot be used as the source liquidity for repay");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;

        if unpacked_reserve.flash_loans_enabled == 0 {
            msg!("Flash borrow disabled for that reserve.");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        // We do not require Reserve to be refreshed because its already checked in FlashBorrow.

        // Make sure this isn't a cpi call
        let current_index = load_current_index_checked(sysvar_instructions)
            .map_err(SuperLendyError::SysvarError)? as usize;
        let current_ixn = load_instruction_at_checked(current_index, sysvar_instructions)
            .map_err(SuperLendyError::SysvarError)?;
        if current_ixn.program_id != *self.program_id {
            msg!(
                "Cpi call found: flash repay IX program {} does not match SuperLendy program_id {} ",
                current_ixn.program_id,
                self.program_id
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        // Loop through instructions backwards, looking for an equivalent borrow to this repay.
        let mut i = current_index;
        loop {
            i -= 1;

            // Get the prev instruction, die if there are no more.
            if let Ok(ixn) = load_instruction_at_checked(i, sysvar_instructions) {
                // 1. Ensure the instruction can be unpacked into a SuperLendyInstruction
                // 2. Ensure the instruction is for this program
                // 3. Ensure that the reserve for the repay matches the borrow
                // 4. Ensure that the repay amount matches the borrow amount
                //
                // If all of these conditions are not met, the flash repay fails.
                // These checks are necessary to prevent single (without matching FlashBorrow)
                // FlashRepay IX. Otherwise, it's possible to pump liquidity with FlashRepay.

                if ixn.program_id != *self.program_id {
                    // If the instruction is not from this program
                    // then we don't care about it, so continue.
                    continue;
                }

                // Attempt to unpack this instruction into a SuperLendyInstruction.
                let unpacked = match SuperLendyInstruction::try_from_slice(ixn.data.as_slice()) {
                    Ok(unpacked) => unpacked,
                    Err(err) => {
                        // If the instruction addressed to SuperLendy but can not be unpacked
                        // throw an error.
                        msg!(
                            "IX at {} addressed to {} but can't be unpacked {}",
                            i,
                            ixn.program_id,
                            err
                        );
                        return Err(SuperLendyError::OperationCanNotBePerformed);
                    }
                };

                match unpacked {
                    SuperLendyInstruction::FlashRepay { .. } => {
                        if ixn.accounts[1].pubkey != *reserve.key {
                            // If the instruction is not for this reserve
                            // then we don't care about it, so continue.
                            msg!("Repay is not for this reserve");
                            continue;
                        }

                        // Throw an error if we encounter another flash repay before
                        // encountering a valid flash borrow.
                        msg!("Multiple sequential flash repays not allowed");
                        return Err(SuperLendyError::OperationCanNotBePerformed);
                    }
                    SuperLendyInstruction::FlashBorrow {
                        amount: repay_liquidity_amount,
                    } => {
                        if ixn.accounts[0].pubkey != *reserve.key {
                            // If the instruction is not for this reserve
                            // then we don't care about it, so continue.
                            continue;
                        }

                        if repay_liquidity_amount == amount {
                            // We found coupled FlashBorrow
                            break;
                        } else {
                            msg!("Liquidity amount for flash repay doesn't match borrow");
                            return Err(SuperLendyError::OperationCanNotBePerformed);
                        }
                    }
                    _ => {
                        // Other SuperLendy IXes
                        continue;
                    }
                }
            } else {
                // If no more instructions and a valid FlashBorrow was never found then
                // return an error.
                msg!("No matching flash borrow found");
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }
        }

        // Ensure there was coupled FlashBorrow IX. If we allow uncoupled FlashRepay then one could
        // pump liquidity and LP token price in the Reserve which opens possibility for donation
        // attack.

        unpacked_reserve.liquidity.repay(
            amount,
            Decimal::from_lamports(amount, unpacked_reserve.liquidity.mint_decimals)?,
        )?;

        // We do not mark Reserve as stale after FlashRepay because Flash operation as a whole do not
        // change anything in the Reserve. This is virtual operation. No money came and no gone. Even
        // Solana slot wasn't changed.

        let spl_token = SplToken::new(token_program);

        spl_token
            .transfer(
                source_wallet,
                Some(liquidity_mint),
                liquidity_supply,
                user_transfer_authority,
                amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .call()?;

        Ok(())
    }

    #[inline(never)]
    pub fn propose_config(&self, index: u8, proposal: ConfigProposal) -> LendyResult<()> {
        msg!("propose_config ix {}", index);

        let ProposeConfigAccounts {
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
            texture_config,
        } = ProposeConfigAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        self.validate_config_change_accounts(
            pool,
            reserve,
            market_price_feed,
            curator,
            curator_pools_authority,
        )?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        let texture_config_data = texture_config.data.borrow();
        let unpacked_texture_config = TextureConfig::try_from_bytes(texture_config_data.as_ref())?;

        if index as usize >= MAX_CONFIG_PROPOSALS {
            msg!(
                "Proposed config index {} greater than max allowed {}",
                index,
                MAX_CONFIG_PROPOSALS - 1
            );
            return Err(OperationCanNotBePerformed);
        }

        if proposal.change_map == 0 {
            // Means that specified proposal entry must be reset
            unpacked_reserve.proposed_configs.0[index as usize] = ConfigProposal::zeroed();
            return Ok(());
        }

        let mut simulated_config = unpacked_reserve.config;
        simulated_config.apply_proposal(proposal)?;

        simulated_config.validate()?;

        if unpacked_reserve
            .config
            .can_be_applied_now(&simulated_config, &unpacked_texture_config.reserve_timelock)
        {
            msg!("supplied proposal can be applied right away. Use AlterReserve IX instead.");
            return Err(OperationCanNotBePerformed);
        }

        let clock = Clock::get().expect("no clock");

        let max_time_lock = proposal.max_time_lock(&unpacked_texture_config.reserve_timelock)?;

        unpacked_reserve.proposed_configs.0[index as usize].can_be_applied_at =
            clock.unix_timestamp + max_time_lock;
        unpacked_reserve.proposed_configs.0[index as usize].config = proposal.config;
        unpacked_reserve.proposed_configs.0[index as usize].change_map = proposal.change_map;

        Ok(())
    }

    #[inline(never)]
    pub fn apply_config_proposal(&self, index: u8) -> LendyResult<()> {
        msg!("apply_config_proposal ix {}", index);

        let ApplyConfigProposalAccounts {
            reserve,
            pool,
            market_price_feed,
            curator_pools_authority,
            curator,
        } = ApplyConfigProposalAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        // The function also checks that market price feed has same quote currency as pool.
        self.validate_config_change_accounts(
            pool,
            reserve,
            market_price_feed,
            curator,
            curator_pools_authority,
        )?;

        if index as usize >= MAX_CONFIG_PROPOSALS {
            msg!(
                "Proposed config index {} greater than max allowed {}",
                index,
                MAX_CONFIG_PROPOSALS - 1
            );
            return Err(OperationCanNotBePerformed);
        }

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;
        let proposal = unpacked_reserve.proposed_configs.0[index as usize];

        // Ensure that market_price_feed checked above same as in proposed config - but only when we are
        // going to change it.
        let fields =
            ConfigFields::from_bits(unpacked_reserve.proposed_configs.0[index as usize].change_map)
                .ok_or(SuperLendyError::Internal(
                    "change_map conversion".to_string(),
                ))?;

        if fields.contains(ConfigFields::MARKET_PRICE_FEED) {
            verify_key(
                market_price_feed.key,
                &proposal.config.market_price_feed,
                "market_price_feed",
            )?;
        }

        let clock = Clock::get().expect("no clock");

        // New config can change IRM and thus interest accruals. Thus need to accrue interest using
        // old settings and only then apply new settings.
        if fields.contains(ConfigFields::IRM) && unpacked_reserve.is_stale(&clock)? {
            msg!("update reserve and try again");
            return Err(SuperLendyError::StaleReserve);
        }

        if proposal.can_be_applied_at > clock.unix_timestamp {
            msg!(
                "proposal at index {} can be applied at {}. Wait for {} sec and try again.",
                index,
                proposal.can_be_applied_at,
                proposal.can_be_applied_at - clock.unix_timestamp
            );
            return Err(OperationCanNotBePerformed);
        }

        let mut config = unpacked_reserve.config;
        config.apply_proposal(proposal)?;
        config.validate()?;

        unpacked_reserve.config = config;

        // Proposal is cleared to prevent subsequent applications. Can be applied only once.
        unpacked_reserve.proposed_configs.0[index as usize] = ConfigProposal::zeroed();

        Ok(())
    }

    fn validate_config_change_accounts(
        &self,
        pool: &AccountInfo<'b>,
        reserve: &AccountInfo<'b>,
        market_price_feed: &AccountInfo<'b>,
        curator: &AccountInfo<'b>,
        curator_pools_authority: &AccountInfo<'b>,
    ) -> LendyResult<()> {
        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        self.check_price_feed(market_price_feed, pool)?;

        verify_curator(pool, curator, curator_pools_authority)?;

        verify_key(pool.key, &unpacked_reserve.pool, "pool vs. reserve.pool")?;

        Ok(())
    }

    #[inline(never)]
    pub fn set_lp_metadata(&self, data: LpTokenMetadata) -> LendyResult<()> {
        msg!("set_lp_metadata ix {:?}", data);

        let SetLpMetadataAccounts {
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
        } = SetLpMetadataAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let reserve_data = reserve.data.borrow();
        let unpacked_reserve = Reserve::try_from_bytes(reserve_data.as_ref())?;

        verify_curator(pool, curator, curator_pools_authority)?;

        verify_key(pool.key, &unpacked_reserve.pool, "pool vs. reserve.pool")?;

        let (expected_metadata_account, _) = find_metadata(lp_mint.key);
        verify_key(
            metadata_account.key,
            &expected_metadata_account,
            "metadata_account",
        )?;

        let (_expected_authority /* checked by accounts macro*/, authority_bump) =
            find_program_authority();

        let metadata = DataV2 {
            name: data.name,
            symbol: data.symbol,
            uri: data.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        if metadata_account.owner == &system_program::ID {
            msg!("will create new metadata account");

            create_metadata(
                mpl_token_metadata_program,
                program_authority,
                curator_pools_authority,
                lp_mint,
                metadata_account,
                system_program,
                sysvar_rent,
                &metadata,
                &[&[pda::AUTHORITY_SEED, &[authority_bump]]],
            )?;
        } else {
            msg!("will update existing metadata account");
            update_metadata(
                mpl_token_metadata_program,
                program_authority,
                metadata_account,
                &metadata,
                &[&[pda::AUTHORITY_SEED, &[authority_bump]]],
            )?;
        }

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_metadata<'b>(
    metadata_program: &AccountInfo<'b>,
    authority: &AccountInfo<'b>,
    payer: &AccountInfo<'b>,
    mint: &AccountInfo<'b>,
    metadata_acc: &AccountInfo<'b>,
    system_program: &AccountInfo<'b>,
    rent: &AccountInfo<'b>,
    token_metadata: &DataV2,
    signers_seeds: &[&[&[u8]]],
) -> LendyResult<()> {
    let mut cpi_builder = CreateMetadataAccountV3CpiBuilder::new(metadata_program);
    cpi_builder
        .metadata(metadata_acc)
        .mint(mint)
        .mint_authority(authority)
        .payer(payer)
        .update_authority(authority, true)
        .system_program(system_program)
        .rent(Some(rent))
        .is_mutable(true)
        .data(token_metadata.clone());

    cpi_builder
        .invoke_signed(signers_seeds)
        .map_err(SuperLendyError::MetaplexError)
}

pub fn update_metadata<'b>(
    metadata_program: &AccountInfo<'b>,
    authority: &AccountInfo<'b>,
    metadata_acc: &AccountInfo<'b>,
    token_metadata: &DataV2,
    signers_seeds: &[&[&[u8]]],
) -> LendyResult<()> {
    let mut cpi_builder = UpdateMetadataAccountV2CpiBuilder::new(metadata_program);
    cpi_builder
        .metadata(metadata_acc)
        .update_authority(authority)
        .data(token_metadata.clone());

    cpi_builder
        .invoke_signed(signers_seeds)
        .map_err(SuperLendyError::MetaplexError)
}
