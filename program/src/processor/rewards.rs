use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::msg;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, PodStateWithExtensions};
use spl_token_2022::pod::PodMint;
use spl_token_2022::state::Account;
use texture_common::account::PodAccount;
use texture_common::remote::system::SystemProgram;
use texture_common::remote::token::SplToken;
use texture_common::utils::verify_key;

use crate::error::SuperLendyError;
use crate::error::SuperLendyError::OperationCanNotBePerformed;
use crate::instruction::{
    ClaimRewardAccounts, InitRewardSupplyAccounts, SetRewardRulesAccounts, WithdrawRewardAccounts,
};
use crate::pda;
use crate::pda::{find_reward_supply, find_rewards_program_authority};
use crate::processor::{mint_decimals, seedvec, verify_token_program, SeedVec};
use crate::state::position::Position;
use crate::state::reserve::{Reserve, RewardRule, RewardRules};
use crate::LendyResult;

impl<'a, 'b> crate::processor::Processor<'a, 'b> {
    #[inline(never)]
    pub fn init_reward_supply(&self) -> LendyResult<()> {
        msg!("init_reward_supply ix");

        let InitRewardSupplyAccounts {
            reward_supply,
            reward_mint,
            pool,
            curator_pools_authority,
            curator,
            reward_authority,
            token_program,
            system_program,
        } = InitRewardSupplyAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        if reward_mint.owner != &spl_token::id() && reward_mint.owner != &spl_token_2022::id() {
            msg!(
                "unrecognized token program for reward mint {}",
                reward_mint.owner
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        crate::processor::verify_curator(pool, curator, curator_pools_authority)?;

        let (expected_reward_supply, reward_supply_bump) =
            find_reward_supply(pool.key, reward_mint.key);

        verify_key(reward_supply.key, &expected_reward_supply, "reward_supply")?;

        let (expected_authority, _authority_bump) = find_rewards_program_authority(pool.key);
        verify_key(
            reward_authority.key,
            &expected_authority,
            "reward authority",
        )?;

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

        let rewards_supply_data_len = if reward_mint.owner == &spl_token_2022::id() {
            let mint_data = reward_mint.data.borrow();
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
        } else if reward_mint.owner == &spl_token::id() {
            spl_token::state::Account::LEN
        } else {
            msg!("unrecognized reward mint owner {}", reward_mint.owner);
            return Err(OperationCanNotBePerformed);
        };

        let pool_key_bytes = pool.key.to_bytes();
        let mint_key_bytes = reward_mint.key.to_bytes();
        let seeds = seedvec![&pool_key_bytes, &mint_key_bytes, pda::REWARD_SUPPLY_SEED];
        create_account(
            reward_supply,
            token_program.key,
            rewards_supply_data_len,
            seeds,
            reward_supply_bump,
        )?;

        let spl_token = SplToken::new(token_program);
        spl_token
            .init_account3(reward_supply, reward_mint, reward_authority)?
            .call()?;

        Ok(())
    }

    #[inline(never)]
    pub fn set_reward_rules(&self, mints_count: usize, rules: RewardRules) -> LendyResult<()> {
        msg!("set_reward_rules ix");

        let SetRewardRulesAccounts {
            reserve,
            pool,
            curator_pools_authority,
            curator,
            reward_mints,
        } = SetRewardRulesAccounts::from_iter(
            &mut self.accounts.iter(),
            mints_count,
            self.program_id,
        )?;

        let mut reserve_data = reserve.data.borrow_mut();
        let unpacked_reserve = Reserve::try_from_bytes_mut(reserve_data.as_mut())?;

        crate::processor::verify_curator(pool, curator, curator_pools_authority)?;

        verify_key(pool.key, &unpacked_reserve.pool, "pool vs. reserve.pool")?;

        if reward_mints.len() != rules.rules.len() {
            msg!(
                "provided {} mints while {} expected",
                reward_mints.len(),
                rules.rules.len()
            );
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        // Verify new rules one-by-one
        for (new_rule, rule_mint) in rules.rules.iter().zip(reward_mints) {
            if !new_rule.is_active() {
                // Do not check deactivated rules.
                continue;
            }

            new_rule.verify(rule_mint)?;
        }

        let clock = Clock::get().expect("no clock");

        // Copy rules one-by-one in to live config
        for (new_rule, existing_rule) in rules
            .rules
            .iter()
            .zip(unpacked_reserve.reward_rules.rules.iter_mut())
        {
            if !new_rule.is_active() {
                // Just make existing rule not active too
                *existing_rule = RewardRule::default();
            } else if new_rule != existing_rule {
                // Copy new rule and set start slot to a current time
                *existing_rule = *new_rule;
                existing_rule.start_slot = clock.slot;
            }
        }
        unpacked_reserve.reward_rules = rules;

        Ok(())
    }

    #[inline(never)]
    pub fn claim_reward(&self) -> LendyResult<()> {
        msg!("claim_reward ix");

        let ClaimRewardAccounts {
            position,
            rewards_supply,
            destination_wallet,
            position_owner,
            pool,
            reward_mint,
            reward_authority,
            token_program,
        } = ClaimRewardAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        if rewards_supply.key == destination_wallet.key {
            msg!("Destination wallet can not be same as reward supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_authority, authority_bump) = find_rewards_program_authority(pool.key);
        verify_key(
            reward_authority.key,
            &expected_authority,
            "reward authority",
        )?;

        let mut position_data = position.data.borrow_mut();
        let position = Position::try_from_bytes_mut(position_data.as_mut())?;

        verify_key(&position.pool, pool.key, "position.pool vs. pool")?;

        verify_key(
            position_owner.key,
            &position.owner,
            "position_owner vs. position.owner",
        )?;

        let expected_rewards_supply = find_reward_supply(pool.key, reward_mint.key);
        verify_key(
            rewards_supply.key,
            &expected_rewards_supply.0,
            "rewards_supply",
        )?;

        let clock = Clock::get().expect("no clock");
        if position.is_stale(&clock)? {
            msg!("Position is stale and must be refreshed");
            return Err(SuperLendyError::StalePosition);
        }

        let reward_record = position.rewards.find_reward(reward_mint.key);

        if reward_record.is_none() {
            msg!("No rewards found for mint {}", reward_mint.key);
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let reward_record = reward_record.unwrap();

        let decimals = mint_decimals(reward_mint)?;

        let claimed_amount = reward_record.1.claim(decimals)?;

        let spl_token = SplToken::new(token_program);
        spl_token
            .transfer(
                rewards_supply,
                Some(reward_mint),
                destination_wallet,
                reward_authority,
                claimed_amount,
                Some(decimals),
            )?
            .signed(&[&[&pool.key.to_bytes(), pda::AUTHORITY_SEED, &[authority_bump]]])?;

        msg!("claimed {}", claimed_amount);

        Ok(())
    }

    #[inline(never)]
    pub fn withdraw_reward(&self, amount: u64) -> LendyResult<()> {
        msg!("withdraw_reward ix");

        let WithdrawRewardAccounts {
            rewards_supply,
            destination_wallet,
            pool,
            curator_pools_authority,
            curator,
            reward_mint,
            reward_authority,
            token_program,
        } = WithdrawRewardAccounts::from_iter(&mut self.accounts.iter(), self.program_id)?;

        verify_token_program(token_program)?;

        if rewards_supply.key == destination_wallet.key {
            msg!("Destination wallet can not be same as reward supply");
            return Err(SuperLendyError::OperationCanNotBePerformed);
        }

        let (expected_authority, authority_bump) = find_rewards_program_authority(pool.key);
        verify_key(
            reward_authority.key,
            &expected_authority,
            "program authority",
        )?;

        crate::processor::verify_curator(pool, curator, curator_pools_authority)?;

        let decimals = mint_decimals(reward_mint)?;

        // We know at that point that curator authority is OK i.e. it corresponds to pool provided.
        // Also rewards_supply corresponds to the pool provided (checked by accounts macro).
        // Thus Curator can withdraw any amount of rewards from that rewards_supply

        let spl_token = SplToken::new(token_program);
        spl_token
            .transfer(
                rewards_supply,
                Some(reward_mint),
                destination_wallet,
                reward_authority,
                amount,
                Some(decimals),
            )?
            .signed(&[&[&pool.key.to_bytes(), pda::AUTHORITY_SEED, &[authority_bump]]])?;

        Ok(())
    }
}
