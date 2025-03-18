use solana_program::clock::Clock;
use solana_program::msg;
use solana_program::program_pack::Pack;
use solana_program::sysvar::Sysvar;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::collections::HashSet;
use texture_common::account::PodAccount;
use texture_common::error;
use texture_common::math::{CheckedAdd, CheckedDiv, CheckedMul, Decimal, MathError};
use texture_common::remote::token::SplToken;
use texture_common::utils::verify_key;

use crate::error::SuperLendyError;
use crate::error::SuperLendyError::InvalidAmount;
use crate::instruction::{
    BorrowAccounts, ClosePositionAccounts, CreatePositionAccounts, LiquidateAccounts,
    LockCollateralAccounts, RefreshPositionAccounts, RepayAccounts, UnlockCollateralAccounts,
    WriteOffBadDebtAccounts,
};
use crate::pda::{
    find_collateral_supply, find_liquidity_supply, find_lp_token_mint, find_program_authority,
};
use crate::processor::{spl_token_mint, verify_curator, verify_token_program, Processor};
use crate::state::curator::Curator;
use crate::state::pool::Pool;
use crate::state::position::{
    InitPositionParams, Position, BORROW_MEMO_LEN, COLLATERAL_MEMO_LEN, POSITION_TYPE_CLASSIC,
    POSITION_TYPE_LONG_SHORT, POSITION_TYPE_LST_LEVERAGE,
};
use crate::state::reserve::{
    CalculateBorrowResult, CalculateLiquidationResult, CalculateRepayResult, Reserve,
    RESERVE_MODE_BORROW_DISABLED, RESERVE_MODE_RETAIN_LIQUIDITY, RESERVE_TYPE_NOT_A_COLLATERAL,
    RESERVE_TYPE_PROTECTED_COLLATERAL,
};
use crate::state::reserve::{REWARD_FOR_BORROW, REWARD_FOR_LIQUIDITY};
use crate::state::texture_cfg::TextureConfig;
use crate::{pda, LendyResult, MAX_AMOUNT};

impl<'a, 'b> Processor<'a, 'b> {
    #[inline(never)]
    pub fn create_position(&self, position_type: u8) -> LendyResult<()> {
        msg!("create_position ix");

        let CreatePositionAccounts {
            position,
            pool,
            owner,
        } = CreatePositionAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        // Unpack the pool just to make sure its really a pool and not some other type SuperLendy account.
        let pool_data = pool.data.borrow();
        let unpacked_pool = Pool::try_from_bytes(pool_data.as_ref())?;

        if position_type != POSITION_TYPE_CLASSIC
            && position_type != POSITION_TYPE_LONG_SHORT
            && position_type != POSITION_TYPE_LST_LEVERAGE
        {
            msg!("Invalid position_type {}", position_type);
            return Err(SuperLendyError::InvalidConfig);
        }

        msg!(
            "Init position {}  for pool {}  position_type {}",
            position.key,
            String::from_utf8_lossy(&unpacked_pool.name),
            position_type
        );

        let position_params = InitPositionParams {
            position_type,
            pool: *pool.key,
            owner: *owner.key,
        };

        // Account itself must be already created (rent exempt) and assigned to Super Lendy
        let mut position_data = position.data.borrow_mut();
        Position::init_bytes(position_data.as_mut(), position_params)?;

        Ok(())
    }

    #[inline(never)]
    pub fn close_position(&self) -> LendyResult<()> {
        msg!("close_position ix");

        let ClosePositionAccounts { position, owner } =
            ClosePositionAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let position_data = position.data.borrow();
        let unpacked_position = Position::try_from_bytes(position_data.as_ref())?;

        verify_key(owner.key, &unpacked_position.owner, "position owner")?;

        if let Some(reason) = unpacked_position.closable() {
            msg!("Position can't be closed because {}", reason);
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let balance = {
            let lamports_data = position.lamports.borrow();
            **lamports_data
        };

        crate::processor::transfer_lamports(position, owner, balance)?;

        Ok(())
    }

    #[inline(never)]
    pub fn refresh_position(&self, deposit_count: usize, borrow_count: usize) -> LendyResult<()> {
        msg!("refresh_position ix");

        let clock = Clock::get().expect("no clock");

        let mut account_info_iter = self.accounts.iter().peekable();

        let RefreshPositionAccounts {
            position: position_info,
            deposits: deposit_infos,
            borrows: borrow_infos,
        } = RefreshPositionAccounts::from_iter(
            &mut account_info_iter,
            deposit_count,
            borrow_count,
            self.program_id,
        )?;

        if account_info_iter.peek().is_some() {
            msg!("Too many position deposit or borrow reserves provided");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }
        drop(account_info_iter);

        let mut position_data = position_info.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;
        let mut rewards = position.rewards;

        let mut deposited_value = Decimal::ZERO;
        let mut borrowed_value = Decimal::ZERO;
        let mut allowed_borrow_value = Decimal::ZERO;
        let mut partly_unhealthy_borrow_value = Decimal::ZERO;
        let mut fully_unhealthy_borrow_value = Decimal::ZERO;

        let mut touched_rewards_records = HashSet::new();

        let mut deposit_infos_iter = deposit_infos.into_iter();
        for (index, collateral) in position.collateral.iter_mut().enumerate() {
            if collateral.deposited_amount == 0 {
                // Means that this is unused collateral record. Should be ignored.
                continue;
            }

            let deposit_reserve_info =
                deposit_infos_iter
                    .next()
                    .ok_or(SuperLendyError::NotEnoughAccountKeys(
                        error::NotEnoughAccountKeys,
                    ))?;

            if collateral.deposit_reserve != *deposit_reserve_info.key {
                msg!(
                    "Deposit reserve of collateral {} does not match the deposit reserve provided",
                    index
                );
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }

            let reserve_data = deposit_reserve_info.data.borrow();
            let deposit_reserve = Reserve::try_from_bytes(&reserve_data)?;
            if deposit_reserve.is_stale(&clock)? {
                msg!(
                    "Deposit reserve provided for collateral {} is stale and must be refreshed",
                    index
                );
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }

            let market_value = deposit_reserve
                .lp_exchange_rate()?
                .decimal_lp_to_liquidity(Decimal::from_lamports(
                    collateral.deposited_amount,
                    deposit_reserve.liquidity.mint_decimals,
                )?)?
                .checked_mul(deposit_reserve.liquidity.market_price()?)?;
            collateral.set_market_value(market_value)?;

            let max_borrow_ltv =
                Decimal::from_basis_points(deposit_reserve.config.max_borrow_ltv_bps as u32)?;
            let partial_liquidation_ltv =
                Decimal::from_basis_points(deposit_reserve.config.partly_unhealthy_ltv_bps as u32)?;
            let full_liquidation_ltv =
                Decimal::from_basis_points(deposit_reserve.config.fully_unhealthy_ltv_bps as u32)?;

            deposited_value = deposited_value.checked_add(market_value)?;

            // This is value of various tokens from that pool which User can borrow until his position will become
            // eligible for partial liquidation.
            allowed_borrow_value =
                allowed_borrow_value.checked_add(market_value.checked_mul(max_borrow_ltv)?)?;

            // Borrow value at which User position become eligible for partial liquidations.
            partly_unhealthy_borrow_value = partly_unhealthy_borrow_value
                .checked_add(market_value.checked_mul(partial_liquidation_ltv)?)?;

            // Borrow value at which User position become eligible for full liquidation at once.
            fully_unhealthy_borrow_value = fully_unhealthy_borrow_value
                .checked_add(market_value.checked_mul(full_liquidation_ltv)?)?;

            // Accrue rewards
            let rewards_records = rewards.accrue_rewards(
                REWARD_FOR_LIQUIDITY,
                Decimal::from_lamports(
                    collateral.deposited_amount,
                    deposit_reserve.liquidity.mint_decimals,
                )?,
                &deposit_reserve.reward_rules,
                clock.slot,
            );

            match rewards_records {
                Ok(touched_records) => {
                    touched_rewards_records.extend(touched_records);
                }
                Err(err) => {
                    msg!(
                        "error occurred while accruing rewards for liquidity: {}",
                        err
                    );
                    // do not revert TX. Do not stop contract operation just because of rewards.
                }
            }
        }

        let mut borrow_infos_iter = borrow_infos.into_iter();
        for (index, borrowed_liquidity) in position.borrows.iter_mut().enumerate() {
            if borrowed_liquidity.borrowed_amount()? == Decimal::ZERO {
                // Means that this is unused liquidity record. Should be ignored.
                continue;
            }

            let borrow_reserve_info =
                borrow_infos_iter
                    .next()
                    .ok_or(SuperLendyError::NotEnoughAccountKeys(
                        error::NotEnoughAccountKeys,
                    ))?;

            if borrowed_liquidity.borrow_reserve != *borrow_reserve_info.key {
                msg!(
                    "Borrow reserve of liquidity {} does not match the borrow reserve provided",
                    index
                );
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }

            let reserve_data = borrow_reserve_info.data.borrow();
            let borrow_reserve = Reserve::try_from_bytes(&reserve_data)?;
            if borrow_reserve.is_stale(&clock)? {
                msg!(
                    "Borrow reserve provided for liquidity {} is stale and must be refreshed",
                    index
                );
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }

            borrowed_liquidity
                .accrue_interest(borrow_reserve.liquidity.cumulative_borrow_rate()?)?;

            let market_value = borrowed_liquidity
                .borrowed_amount()?
                .checked_mul(borrow_reserve.liquidity.market_price()?)?;

            borrowed_liquidity.set_market_value(market_value)?;

            borrowed_value = borrowed_value.checked_add(market_value)?;

            // Accrue rewards
            let rewards_records = rewards.accrue_rewards(
                REWARD_FOR_BORROW,
                borrowed_liquidity.borrowed_amount()?,
                &borrow_reserve.reward_rules,
                clock.slot,
            );

            match rewards_records {
                Ok(touched_records) => {
                    touched_rewards_records.extend(touched_records);
                }
                Err(err) => {
                    msg!("error occurred while accruing rewards for borrow: {}", err);
                    // do not revert TX. Do not stop contract operation just because of rewards.
                }
            }
        }

        position.set_deposited_value(deposited_value)?;
        position.set_borrowed_value(borrowed_value)?;
        position.set_allowed_borrow_value(allowed_borrow_value)?;
        position.set_partly_unhealthy_borrow_value(partly_unhealthy_borrow_value)?;
        position.set_fully_unhealthy_borrow_value(fully_unhealthy_borrow_value)?;

        rewards.set_accrued_slot(clock.slot, touched_rewards_records);
        position.rewards = rewards;

        position
            .last_update
            .update(clock.slot, clock.unix_timestamp);

        Ok(())
    }

    #[inline(never)]
    pub fn lock_collateral(&self, amount: u64, memo: [u8; COLLATERAL_MEMO_LEN]) -> LendyResult<()> {
        msg!("lock_collateral ix: {}", amount);

        if amount == 0 {
            msg!("Collateral amount to lock cannot be zero");
            return Err(InvalidAmount);
        }

        let LockCollateralAccounts {
            position,
            source_lp_wallet,
            reserve_collateral_supply,
            owner,
            reserve,
            lp_token_program,
        } = LockCollateralAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        if reserve_collateral_supply.key == source_lp_wallet.key {
            msg!("reserve_collateral_supply should not point to source_lp_wallet");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let mut position_data = position.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;

        let reserve_data = reserve.data.borrow();
        let unpacked_reserve = Reserve::try_from_bytes(&reserve_data)?;

        // Check that position and reserve belongs to the same pool
        if position.pool != unpacked_reserve.pool {
            msg!("Position and reserve belongs to different pools");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let clock = Clock::get().expect("no clock");
        if unpacked_reserve.is_stale(&clock)? {
            msg!(
                "Reserve {} is stale and must be refreshed prior to lock collateral",
                reserve.key
            );
            return Err(SuperLendyError::StaleReserve);
        }

        // Position needs to be refreshed to accrue rewards. Because after increase of the locked
        // collateral rewards base will change.
        if position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed");
            return Err(SuperLendyError::StalePosition);
        }

        verify_key(owner.key, &position.owner, "position owner")?;

        let expected_collateral_supply = find_collateral_supply(reserve.key);
        verify_key(
            reserve_collateral_supply.key,
            &expected_collateral_supply.0,
            "reserve collateral supply",
        )?;

        if unpacked_reserve.reserve_type == RESERVE_TYPE_NOT_A_COLLATERAL {
            msg!("Reserve do not allow collateral lock. This Reserve for borrowing only.");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let collateral = position.find_or_add_collateral(*reserve.key)?;

        let lp_amount_to_lock = if amount == MAX_AMOUNT {
            // lock all tokens from user's wallet
            let unpacked_source_lp_wallet = spl_token::state::Account::unpack(
                &source_lp_wallet.data.borrow(),
            )
            .map_err(|err| SuperLendyError::AccountUnpackError(*source_lp_wallet.key, err))?;

            unpacked_source_lp_wallet.amount
        } else {
            amount
        };

        collateral.deposit(
            lp_amount_to_lock,
            unpacked_reserve.lp_market_price()?,
            unpacked_reserve.liquidity.mint_decimals,
        )?;
        collateral.memo = memo;

        position.mark_stale();

        let spl_token = SplToken::new(lp_token_program);
        spl_token
            .transfer(
                source_lp_wallet,
                None,
                reserve_collateral_supply,
                owner,
                lp_amount_to_lock,
                None,
            )?
            .call()?;

        Ok(())
    }

    #[inline(never)]
    pub fn unlock_collateral(&self, amount: u64) -> LendyResult<()> {
        msg!("unlock_collateral ix: {}", amount);

        if amount == 0 {
            msg!("Collateral amount to unlock cannot be zero");
            return Err(InvalidAmount);
        }

        let UnlockCollateralAccounts {
            position,
            reserve_collateral_supply,
            destination_lp_wallet,
            owner,
            reserve,
            program_authority,
            lp_token_program,
        } = UnlockCollateralAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        if reserve_collateral_supply.key == destination_lp_wallet.key {
            msg!("reserve_collateral_supply should not point to destination_lp_wallet");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        let mut position_data = position.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;

        let reserve_data = reserve.data.borrow();
        let unpacked_reserve = Reserve::try_from_bytes(&reserve_data)?;

        if unpacked_reserve.mode == RESERVE_MODE_RETAIN_LIQUIDITY {
            msg!("reserve do not allow unlocking collateral");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        // Check that position and reserve belongs to the same pool
        if position.pool != unpacked_reserve.pool {
            msg!("Position and reserve belongs to different pools");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let clock = Clock::get().expect("no clock");
        if unpacked_reserve.is_stale(&clock)? {
            msg!(
                "Reserve {} is stale and must be refreshed prior to unlock collateral",
                reserve.key
            );
            return Err(SuperLendyError::StaleReserve);
        }

        // Position needs to be refreshed to accrue rewards. Because after decrease of the locked
        // collateral rewards base will change.
        // Also its needed to update collateral.market_value
        if position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed");
            return Err(SuperLendyError::StalePosition);
        }

        verify_key(owner.key, &position.owner, "position owner")?;

        let expected_collateral_supply = find_collateral_supply(reserve.key);
        verify_key(
            reserve_collateral_supply.key,
            &expected_collateral_supply.0,
            "reserve collateral supply",
        )?;

        let (collateral, collateral_index) = position.find_collateral(*reserve.key)?;
        if collateral.deposited_amount == 0 {
            msg!("Collateral deposited amount is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if position.deposited_value()? == Decimal::ZERO {
            msg!("Position deposited value is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let withdraw_amount = if !position.have_any_borrowings() {
            collateral.deposited_amount.min(amount)
        } else {
            // Determine how much collateral (in value terms) we can let go
            let max_withdraw_value = position.max_withdraw_value(Decimal::from_basis_points(
                unpacked_reserve.config.max_borrow_ltv_bps as u32,
            )?)?;

            if max_withdraw_value == Decimal::ZERO {
                msg!("Maximum withdraw value is zero");
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }

            let withdraw_amount = if amount == MAX_AMOUNT {
                // user want's to unlock maximum it can. Calculate how much collateral we can unlock
                // based on max allowed withdraw_value.
                // Example: collateral is 1 SOL, market_price = 100$,
                // max_withdraw_value = 90$ (because there is some borrowings).
                // Then max collateral contract allow to unlock is 90/100 = 0.9 SOL
                // Also need to take into account decimals.
                let max_unlockable_collateral = max_withdraw_value
                    .checked_div(unpacked_reserve.liquidity.market_price()?)?
                    .to_lamports_floor(unpacked_reserve.liquidity.mint_decimals)?;

                max_unlockable_collateral.min(collateral.deposited_amount)
            } else {
                let withdraw_amount = amount.min(collateral.deposited_amount);
                let withdraw_pct =
                    Decimal::from_i128_with_scale(withdraw_amount as i128, 0)?.checked_div(
                        Decimal::from_i128_with_scale(collateral.deposited_amount as i128, 0)?,
                    )?; // Simple math here, thus from_i128_with_scale
                let withdraw_value = collateral.market_value()?.checked_mul(withdraw_pct)?;
                if withdraw_value > max_withdraw_value {
                    msg!(
                        "Withdraw value {} cannot exceed maximum withdraw value {}",
                        withdraw_value,
                        max_withdraw_value
                    );
                    return Err(SuperLendyError::OperationCanNotBePerformed);
                }
                withdraw_amount
            };

            if withdraw_amount == 0 {
                msg!("Withdraw amount is too small to transfer");
                return Err(SuperLendyError::OperationCanNotBePerformed);
            }
            withdraw_amount
        };

        position.withdraw(withdraw_amount, collateral_index)?;
        position.mark_stale();

        msg!(
            "transfer {} LPs to user's wallet {}",
            withdraw_amount,
            destination_lp_wallet.key
        );

        let spl_token = SplToken::new(lp_token_program);
        spl_token
            .transfer(
                reserve_collateral_supply,
                None,
                destination_lp_wallet,
                program_authority,
                withdraw_amount,
                None,
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn borrow(
        &self,
        amount: u64,
        slippage_limit: u64,
        memo: [u8; BORROW_MEMO_LEN],
    ) -> LendyResult<()> {
        msg!("borrow ix: {}", amount);

        if amount == 0 {
            msg!("Amount to borrow cannot be zero");
            return Err(InvalidAmount);
        }

        let BorrowAccounts {
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
        } = BorrowAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        if destination_liquidity_wallet.key == reserve_liquidity_supply.key
            || destination_liquidity_wallet.key == curator_fee_receiver.key
            || destination_liquidity_wallet.key == texture_fee_receiver.key
        {
            msg!("destination_liquidity_wallet must be external to the Super Lendy");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let pool_data = pool.data.borrow();
        let unpacked_pool = Pool::try_from_bytes(&pool_data)?;

        let curator_data = curator.data.borrow();
        let unpacked_curator = Curator::try_from_bytes(&curator_data)?;

        verify_key(
            &unpacked_pool.curator,
            curator.key,
            "pool.curator vs. curator",
        )?;

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        let mut position_data = position.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;

        if unpacked_reserve.reserve_type == RESERVE_TYPE_PROTECTED_COLLATERAL
            || unpacked_reserve.mode == RESERVE_MODE_BORROW_DISABLED
            || unpacked_reserve.mode == RESERVE_MODE_RETAIN_LIQUIDITY
        {
            msg!("reserve do not allow borrowing");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let texture_config_data = texture_config.data.borrow();
        let unpacked_texture_config = TextureConfig::try_from_bytes(&texture_config_data)?;

        // Curator's fee receiver should the ATA from curator.fee_authority
        let expected_curator_fee_receiver = get_associated_token_address_with_program_id(
            &unpacked_curator.fees_authority,
            &unpacked_reserve.liquidity.mint,
            token_program.key,
        );

        verify_key(
            curator_fee_receiver.key,
            &expected_curator_fee_receiver,
            "curator_fee_receiver",
        )?;

        let expected_texture_fee_receiver = get_associated_token_address_with_program_id(
            &unpacked_texture_config.fees_authority,
            &unpacked_reserve.liquidity.mint,
            token_program.key,
        );

        verify_key(
            &unpacked_reserve.pool,
            pool.key,
            "pool from reserve doesn't match pool provided",
        )?;

        verify_key(
            texture_fee_receiver.key,
            &expected_texture_fee_receiver,
            "texture_fee_receiver",
        )?;

        // Check that position and reserve belongs to the same pool
        if position.pool != unpacked_reserve.pool {
            msg!("Position and reserve belongs to different pools");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        //Reserve checks
        let clock = Clock::get().expect("no clock");
        if unpacked_reserve.is_stale(&clock)? {
            msg!("Reserve {} is stale and must be refreshed prior to borrow. Last updated on {}. Current slot {}", reserve.key, unpacked_reserve.last_update.slot, clock.slot);
            return Err(SuperLendyError::StaleReserve);
        }

        let expected_liquidity_supply = find_liquidity_supply(reserve.key);
        verify_key(
            reserve_liquidity_supply.key,
            &expected_liquidity_supply.0,
            "reserve liquidity supply",
        )?;

        // Position checks
        verify_key(borrower.key, &position.owner, "position owner")?;

        if position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed prior to borrow");
            return Err(SuperLendyError::StalePosition);
        }

        if !position.have_any_deposits() {
            msg!("Position has no deposits to borrow against");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if position.deposited_value()? == Decimal::ZERO {
            msg!("Position deposits have zero value");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let remaining_borrow_value = position.remaining_borrow_value()?;
        if remaining_borrow_value == Decimal::ZERO {
            msg!("Remaining borrow value is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let borrow_value = if amount == MAX_AMOUNT {
            let max_borrow_value = unpacked_reserve.max_borrow_value()?;
            msg!(
                "max_borrow_value = {}    remaining_borrow_value = {} ",
                max_borrow_value,
                remaining_borrow_value
            );
            remaining_borrow_value.min(max_borrow_value)
        } else {
            remaining_borrow_value
        };

        let CalculateBorrowResult {
            borrow_amount,
            receive_amount,
            curator_borrow_fee,
            texture_borrow_fee,
        } = unpacked_reserve.calculate_borrow(
            amount,
            borrow_value,
            unpacked_texture_config.borrow_fee_rate_bps,
        )?;

        if receive_amount == 0 {
            msg!("Borrow amount is too small to receive liquidity after fees");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if amount == MAX_AMOUNT && receive_amount < slippage_limit {
            msg!("Received liquidity would be smaller than the desired slippage limit");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let borrowed_lamports = receive_amount
            .checked_add(curator_borrow_fee)
            .ok_or(SuperLendyError::MathError(MathError(format!(
                "borrow(): checked_add {} + {}",
                receive_amount, curator_borrow_fee
            ))))?
            .checked_add(texture_borrow_fee)
            .ok_or(SuperLendyError::MathError(MathError(format!(
                "borrow(): checked_add {} + {} + {}",
                receive_amount, curator_borrow_fee, texture_borrow_fee
            ))))?;

        unpacked_reserve
            .liquidity
            .borrow(borrow_amount, borrowed_lamports)?;
        unpacked_reserve.mark_stale();

        if unpacked_reserve.liquidity.utilization_rate()?
            > Decimal::from_basis_points(unpacked_reserve.config.max_borrow_utilization_bps as u32)?
        {
            msg!(
                "Borrow results in utilization rate {} which is greater then threshold {}",
                unpacked_reserve.liquidity.utilization_rate()?,
                Decimal::from_basis_points(
                    unpacked_reserve.config.max_borrow_utilization_bps as u32
                )?
            );
            return Err(SuperLendyError::ResourceExhausted);
        }

        let borrowed_liquidity = position.find_or_add_borrowed_liquidity(
            *reserve.key,
            unpacked_reserve.liquidity.cumulative_borrow_rate()?,
        )?;

        borrowed_liquidity.borrow(borrow_amount, unpacked_reserve.liquidity.market_price()?)?;
        borrowed_liquidity.memo = memo;

        position.mark_stale();

        let spl_token = SplToken::new(token_program);

        if curator_borrow_fee > 0 {
            spl_token
                .transfer(
                    reserve_liquidity_supply,
                    Some(liquidity_mint),
                    curator_fee_receiver,
                    program_authority,
                    curator_borrow_fee,
                    Some(unpacked_reserve.liquidity.mint_decimals),
                )?
                .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

            msg!("curator's borrow fee {}", curator_borrow_fee);
        }

        if texture_borrow_fee > 0 {
            spl_token
                .transfer(
                    reserve_liquidity_supply,
                    Some(liquidity_mint),
                    texture_fee_receiver,
                    program_authority,
                    texture_borrow_fee,
                    Some(unpacked_reserve.liquidity.mint_decimals),
                )?
                .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

            msg!("texture's borrow fee {}", texture_borrow_fee);
        }

        spl_token
            .transfer(
                reserve_liquidity_supply,
                Some(liquidity_mint),
                destination_liquidity_wallet,
                program_authority,
                receive_amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        msg!("borrowed {}", receive_amount);

        Ok(())
    }

    #[inline(never)]
    pub fn repay(&self, amount: u64) -> LendyResult<()> {
        msg!("repay ix: {}", amount);

        if amount == 0 {
            msg!("Amount to repay cannot be zero");
            return Err(InvalidAmount);
        }

        let RepayAccounts {
            position,
            source_liquidity_wallet,
            reserve_liquidity_supply,
            user_authority,
            reserve,
            liquidity_mint,
            token_program,
        } = RepayAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        let mut position_data = position.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        verify_key(
            liquidity_mint.key,
            &unpacked_reserve.liquidity.mint,
            "liquidity mint",
        )?;

        // Check that source_liquidity_wallet is external to the contract. Practically it is not necessary
        // as user_authority ensures that it is something external. But just to validate input and give
        // caller better clue about what is wrong in accounts...
        if source_liquidity_wallet.key == reserve_liquidity_supply.key {
            msg!("source_liquidity_wallet can not be reserve_liquidity_supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        // Check that position and reserve belongs to the same pool
        if position.pool != unpacked_reserve.pool {
            msg!("Position and reserve belongs to different pools");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        //Reserve checks
        let clock = Clock::get().expect("no clock");
        if unpacked_reserve.is_stale(&clock)? {
            msg!("Reserve is stale and must be refreshed prior to repay");
            return Err(SuperLendyError::StaleReserve);
        }

        let expected_liquidity_supply = find_liquidity_supply(reserve.key);
        verify_key(
            reserve_liquidity_supply.key,
            &expected_liquidity_supply.0,
            "reserve liquidity supply",
        )?;

        if position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed prior to repay");
            return Err(SuperLendyError::StalePosition);
        }

        if !position.have_any_borrowings() {
            msg!("Position has no borrowings");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (borrowed_liquidity, liquidity_index) =
            position.find_borrowed_liquidity(*reserve.key)?;

        if borrowed_liquidity.borrowed_amount()? == Decimal::ZERO {
            msg!("Liquidity borrowed amount is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let CalculateRepayResult {
            settle_amount, /* this value used in internal contract math to account repay */
            repay_amount, /* this is approximated (ceiled) amount will be transferred from Borrower to Reserve supply */
        } = unpacked_reserve.calculate_repay(amount, borrowed_liquidity.borrowed_amount()?)?;

        if repay_amount == 0 {
            msg!("Repay amount is too small to transfer liquidity");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        unpacked_reserve
            .liquidity
            .repay(repay_amount, settle_amount)?;
        unpacked_reserve.mark_stale();

        position.repay(settle_amount, liquidity_index)?;
        position.mark_stale();

        let spl_token = SplToken::new(token_program);

        spl_token
            .transfer(
                source_liquidity_wallet,
                Some(liquidity_mint),
                reserve_liquidity_supply,
                user_authority,
                repay_amount,
                Some(unpacked_reserve.liquidity.mint_decimals),
            )?
            .call()?;

        Ok(())
    }

    #[inline(never)]
    pub fn liquidate(&self, liquidity_amount: u64) -> LendyResult<()> {
        msg!("liquidate ix: {}", liquidity_amount);

        if liquidity_amount == 0 {
            msg!("Amount to liquidate cannot be zero");
            return Err(InvalidAmount);
        }

        let LiquidateAccounts {
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
        } = LiquidateAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(principal_token_program)?;

        let mut position_data = position.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;

        let mut principal_reserve_data = principal_reserve.data.borrow_mut();
        let unpacked_principal_reserve =
            Reserve::try_from_bytes_mut(principal_reserve_data.as_mut())?;

        verify_key(
            principal_reserve_liquidity_mint.key,
            &unpacked_principal_reserve.liquidity.mint,
            "principal reserve liquidity mint",
        )?;

        // Check lp_wallet.mint & liquidity_wallet.mint
        {
            let lp_user_wallet_mint = spl_token_mint(destination_lp_wallet)?;
            let (lp_mint, _lp_mint_bump) = find_lp_token_mint(collateral_reserve.key);
            verify_key(&lp_user_wallet_mint, &lp_mint, "lp_wallet.mint")?;

            let liquidity_user_wallet_mint = spl_token_mint(repayment_source_wallet)?;
            verify_key(
                &liquidity_user_wallet_mint,
                principal_reserve_liquidity_mint.key,
                "liquidity_wallet.mint",
            )?;
        }

        let collateral_reserve_data = collateral_reserve.data.borrow();
        let unpacked_collateral_reserve =
            Reserve::try_from_bytes(collateral_reserve_data.as_ref())?;

        // Check that repayment_source_wallet and destination_lp_wallet are external to the contract
        // as it supposed to be.
        if repayment_source_wallet.key == principal_reserve_liquidity_supply.key
            || repayment_source_wallet.key == collateral_reserve_lp_supply.key
        {
            msg!("repayment_source_wallet should not be neither principal_reserve_liquidity_supply nor collateral_reserve_lp_supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if destination_lp_wallet.key == principal_reserve_liquidity_supply.key
            || repayment_source_wallet.key == collateral_reserve_lp_supply.key
        {
            msg!("destination_lp_wallet should not be neither principal_reserve_liquidity_supply nor collateral_reserve_lp_supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_authority, authority_bump) = find_program_authority();
        verify_key(
            program_authority.key,
            &expected_authority,
            "program authority",
        )?;

        // Check that position and both reserve belongs to the same pool
        if position.pool != unpacked_principal_reserve.pool {
            msg!("Position and principal reserve belongs to different pools");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }
        if position.pool != unpacked_collateral_reserve.pool {
            msg!("Position and collateral reserve belongs to different pools");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        //Reserves checks
        let clock = Clock::get().expect("no clock");
        if unpacked_principal_reserve.is_stale(&clock)? {
            msg!("Principal Reserve is stale and must be refreshed");
            return Err(SuperLendyError::StaleReserve);
        }

        if unpacked_collateral_reserve.is_stale(&clock)? {
            msg!("Collateral Reserve is stale and must be refreshed");
            return Err(SuperLendyError::StaleReserve);
        }

        let expected_liquidity_supply = find_liquidity_supply(principal_reserve.key);
        verify_key(
            principal_reserve_liquidity_supply.key,
            &expected_liquidity_supply.0,
            "principal_reserve_liquidity_supply",
        )?;

        let expected_lp_supply = find_collateral_supply(collateral_reserve.key);
        verify_key(
            collateral_reserve_lp_supply.key,
            &expected_lp_supply.0,
            "collateral_reserve_lp_supply",
        )?;

        if position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed");
            return Err(SuperLendyError::StalePosition);
        }

        if position.deposited_value()? == Decimal::ZERO {
            msg!("Position has no deposited value");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if position.borrowed_value()? == Decimal::ZERO {
            msg!("Position has no borrowed value");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        if position.borrowed_value()? < position.partly_unhealthy_borrow_value()? {
            msg!("Position is healthy and cannot be liquidated");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (borrowed_liquidity, borrowed_liquidity_index) =
            position.find_borrowed_liquidity(*principal_reserve.key)?;

        if borrowed_liquidity.market_value()? == Decimal::ZERO {
            msg!("Position's borrowed value is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (collateral, collateral_index) = position.find_collateral(*collateral_reserve.key)?;

        // When collateral market value is zero it means that Reserve have bad debt. It's up to Curator
        // to decide when to write off bad debt via WriteOffBadDebt
        if collateral.market_value()? == Decimal::ZERO {
            msg!("Position's deposit value is zero");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let CalculateLiquidationResult {
            settle_amount,
            repay_amount,
            withdraw_amount,
        } = unpacked_collateral_reserve.calculate_liquidation(
            liquidity_amount,
            position,
            borrowed_liquidity,
            collateral,
            unpacked_principal_reserve.liquidity.mint_decimals,
        )?;

        if repay_amount == 0 {
            msg!("Liquidation is too small to transfer liquidity");
            return Err(SuperLendyError::LiquidationTooSmall);
        }
        if withdraw_amount == 0 {
            msg!("Liquidation is too small to receive collateral");
            return Err(SuperLendyError::LiquidationTooSmall);
        }

        unpacked_principal_reserve
            .liquidity
            .repay(repay_amount, settle_amount)?;
        unpacked_principal_reserve.mark_stale();

        position.repay(settle_amount, borrowed_liquidity_index)?;
        position.withdraw(withdraw_amount, collateral_index)?;
        position.mark_stale();

        let principal_spl_token = SplToken::new(principal_token_program);

        principal_spl_token
            .transfer(
                repayment_source_wallet,
                Some(principal_reserve_liquidity_mint),
                principal_reserve_liquidity_supply,
                liquidator,
                repay_amount,
                Some(unpacked_principal_reserve.liquidity.mint_decimals),
            )?
            .call()?;

        let collateral_spl_token = SplToken::new(collateral_token_program);
        collateral_spl_token
            .transfer(
                collateral_reserve_lp_supply,
                None,
                destination_lp_wallet,
                program_authority,
                withdraw_amount,
                None,
            )?
            .signed(&[&[pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }

    #[inline(never)]
    pub fn write_off_bad_debt(&self, amount: u64) -> LendyResult<()> {
        msg!("write_off_bad_debt ix");

        if amount == 0 {
            msg!("Amount to write off cannot be zero");
            return Err(InvalidAmount);
        }

        let WriteOffBadDebtAccounts {
            pool,
            reserve,
            position,
            curator_pools_authority,
            curator,
        } = WriteOffBadDebtAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        let mut position_data = position.data.borrow_mut();
        let unpacked_position = Position::try_from_bytes_mut(position_data.as_mut())?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        if unpacked_position.pool != unpacked_reserve.pool || unpacked_position.pool != *pool.key {
            msg!("Position, reserve and provided pool do not match");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        verify_curator(pool, curator, curator_pools_authority)?;

        let clock = Clock::get().expect("no clock");
        if unpacked_position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed");
            return Err(SuperLendyError::StalePosition);
        }

        if unpacked_reserve.is_stale(&clock)? {
            msg!("Reserve is stale and must be refreshed");
            return Err(SuperLendyError::StaleReserve);
        }

        if unpacked_position.deposited_value()? != Decimal::ZERO {
            msg!("Position has {} deposited value. Liquidate it first and then try to write off bad debt again.", unpacked_position.deposited_value()?);
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        unpacked_position
            .find_borrowed_liquidity_mut(*reserve.key)?
            .write_off_bad_debt(amount, unpacked_reserve.liquidity.mint_decimals)?;
        unpacked_position.mark_stale();

        unpacked_reserve.liquidity.write_off_bad_debt(amount)?;
        unpacked_reserve.mark_stale();

        Ok(())
    }
}
