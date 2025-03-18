use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use bytemuck::Zeroable;
use derive_more::Display;
use hex::ToHex;
use price_proxy::state::price_feed::{
    FeedType, PriceFeed, PriceFeedSource, WormholeVerificationLevel,
};
use price_proxy::state::utils::str_to_array;
use price_proxy_client::PriceProxyClient;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use solana_account_decoder::UiAccountData;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::{RpcError, RpcResponseErrorData, TokenAccountsFilter};
use solana_client::rpc_response::RpcSimulateTransactionResult;
use solana_sdk::address_lookup_table::instruction::{create_lookup_table, extend_lookup_table};
use solana_sdk::address_lookup_table::state::AddressLookupTable;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;
use solana_sdk::{bs58, nonce, system_instruction};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token::solana_program;
use spl_token_2022::extension::StateWithExtensions;
use spl_token_2022::state::Mint;
use texture_common::account::PodAccount;
use texture_common::math::{CheckedAdd, CheckedMul, Decimal};
use tokio::time::sleep;

use crate::position_generator::gen_unhealthy_positions;
use super_lendy::instruction::{
    AlterCurator, AlterPool, AlterReserve, AlterTextureConfig, ApplyConfigProposal, Borrow,
    ClaimCuratorPerformanceFees, ClaimReward, ClaimTexturePerformanceFees, ClosePosition,
    CreateCurator, CreatePool, CreatePosition, CreateReserve, CreateTextureConfig, DeleteReserve,
    DepositLiquidity, FlashBorrow, FlashRepay, InitRewardSupply, Liquidate, LockCollateral,
    ProposeConfig, RefreshPosition, RefreshReserve, Repay, SetRewardRules,
    TransferTextureConfigOwnership, UnlockCollateral, Version, WithdrawLiquidity, WithdrawReward,
    WriteOffBadDebt,
};
use super_lendy::pda::{
    find_collateral_supply, find_liquidity_supply, find_lp_token_mint, find_program_authority,
    find_reward_supply, find_rewards_program_authority,
};
use super_lendy::state::curator::{
    Curator, CuratorParams, CURATOR_LOGO_URL_MAX_LEN, CURATOR_NAME_MAX_LEN,
    CURATOR_WEBSITE_URL_MAX_LEN,
};
use super_lendy::state::pool::{Pool, PoolParams, CURRENCY_SYMBOL_MAX_LEN, POOL_NAME_MAX_LEN};
use super_lendy::state::position::{
    Position, BORROW_MEMO_LEN, COLLATERAL_MEMO_LEN, POSITION_TYPE_CLASSIC,
    POSITION_TYPE_LONG_SHORT, POSITION_TYPE_LST_LEVERAGE,
};
use super_lendy::state::reserve::{
    ConfigFields, ConfigProposal, LpExchangeRate, Reserve, ReserveConfig, MAX_REWARD_RULES,
    RESERVE_MODE_BORROW_DISABLED, RESERVE_MODE_NORMAL, RESERVE_MODE_RETAIN_LIQUIDITY,
    RESERVE_TYPE_NORMAL, RESERVE_TYPE_NOT_A_COLLATERAL, RESERVE_TYPE_PROTECTED_COLLATERAL,
    REWARD_FOR_BORROW, REWARD_FOR_LIQUIDITY, REWARD_RULE_NAME_MAX_LEN,
};
use super_lendy::state::texture_cfg::{TextureConfig, TextureConfigParams};
use super_lendy::state::{SCALE, WAD};
use super_lendy::{MAX_AMOUNT, SUPER_LENDY_ID, TEXTURE_CONFIG_ID};
use utils::loaders::{load_curators, load_pools, load_positions, load_reserves};

pub struct App {
    pub rpc: RpcClient,
    pub url: String,
    pub authority: Keypair,
    pub priority_fee: Option<u64>,
    pub multisig: Option<Pubkey>,
}

impl App {
    pub async fn send_transaction_by(
        &self,
        mut ixs: Vec<Instruction>,
        signers: &impl Signers,
    ) -> Result<Signature> {
        if let Some(priority_fee) = self.priority_fee {
            let priority_fee_ix = ComputeBudgetInstruction::set_compute_unit_price(priority_fee);
            ixs.push(priority_fee_ix);
        }
        let mut tx = Transaction::new_with_payer(ixs.as_ref(), Some(&self.authority.pubkey()));
        let blockhash = self.rpc.get_latest_blockhash().await?;

        tx.sign(signers, blockhash);

        let signature = self
            .rpc
            .send_and_confirm_transaction_with_spinner(&tx)
            .await
            .map_err(with_logs)?;

        println!("Signature: {}", signature);
        Ok(signature)
    }

    pub async fn process_transaction(
        &self,
        ixs: Vec<Instruction>,
        signers: &impl Signers,
        num_retries: usize,
    ) -> Result<Signature> {
        let mut consecutive_errors = 0;
        loop {
            match self.send_transaction_by(ixs.clone(), signers).await {
                Ok(signature) => return Ok(signature),
                Err(err) => {
                    println!(
                        "{}'nd attempt to send tx. {:#?} ",
                        consecutive_errors,
                        err.source().unwrap()
                    );
                    consecutive_errors += 1;
                    if consecutive_errors > num_retries {
                        return Err(err);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Prepares TX which will be valid any time in future and prints it Base58 encoded.
    /// `additional_signer` - usually newly created keypair of account. There will be one
    /// permanent signer - multisig authority. So when only it needed - leave additional_signer empty.
    pub async fn prepare_immortal_tx(
        &self,
        ixs: Vec<Instruction>,
        additional_signer: Option<&Keypair>,
    ) {
        let main_message = Message::new(&ixs, Some(&self.authority.pubkey()));

        let nonce_account = Keypair::new(); // Аккаунт, который будет хранить durable nonce

        let rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(80)
            .await
            .unwrap();

        let create_nonce_account_ixs = system_instruction::create_nonce_account(
            &self.authority.pubkey(),
            &nonce_account.pubkey(),
            &self.authority.pubkey(),
            rent,
        );

        self.send_transaction_by(create_nonce_account_ixs, &[&self.authority])
            .await
            .expect("ups...");

        println!("Durable Nonce Account Created: {}", nonce_account.pubkey());

        sleep(Duration::from_secs(5)).await;

        let nonce_data = self
            .rpc
            .get_account(&nonce_account.pubkey())
            .await
            .expect("Failed to fetch nonce account data");

        let nonce_state = match bincode::deserialize::<nonce::state::Versions>(&nonce_data.data) {
            Ok(nonce_versions) => match nonce_versions {
                nonce::state::Versions::Legacy(nonce_state) => match *nonce_state {
                    solana_sdk::nonce::state::State::Uninitialized => {
                        panic!("Nonce not initted");
                    }
                    solana_sdk::nonce::state::State::Initialized(_state) => _state,
                },
                _ => panic!("Unknown verstion of Nonce account"),
            },
            Err(err) => {
                panic!("deser error: {:?}", err);
            }
        };

        println!("Durable Nonce: {}", nonce_state.durable_nonce.as_hash());
        println!("Durable Nonce authority: {}", nonce_state.authority);

        let tx = if let Some(additional_signer) = additional_signer {
            Transaction::new(
                &[&self.authority, additional_signer],
                main_message,
                *nonce_state.durable_nonce.as_hash(),
            )
        } else {
            Transaction::new(
                &[&self.authority],
                main_message,
                *nonce_state.durable_nonce.as_hash(),
            )
        };

        let serialized_tx = bincode::serialize(&tx).expect("Failed to serialize transaction");
        let base58_tx = bs58::encode(serialized_tx).into_string();

        println!("Serialized Transaction (Base58): {}", base58_tx);
    }

    pub async fn account_exists(&self, key: &Pubkey) -> Result<bool> {
        match self.rpc.get_account(key).await {
            Ok(_) => Ok(true),
            Err(ClientError {
                kind: ClientErrorKind::RpcError(RpcError::ForUser(msg)),
                ..
            }) if msg.starts_with("AccountNotFound") => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    /// Creates lookup tables and extends them with accounts useful within given Pool.
    /// One LUT is created for each Pool.
    /// For each Pool we put in to LUT:
    /// 1. Pool address - this is always first address in the LUT
    /// 2. Curator address
    /// 3. program_authority - PDA used in many IXes
    /// 4. Token Program
    /// 5. Token2022 Program
    /// All reserves, mentioned in the Pool become source of accounts for the LUT.
    /// For each Reserve we put in to LUT:
    /// 1. Reserve address
    /// 2. Reserve's liquidity mint
    /// 3. Reserve's LP mint
    /// 4. Reserve's liquidity supply
    /// 5. Reserve's collateral supply
    /// Roughly one LUT can hold info for 50 Reserves - more that enough.
    /// `current_lut_config` - current LUT config.
    pub async fn create_or_update_luts(
        &self,
        current_lut_config: Vec<LutCfgEntry>,
    ) -> Vec<LutCfgEntry> {
        let pools = load_pools(&self.rpc).await.expect("loading pools");
        let reserves = load_reserves(&self.rpc).await.expect("loading reserves");

        let mut out_config: Vec<LutCfgEntry> = Vec::new();

        for (pool_key, pool) in pools {
            println!(
                "=============== Processing pool {} ==================",
                pool_key
            );

            let that_pool_reserves: HashMap<Pubkey, Reserve> = reserves
                .iter()
                .filter(|(_r_key, r)| r.pool == pool_key)
                .map(|(k, v)| (*k, *v))
                .collect();

            if let Some(existing_lut) = current_lut_config.iter().find(|e| e.pool == pool_key) {
                self.try_update_lut(existing_lut, &that_pool_reserves, 5)
                    .await
                    .map_err(|err| {
                        println!("Error while updating LUT for Pool {}: {}", pool_key, err)
                    })
                    .ok();
            } else {
                match self
                    .create_lut(&pool_key, &pool, &that_pool_reserves, 5)
                    .await
                {
                    Ok(table_key) => {
                        out_config.push(LutCfgEntry {
                            pool: pool_key,
                            lut: table_key,
                        });
                    }
                    Err(err) => {
                        println!("Error while creating LUT for Pool {}: {}", pool_key, err);
                    }
                }
            }
        }

        // Sort LUT entries by pool key so config file stays stable in its structure
        out_config.sort_by(|a, b| a.pool.to_string().cmp(&b.pool.to_string()));

        out_config
    }

    pub async fn send_tx_with_retries(
        &self,
        ixs: Vec<Instruction>,
        signers: &impl Signers,
        num_retries: usize,
    ) -> Result<Signature> {
        let mut cnt = 0;

        loop {
            match self.send_transaction_by(ixs.clone(), signers).await {
                Ok(signature) => {
                    return Ok(signature);
                }
                Err(err) => {
                    println!("error sending TX: {}", err);
                    sleep(Duration::from_secs(1)).await;
                    cnt += 1;

                    if cnt >= num_retries {
                        return Err(anyhow!("retries limit {} reached", num_retries));
                    }
                }
            }
        }
    }

    pub async fn create_lut(
        &self,
        pool_key: &Pubkey,
        pool: &Pool,
        reserves: &HashMap<Pubkey, Reserve>,
        num_retries: usize,
    ) -> Result<Pubkey> {
        let (create_table_ix, table_address) = create_lookup_table(
            self.authority.pubkey(),
            self.authority.pubkey(),
            self.rpc.get_slot().await.unwrap(),
        );

        sleep(Duration::from_secs(3)).await;

        self.send_tx_with_retries(vec![create_table_ix], &[&self.authority], num_retries)
            .await?;

        // Put common info for whole Pool
        let addresses = vec![
            *pool_key,
            pool.curator,
            find_program_authority().0,
            spl_token::id(),
            spl_token_2022::id(),
        ];

        let extend_lut = extend_lookup_table(
            table_address,
            self.authority.pubkey(),
            Some(self.authority.pubkey()),
            addresses,
        );

        self.send_tx_with_retries(vec![extend_lut], &[&self.authority], num_retries)
            .await?;
        println!("✅ LUT {} added common Pool addresses", table_address);

        for (reserve_key, reserve) in reserves {
            let mut addresses = Vec::new();

            let (lp_token_mint, _) = find_lp_token_mint(reserve_key);
            let (liquidity_supply, _) = find_liquidity_supply(reserve_key);
            let (collateral_supply, _) = find_collateral_supply(reserve_key);

            addresses.push(*reserve_key);
            addresses.push(lp_token_mint);
            addresses.push(reserve.liquidity.mint);
            addresses.push(liquidity_supply);
            addresses.push(collateral_supply);

            let extend_lut = extend_lookup_table(
                table_address,
                self.authority.pubkey(),
                Some(self.authority.pubkey()),
                addresses,
            );

            self.send_tx_with_retries(vec![extend_lut], &[&self.authority], num_retries)
                .await?;

            println!("✅ LUT {} added reserve {}", table_address, reserve_key);
        }

        println!("Created LUT: {} for Pool {}", table_address, pool_key);

        Ok(table_address)
    }

    pub async fn try_update_lut(
        &self,
        existing_lut: &LutCfgEntry,
        reserves: &HashMap<Pubkey, Reserve>,
        num_retries: usize,
    ) -> Result<()> {
        match self.rpc.get_account(&existing_lut.lut).await {
            Ok(account) => match AddressLookupTable::deserialize(&account.data) {
                Ok(lut) => {
                    let mut lut_updated = false;
                    for (reserve_key, reserve) in reserves {
                        if lut.addresses.iter().any(|addr| addr == reserve_key) {
                            continue;
                        }

                        let mut addresses = Vec::new();

                        let (lp_token_mint, _) = find_lp_token_mint(reserve_key);
                        let (liquidity_supply, _) = find_liquidity_supply(reserve_key);
                        let (collateral_supply, _) = find_collateral_supply(reserve_key);

                        addresses.push(*reserve_key);
                        addresses.push(lp_token_mint);
                        addresses.push(reserve.liquidity.mint);
                        addresses.push(liquidity_supply);
                        addresses.push(collateral_supply);

                        let extend_lut = extend_lookup_table(
                            existing_lut.lut,
                            self.authority.pubkey(),
                            Some(self.authority.pubkey()),
                            addresses,
                        );

                        self.send_tx_with_retries(
                            vec![extend_lut],
                            &[&self.authority],
                            num_retries,
                        )
                        .await?;

                        println!("✅ LUT {} added reserve {}", existing_lut.lut, reserve_key);
                        lut_updated = true;
                    }

                    if !lut_updated {
                        println!("✅ LUT {} is up to date", existing_lut.lut);
                    }
                }
                Err(e) => println!("🚨 LUT deser: {:?}", e),
            },
            Err(e) => println!("🚨 getting LUT account: {:?}", e),
        }

        Ok(())
    }

    pub async fn show_lut(&self, lut_addr: &Pubkey) {
        match self.rpc.get_account(lut_addr).await {
            Ok(account) => match AddressLookupTable::deserialize(&account.data) {
                Ok(lut) => {
                    println!("LUT at address {}", lut_addr);
                    println!("Authority         : {:?}", lut.meta.authority);
                    println!("Deactivation slot : {}", lut.meta.deactivation_slot);
                    println!("Last extended slot: {}", lut.meta.last_extended_slot);

                    for (i, address) in lut.addresses.iter().enumerate() {
                        println!("{}: {}", i + 1, address);
                    }
                }
                Err(e) => println!("🚨 LUT deser: {:?}", e),
            },
            Err(e) => println!("🚨 getting LUT account: {:?}", e),
        }
    }

    pub async fn make_fees_ata(&self, curator_key: &Pubkey) {
        let curators = load_curators(&self.rpc).await.expect("loading curators");
        let curator = curators
            .get(curator_key)
            .expect("no curator with specified key in Solana");

        // Pools with given curator
        let pools: Vec<Pubkey> = load_pools(&self.rpc)
            .await
            .expect("loading pools")
            .iter()
            .filter_map(|(k, v)| {
                if v.curator == *curator_key {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect();

        // Liquidity mints from all reserves curated by given curator with non zero fees
        let mut mints: Vec<Pubkey> = load_reserves(&self.rpc)
            .await
            .expect("loading reserves")
            .iter()
            .filter_map(|(_k, v)| {
                if pools.iter().any(|curators_pool| v.pool == *curators_pool) {
                    Some(v)
                } else {
                    None
                }
            }) // reserves which belongs to Curator
            .filter_map(|reserve| {
                if reserve.config.fees.curator_performance_fee_rate_bps != 0
                    || reserve.config.fees.curator_borrow_fee_rate_bps != 0
                {
                    Some(reserve.liquidity.mint)
                } else {
                    None
                }
            })
            .collect();

        mints.sort();
        mints.dedup();

        for mint in mints.iter() {
            let token_program = self.token_program_by_mint(mint).await;

            let wallet = get_associated_token_address_with_program_id(
                &curator.fees_authority,
                mint,
                &token_program,
            );

            if !self.account_exists(&wallet).await.unwrap() {
                println!("Will create ATA wallet {} for mint {}", wallet, mint);

                let ix = create_associated_token_account(
                    &self.authority.pubkey(),
                    &curator.fees_authority,
                    mint,
                    &token_program,
                );

                self.send_transaction_by(vec![ix], &vec![&self.authority])
                    .await
                    .expect("Sending create ATA TX");
            }
        }
    }

    pub async fn contract_version(&self) {
        let ix = Version { no_error: false }.into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Read logs below to see the contract version");
    }

    pub async fn create_texture_config(&self, params: TextureConfigParams, global_cfg: Keypair) {
        let create_ix = system_instruction::create_account(
            &self.authority.pubkey(),
            &global_cfg.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(Pool::SIZE)
                .await
                .expect("getting rent"),
            TextureConfig::SIZE as u64,
            &SUPER_LENDY_ID,
        );

        let ix = CreateTextureConfig {
            owner: self.authority.pubkey(),
            params,
        }
        .into_instruction();

        self.send_transaction_by(vec![create_ix, ix], &[&self.authority, &global_cfg])
            .await
            .expect("Sending TX");

        println!("Created global config: {}", global_cfg.pubkey());
    }

    pub async fn transfer_texture_config_ownership(&self, new_authority: Keypair) {
        let ix = TransferTextureConfigOwnership {
            owner: self.authority.pubkey(),
            new_owner: new_authority.pubkey(),
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority, &new_authority])
            .await
            .expect("Sending TX");

        println!("Authority transferred");
    }

    pub async fn show_texture_config(&self) {
        let cfg_data = self
            .rpc
            .get_account_data(&TEXTURE_CONFIG_ID)
            .await
            .expect("getting Pool account");
        let cfg = TextureConfig::try_from_bytes(&cfg_data).expect("unpacking Global Config");

        println!(
            "Super Lendy Texture global config at address {}",
            TEXTURE_CONFIG_ID
        );
        println!("Owner                              : {}", cfg.owner);
        println!(
            "performance_fee_authority          : {}",
            cfg.fees_authority
        );
        println!(
            "performance_fee_rate_bps           : {}",
            cfg.performance_fee_rate_bps
        );
        println!(
            "borrow_fee_rate_bps                : {}",
            cfg.borrow_fee_rate_bps
        );

        println!(
            "market_price_feed_lock_sec         : {}",
            cfg.reserve_timelock.market_price_feed_lock_sec
        );
        println!(
            "irm_lock_sec                       : {}",
            cfg.reserve_timelock.irm_lock_sec
        );
        println!(
            "liquidation_bonus_lock_sec         : {}",
            cfg.reserve_timelock.liquidation_bonus_lock_sec
        );
        println!(
            "unhealthy_ltv_lock_sec             : {}",
            cfg.reserve_timelock.unhealthy_ltv_lock_sec
        );
        println!(
            "partial_liquidation_factor_lock_sec: {}",
            cfg.reserve_timelock.partial_liquidation_factor_lock_sec
        );
        println!(
            "max_total_liquidity_lock_sec       : {}",
            cfg.reserve_timelock.max_total_liquidity_lock_sec
        );
        println!(
            "max_borrow_ltv_lock_sec            : {}",
            cfg.reserve_timelock.max_borrow_ltv_lock_sec
        );
        println!(
            "max_borrow_utilization_lock_sec    : {}",
            cfg.reserve_timelock.max_borrow_utilization_lock_sec
        );
        println!(
            "price_stale_threshold_lock_sec     : {}",
            cfg.reserve_timelock.price_stale_threshold_lock_sec
        );
        println!(
            "max_withdraw_utilization_lock_sec  : {}",
            cfg.reserve_timelock.max_withdraw_utilization_lock_sec
        );
        println!(
            "fees_lock_sec                      : {}",
            cfg.reserve_timelock.fees_lock_sec
        );
        println!("-------------------------------------");
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn alter_texture_config(
        &self,
        performance_fee_authority: Option<Pubkey>,
        performance_fee_rate_bps: Option<u16>,
        borrow_fee_rate_bps: Option<u16>,
        market_price_feed_lock_sec: Option<u32>,
        irm_lock_sec: Option<u32>,
        liquidation_bonus_lock_sec: Option<u32>,
        unhealthy_ltv_lock_sec: Option<u32>,
        partial_liquidation_factor_lock_sec: Option<u32>,
        max_total_liquidity_lock_sec: Option<u32>,
        max_borrow_ltv_lock_sec: Option<u32>,
        max_borrow_utilization_lock_sec: Option<u32>,
        price_stale_threshold_lock_sec: Option<u32>,
        max_withdraw_utilization_lock_sec: Option<u32>,
        fees_lock_sec: Option<u32>,
    ) {
        let cfg_data = self
            .rpc
            .get_account_data(&TEXTURE_CONFIG_ID)
            .await
            .expect("getting Pool account");
        let cfg = TextureConfig::try_from_bytes(&cfg_data).expect("unpacking Global Config");

        let mut params = TextureConfigParams {
            borrow_fee_rate_bps: cfg.borrow_fee_rate_bps,
            performance_fee_rate_bps: cfg.performance_fee_rate_bps,
            fees_authority: cfg.fees_authority,
            reserve_timelock: cfg.reserve_timelock,
        };

        if let Some(performance_fee_authority) = performance_fee_authority {
            params.fees_authority = performance_fee_authority;
        }

        if let Some(performance_fee_rate_bps) = performance_fee_rate_bps {
            params.performance_fee_rate_bps = performance_fee_rate_bps;
        }

        if let Some(borrow_fee_rate_bps) = borrow_fee_rate_bps {
            params.borrow_fee_rate_bps = borrow_fee_rate_bps;
        }

        if let Some(market_price_feed_lock_sec) = market_price_feed_lock_sec {
            params.reserve_timelock.market_price_feed_lock_sec = market_price_feed_lock_sec;
        }

        if let Some(irm_lock_sec) = irm_lock_sec {
            params.reserve_timelock.irm_lock_sec = irm_lock_sec;
        }

        if let Some(liquidation_bonus_lock_sec) = liquidation_bonus_lock_sec {
            params.reserve_timelock.liquidation_bonus_lock_sec = liquidation_bonus_lock_sec;
        }

        if let Some(unhealthy_ltv_lock_sec) = unhealthy_ltv_lock_sec {
            params.reserve_timelock.unhealthy_ltv_lock_sec = unhealthy_ltv_lock_sec;
        }

        if let Some(partial_liquidation_factor_lock_sec) = partial_liquidation_factor_lock_sec {
            params.reserve_timelock.partial_liquidation_factor_lock_sec =
                partial_liquidation_factor_lock_sec;
        }

        if let Some(max_total_liquidity_lock_sec) = max_total_liquidity_lock_sec {
            params.reserve_timelock.max_total_liquidity_lock_sec = max_total_liquidity_lock_sec;
        }

        if let Some(max_borrow_ltv_lock_sec) = max_borrow_ltv_lock_sec {
            params.reserve_timelock.max_borrow_ltv_lock_sec = max_borrow_ltv_lock_sec;
        }

        if let Some(max_borrow_utilization_lock_sec) = max_borrow_utilization_lock_sec {
            params.reserve_timelock.max_borrow_utilization_lock_sec =
                max_borrow_utilization_lock_sec;
        }

        if let Some(price_stale_threshold_lock_sec) = price_stale_threshold_lock_sec {
            params.reserve_timelock.price_stale_threshold_lock_sec = price_stale_threshold_lock_sec;
        }

        if let Some(max_withdraw_utilization_lock_sec) = max_withdraw_utilization_lock_sec {
            params.reserve_timelock.max_withdraw_utilization_lock_sec =
                max_withdraw_utilization_lock_sec;
        }

        if let Some(fees_lock_sec) = fees_lock_sec {
            params.reserve_timelock.fees_lock_sec = fees_lock_sec;
        }

        let ix = AlterTextureConfig {
            owner: self.authority.pubkey(),
            params,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Altered global config");
    }

    pub async fn create_curator(&self, params: CuratorParams) {
        let keypair = Keypair::new();

        let create_ix = system_instruction::create_account(
            &self.authority.pubkey(),
            &keypair.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(Curator::SIZE)
                .await
                .expect("getting rent"),
            Curator::SIZE as u64,
            &SUPER_LENDY_ID,
        );

        let ix = CreateCurator {
            curator: keypair.pubkey(),
            global_config_owner: self.authority.pubkey(),
            params,
        }
        .into_instruction();

        self.send_transaction_by(vec![create_ix, ix], &[&self.authority, &keypair])
            .await
            .expect("Sending TX");

        println!("Created curator: {}", keypair.pubkey());
    }

    pub async fn list_curators(&self) {
        let curators = load_curators(&self.rpc).await.expect("loading curators");

        for (addr, curator) in curators.iter() {
            println!("Curator at address {}", addr);
            println!("Owner            : {}", curator.owner);
            println!("Pools authority  : {}", curator.pools_authority);
            println!("Vaults authority : {}", curator.vaults_authority);
            println!("Fee authority    : {}", curator.fees_authority);
            println!(
                "Name             : {}",
                String::from_utf8_lossy(&curator.name)
            );
            println!(
                "Website          : {}",
                String::from_utf8_lossy(&curator.website_url)
            );
            println!(
                "Logo             : {}",
                String::from_utf8_lossy(&curator.logo_url)
            );
            println!("-------------------------------------");
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn alter_curator(
        &self,
        curator: Pubkey,
        name: Option<String>,
        logo_url: Option<String>,
        website_url: Option<String>,
        owner: Option<Pubkey>,
        fee_authority: Option<Pubkey>,
        pools_authority: Option<Pubkey>,
        vaults_authority: Option<Pubkey>,
    ) {
        let curator_data = self
            .rpc
            .get_account_data(&curator)
            .await
            .expect("getting Curator account");
        let unpacked_curator = Curator::try_from_bytes(&curator_data).expect("unpacking Curator");

        let mut params = CuratorParams {
            owner: unpacked_curator.owner,
            fees_authority: unpacked_curator.fees_authority,
            pools_authority: unpacked_curator.pools_authority,
            vaults_authority: unpacked_curator.vaults_authority,
            name: unpacked_curator.name,
            logo_url: unpacked_curator.logo_url,
            website_url: unpacked_curator.website_url,
        };

        if let Some(name) = name {
            if name.len() > CURATOR_NAME_MAX_LEN {
                println!("Name is too long. {} symbols max.", CURATOR_NAME_MAX_LEN);
                return;
            }

            let mut bytes_zero_ended = [0; CURATOR_NAME_MAX_LEN];
            let bytes = name.as_bytes();
            bytes_zero_ended[..bytes.len()].copy_from_slice(bytes);

            params.name = bytes_zero_ended;
        }

        if let Some(logo_url) = logo_url {
            if logo_url.len() > CURATOR_LOGO_URL_MAX_LEN {
                println!(
                    "Name is too long. {} symbols max.",
                    CURATOR_LOGO_URL_MAX_LEN
                );
                return;
            }

            let mut bytes_zero_ended = [0; CURATOR_LOGO_URL_MAX_LEN];
            let bytes = logo_url.as_bytes();
            bytes_zero_ended[..bytes.len()].copy_from_slice(bytes);

            params.logo_url = bytes_zero_ended;
        }

        if let Some(website_url) = website_url {
            if website_url.len() > CURATOR_WEBSITE_URL_MAX_LEN {
                println!(
                    "Name is too long. {} symbols max.",
                    CURATOR_WEBSITE_URL_MAX_LEN
                );
                return;
            }

            let mut bytes_zero_ended = [0; CURATOR_WEBSITE_URL_MAX_LEN];
            let bytes = website_url.as_bytes();
            bytes_zero_ended[..bytes.len()].copy_from_slice(bytes);

            params.website_url = bytes_zero_ended;
        }

        if let Some(new_owner) = owner {
            params.owner = new_owner;
        }

        if let Some(new_fee_authority) = fee_authority {
            params.fees_authority = new_fee_authority;
        }

        if let Some(pools_authority) = pools_authority {
            params.pools_authority = pools_authority;
        }

        if let Some(vaults_authority) = vaults_authority {
            params.vaults_authority = vaults_authority;
        }

        let ix = AlterCurator {
            curator,
            owner: self.authority.pubkey(),
            params,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Altered curator");
    }

    pub async fn create_pool(&self, curator: Pubkey, params: PoolParams) {
        let new_keypair = Keypair::new();

        let create_ix = system_instruction::create_account(
            &self.authority.pubkey(),
            &new_keypair.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(Pool::SIZE)
                .await
                .expect("getting rent"),
            Pool::SIZE as u64,
            &SUPER_LENDY_ID,
        );

        // Pool is created by someone who already have Curator account.
        // The command must be called with curator_pools_authority authority.
        let ix = CreatePool {
            pool: new_keypair.pubkey(),
            curator,
            params,
            curator_pools_authority: self.multisig.unwrap_or(self.authority.pubkey()),
        }
        .into_instruction();

        if self.multisig.is_some() {
            self.prepare_immortal_tx(vec![create_ix, ix], Some(&new_keypair))
                .await;
        } else {
            self.send_transaction_by(vec![create_ix, ix], &[&self.authority, &new_keypair])
                .await
                .expect("Sending TX");

            println!("Created pool: {}", new_keypair.pubkey());
        }
    }

    pub async fn list_pools(&self, pool_addr: Option<Pubkey>, curator: Option<Pubkey>) {
        let pools = load_pools(&self.rpc).await.expect("loading pools");
        let filtered_pools = pools
            .into_iter()
            .filter(|&(pool_key, _)| {
                if let Some(pool_addr) = pool_addr {
                    pool_key == pool_addr
                } else {
                    true
                }
            })
            .filter(|&(_, pool)| {
                if let Some(curator) = curator {
                    pool.curator == curator
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        for (key, pool) in filtered_pools.iter() {
            println!("Lendy Pool at address {}", key);
            println!(
                "Name                         : {}",
                String::from_utf8_lossy(&pool.name)
            );
            println!(
                "Market price currency symbol : {}",
                String::from_utf8_lossy(&pool.market_price_currency_symbol)
            );
            println!("Curator                      : {}", pool.curator);
            println!("Visible                      : {}", pool.visible);
            println!("-------------------------------------");
        }

        println!("Total pools number {}", filtered_pools.len());
    }

    pub async fn alter_pool(
        &self,
        pool_key: Pubkey,
        name: Option<String>,
        market_price_currency_symbol: Option<String>,
        visible: Option<bool>,
    ) {
        let pool_data = self
            .rpc
            .get_account_data(&pool_key)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let mut params = PoolParams {
            name: pool.name,
            market_price_currency_symbol: pool.market_price_currency_symbol,
            visible: pool.visible,
        };

        let curator_data = self
            .rpc
            .get_account_data(&pool.curator)
            .await
            .expect("getting Curator account");
        let curator = Curator::try_from_bytes(&curator_data).expect("unpacking Curator");

        if let Some(name) = name {
            if name.len() > POOL_NAME_MAX_LEN {
                println!("Name is too long. {} symbols max.", POOL_NAME_MAX_LEN);
                return;
            }
            params.name = str_to_array(&name);
        }

        if let Some(market_price_currency_symbol) = market_price_currency_symbol {
            if market_price_currency_symbol.len() > CURRENCY_SYMBOL_MAX_LEN {
                println!(
                    "Currency symbol is too long. {} symbols max.",
                    CURRENCY_SYMBOL_MAX_LEN
                );
                return;
            }
            params.market_price_currency_symbol = str_to_array(&market_price_currency_symbol);
        }

        if let Some(enabled) = visible {
            params.visible = u8::from(enabled);
        }

        let ix = AlterPool {
            pool: pool_key,
            curator_pools_authority: curator.pools_authority,
            params,
            curator: pool.curator,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Altered pool: {}", pool_key);
    }

    pub async fn token_program_by_mint(&self, liquidity_mint: &Pubkey) -> Pubkey {
        let mint_account = self
            .rpc
            .get_account(liquidity_mint)
            .await
            .expect("getting mint account");
        mint_account.owner
    }

    pub async fn create_reserve(
        &self,
        curator: Pubkey,
        pool: Pubkey,
        liquidity_mint: Pubkey,
        market_price_feed: Pubkey,
        params: ReserveConfig,
        reserve_type: u8,
    ) {
        let new_keypair = Keypair::new();

        let create_ix = system_instruction::create_account(
            &self.authority.pubkey(),
            &new_keypair.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(Reserve::SIZE)
                .await
                .expect("getting rent"),
            Reserve::SIZE as u64,
            &SUPER_LENDY_ID,
        );

        let ix = CreateReserve {
            reserve: new_keypair.pubkey(),
            pool,
            curator_pools_authority: self.authority.pubkey(),
            liquidity_mint,
            market_price_feed,
            liquidity_token_program: self.token_program_by_mint(&liquidity_mint).await,
            params,
            reserve_type,
            curator,
        }
        .into_instruction();

        self.send_transaction_by(vec![create_ix, ix], &[&self.authority, &new_keypair])
            .await
            .expect("Sending TX");

        println!("Created reserve: {}", new_keypair.pubkey());
    }

    pub async fn list_reserves(
        &self,
        reserve_addr: Option<Pubkey>,
        pool: Option<Pubkey>,
        mint: Option<Pubkey>,
    ) {
        let reserve = load_reserves(&self.rpc).await.expect("loading reserves");
        let filtered_reserves = reserve
            .into_iter()
            .filter(|&(reserve_key, _)| {
                if let Some(reserve_addr) = reserve_addr {
                    reserve_key == reserve_addr
                } else {
                    true
                }
            })
            .filter(|&(_, reserve)| {
                if let Some(pool) = pool {
                    reserve.pool == pool
                } else {
                    true
                }
            })
            .filter(|&(_, reserve)| {
                if let Some(mint) = mint {
                    reserve.liquidity.mint == mint
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        let current_slot = self.rpc.get_slot().await.expect("getting current slot");

        for (key, reserve) in filtered_reserves.iter() {
            println!("Super Lendy Reserve at address {}", key);
            if reserve.reserve_type == RESERVE_TYPE_NORMAL {
                println!("Reserve Type                     : Normal");
            }
            if reserve.reserve_type == RESERVE_TYPE_PROTECTED_COLLATERAL {
                println!(
                    "Reserve Type                     : BORROWING DISABLED (protected collateral)"
                );
            }
            if reserve.reserve_type == RESERVE_TYPE_NOT_A_COLLATERAL {
                println!(
                    "Reserve Type                     : BORROW ONLY (not accepted as collateral)"
                );
            }
            if reserve.mode == RESERVE_MODE_NORMAL {
                println!("Reserve Mode                     : Normal");
            }
            if reserve.mode == RESERVE_MODE_BORROW_DISABLED {
                println!("Reserve Mode                     : BORROWING DISABLED");
            }
            if reserve.mode == RESERVE_MODE_RETAIN_LIQUIDITY {
                println!("Reserve Mode                     : BORROW, UNLOCK, WITHDRAW DISABLED");
            }
            if reserve.flash_loans_enabled == 0 {
                println!("Flash loans                      : Disabled");
            } else {
                println!("Flash loans                      : Enabled");
            }
            println!("Pool                             : {}", reserve.pool);
            println!(
                "Liquidity Mint                   : {}",
                reserve.liquidity.mint
            );
            println!(
                "LP token mint                    : {}",
                find_lp_token_mint(key).0
            );

            println!("-------------- Overall current operational metrics ----------------");
            println!(
                "Last update slot                 : {}  (in past for {} slots)",
                reserve.last_update.slot,
                current_slot - reserve.last_update.slot
            );
            println!(
                "Borrow rate            (decimal) : {}     {} %",
                reserve.liquidity.borrow_rate().unwrap_or_default(),
                reserve
                    .liquidity
                    .borrow_rate()
                    .unwrap_or_default()
                    .checked_mul(Decimal::from_i128_with_scale(100, 0).unwrap())
                    .unwrap()
            );
            println!(
                "LP exchange rate       (decimal) : {}",
                reserve
                    .lp_exchange_rate()
                    .unwrap_or(LpExchangeRate(Decimal::ZERO))
                    .0
            );
            println!(
                "Available amount                 : {}   {}",
                Decimal::from_lamports(
                    reserve.liquidity.available_amount,
                    reserve.liquidity.mint_decimals
                )
                .unwrap_or_default(),
                reserve.liquidity.available_amount
            );
            println!(
                "Borrowed amount                  : {}   {}",
                reserve.liquidity.borrowed_amount().unwrap_or_default(),
                reserve
                    .liquidity
                    .borrowed_amount()
                    .unwrap_or_default()
                    .to_lamports_round(reserve.liquidity.mint_decimals)
                    .unwrap_or_default()
            );
            println!(
                "Total liquidity                  : {}",
                reserve.liquidity.total_liquidity().unwrap_or_default()
            );
            println!(
                "Utilization rate                 : {}",
                reserve.liquidity.utilization_rate().unwrap_or_default()
            );
            println!(
                "Cumulative borrow rate           : {}",
                reserve
                    .liquidity
                    .cumulative_borrow_rate()
                    .unwrap_or_default()
            );
            println!(
                "Market price of liquidity token  : {}",
                reserve.liquidity.market_price().unwrap_or_default()
            );
            println!(
                "Total LP supply                  : {}",
                reserve.collateral.lp_total_supply
            );
            println!(
                "Accrued Curator's perf. fee      : {}",
                reserve
                    .liquidity
                    .curator_performance_fee()
                    .unwrap_or_default()
            );
            println!(
                "Accrued Texture's perf. fee      : {}",
                reserve
                    .liquidity
                    .texture_performance_fee()
                    .unwrap_or_default()
            );

            println!("----------------------------- Settings ----------------------------");
            println!(
                "Price feed                       : {}",
                reserve.config.market_price_feed
            );
            println!("Interest Rate model              : {}", reserve.config.irm);
            println!(
                "Liquidation bonus (bps)          : {}",
                reserve.config.liquidation_bonus_bps
            );
            println!(
                "Market price freshness threshold : {} sec",
                reserve.config.price_stale_threshold_sec
            );
            println!(
                "Maximum borrow utilization (bps) : {}",
                reserve.config.max_borrow_utilization_bps
            );
            println!(
                "Maximum withdraw utiliz. (bps)   : {}",
                reserve.config.max_withdraw_utilization_bps
            );
            println!(
                "Maximum borrow ltv (bps)         : {}",
                reserve.config.max_borrow_ltv_bps
            );
            println!(
                "Maximum total liquidity          : {}",
                reserve.config.max_total_liquidity
            );
            println!(
                "Partly unhealthy LTV (bps)       : {}",
                reserve.config.partly_unhealthy_ltv_bps
            );
            println!(
                "Fully unhealthy LTV (bps)        : {}",
                reserve.config.fully_unhealthy_ltv_bps
            );
            println!(
                "Partial liquidation amount (bps) : {}",
                reserve.config.partial_liquidation_factor_bps
            );
            println!(
                "Curator's borrow fee       (bps) : {}",
                reserve.config.fees.curator_borrow_fee_rate_bps
            );
            println!(
                "Curator's perf. fee        (bps) : {}",
                reserve.config.fees.curator_performance_fee_rate_bps
            );

            println!("------------------------- Reward rules ----------------------------");
            for (index, rule) in reserve.reward_rules.rules.iter().enumerate() {
                if rule.reward_mint == Pubkey::default() {
                    println!("{}: vacant", index);
                } else {
                    println!(
                        "{}: name: {}   rate: {}  reward mint: {} ",
                        index,
                        String::from_utf8_lossy(&rule.name),
                        rule.rate().expect("rate from bits"),
                        rule.reward_mint
                    );
                }
            }
            println!("--------------------- Proposed configs ----------------------------");
            for (index, proposal) in reserve.proposed_configs.0.iter().enumerate() {
                if proposal.can_be_applied_at == 0 {
                    println!("{}: vacant", index);
                } else {
                    let seconds_to_apply =
                        proposal.can_be_applied_at - chrono::Utc::now().timestamp();
                    println!(
                        "{}: time_to_apply: {}  {}",
                        index,
                        if seconds_to_apply < 0 {
                            "NOW".to_string()
                        } else {
                            format!("{} sec", seconds_to_apply)
                        },
                        proposal
                    );
                }
            }

            println!("==========================================================================");
        }

        println!("Total reserves number {}", filtered_reserves.len());
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn alter_reserve(
        &self,
        reserve_key: Pubkey,
        market_price_feed: Option<Pubkey>,
        irm: Option<Pubkey>,
        liquidation_bonus_bps: Option<u16>,
        partly_unhealthy_ltv_bps: Option<u16>,
        partial_liquidation_amount_bps: Option<u16>,
        fully_unhealthy_ltv_bps: Option<u16>,
        curator_borrow_fee_bps: Option<u16>,
        curator_performance_fee_bps: Option<u16>,
        max_borrow_utilization_bps: Option<u16>,
        max_withdraw_utilization_bps: Option<u16>,
        max_total_liquidity: Option<u64>,
        max_borrow_ltv_bps: Option<u16>,
        price_stale_threshold_sec: Option<u32>,
        mode: Option<u8>,
        flash_loans_enabled: Option<bool>,
    ) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let mut new_config = reserve.config;

        if let Some(market_price_feed) = market_price_feed {
            new_config.market_price_feed = market_price_feed;
        }

        if let Some(irm) = irm {
            new_config.irm = irm;
        }

        if let Some(liquidation_bonus_bps) = liquidation_bonus_bps {
            new_config.liquidation_bonus_bps = liquidation_bonus_bps;
        }

        if let Some(partly_unhealthy_ltv_bps) = partly_unhealthy_ltv_bps {
            new_config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps;
        }

        if let Some(partial_liquidation_amount_bps) = partial_liquidation_amount_bps {
            new_config.partial_liquidation_factor_bps = partial_liquidation_amount_bps;
        }

        if let Some(fully_unhealthy_ltv_bps) = fully_unhealthy_ltv_bps {
            new_config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps;
        }

        if let Some(curator_borrow_fee_bps) = curator_borrow_fee_bps {
            new_config.fees.curator_borrow_fee_rate_bps = curator_borrow_fee_bps;
        }

        if let Some(curator_performance_fee_bps) = curator_performance_fee_bps {
            new_config.fees.curator_performance_fee_rate_bps = curator_performance_fee_bps;
        }

        if let Some(max_utilization_bps) = max_borrow_utilization_bps {
            new_config.max_borrow_utilization_bps = max_utilization_bps;
        }

        if let Some(max_utilization_bps) = max_withdraw_utilization_bps {
            new_config.max_withdraw_utilization_bps = max_utilization_bps;
        }

        if let Some(max_total_liquidity) = max_total_liquidity {
            new_config.max_total_liquidity = max_total_liquidity;
        }

        if let Some(max_borrow_ltv_bps) = max_borrow_ltv_bps {
            new_config.max_borrow_ltv_bps = max_borrow_ltv_bps;
        }

        if let Some(price_stale_threshold_sec) = price_stale_threshold_sec {
            new_config.price_stale_threshold_sec = price_stale_threshold_sec;
        }

        let mode = mode.unwrap_or(reserve.mode);

        let flash_loans_enabled = if let Some(flash_loans_enabled) = flash_loans_enabled {
            flash_loans_enabled as u8
        } else {
            0
        };

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let ix = AlterReserve {
            reserve: reserve_key,
            pool: reserve.pool,
            market_price_feed: new_config.market_price_feed,
            curator_pools_authority: self.authority.pubkey(),
            params: new_config,
            curator: pool.curator,
            mode,
            flash_loans_enabled,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Altered reserve: {}", reserve_key);
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn propose_config_change(
        &self,
        reserve_key: Pubkey,
        index: u8,
        market_price_feed: Option<Pubkey>,
        irm: Option<Pubkey>,
        liquidation_bonus_bps: Option<u16>,
        partly_unhealthy_ltv_bps: Option<u16>,
        partial_liquidation_amount_bps: Option<u16>,
        fully_unhealthy_ltv_bps: Option<u16>,
        curator_borrow_fee_bps: Option<u16>,
        curator_performance_fee_bps: Option<u16>,
        max_borrow_utilization_bps: Option<u16>,
        max_withdraw_utilization_bps: Option<u16>,
        max_total_liquidity: Option<u64>,
        max_borrow_ltv_bps: Option<u16>,
        price_stale_threshold_sec: Option<u32>,
    ) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let mut new_config = ReserveConfig::zeroed();
        let mut change_map = ConfigFields::empty();

        let market_price_feed = if let Some(market_price_feed) = market_price_feed {
            change_map.insert(ConfigFields::MARKET_PRICE_FEED);
            new_config.market_price_feed = market_price_feed;
            market_price_feed
        } else {
            reserve.config.market_price_feed
        };

        if let Some(irm) = irm {
            change_map.insert(ConfigFields::IRM);
            new_config.irm = irm;
        }

        if let Some(liquidation_bonus_bps) = liquidation_bonus_bps {
            change_map.insert(ConfigFields::LIQUIDATION_BONUS);
            new_config.liquidation_bonus_bps = liquidation_bonus_bps;
        }

        if let Some(partly_unhealthy_ltv_bps) = partly_unhealthy_ltv_bps {
            change_map.insert(ConfigFields::PARTLY_UNHEALTHY_LTV);
            new_config.partly_unhealthy_ltv_bps = partly_unhealthy_ltv_bps;
        }

        if let Some(partial_liquidation_amount_bps) = partial_liquidation_amount_bps {
            change_map.insert(ConfigFields::PARTIAL_LIQUIDATION_FACTOR);
            new_config.partial_liquidation_factor_bps = partial_liquidation_amount_bps;
        }

        if let Some(fully_unhealthy_ltv_bps) = fully_unhealthy_ltv_bps {
            change_map.insert(ConfigFields::FULLY_UNHEALTHY_LTV);
            new_config.fully_unhealthy_ltv_bps = fully_unhealthy_ltv_bps;
        }

        if let Some(curator_borrow_fee_bps) = curator_borrow_fee_bps {
            change_map.insert(ConfigFields::CURATOR_BORROW_FEE_RATE);
            new_config.fees.curator_borrow_fee_rate_bps = curator_borrow_fee_bps;
        }

        if let Some(curator_performance_fee_bps) = curator_performance_fee_bps {
            change_map.insert(ConfigFields::CURATOR_PERFORMANCE_FEE_RATE);
            new_config.fees.curator_performance_fee_rate_bps = curator_performance_fee_bps;
        }

        if let Some(max_utilization_bps) = max_borrow_utilization_bps {
            change_map.insert(ConfigFields::MAX_BORROW_UTILIZATION);
            new_config.max_borrow_utilization_bps = max_utilization_bps;
        }

        if let Some(max_utilization_bps) = max_withdraw_utilization_bps {
            change_map.insert(ConfigFields::MAX_WITHDRAW_UTILIZATION);
            new_config.max_withdraw_utilization_bps = max_utilization_bps;
        }

        if let Some(max_total_liquidity) = max_total_liquidity {
            change_map.insert(ConfigFields::MAX_TOTAL_LIQUIDITY);
            new_config.max_total_liquidity = max_total_liquidity;
        }

        if let Some(max_borrow_ltv_bps) = max_borrow_ltv_bps {
            change_map.insert(ConfigFields::MAX_BORROW_LTV);
            new_config.max_borrow_ltv_bps = max_borrow_ltv_bps;
        }

        if let Some(price_stale_threshold_sec) = price_stale_threshold_sec {
            change_map.insert(ConfigFields::PRICE_STALE_THRESHOLD);
            new_config.price_stale_threshold_sec = price_stale_threshold_sec;
        }

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let ix = ProposeConfig {
            reserve: reserve_key,
            pool: reserve.pool,
            market_price_feed,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            index,
            proposal: ConfigProposal {
                can_be_applied_at: 0,
                change_map: change_map.bits(),
                config: new_config,
            },
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Config change proposed");
    }

    pub async fn clear_proposed_config_change(&self, reserve_key: Pubkey, index: u8) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let zero_config = ReserveConfig::zeroed();

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let ix = ProposeConfig {
            reserve: reserve_key,
            pool: reserve.pool,
            market_price_feed: reserve.config.market_price_feed,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            index,
            proposal: ConfigProposal {
                can_be_applied_at: 0,
                change_map: 0,
                config: zero_config,
            },
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Proposal cleared");
    }

    pub async fn apply_proposed_config_change(&self, reserve_key: Pubkey, index: u8) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let fields =
            ConfigFields::from_bits(reserve.proposed_configs.0[index as usize].change_map).unwrap();
        let market_price_feed = if fields.contains(ConfigFields::MARKET_PRICE_FEED) {
            reserve.proposed_configs.0[index as usize]
                .config
                .market_price_feed
        } else {
            reserve.config.market_price_feed
        };

        let ix = ApplyConfigProposal {
            reserve: reserve_key,
            pool: reserve.pool,
            market_price_feed,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            index,
        }
        .into_instruction();

        let refresh = RefreshReserve {
            reserve: reserve_key,
            market_price_feed: reserve.config.market_price_feed,
            irm: reserve.config.irm,
        }
        .into_instruction();

        self.update_prices(&[reserve_key]).await;

        self.send_transaction_by(vec![refresh, ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Proposal applied");
    }

    pub async fn init_reward_supply(&self, pool_key: Pubkey, reward_mint: Pubkey) {
        let pool_data = self
            .rpc
            .get_account_data(&pool_key)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let ix = InitRewardSupply {
            reward_mint,
            pool: pool_key,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            token_program: self.token_program_by_mint(&reward_mint).await,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!(
            "Created reward supply: {}",
            find_reward_supply(&pool_key, &reward_mint).0
        );
    }

    pub async fn claim_reward(&self, position: Pubkey, pool_key: Pubkey, reward_mint: Pubkey) {
        let destination_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reward_mint,
            &self.token_program_by_mint(&reward_mint).await,
        );

        if !self
            .account_exists(&destination_wallet)
            .await
            .expect("check destination_wallet existance")
        {
            println!("Creating wallet {} for reward tokens.", destination_wallet);

            let mint_account = self
                .rpc
                .get_account(&reward_mint)
                .await
                .expect("getting mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &reward_mint,
                &mint_account.owner,
            );

            self.send_transaction_by(vec![ix], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let claim_reward_ix = ClaimReward {
            position,
            destination_wallet,
            reward_mint,
            pool: pool_key,
            position_owner: self.authority.pubkey(),
            token_program: self.token_program_by_mint(&reward_mint).await,
        }
        .into_instruction();

        let refresh_position_info = self.refresh_position_ix(position).await;
        let mut ixs = refresh_position_info.0;
        ixs.push(claim_reward_ix);

        let version = Version { no_error: true }.into_instruction();
        ixs.push(version);

        self.update_prices(&refresh_position_info.1).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Claimed");
    }

    pub fn reward_supply_addr(&self, pool_key: Pubkey, reward_mint: Pubkey) {
        println!("{}", find_reward_supply(&pool_key, &reward_mint).0);
    }

    pub async fn deposit_reward(&self, pool_key: Pubkey, reward_mint: Pubkey, amount: u64) {
        let reward_supply = find_reward_supply(&pool_key, &reward_mint).0;

        let source_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reward_mint,
            &self.token_program_by_mint(&reward_mint).await,
        );

        if !self
            .account_exists(&source_wallet)
            .await
            .expect("check source_wallet existance")
        {
            println!(
                "Error: you don't have SPL account with {} tokens to fund rewards",
                reward_mint
            );
            return;
        }

        let mint_account = self
            .rpc
            .get_account(&reward_mint)
            .await
            .expect("getting mint account");

        let ix = if mint_account.owner == spl_token::id() {
            spl_token::instruction::transfer(
                &spl_token::id(),
                &source_wallet,
                &reward_supply,
                &self.authority.pubkey(),
                &[&self.authority.pubkey()],
                amount,
            )
            .unwrap()
        } else if mint_account.owner == spl_token_2022::id() {
            let mint_data = mint_account.data;
            let mint_state =
                StateWithExtensions::<Mint>::unpack(&mint_data).expect("unpacking 2022 mint");

            spl_token_2022::instruction::transfer_checked(
                &spl_token_2022::id(),
                &source_wallet,
                &reward_mint,
                &reward_supply,
                &self.authority.pubkey(),
                &[&self.authority.pubkey()],
                amount,
                mint_state.base.decimals,
            )
            .unwrap()
        } else {
            println!("unrecognized mint owner program {}", mint_account.owner);
            return;
        };

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Deposited");
    }

    pub async fn withdraw_reward(&self, pool_key: Pubkey, reward_mint: Pubkey, amount: u64) {
        let pool_data = self
            .rpc
            .get_account_data(&pool_key)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let destination_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reward_mint,
            &self.token_program_by_mint(&reward_mint).await,
        );

        if !self
            .account_exists(&destination_wallet)
            .await
            .expect("check destination_wallet existance")
        {
            println!("Creating wallet {} for reward tokens.", destination_wallet);

            let mint_account = self
                .rpc
                .get_account(&reward_mint)
                .await
                .expect("getting mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &reward_mint,
                &mint_account.owner,
            );

            self.send_transaction_by(vec![ix], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let ix = WithdrawReward {
            destination_wallet,
            reward_mint,
            pool: pool_key,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            token_program: self.token_program_by_mint(&reward_mint).await,
            amount,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Withdrawn");
    }

    pub async fn list_rewards_balances(&self, pool: Pubkey) {
        let rewards_authority = find_rewards_program_authority(&pool);

        let mut accounts = self
            .rpc
            .get_token_accounts_by_owner(
                &rewards_authority.0,
                TokenAccountsFilter::ProgramId(spl_token::id()),
            )
            .await
            .expect("getting token accounts");

        let accounts_2022 = self
            .rpc
            .get_token_accounts_by_owner(
                &rewards_authority.0,
                TokenAccountsFilter::ProgramId(spl_token_2022::id()),
            )
            .await
            .expect("getting token accounts");

        accounts.extend(accounts_2022);

        println!(
            "Rewards balances for pool {} with rewards authority {}",
            pool, rewards_authority.0
        );

        for account in accounts {
            // println!("account {}    data {:?}", account.pubkey, account.account.data);
            match account.account.data {
                UiAccountData::LegacyBinary(_) => {}
                UiAccountData::Json(parsed) => {
                    println!(
                        "account {}   mint {}   amount {}",
                        account.pubkey,
                        parsed.parsed.get("info").unwrap().get("mint").unwrap(),
                        parsed
                            .parsed
                            .get("info")
                            .unwrap()
                            .get("tokenAmount")
                            .unwrap()
                            .get("amount")
                            .unwrap()
                    );
                }
                UiAccountData::Binary(_, _) => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn set_reward_rule(
        &self,
        reserve_key: Pubkey,
        index: usize,
        name: String,
        reward_mint: Pubkey,
        deposits: bool,
        borrows: bool,
        rate: f64,
    ) {
        if deposits && borrows {
            println!("Choose either `deposits` or `borrow` but not both!");
            return;
        }

        if !deposits && !borrows {
            println!("Choose either `deposits` or `borrow`");
            return;
        }

        // E.g. MAX_REWARD_RULES = 10. Then index can be 0 .. 9
        if index >= MAX_REWARD_RULES {
            println!("Rule index must be in range [0;{})", MAX_REWARD_RULES);
            return;
        }

        let decimal_rate = Decimal::from_i128_with_scale((rate * WAD as f64) as i128, SCALE)
            .expect("rate conversion");

        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let mut new_rules = reserve.reward_rules;

        let mut name_bytes_zero_ended = [0; REWARD_RULE_NAME_MAX_LEN];
        let name_bytes = name.as_bytes();
        name_bytes_zero_ended[..name_bytes.len()].copy_from_slice(name_bytes);

        new_rules.rules[index].name = name_bytes_zero_ended;
        new_rules.rules[index].reward_mint = reward_mint;
        new_rules.rules[index].reason = if deposits {
            REWARD_FOR_LIQUIDITY
        } else {
            REWARD_FOR_BORROW
        };
        new_rules.rules[index]
            .set_rate(decimal_rate)
            .expect("set_rate");

        // In any case contract will put current Solana slot there.
        new_rules.rules[index].start_slot = 0;

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let reward_mints = new_rules
            .rules
            .iter()
            .map(|rule| rule.reward_mint)
            .collect();

        let ix = SetRewardRules {
            reserve: reserve_key,
            pool: reserve.pool,
            curator_pools_authority: self.authority.pubkey(),
            rules: new_rules,
            curator: pool.curator,
            reward_mints,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Rule set");
    }

    pub async fn get_decimals_by_mint(&self, mint: &Pubkey) -> u8 {
        match self.rpc.get_account(mint).await {
            Ok(account) => {
                if account.owner == spl_token::id() {
                    let mint =
                        spl_token::state::Mint::unpack(&account.data).expect("unpacking mint");
                    mint.decimals
                } else if account.owner == spl_token_2022::id() {
                    let mint =
                        StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&account.data)
                            .expect("unpacking Token2022 mint");
                    mint.base.decimals
                } else {
                    println!(
                        "ERROR: unrecognized owner {} of mint account {}",
                        account.owner, mint
                    );
                    panic!();
                }
            }
            Err(err) => {
                println!("ERROR: getting mint account {}: {}", mint, err);
                panic!();
            }
        }
    }

    pub async fn refresh_reserve(&self, reserve_key: Pubkey) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let ix = RefreshReserve {
            reserve: reserve_key,
            market_price_feed: reserve.config.market_price_feed,
            irm: reserve.config.irm,
        }
        .into_instruction();

        self.update_prices(&[reserve_key]).await;

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Refreshed reserve: {}", reserve_key);
    }

    pub async fn delete_reserve(&self, reserve_key: Pubkey) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let ix = DeleteReserve {
            reserve: reserve_key,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            pool: reserve.pool,
        }
        .into_instruction();

        self.send_transaction_by(vec![ix], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Reserve deleted: {}", reserve_key);
    }

    pub async fn deposit(&self, reserve_key: Pubkey, amount: u64) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let lp_mint = find_lp_token_mint(&reserve_key);

        let source_liquidity_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        println!(
            "Will use source liquidity wallet: {}",
            source_liquidity_wallet
        );

        if !self
            .account_exists(&source_liquidity_wallet)
            .await
            .expect("check source_liquidity_wallet existance")
        {
            println!(
                "Creating wallet {} for liquidity tokens.",
                source_liquidity_wallet
            );

            let mint_account = self
                .rpc
                .get_account(&reserve.liquidity.mint)
                .await
                .expect("getting mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &reserve.liquidity.mint,
                &mint_account.owner,
            );

            let version = Version { no_error: true }.into_instruction();

            self.send_transaction_by(vec![ix, version], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let destination_lp_wallet =
            get_associated_token_address(&self.authority.pubkey(), &lp_mint.0);

        if !self
            .account_exists(&destination_lp_wallet)
            .await
            .expect("check destination_lp_wallet existance")
        {
            println!("Creating wallet {} for LP tokens.", destination_lp_wallet);

            let lp_mint_account = self
                .rpc
                .get_account(&lp_mint.0)
                .await
                .expect("getting LP mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &lp_mint.0,
                &lp_mint_account.owner,
            );

            self.send_transaction_by(vec![ix], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let deposit = DepositLiquidity {
            reserve: reserve_key,
            authority: self.authority.pubkey(),
            source_liquidity_wallet,
            destination_lp_wallet,
            amount,
            liquidity_mint: reserve.liquidity.mint,
            liquidity_token_program: self.token_program_by_mint(&reserve.liquidity.mint).await,
        }
        .into_instruction();

        let refresh = RefreshReserve {
            reserve: reserve_key,
            market_price_feed: reserve.config.market_price_feed,
            irm: reserve.config.irm,
        }
        .into_instruction();

        self.update_prices(&[reserve_key]).await;

        let version = Version { no_error: true }.into_instruction();

        self.send_transaction_by(vec![refresh, deposit, version], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Deposited {} to reserve: {}", amount, reserve_key);
    }

    pub async fn withdraw(&self, reserve_key: Pubkey, lp_amount: u64) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let lp_mint = find_lp_token_mint(&reserve_key);

        let destination_liquidity_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );
        let source_lp_wallet = get_associated_token_address(&self.authority.pubkey(), &lp_mint.0);

        if !self
            .account_exists(&source_lp_wallet)
            .await
            .expect("check source_lp_wallet existance")
        {
            println!(
                "Source LP wallet {} with token {} doesn't exist",
                source_lp_wallet, lp_mint.0
            );
        }

        if !self
            .account_exists(&destination_liquidity_wallet)
            .await
            .expect("check destination_liquidity_wallet existance")
        {
            println!(
                "Creating wallet {} for liquidity tokens.",
                destination_liquidity_wallet
            );

            let lp_mint_account = self
                .rpc
                .get_account(&lp_mint.0)
                .await
                .expect("getting LP mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &reserve.liquidity.mint,
                &lp_mint_account.owner,
            );

            self.send_transaction_by(vec![ix], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let refresh = RefreshReserve {
            reserve: reserve_key,
            market_price_feed: reserve.config.market_price_feed,
            irm: reserve.config.irm,
        }
        .into_instruction();

        let withdraw = WithdrawLiquidity {
            reserve: reserve_key,
            authority: self.authority.pubkey(),
            source_lp_wallet,
            destination_liquidity_wallet,
            liquidity_token_program: self.token_program_by_mint(&reserve.liquidity.mint).await,
            lp_amount,
            liquidity_mint: reserve.liquidity.mint,
        }
        .into_instruction();

        self.update_prices(&[reserve_key]).await;

        self.send_transaction_by(vec![refresh, withdraw], &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Withdrawn liquidity from reserve: {}", reserve_key);
    }

    pub async fn create_position(&self, pool: Pubkey, long_short: bool) {
        let new_keypair = Keypair::new();

        let create_ix = system_instruction::create_account(
            &self.authority.pubkey(),
            &new_keypair.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(Position::SIZE)
                .await
                .expect("getting rent"),
            Position::SIZE as u64,
            &SUPER_LENDY_ID,
        );

        let ix = CreatePosition {
            position: new_keypair.pubkey(),
            pool,
            owner: self.authority.pubkey(),
            position_type: if long_short {
                POSITION_TYPE_LONG_SHORT
            } else {
                POSITION_TYPE_CLASSIC
            },
        }
        .into_instruction();

        self.send_transaction_by(vec![create_ix, ix], &[&self.authority, &new_keypair])
            .await
            .expect("Sending TX");

        println!("Created user position: {}", new_keypair.pubkey());
    }

    pub async fn close_position(&self, position: Option<Pubkey>, pool: Option<Pubkey>) {
        if position.is_none() && pool.is_none() {
            println!("Choose position to close");
            return;
        }

        if let Some(pool) = pool {
            let positions = load_positions(&self.rpc).await.expect("loading positions");
            let filtered_keys = positions
                .into_iter()
                .filter_map(|(key, position)| {
                    if position.owner == self.authority.pubkey() && position.pool == pool {
                        Some((key, position))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let mut keys = vec![];
            let mut ixs = vec![];
            for (key, position) in filtered_keys.into_iter() {
                println!("Closing position {}", key);

                for borrow_reserve in position.borrows.iter().filter_map(|borrow| {
                    if borrow.borrowed_amount != 0 {
                        Some(borrow.borrow_reserve)
                    } else {
                        None
                    }
                }) {
                    println!("Repay for reserve {}", borrow_reserve);
                    self.repay(key, borrow_reserve, None).await
                }
                for deposit_reserve in position.collateral.iter().filter_map(|collateral| {
                    if collateral.deposited_amount != 0 {
                        Some(collateral.deposit_reserve)
                    } else {
                        None
                    }
                }) {
                    println!("Unlock for reserve {}", deposit_reserve);
                    self.unlock(key, deposit_reserve, MAX_AMOUNT).await
                }
                keys.push(key);
                ixs.push(
                    ClosePosition {
                        position: key,
                        owner: self.authority.pubkey(),
                    }
                    .into_instruction(),
                );
                if ixs.len() >= 5 {
                    ixs.push(Version { no_error: true }.into_instruction());
                    self.process_transaction(ixs.clone(), &[&self.authority], 5)
                        .await
                        .expect("Sending TX");

                    println!("Closed positions: {:?}", keys);
                    keys.clear();
                    ixs.clear();
                }
            }

            if !ixs.is_empty() {
                ixs.push(Version { no_error: true }.into_instruction());
                self.process_transaction(ixs, &[&self.authority], 5)
                    .await
                    .expect("Sending TX");

                println!("Closed positions: {:?}", keys);
            };
        } else if let Some(position) = position {
            let ix = ClosePosition {
                position,
                owner: self.authority.pubkey(),
            }
            .into_instruction();

            self.send_transaction_by(vec![ix], &[&self.authority])
                .await
                .expect("Sending TX");

            println!("Closed position: {}", position);
        } else {
            println!("Choose position to close");
        }
    }

    pub async fn refresh_position(&self, position_key: Pubkey) {
        let (mut ixs, reserves) = self.refresh_position_ix(position_key).await;

        self.update_prices(&reserves).await;

        ixs.push(Version { no_error: true }.into_instruction());

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Refreshed");
    }

    pub async fn list_positions(
        &self,
        position_addr: Option<Pubkey>,
        owner: Option<Pubkey>,
        pool: Option<Pubkey>,
    ) {
        let positions = load_positions(&self.rpc).await.expect("loading positions");
        let filtered_positions = positions
            .into_iter()
            .filter(|&(position_key, _)| {
                if let Some(position_addr) = position_addr {
                    position_key == position_addr
                } else {
                    true
                }
            })
            .filter(|&(_, position)| {
                if let Some(owner) = owner {
                    position.owner == owner
                } else {
                    true
                }
            })
            .filter(|&(_, position)| {
                if let Some(pool) = pool {
                    position.pool == pool
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        for (key, position) in filtered_positions.iter() {
            println!("Lendy Position at address {}", key);
            println!(
                "Type                 : {}",
                if position.position_type == POSITION_TYPE_CLASSIC {
                    "Lend/Borrow"
                } else if position.position_type == POSITION_TYPE_LONG_SHORT {
                    "Long/Short"
                } else if position.position_type == POSITION_TYPE_LST_LEVERAGE {
                    "LST leverage"
                } else {
                    "unknown"
                }
            );
            println!("Owner                : {}", position.owner);
            println!("Pool                 : {}", position.pool);
            println!("Last update (slot)   : {}", position.last_update.slot);
            println!(
                "Collateral value     : {}",
                position.deposited_value().unwrap_or_default()
            );
            println!(
                "Borrowed value       : {}",
                position.borrowed_value().unwrap_or_default()
            );
            println!(
                "Allowed borrow value : {}",
                position.allowed_borrow_value().unwrap_or_default()
            );
            println!(
                "LTV                  : {}",
                position.ltv().unwrap_or_default()
            );

            let mut total_borrowed_value = Decimal::ZERO;
            let mut total_locked_collateral_value = Decimal::ZERO;

            if position.position_type == POSITION_TYPE_LONG_SHORT {
                println!("---------------------- Longs -----------------------");
            } else {
                println!("-------------------- Collateral ---------------------");
            }
            for (index, collateral) in position.collateral.iter().enumerate() {
                if collateral.deposited_amount > 0 {
                    print!(
                        "{}: reserve {}  amount {}  value {}  memo {}",
                        index,
                        collateral.deposit_reserve,
                        collateral.deposited_amount,
                        collateral.market_value().unwrap_or_default(),
                        String::from_utf8_lossy(&collateral.memo)
                    );

                    if position.position_type == POSITION_TYPE_LONG_SHORT {
                        println!("PnL {}", collateral.pnl().unwrap_or_default());
                    } else {
                        println!(" ");
                    }

                    total_locked_collateral_value = total_locked_collateral_value
                        .checked_add(collateral.market_value().unwrap_or_default())
                        .expect("total_locked_collateral_value");
                }
            }
            if position.position_type == POSITION_TYPE_LONG_SHORT {
                println!("---------------------- Shorts -----------------------");
            } else {
                println!("-------------------- Borrowings ---------------------");
            }
            for (index, borrow) in position.borrows.iter().enumerate() {
                if borrow.borrowed_amount().unwrap_or_default() > Decimal::ZERO {
                    print!(
                        "{}: reserve {}  amount {}  value {}  memo {}",
                        index,
                        borrow.borrow_reserve,
                        borrow.borrowed_amount().unwrap_or_default(),
                        borrow.market_value().unwrap_or_default(),
                        String::from_utf8_lossy(&borrow.memo)
                    );

                    if position.position_type == POSITION_TYPE_LONG_SHORT {
                        println!("PnL {}", borrow.pnl().unwrap_or_default());
                    } else {
                        println!(" ");
                    }

                    total_borrowed_value = total_borrowed_value
                        .checked_add(borrow.market_value().unwrap_or_default())
                        .expect("total_borrowed_value");
                }
            }
            println!("--------------------- Rewards ----------------------");
            for (index, reward) in position.rewards.rewards.iter().enumerate() {
                if reward.accrued_amount > 0 {
                    println!(
                        "{}: amount {}   reward mint {}",
                        index,
                        reward.accrued_amount().unwrap(),
                        reward.reward_mint
                    );
                }
            }
            println!("-------------------------------------");

            println!(
                "Total collateral value   : {}",
                total_locked_collateral_value
            );
            println!("Total borrowed value     : {}", total_borrowed_value);
            println!("===========================================================================");
        }

        println!("Total positions number {}", filtered_positions.len());
    }
    pub async fn lock(
        &self,
        position: Pubkey,
        reserve_key: Pubkey,
        amount: u64,
        memo: Option<String>,
    ) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let lp_mint = find_lp_token_mint(&reserve_key);

        let source_lp_wallet = get_associated_token_address(&self.authority.pubkey(), &lp_mint.0);

        let position_data = self
            .rpc
            .get_account_data(&position)
            .await
            .expect("getting position account");
        let unpacked_position =
            Position::try_from_bytes(&position_data).expect("unpacking position");

        let memo = if let Some(memo) = memo {
            let mut memo_bytes_zero_ended = [0; COLLATERAL_MEMO_LEN];
            let memo_bytes = memo.as_bytes();
            memo_bytes_zero_ended[..memo_bytes.len()].copy_from_slice(memo_bytes);
            memo_bytes_zero_ended
        } else {
            match unpacked_position.find_collateral(reserve_key) {
                Ok((collateral, _index)) => collateral.memo,
                Err(_) => {
                    // collateral not found. Just provide zeroed memo.
                    [0; COLLATERAL_MEMO_LEN]
                }
            }
        };

        let refresh_position_info = self.refresh_position_ix(position).await;
        let mut ixs = refresh_position_info.0;

        if !refresh_position_info
            .1
            .iter()
            .any(|key| key == &reserve_key)
        {
            let refresh_reserve = RefreshReserve {
                reserve: reserve_key,
                market_price_feed: reserve.config.market_price_feed,
                irm: reserve.config.irm,
            }
            .into_instruction();

            ixs.push(refresh_reserve);
        }

        let mut reserves_to_updates_prices_for = refresh_position_info.1;
        reserves_to_updates_prices_for.push(reserve_key);

        let ix = LockCollateral {
            position,
            reserve: reserve_key,
            source_lp_wallet,
            owner: self.authority.pubkey(),
            amount,
            memo,
        }
        .into_instruction();

        ixs.push(ix);

        let version = Version { no_error: true }.into_instruction();
        ixs.push(version);

        self.update_prices(&reserves_to_updates_prices_for).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Locked {} LPs in reserve: {}", amount, reserve_key);
    }

    pub async fn unlock(&self, position: Pubkey, reserve_key: Pubkey, amount: u64) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let lp_mint = find_lp_token_mint(&reserve_key);

        let destination_lp_wallet =
            get_associated_token_address(&self.authority.pubkey(), &lp_mint.0);

        let refresh_position_info = self.refresh_position_ix(position).await;
        let mut ixs = refresh_position_info.0;

        if !refresh_position_info
            .1
            .iter()
            .any(|key| key == &reserve_key)
        {
            let refresh_reserve = RefreshReserve {
                reserve: reserve_key,
                market_price_feed: reserve.config.market_price_feed,
                irm: reserve.config.irm,
            }
            .into_instruction();

            ixs.push(refresh_reserve);
        }

        let mut reserves_to_updates_prices_for = refresh_position_info.1;
        reserves_to_updates_prices_for.push(reserve_key);

        self.update_prices(&reserves_to_updates_prices_for).await;

        let ix = UnlockCollateral {
            position,
            reserve: reserve_key,
            destination_lp_wallet,
            owner: self.authority.pubkey(),
            amount,
        }
        .into_instruction();

        ixs.push(ix);

        let version = Version { no_error: true }.into_instruction();

        ixs.push(version);

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Unlocked {} LPs from reserve: {}", amount, reserve_key);
    }

    pub async fn refresh_reserves_ix(&self, reserves: &[Pubkey]) -> Vec<Instruction> {
        let mut ixs = Vec::new();

        for reserve in reserves {
            let reserve_data = self
                .rpc
                .get_account_data(reserve)
                .await
                .expect("getting Reserve account");
            let unpacked_reserve =
                Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

            let refresh_reserve = RefreshReserve {
                reserve: *reserve,
                market_price_feed: unpacked_reserve.config.market_price_feed,
                irm: unpacked_reserve.config.irm,
            }
            .into_instruction();

            ixs.push(refresh_reserve);
        }

        ixs
    }

    // Refresh all Reserves mentioned in the Position and then refresh Position itself.
    pub async fn refresh_position_ix(
        &self,
        position_key: Pubkey,
    ) -> (Vec<Instruction>, /* all reserves*/ Vec<Pubkey>) {
        let position_data = self
            .rpc
            .get_account_data(&position_key)
            .await
            .expect("getting Position account");
        let position = Position::try_from_bytes(&position_data).expect("unpacking Position");

        let deposits_reserves: Vec<Pubkey> = position
            .collateral
            .iter()
            .filter_map(|dep| {
                if dep.deposited_amount > 0 {
                    Some(dep.deposit_reserve)
                } else {
                    None
                }
            })
            .collect();

        let borrows_reserves: Vec<Pubkey> = position
            .borrows
            .iter()
            .filter_map(|bor| {
                if bor.borrowed_amount().unwrap_or_default() > Decimal::ZERO {
                    Some(bor.borrow_reserve)
                } else {
                    None
                }
            })
            .collect();

        let ix = RefreshPosition {
            position: position_key,
            deposits: deposits_reserves.clone(),
            borrows: borrows_reserves.clone(),
        }
        .into_instruction();

        let mut refresh_ixs = self.refresh_reserves_ix(&deposits_reserves).await;
        let refresh_borrows = self.refresh_reserves_ix(&borrows_reserves).await;

        refresh_ixs.extend(refresh_borrows.iter().cloned());
        refresh_ixs.push(ix);

        let mut all_reserves = deposits_reserves;
        all_reserves.extend(borrows_reserves);

        (refresh_ixs, all_reserves)
    }

    pub async fn borrow(
        &self,
        position: Pubkey,
        reserve_key: Pubkey,
        amount: u64,
        slippage: u64,
        memo: Option<String>,
    ) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let destination_liquidity_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        if !self
            .account_exists(&destination_liquidity_wallet)
            .await
            .expect("check destination_liquidity_wallet existance")
        {
            println!(
                "Creating wallet {} for liquidity tokens.",
                destination_liquidity_wallet
            );

            let mint_account = self
                .rpc
                .get_account(&reserve.liquidity.mint)
                .await
                .expect("getting mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &reserve.liquidity.mint,
                &mint_account.owner,
            );

            let version = Version { no_error: true }.into_instruction();

            self.send_transaction_by(vec![ix, version], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let cfg_data = self
            .rpc
            .get_account_data(&TEXTURE_CONFIG_ID)
            .await
            .expect("getting Pool account");
        let cfg = TextureConfig::try_from_bytes(&cfg_data).expect("unpacking Global Config");

        let texture_fee_receiver = get_associated_token_address_with_program_id(
            &cfg.fees_authority,
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let curator_data = self
            .rpc
            .get_account_data(&pool.curator)
            .await
            .expect("getting Curator account");
        let curator = Curator::try_from_bytes(&curator_data).expect("unpacking Curator");

        let curator_fee_receiver = get_associated_token_address_with_program_id(
            &curator.fees_authority,
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        if !self
            .account_exists(&curator_fee_receiver)
            .await
            .expect("check account existance")
        {
            println!(
                "Curator's fee receiver account {} for token {} doesn't exist",
                curator_fee_receiver, reserve.liquidity.mint
            );
        }

        if !self
            .account_exists(&texture_fee_receiver)
            .await
            .expect("check account existance")
        {
            println!(
                "Texture fee receiver account {} for token {} doesn't exist",
                texture_fee_receiver, reserve.liquidity.mint
            );
        }

        let position_data = self
            .rpc
            .get_account_data(&position)
            .await
            .expect("getting position account");
        let unpacked_position =
            Position::try_from_bytes(&position_data).expect("unpacking position");

        let memo = if let Some(memo) = memo {
            let mut memo_bytes_zero_ended = [0; BORROW_MEMO_LEN];
            let memo_bytes = memo.as_bytes();
            memo_bytes_zero_ended[..memo_bytes.len()].copy_from_slice(memo_bytes);
            memo_bytes_zero_ended
        } else {
            match unpacked_position.find_borrowed_liquidity(reserve_key) {
                Ok((borrow, _index)) => borrow.memo,
                Err(_) => {
                    // collateral not found. Just provide zeroed memo.
                    [0; BORROW_MEMO_LEN]
                }
            }
        };

        let borrow_ix = Borrow {
            position,
            reserve: reserve_key,
            pool: reserve.pool,
            destination_liquidity_wallet,
            curator_fee_receiver,
            texture_fee_receiver,
            borrower: self.authority.pubkey(),
            amount,
            slippage_limit: slippage,
            curator: pool.curator,
            memo,
            token_program: self.token_program_by_mint(&reserve.liquidity.mint).await,
            liquidity_mint: reserve.liquidity.mint,
        }
        .into_instruction();

        let refresh_position_info = self.refresh_position_ix(position).await;
        let mut ixs = refresh_position_info.0;
        let refresh_borrow_reserve = self.refresh_reserves_ix(&[reserve_key]).await;
        ixs.extend(refresh_borrow_reserve);
        ixs.push(borrow_ix);

        let mut reserves_to_updates_prices_for = refresh_position_info.1;
        reserves_to_updates_prices_for.push(reserve_key);

        self.update_prices(&reserves_to_updates_prices_for).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Borrowed");
    }

    pub async fn repay(&self, position: Pubkey, reserve_key: Pubkey, amount: Option<u64>) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let source_liquidity_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        let repay_ix = Repay {
            position,
            reserve: reserve_key,
            source_liquidity_wallet,
            user_authority: self.authority.pubkey(),
            token_program: self.token_program_by_mint(&reserve.liquidity.mint).await,
            amount: amount.unwrap_or(MAX_AMOUNT),
            liquidity_mint: reserve.liquidity.mint,
        }
        .into_instruction();

        let refresh_position_info = self.refresh_position_ix(position).await;
        let mut ixs = refresh_position_info.0;
        ixs.push(repay_ix);

        ixs.push(Version { no_error: true }.into_instruction());

        self.update_prices(&refresh_position_info.1).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Repaid");
    }

    pub async fn claim_curator_perf_fee(&self, reserve_key: Pubkey) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let pool_data = self
            .rpc
            .get_account_data(&reserve.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let curator_data = self
            .rpc
            .get_account_data(&pool.curator)
            .await
            .expect("getting Curator account");
        let curator = Curator::try_from_bytes(&curator_data).expect("unpacking Curator");

        let fee_receiver = get_associated_token_address_with_program_id(
            &curator.fees_authority,
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        let claim_ix = ClaimCuratorPerformanceFees {
            reserve: reserve_key,
            pool: reserve.pool,
            curator: pool.curator,
            token_program: self.token_program_by_mint(&reserve.liquidity.mint).await,
            fee_receiver,
            liquidity_mint: reserve.liquidity.mint,
        }
        .into_instruction();

        let mut ixs = self.refresh_reserves_ix(&[reserve_key]).await;

        ixs.push(claim_ix);

        self.update_prices(&[reserve_key]).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Claimed");
    }

    pub async fn claim_texture_perf_fee(&self, reserve_key: Pubkey) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve_key)
            .await
            .expect("getting Reserve account");
        let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        // Read global config
        let cfg_data = self
            .rpc
            .get_account_data(&TEXTURE_CONFIG_ID)
            .await
            .expect("getting Pool account");
        let cfg = TextureConfig::try_from_bytes(&cfg_data).expect("unpacking Global Config");

        // Find or create SPL wallet for fees
        let fee_receiver_wallet = get_associated_token_address_with_program_id(
            &cfg.fees_authority,
            &reserve.liquidity.mint,
            &self.token_program_by_mint(&reserve.liquidity.mint).await,
        );

        if !self
            .account_exists(&fee_receiver_wallet)
            .await
            .expect("check fee_receiver_wallet existance")
        {
            println!("Creating wallet {} for fees", fee_receiver_wallet);

            let mint_account = self
                .rpc
                .get_account(&reserve.liquidity.mint)
                .await
                .expect("getting mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &reserve.liquidity.mint,
                &mint_account.owner,
            );

            self.send_transaction_by(vec![ix], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let claim_ix = ClaimTexturePerformanceFees {
            reserve: reserve_key,
            fee_receiver: fee_receiver_wallet,
            liquidity_mint: reserve.liquidity.mint,
            token_program: self.token_program_by_mint(&reserve.liquidity.mint).await,
        }
        .into_instruction();

        let mut ixs = self.refresh_reserves_ix(&[reserve_key]).await;

        ixs.push(claim_ix);

        self.update_prices(&[reserve_key]).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Claimed");
    }

    pub async fn liquidate(
        &self,
        position: Pubkey,
        principal_reserve_key: Pubkey,
        collateral_reserve_key: Pubkey,
        principal_amount: Option<u64>,
    ) {
        let principal_reserve_data = self
            .rpc
            .get_account_data(&principal_reserve_key)
            .await
            .expect("getting principal_reserve account");
        let principal_reserve =
            Reserve::try_from_bytes(&principal_reserve_data).expect("unpacking principal Reserve");

        let repayment_source_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &principal_reserve.liquidity.mint,
            &self
                .token_program_by_mint(&principal_reserve.liquidity.mint)
                .await,
        );

        if !self
            .account_exists(&repayment_source_wallet)
            .await
            .expect("check repayment_source_wallet existance")
        {
            println!("You don't have SPL wallet for {} tokens. Get this tokens first and then try to liquidate again.", principal_reserve.liquidity.mint);
            return;
        }

        let collateral_lp_mint = find_lp_token_mint(&collateral_reserve_key);

        let destination_lp_wallet =
            get_associated_token_address(&self.authority.pubkey(), &collateral_lp_mint.0);

        if !self
            .account_exists(&destination_lp_wallet)
            .await
            .expect("check destination_lp_wallet existance")
        {
            println!("Creating wallet {} for LPs", destination_lp_wallet);

            let lp_mint_account = self
                .rpc
                .get_account(&collateral_lp_mint.0)
                .await
                .expect("getting mint account");

            let ix = create_associated_token_account(
                &self.authority.pubkey(),
                &self.authority.pubkey(),
                &collateral_lp_mint.0,
                &lp_mint_account.owner,
            );

            self.send_transaction_by(vec![ix], &vec![&self.authority])
                .await
                .expect("Sending create ATA TX");
        }

        let liquidate_ix = Liquidate {
            repayment_source_wallet,
            destination_lp_wallet,
            principal_reserve: principal_reserve_key,
            collateral_reserve: collateral_reserve_key,
            position,
            liquidator: self.authority.pubkey(),
            principal_reserve_liquidity_mint: principal_reserve.liquidity.mint,
            principal_token_program: self
                .token_program_by_mint(&principal_reserve.liquidity.mint)
                .await,
            liquidity_amount: principal_amount.unwrap_or(MAX_AMOUNT),
        }
        .into_instruction();

        let refresh_position_info = self.refresh_position_ix(position).await;
        let mut ixs = refresh_position_info.0;
        ixs.push(liquidate_ix);

        self.update_prices(&refresh_position_info.1).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Liquidated");
    }

    pub async fn write_off_bad_debt(
        &self,
        position_key: Pubkey,
        reserve_key: Pubkey,
        amount: Option<u64>,
    ) {
        let position_data = self
            .rpc
            .get_account_data(&position_key)
            .await
            .expect("getting position account");
        let position = Position::try_from_bytes(&position_data).expect("unpacking Position");

        let pool_data = self
            .rpc
            .get_account_data(&position.pool)
            .await
            .expect("getting Pool account");
        let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

        let write_off_ix = WriteOffBadDebt {
            pool: position.pool,
            reserve: reserve_key,
            position: position_key,
            curator_pools_authority: self.authority.pubkey(),
            curator: pool.curator,
            amount: amount.unwrap_or(MAX_AMOUNT),
        }
        .into_instruction();

        let refresh_position_info = self.refresh_position_ix(position_key).await;
        let mut ixs = refresh_position_info.0;
        ixs.push(write_off_ix);

        let version = Version { no_error: true }.into_instruction();
        ixs.push(version);

        self.update_prices(&refresh_position_info.1).await;

        self.send_transaction_by(ixs, &[&self.authority])
            .await
            .expect("Sending TX");

        println!("Bad debt written off");
    }

    #[async_recursion]
    pub async fn update_prices(&self, reserves: &[Pubkey]) {
        let client = PriceProxyClient {
            rpc: RpcClient::new(self.url.clone()),
            authority: Keypair::from_base58_string(&self.authority.to_base58_string()),
            priority_fee: None,
        };
        for reserve in reserves {
            let reserve_data = self
                .rpc
                .get_account_data(reserve)
                .await
                .expect("getting Reserve account");
            let unpacked_reserve =
                Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");
            let price_feed_key = unpacked_reserve.config.market_price_feed;
            let price_feed_data = self
                .rpc
                .get_account_data(&price_feed_key)
                .await
                .expect("getting PriceFeed account");
            let price_feed =
                PriceFeed::try_from_bytes(&price_feed_data).expect("unpacking PriceFeed");

            let transform_price_update = if price_feed.feed_type() == FeedType::Transform
                && price_feed.transform_source() == PriceFeedSource::Pyth
            {
                let price_update_keypair = Keypair::new();
                let price_update_key = price_update_keypair.pubkey();

                // Convert feed pubkey to hex.
                let hex: String = price_feed.transform_source_address.encode_hex();

                // Get a Hermes update from Hermes stable.
                let message = client
                    .get_message_by_hex(&hex, None)
                    .await
                    .expect("get message");
                let message = message.first().unwrap();

                // 1. Post a Pyth price update onto Solana.
                let price_update = match price_feed.verification_level() {
                    WormholeVerificationLevel::Full => {
                        let encoded_vaa_keypair = Keypair::new();
                        let encoded_vaa = encoded_vaa_keypair.pubkey();

                        let write_encoded_vaa =
                            client.write_encoded_vaa_ix(message, encoded_vaa).await;
                        self.send_transaction_by(
                            write_encoded_vaa,
                            &[&self.authority, &encoded_vaa_keypair],
                        )
                        .await
                        .expect("Sending TX");
                        client
                            .post_update_ix(price_update_key, message, encoded_vaa)
                            .await
                    }
                    WormholeVerificationLevel::Partial => {
                        client
                            .post_update_atomic_ix(price_update_key, message)
                            .await
                    }
                };
                self.send_transaction_by(price_update, &[&self.authority, &price_update_keypair])
                    .await
                    .expect("Sending TX");

                Some(price_update_key)
            } else {
                None
            };

            let transform_source_address =
                if let Some(transform_price_update) = transform_price_update {
                    transform_price_update
                } else {
                    price_feed.transform_source_address
                };

            match price_feed.source() {
                PriceFeedSource::Pyth => {
                    let price_update_keypair = Keypair::new();
                    let price_update_key = price_update_keypair.pubkey();

                    // Convert feed pubkey to hex.
                    let hex: String = price_feed.source_address.encode_hex();

                    // Get a Hermes update from Hermes stable.
                    let message = client
                        .get_message_by_hex(&hex, None)
                        .await
                        .expect("get message");
                    let message = message.first().unwrap();

                    // 1. Post a Pyth price update onto Solana.
                    let price_update = match price_feed.verification_level() {
                        WormholeVerificationLevel::Full => {
                            let encoded_vaa_keypair = Keypair::new();
                            let encoded_vaa = encoded_vaa_keypair.pubkey();

                            let write_encoded_vaa =
                                client.write_encoded_vaa_ix(message, encoded_vaa).await;
                            self.send_transaction_by(
                                write_encoded_vaa,
                                &[&self.authority, &encoded_vaa_keypair],
                            )
                            .await
                            .expect("Sending TX");
                            client
                                .post_update_ix(price_update_key, message, encoded_vaa)
                                .await
                        }
                        WormholeVerificationLevel::Partial => {
                            client
                                .post_update_atomic_ix(price_update_key, message)
                                .await
                        }
                    };
                    self.send_transaction_by(
                        price_update,
                        &[&self.authority, &price_update_keypair],
                    )
                    .await
                    .expect("Sending TX");

                    // 2. Update Price-feed.
                    let update_price = client
                        .update_price_ix(
                            price_feed_key,
                            price_update_key,
                            transform_source_address,
                            u32::MAX as u64,
                        )
                        .await;

                    self.send_transaction_by(update_price, &[&self.authority])
                        .await
                        .expect("Sending TX");
                    println!("Updated price for feed: {}", price_feed_key);

                    // 3. Close a price update account, recovering the rent.
                    let close_price_update = client.close_price_update_ix(price_update_key).await;
                    self.send_transaction_by(close_price_update, &[&self.authority])
                        .await
                        .expect("Sending TX");
                    println!(
                        "Close a price update account: {}, recovering the rent",
                        price_update_key
                    );
                }
                PriceFeedSource::OffChain => {
                    client
                        .write_price_ix(
                            price_feed_key,
                            price_feed.try_price().unwrap(),
                            chrono::Utc::now().timestamp(),
                        )
                        .await;
                    println!("Updated price for feed: {}", price_feed_key);
                }
                PriceFeedSource::Switchboard | PriceFeedSource::StakePool => {
                    // For switchboard or stakepool we just call PriceProxy::update_price for relevant price feed.
                    let update_price_ixs = client
                        .update_price_ix(
                            price_feed_key,
                            price_feed.source_address,
                            transform_source_address,
                            u32::MAX as u64,
                        )
                        .await;

                    self.send_transaction_by(update_price_ixs, &[&self.authority])
                        .await
                        .expect("Sending TX");
                    println!("Updated price for feed: {}", price_feed_key);
                }
                PriceFeedSource::SuperLendy => {
                    // The algorithm:
                    // 1. Refresh Reserve0 which is the source of LP tokens (used as liquidity in Reserve1 -
                    //    reserve of our final interest)
                    // 2. Call PriceProxy::UpdatePrice for the price feed used in Reserve1. This action will use
                    //    LP token price, calculated by SuperLendy on step 1.
                    self.update_prices(&[price_feed.source_address]).await;

                    let mut ixs = self.refresh_reserves_ix(&[price_feed.source_address]).await;

                    let update_price_ixs = client
                        .update_price_ix(
                            price_feed_key,
                            price_feed.source_address,
                            transform_source_address,
                            u32::MAX as u64,
                        )
                        .await;

                    ixs.extend(update_price_ixs);

                    self.send_transaction_by(ixs, &[&self.authority])
                        .await
                        .expect("Sending TX");
                    println!("Updated price for feed: {}", price_feed_key);
                }
                _ => {
                    println!("ERROR Bad source {}", price_feed.source());
                }
            }

            if let Some(transform_price_update) = transform_price_update {
                // Close a price update account, recovering the rent.
                let close_price_update = client.close_price_update_ix(transform_price_update).await;
                self.send_transaction_by(close_price_update, &[&self.authority])
                    .await
                    .expect("Sending TX");
                println!(
                    "Close a price update account: {}, recovering the rent",
                    transform_price_update
                );
            }
        }
    }

    pub async fn flash_test(&self, reserve: Pubkey, amount: u64) {
        let reserve_data = self
            .rpc
            .get_account_data(&reserve)
            .await
            .expect("getting Reserve account");
        let unpacked_reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");

        let ata_wallet = get_associated_token_address_with_program_id(
            &self.authority.pubkey(),
            &unpacked_reserve.liquidity.mint,
            &self
                .token_program_by_mint(&unpacked_reserve.liquidity.mint)
                .await,
        );

        if !self
            .account_exists(&ata_wallet)
            .await
            .expect("check destination_wallet existance")
        {
            println!("You do NOT have ATA wallet fro {} tokens with non zero balance. Get the wallet first and then try again", unpacked_reserve.liquidity.mint);
            return;
        }

        // Create new SPL token wallet for that user which is not ATA. Will receive flash loaned tokens there.
        let new_wallet_kp = Keypair::new();
        let create_ix = system_instruction::create_account(
            &self.authority.pubkey(),
            &new_wallet_kp.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
                .await
                .expect("getting rent"),
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        );

        let init_wallet = spl_token::instruction::initialize_account3(
            &spl_token::id(),
            &new_wallet_kp.pubkey(),
            &unpacked_reserve.liquidity.mint,
            &self.authority.pubkey(),
        )
        .unwrap();

        self.send_transaction_by(
            vec![create_ix, init_wallet],
            &vec![&self.authority, &new_wallet_kp],
        )
        .await
        .expect("Sending create ATA TX");

        println!("Created test wallet {}", new_wallet_kp.pubkey());

        let borrow_ix = FlashBorrow {
            reserve,
            destination_wallet: new_wallet_kp.pubkey(),
            amount,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: self
                .token_program_by_mint(&unpacked_reserve.liquidity.mint)
                .await,
            liquidity_mint: unpacked_reserve.liquidity.mint,
        }
        .into_instruction();

        let repay_ix = FlashRepay {
            reserve,
            source_wallet: ata_wallet,
            amount,
            user_transfer_authority: self.authority.pubkey(),
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: self
                .token_program_by_mint(&unpacked_reserve.liquidity.mint)
                .await,
            liquidity_mint: unpacked_reserve.liquidity.mint,
        }
        .into_instruction();

        self.update_prices(&[reserve]).await;

        let refresh_reserve = self.refresh_reserves_ix(&[reserve]).await;

        self.send_transaction_by(
            vec![
                refresh_reserve[0].clone(),
                borrow_ix,
                refresh_reserve[0].clone(),
                repay_ix,
            ],
            &[&self.authority],
        )
        .await
        .expect("Sending TX");

        println!("Withdrawn");
    }

    // Reads SuperBack's tokens config file and enrich it with token `standard` field.
    pub async fn put_tokens_standards(&self, input_cfg_path: String, output_cfg_path: String) {
        let data = std::fs::read_to_string(input_cfg_path).expect("read tokens config");
        let input_config: Vec<TokenCfgEntry> =
            serde_json::from_str(&data).expect("parse tokens config");
        let mut output_config: Vec<TokenCfgEntry> = Vec::new();

        println!("read {} cfg entries", input_config.len());

        for config_entry in input_config {
            let mut updated_entry = config_entry;

            match self.rpc.get_account(&updated_entry.mint).await {
                Ok(account) => {
                    if account.owner == spl_token::id() {
                        let mint =
                            spl_token::state::Mint::unpack(&account.data).expect("unpacking mint");
                        updated_entry.standard = Some("Token".to_string());
                        updated_entry.decimals = Some(mint.decimals);
                    } else if account.owner == spl_token_2022::id() {
                        let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(
                            &account.data,
                        )
                        .expect("unpacking mint");
                        updated_entry.standard = Some("Token2022".to_string());
                        updated_entry.decimals = Some(mint.base.decimals);
                    } else {
                        println!(
                            "ERROR: unrecognized owner {} of mint account {}",
                            account.owner, updated_entry.mint
                        );
                        continue;
                    }
                    output_config.push(updated_entry);
                }
                Err(err) => {
                    println!(
                        "ERROR: getting mint account {}: {}",
                        updated_entry.mint, err
                    );
                }
            }
            print!(".");
        }
        println!("+");

        let out_string =
            serde_json::to_string_pretty(&output_config).expect("to string conversion");
        std::fs::write(&output_cfg_path, out_string).expect("write tokens config")
    }

    pub async fn execute_miltisig(&self, multisig: Keypair, base58_tx: String) {
        let decoded_tx = bs58::decode(base58_tx).into_vec().unwrap();
        let mut tx: Transaction = bincode::deserialize(&decoded_tx).unwrap();

        let blockhash = self
            .rpc
            .get_latest_blockhash()
            .await
            .expect("get_latest_blockhash");

        println!("sign by {:?}", multisig);
        println!("tx {:?}", tx);

        tx.sign(&[&multisig], blockhash);

        println!("tx after sign {:?}", tx);

        let signature = self
            .rpc
            .send_and_confirm_transaction_with_spinner(&tx)
            .await
            .map_err(with_logs)
            .unwrap();

        println!("Signature: {}", signature);
    }

    pub async fn gen_unhealthy_positions(
        &self,
        position_config: String,
        price_feed_authority: Keypair,
    ) {
        gen_unhealthy_positions(self, position_config, price_feed_authority).await
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Display, PartialEq)]
#[display(
    fmt = "{}",
    "serde_json::to_string(self).expect(\"token config to json\")"
)]
pub struct TokenCfgEntry {
    pub symbol: String,
    #[serde_as(as = "DisplayFromStr")]
    pub mint: Pubkey,
    pub logo_url: String,
    pub standard: Option<String>,
    pub decimals: Option<u8>,
}
struct Logs(Vec<String>);

impl Display for Logs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\nLogs:")?;

        for (i, log) in self.0.iter().enumerate() {
            writeln!(f, "    {:>3}: {}", i + 1, log)?;
        }
        Ok(())
    }
}

pub fn with_logs(mut error: ClientError) -> anyhow::Error {
    let logs = match error.kind {
        ClientErrorKind::RpcError(RpcError::RpcResponseError {
            data:
                RpcResponseErrorData::SendTransactionPreflightFailure(RpcSimulateTransactionResult {
                    ref mut logs,
                    ..
                }),
            ..
        }) => logs.take().map(Logs),
        _ => None,
    };

    if let Some(logs) = logs {
        anyhow::Error::from(error).context(logs)
    } else {
        error.into()
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Display, PartialEq)]
#[display(
    fmt = "{}",
    "serde_json::to_string(self).expect(\"LUT config to json\")"
)]
pub struct LutCfgEntry {
    #[serde_as(as = "DisplayFromStr")]
    pub pool: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lut: Pubkey,
}

pub fn read_lut_config(path: &str) -> Result<Vec<LutCfgEntry>> {
    let text = std::fs::read_to_string(path).expect("Can't read LUT config provided");
    let config: Vec<LutCfgEntry> = serde_json::from_str(&text)?;
    Ok(config)
}
