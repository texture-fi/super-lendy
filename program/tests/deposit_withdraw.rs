#![cfg(feature = "test-bpf")]

use std::str::FromStr;

use bytemuck::Zeroable;
use price_proxy::state::utils::str_to_array;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use texture_common::account::PodAccount;
use texture_common::math::Decimal;
use tracing::info;

use super_lendy::pda::{find_liquidity_supply, find_lp_token_mint};
use super_lendy::state::curator::CuratorParams;
use super_lendy::state::pool::PoolParams;
use super_lendy::state::position::Position;
use super_lendy::state::reserve::{Reserve, ReserveConfig, ReserveFeesConfig, RESERVE_TYPE_NORMAL};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    borrow, create_curator, create_pool, create_reserve, create_texture_config, deposit_liquidity,
    lock_collateral, refresh_position, withdraw_liquidity,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, get_token_account, init_program_test,
    init_token_accounts, lender_keypair, texture_config_keypair, Runner, LAMPORTS,
    LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#23f7fd906b434bdeb6d4043b10c0bb03
pub async fn deposit_success(liquidity_mint: Pubkey) {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    init_token_accounts(&mut runner, &liquidity_mint);
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: ReserveTimelock {
            market_price_feed_lock_sec: 0,
            irm_lock_sec: 0,
            liquidation_bonus_lock_sec: 0,
            unhealthy_ltv_lock_sec: 0,
            partial_liquidation_factor_lock_sec: 0,
            max_total_liquidity_lock_sec: 0,
            max_borrow_ltv_lock_sec: 0,
            max_borrow_utilization_lock_sec: 0,
            price_stale_threshold_lock_sec: 0,
            max_withdraw_utilization_lock_sec: 0,
            fees_lock_sec: 0,
            _padding: 0,
        },
    };
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    // CREATE CURATOR

    let params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: pool_authority_pubkey,
        pools_authority: pool_authority_pubkey,
        vaults_authority: pool_authority_pubkey,
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };
    create_curator(
        &mut ctx,
        &curator_keypair,
        &admin_keypair,
        &owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    let params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        &mut ctx,
        &pool_keypair,
        &pool_authority_keypair,
        curator_pubkey,
        params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 9000,
    };

    create_reserve(
        &mut ctx,
        &reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    // CREATE LP TOKEN WALLET

    let lp_mint = find_lp_token_mint(&reserve_pubkey).0;
    let destination_lp_wallet =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let mint_acc = get_account(&mut ctx.banks_client, liquidity_mint)
        .await
        .expect("get mint acc");
    let source_liquidity_wallet = get_associated_token_address_with_program_id(
        &lender_pubkey,
        &liquidity_mint,
        &mint_acc.owner,
    );
    let amount = 100_u64;

    // DEPOSIT LIQUIDITY

    let source_token_acc0 = get_token_account(&mut ctx.banks_client, source_liquidity_wallet)
        .await
        .expect("get token acc");
    let destination_token_acc0 = get_token_account(&mut ctx.banks_client, destination_lp_wallet)
        .await
        .expect("get token acc");

    info!("deposit liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        amount,
    )
    .await
    .expect("deposit_liquidity");

    let source_token_acc1 = get_token_account(&mut ctx.banks_client, source_liquidity_wallet)
        .await
        .expect("get token acc");
    let destination_token_acc1 = get_token_account(&mut ctx.banks_client, destination_lp_wallet)
        .await
        .expect("get token acc");

    // CHECK DECREASE SOURCE TOKEN WALLET BALANCE
    assert_eq!(source_token_acc1.amount, source_token_acc0.amount - amount);

    // CHECK INCREASE DESTINATION LP TOKEN WALLET BALANCE
    assert_eq!(
        destination_token_acc1.amount,
        destination_token_acc0.amount + amount
    )
}

#[tokio::test]
async fn run_deposit_success() {
    let spl_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    deposit_success(spl_mint).await; // run with spl_token SOL mint

    let spl_mint2022 = Pubkey::from_str("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo").unwrap();
    deposit_success(spl_mint2022).await; // run with spl_token2022 pyUSD mint
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#23f7fd906b434bdeb6d4043b10c0bb03
#[tokio::test]
async fn deposit_incorrect_source() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    init_token_accounts(&mut runner, &liquidity_mint);
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: ReserveTimelock {
            market_price_feed_lock_sec: 0,
            irm_lock_sec: 0,
            liquidation_bonus_lock_sec: 0,
            unhealthy_ltv_lock_sec: 0,
            partial_liquidation_factor_lock_sec: 0,
            max_total_liquidity_lock_sec: 0,
            max_borrow_ltv_lock_sec: 0,
            max_borrow_utilization_lock_sec: 0,
            price_stale_threshold_lock_sec: 0,
            max_withdraw_utilization_lock_sec: 0,
            fees_lock_sec: 0,
            _padding: 0,
        },
    };
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    // CREATE CURATOR

    let params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: pool_authority_pubkey,
        pools_authority: pool_authority_pubkey,
        vaults_authority: pool_authority_pubkey,
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };
    create_curator(
        &mut ctx,
        &curator_keypair,
        &admin_keypair,
        &owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    let params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        &mut ctx,
        &pool_keypair,
        &pool_authority_keypair,
        curator_pubkey,
        params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 9000,
    };

    create_reserve(
        &mut ctx,
        &reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    // CREATE LP TOKEN WALLET

    let lp_mint = find_lp_token_mint(&reserve_pubkey).0;
    let destination_lp_wallet =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let amount = 100_u64;

    // TRY TO DEPOSIT LIQUIDITY FROM RESERVE LIQUIDITY SUPPLY

    let source_liquidity_wallet = find_liquidity_supply(&reserve_pubkey).0;

    info!("deposit liquidity from reserve liquidity supply");
    let result = deposit_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        amount,
    )
    .await;

    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#3e3b7da2d79f40b08fa48ca32c348451
#[tokio::test]
async fn deposit_max_total_liquidity() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    init_token_accounts(&mut runner, &liquidity_mint);
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: ReserveTimelock {
            market_price_feed_lock_sec: 0,
            irm_lock_sec: 0,
            liquidation_bonus_lock_sec: 0,
            unhealthy_ltv_lock_sec: 0,
            partial_liquidation_factor_lock_sec: 0,
            max_total_liquidity_lock_sec: 0,
            max_borrow_ltv_lock_sec: 0,
            max_borrow_utilization_lock_sec: 0,
            price_stale_threshold_lock_sec: 0,
            max_withdraw_utilization_lock_sec: 0,
            fees_lock_sec: 0,
            _padding: 0,
        },
    };
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    // CREATE CURATOR

    let params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: pool_authority_pubkey,
        pools_authority: pool_authority_pubkey,
        vaults_authority: pool_authority_pubkey,
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };
    create_curator(
        &mut ctx,
        &curator_keypair,
        &admin_keypair,
        &owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    let params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        &mut ctx,
        &pool_keypair,
        &pool_authority_keypair,
        curator_pubkey,
        params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let max_total_liquidity = 100_000_000;
    let config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 9000,
    };

    create_reserve(
        &mut ctx,
        &reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    // CREATE LP TOKEN WALLET

    let lp_mint = find_lp_token_mint(&reserve_pubkey).0;
    let destination_lp_wallet =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_liquidity_wallet = get_associated_token_address(&lender_pubkey, &liquidity_mint);

    // TRY TO DEPOSIT max_total_liquidity + 1

    let source_token_acc0 = get_token_account(&mut ctx.banks_client, source_liquidity_wallet)
        .await
        .expect("get token acc");
    let destination_token_acc0 = get_token_account(&mut ctx.banks_client, destination_lp_wallet)
        .await
        .expect("get token acc");

    info!("try to deposit max_total_liquidity + 1");
    let result = deposit_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        max_total_liquidity + 1,
    )
    .await;
    assert!(result.is_err());

    let source_token_acc1 = get_token_account(&mut ctx.banks_client, source_liquidity_wallet)
        .await
        .expect("get token acc");
    let destination_token_acc1 = get_token_account(&mut ctx.banks_client, destination_lp_wallet)
        .await
        .expect("get token acc");

    // CHECK BALANCES NOT CHANGED
    assert_eq!(source_token_acc1.amount, source_token_acc0.amount);
    assert_eq!(destination_token_acc1.amount, destination_token_acc0.amount);

    // CHECK TOTAL LIQUIDITY NOT CHANGED
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(reserve.liquidity.total_liquidity().unwrap(), Decimal::ZERO)
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#23f7fd906b434bdeb6d4043b10c0bb03
pub async fn withdraw_success(liquidity_mint: Pubkey) {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    init_token_accounts(&mut runner, &liquidity_mint);
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: ReserveTimelock {
            market_price_feed_lock_sec: 0,
            irm_lock_sec: 0,
            liquidation_bonus_lock_sec: 0,
            unhealthy_ltv_lock_sec: 0,
            partial_liquidation_factor_lock_sec: 0,
            max_total_liquidity_lock_sec: 0,
            max_borrow_ltv_lock_sec: 0,
            max_borrow_utilization_lock_sec: 0,
            price_stale_threshold_lock_sec: 0,
            max_withdraw_utilization_lock_sec: 0,
            fees_lock_sec: 0,
            _padding: 0,
        },
    };
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    // CREATE CURATOR

    let params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: pool_authority_pubkey,
        pools_authority: pool_authority_pubkey,
        vaults_authority: pool_authority_pubkey,
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };
    create_curator(
        &mut ctx,
        &curator_keypair,
        &admin_keypair,
        &owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    let params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        &mut ctx,
        &pool_keypair,
        &pool_authority_keypair,
        curator_pubkey,
        params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 9000,
    };

    create_reserve(
        &mut ctx,
        &reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    // CREATE LP TOKEN WALLET

    let lp_mint = find_lp_token_mint(&reserve_pubkey).0;
    let destination_lp_wallet =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let mint_acc = get_account(&mut ctx.banks_client, liquidity_mint)
        .await
        .expect("get mint acc");
    let source_liquidity_wallet = get_associated_token_address_with_program_id(
        &lender_pubkey,
        &liquidity_mint,
        &mint_acc.owner,
    );
    let amount = 100_u64;

    deposit_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        amount,
    )
    .await
    .expect("deposit_liquidity");

    // WITHDRAW LIQUIDITY

    let source_token_acc0 = get_token_account(&mut ctx.banks_client, source_liquidity_wallet)
        .await
        .expect("get token acc");
    let destination_token_acc0 = get_token_account(&mut ctx.banks_client, destination_lp_wallet)
        .await
        .expect("get token acc");

    info!("withdraw liquidity");
    withdraw_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        amount,
    )
    .await
    .expect("withdraw liquidity");

    let source_token_acc1 = get_token_account(&mut ctx.banks_client, source_liquidity_wallet)
        .await
        .expect("get token acc");
    let destination_token_acc1 = get_token_account(&mut ctx.banks_client, destination_lp_wallet)
        .await
        .expect("get token acc");

    // CHECK INCREASE SOURCE TOKEN WALLET BALANCE
    assert_eq!(source_token_acc1.amount, source_token_acc0.amount + amount);

    // CHECK DECREASE DESTINATION LP TOKEN WALLET BALANCE
    assert_eq!(
        destination_token_acc1.amount,
        destination_token_acc0.amount - amount
    )
}

#[tokio::test]
async fn run_withdraw_success() {
    let spl_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    withdraw_success(spl_mint).await; // run with spl_token SOL mint

    let spl_mint2022 = Pubkey::from_str("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo").unwrap();
    withdraw_success(spl_mint2022).await; // run with spl_token2022 pyUSD mint
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#23f7fd906b434bdeb6d4043b10c0bb03
#[tokio::test]
async fn withdraw_incorrect_destination() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    init_token_accounts(&mut runner, &liquidity_mint);
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: ReserveTimelock {
            market_price_feed_lock_sec: 0,
            irm_lock_sec: 0,
            liquidation_bonus_lock_sec: 0,
            unhealthy_ltv_lock_sec: 0,
            partial_liquidation_factor_lock_sec: 0,
            max_total_liquidity_lock_sec: 0,
            max_borrow_ltv_lock_sec: 0,
            max_borrow_utilization_lock_sec: 0,
            price_stale_threshold_lock_sec: 0,
            max_withdraw_utilization_lock_sec: 0,
            fees_lock_sec: 0,
            _padding: 0,
        },
    };
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    // CREATE CURATOR

    let params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: pool_authority_pubkey,
        pools_authority: pool_authority_pubkey,
        vaults_authority: pool_authority_pubkey,
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };
    create_curator(
        &mut ctx,
        &curator_keypair,
        &admin_keypair,
        &owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    let params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        &mut ctx,
        &pool_keypair,
        &pool_authority_keypair,
        curator_pubkey,
        params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 9000,
    };

    create_reserve(
        &mut ctx,
        &reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    // CREATE LP TOKEN WALLET

    let lp_mint = find_lp_token_mint(&reserve_pubkey).0;
    let destination_lp_wallet =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_liquidity_wallet = get_associated_token_address(&lender_pubkey, &liquidity_mint);
    let amount = 100_u64;

    deposit_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        amount,
    )
    .await
    .expect("deposit_liquidity");

    // WITHDRAW LIQUIDITY FROM RESERVE LIQUIDITY SUPPLY

    let destination_liquidity_wallet = find_liquidity_supply(&reserve_pubkey).0;

    info!("withdraw liquidity from reserve liquidity supply");
    let result = withdraw_liquidity(
        &mut ctx,
        reserve_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        destination_liquidity_wallet,
        destination_lp_wallet,
        amount,
    )
    .await;

    assert!(result.is_err())
}

// Exchange maximum LPs possible. Reserve limits the withdrawal.
// 1. Lender deposits 10 SOL
// 2. Borrower deposits USDC collateral and borrows 1 SOL
// 3. Lender try to withdraw MAX SOL liquidity. He receives only 5 SOL because SOL reserve must maintain 50% max_withdraw_ltv
#[tokio::test]
async fn withdraw_max_limited_by_reserve() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let position_pubkey = borrower_position_keypair.pubkey();

    let texture_owner_keypair = Keypair::new();
    let texture_owner_pubkey = texture_owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();

    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();

    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();

    let reserve_sol1_keypair = Keypair::new();
    let reserve_sol1_pubkey = reserve_sol1_keypair.pubkey();
    let reserve_sol2_keypair = Keypair::new();
    let reserve_usdc_keypair = Keypair::new();
    let reserve_usdc_pubkey = reserve_usdc_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_pubkey, LAMPORTS);
    runner.add_native_wallet(borrower_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    // 1 SOL = 100 USD
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    // 1 USDC = 1.001 USD
    let usdc_price_feed = add_price_feed_acc(&mut runner, "usdc-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let liquidity_sol_mint =
        Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let liquidity_usdc_mint =
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    init_token_accounts(&mut runner, &liquidity_sol_mint);
    init_token_accounts(&mut runner, &liquidity_usdc_mint);

    let mut ctx = runner.start_with_context().await;

    setup_lendy_env(
        &mut ctx,
        &admin_keypair,
        &borrower_keypair,
        &curator_keypair,
        &pool_keypair,
        &reserve_sol1_keypair,
        &reserve_sol2_keypair,
        &reserve_usdc_keypair,
        &texture_owner_keypair,
        &texture_config_keypair,
        &pool_authority_keypair,
        &borrower_position_keypair,
        irm,
    )
    .await;

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        10 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT 100 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 2000 * LAMPORTS_PER_USDC;

    info!("deposit {} into USDC reserve", deposit_usdc_amount);
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_usdc,
        dest_borrower_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    info!("lock {} collateral lp", deposit_usdc_amount);
    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // CHECK deposited_value = deposit_amount * lp_exchange_rate
    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_lamports(2002 * LAMPORTS_PER_SOL, 9).unwrap()
    );
    // CHECK allowed_borrow_value = deposit_amount * lp_exchange_rate * %max_borrow_ltv
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(18018, 1).unwrap()
    );

    // BORROW MAX SOL contract can give

    let dest_borrower_liq_wallet_sol =
        get_associated_token_address(&borrower_pubkey, &liquidity_sol_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_sol_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_sol_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow MAX SOL after lock deposited collateral");
    borrow(
        &mut ctx,
        position_pubkey,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_sol,
        u64::MAX,
        1,
    )
    .await
    .expect("borrow");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // Position's allowed_borrow_value is 1801 (checked above). So borrow limit comes from
        // SOL reserve we are borrowing from.
        // Because Reserve has just 10 SOLs and max_borrow_utilization_bps = 50% it can give only 5 SOLs
        // which results in 500 USD value.
        Decimal::from_i128_with_scale(500, 0).unwrap()
    );
    // CHECK BorrowedLiquidity amount
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .expect("find_borrowed_liquidity")
        .0;
    let borrowed_amount = borrowed_liquidity.borrowed_amount().unwrap();

    assert_eq!(
        borrowed_amount,
        Decimal::from_lamports(5 * LAMPORTS_PER_SOL, 9).unwrap()
    );
}

// Exchange maximum LPs possible. Reserve has plenty of liquidity. User has some LPs on his wallet.
// And he wants to exchange all of them to liquidity.
// 1. Lender deposits 10 SOL
// 2. Lender try to withdraw MAX SOL liquidity. He receives only 10 SOL because SOL reserve does not have borrows.
#[tokio::test]
async fn withdraw_max_limited_by_position() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();

    let texture_owner_keypair = Keypair::new();
    let texture_owner_pubkey = texture_owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();

    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let pool_keypair = Keypair::new();

    let curator_keypair = Keypair::new();

    let reserve_sol1_keypair = Keypair::new();
    let reserve_sol1_pubkey = reserve_sol1_keypair.pubkey();
    let reserve_sol2_keypair = Keypair::new();
    let reserve_usdc_keypair = Keypair::new();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_pubkey, LAMPORTS);
    runner.add_native_wallet(borrower_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    // 1 SOL = 100 USD
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    // 1 USDC = 1.001 USD
    let _usdc_price_feed = add_price_feed_acc(&mut runner, "usdc-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let liquidity_sol_mint =
        Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let liquidity_usdc_mint =
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    init_token_accounts(&mut runner, &liquidity_sol_mint);
    init_token_accounts(&mut runner, &liquidity_usdc_mint);

    let mut ctx = runner.start_with_context().await;

    setup_lendy_env(
        &mut ctx,
        &admin_keypair,
        &borrower_keypair,
        &curator_keypair,
        &pool_keypair,
        &reserve_sol1_keypair,
        &reserve_sol2_keypair,
        &reserve_usdc_keypair,
        &texture_owner_keypair,
        &texture_config_keypair,
        &pool_authority_keypair,
        &borrower_position_keypair,
        irm,
    )
    .await;

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        10 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    let source_token_acc0 = get_token_account(&mut ctx.banks_client, source_lender_liq_wallet_sol)
        .await
        .expect("get token acc");
    let destination_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_lender_lp_wallet_sol)
            .await
            .expect("get token acc");

    info!("withdraw liquidity");
    withdraw_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        u64::MAX,
    )
    .await
    .expect("withdraw liquidity");

    let source_token_acc1 = get_token_account(&mut ctx.banks_client, source_lender_liq_wallet_sol)
        .await
        .expect("get token acc");
    let destination_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_lender_lp_wallet_sol)
            .await
            .expect("get token acc");

    // CHECK INCREASE SOURCE TOKEN WALLET BALANCE
    assert_eq!(
        source_token_acc1.amount,
        source_token_acc0.amount + 10_000_000_000
    );

    // CHECK DECREASE DESTINATION LP TOKEN WALLET BALANCE
    assert_eq!(
        destination_token_acc1.amount,
        destination_token_acc0.amount - 10_000_000_000
    )
}
