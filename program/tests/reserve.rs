#![cfg(feature = "test-bpf")]

use std::str::FromStr;

use bytemuck::Zeroable;
use price_proxy::state::utils::str_to_array;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use texture_common::account::PodAccount;
use tracing::info;

use super_lendy::pda::find_lp_token_mint;
use super_lendy::state::curator::CuratorParams;
use super_lendy::state::pool::PoolParams;
use super_lendy::state::reserve::{
    Reserve, ReserveConfig, ReserveFeesConfig, RESERVE_MODE_BORROW_DISABLED, RESERVE_MODE_NORMAL,
    RESERVE_MODE_RETAIN_LIQUIDITY, RESERVE_TYPE_NORMAL, RESERVE_TYPE_PROTECTED_COLLATERAL,
};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    alter_reserve, borrow, create_curator, create_pool, create_reserve, create_texture_config,
    deposit_liquidity, lock_collateral, refresh_position, unlock_collateral, withdraw_liquidity,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, init_program_test, init_token_accounts,
    lender_keypair, texture_config_keypair, Runner, LAMPORTS, LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#f586eb97d7dd46e69be432b47ad9c378
#[tokio::test]
async fn create_alter_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_owner_pubkey = texture_owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_pubkey, LAMPORTS);

    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    let usdc_price_feed = add_price_feed_acc(&mut runner, "usdc-usd").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let texture_params = TextureConfigParams {
        borrow_fee_rate_bps: 3000,
        performance_fee_rate_bps: 4000,
        fees_authority: texture_owner_keypair.pubkey(),
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
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        texture_params,
    )
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
        &texture_owner_keypair,
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

    let mut fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let mut config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm: Pubkey::from_str("o3e5ZHy2J43m8UeYKsaRiWatwLCKvfgCSCZNqSpyE8A").unwrap(),
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 1,
        max_withdraw_utilization_bps: 9000,
    };
    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    info!("create reserve");
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

    macro_rules! validate {
        ($this:ident, $params:ident) => {
            assert_eq!($this.reserve_type, 0);
            assert_eq!($this.pool, pool_pubkey);
            assert_eq!($this.liquidity.mint, liquidity_mint);
            assert_eq!($this.config, $params);
        };
    }

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    validate!(reserve, config);

    // ALTER RESERVE

    config.market_price_feed = usdc_price_feed;
    config.irm = Pubkey::new_unique();
    config.liquidation_bonus_bps += 1;
    config.max_borrow_ltv_bps += 1;
    config.fully_unhealthy_ltv_bps += 1;
    config.partial_liquidation_factor_bps += 1;
    config.partly_unhealthy_ltv_bps += 1;

    fees_config.curator_borrow_fee_rate_bps += 1;
    fees_config.curator_performance_fee_rate_bps += 1;
    config.fees = fees_config;

    info!("alter reserve");
    alter_reserve(
        &mut ctx,
        reserve_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        config,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    validate!(reserve, config);
}

#[tokio::test]
async fn alter_reserve_mode() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let position_borrower_pubkey = borrower_position_keypair.pubkey();

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
    let reserve_sol2_keypair = Keypair::new();
    let reserve_usdc_keypair = Keypair::new();
    let reserve_usdc_pubkey = reserve_usdc_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_pubkey, LAMPORTS);
    runner.add_native_wallet(borrower_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    // 1 SOL = 100 USD
    add_price_feed_acc(&mut runner, "sol-usd").await;
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

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_usdc,
        dest_borrower_lp_wallet_usdc,
        1_000 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        500 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let config = reserve.config;

    // SWITCH RESERVE MODE TO BORROW DISABLED

    info!("switch reserve mode to borrow disabled");
    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        config,
        RESERVE_MODE_BORROW_DISABLED,
    )
    .await
    .expect("alter_reserve");

    // TRY TO BORROW FROM RESERVE MODE BORROW DISABLED

    let dest_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("try to borrow from reserve mode borrow disabled");
    let result = borrow(
        &mut ctx,
        position_borrower_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        100 * LAMPORTS_PER_USDC,
        1,
    )
    .await;
    assert!(result.is_err());

    // WITHDRAW & UNLOCK COLLATERAL FROM RESERVE MODE BORROW DISABLED

    info!("withdraw & unlock collateral from reserve mode borrow disabled");

    withdraw_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_usdc,
        dest_borrower_lp_wallet_usdc,
        100 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    unlock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        100 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("unlock_collateral");

    // SWITCH RESERVE MODE TO WITHDRAW DISABLED

    info!("switch reserve mode to withdraw disabled");
    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        config,
        RESERVE_MODE_RETAIN_LIQUIDITY,
    )
    .await
    .expect("alter_reserve");

    // TRY TO BORROW FROM RESERVE MODE WITHDRAW DISABLED

    info!("try to borrow from reserve mode withdraw disabled");
    let result = borrow(
        &mut ctx,
        position_borrower_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        100 * LAMPORTS_PER_USDC,
        1,
    )
    .await;
    assert!(result.is_err());

    // TRY TO WITHDRAW & UNLOCK COLLATERAL FROM RESERVE MODE WITHDRAW DISABLED

    info!("try to withdraw & unlock collateral from reserve mode withdraw disabled");

    let result = withdraw_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_usdc,
        dest_borrower_lp_wallet_usdc,
        100 * LAMPORTS_PER_USDC,
    )
    .await;
    assert!(result.is_err());

    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let result = unlock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        100 * LAMPORTS_PER_USDC,
    )
    .await;
    assert!(result.is_err());
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#974452077c6b4374935b40aa53109790
#[tokio::test]
async fn create_reserve_borrow_enabled_disabled() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let reserve1_keypair = Keypair::new();
    let reserve1_pubkey = reserve1_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_keypair.pubkey(), LAMPORTS);

    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let mut ctx = runner.start_with_context().await;

    info!("create texture config");

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: Zeroable::zeroed(),
    };
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        params,
    )
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
        &texture_owner_keypair,
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

    // CREATE RESERVE BORROW ENABLED

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm: Pubkey::from_str("o3e5ZHy2J43m8UeYKsaRiWatwLCKvfgCSCZNqSpyE8A").unwrap(),
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 1,
        max_withdraw_utilization_bps: 9000,
    };
    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    info!("create reserve borrow enabled");
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

    macro_rules! validate {
        ($this:ident, $params:ident) => {
            assert_eq!($this.reserve_type, 0);
            assert_eq!($this.pool, pool_pubkey);
            assert_eq!($this.liquidity.mint, liquidity_mint);
            assert_eq!($this.config, $params);
        };
    }

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    validate!(reserve, config);

    info!("create reserve borrow disabled");
    create_reserve(
        &mut ctx,
        &reserve1_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_PROTECTED_COLLATERAL,
    )
    .await
    .expect("create_reserve");

    macro_rules! validate {
        ($this:ident, $params:ident) => {
            assert_eq!($this.reserve_type, 1);
            assert_eq!($this.pool, pool_pubkey);
            assert_eq!($this.liquidity.mint, liquidity_mint);
            assert_eq!($this.config, $params);
        };
    }

    let reserve_acc = get_account(&mut ctx.banks_client, reserve1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    validate!(reserve, config);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#a56a60e0903a4774bce6b06c4495be04
#[tokio::test]
async fn create_incorrect_authority() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_keypair.pubkey(), LAMPORTS);

    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let mut ctx = runner.start_with_context().await;

    info!("create texture config");

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: Zeroable::zeroed(),
    };
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        params,
    )
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
        &texture_owner_keypair,
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
        irm: Pubkey::from_str("o3e5ZHy2J43m8UeYKsaRiWatwLCKvfgCSCZNqSpyE8A").unwrap(),
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 1,
        max_withdraw_utilization_bps: 9000,
    };
    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    info!("create reserve with incorrect authority");
    let result = create_reserve(
        &mut ctx,
        &reserve_keypair,
        pool_pubkey,
        &owner_keypair,
        curator_pubkey,
        liquidity_mint,
        sol_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await;

    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#2fd9c16ea1cb4b3da93ecf0c68638c79
#[tokio::test]
async fn alter_incorrect_authority() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let reserve_keypair = Keypair::new();
    let reserve_pubkey = reserve_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_keypair.pubkey(), LAMPORTS);

    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;

    let mut ctx = runner.start_with_context().await;

    info!("create texture config");

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: Zeroable::zeroed(),
    };
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        params,
    )
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
        &texture_owner_keypair,
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

    let mut fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 22,
        curator_performance_fee_rate_bps: 400,
        _padding: Zeroable::zeroed(),
    };
    let mut config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm: Pubkey::from_str("o3e5ZHy2J43m8UeYKsaRiWatwLCKvfgCSCZNqSpyE8A").unwrap(),
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1000,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 1,
        max_withdraw_utilization_bps: 9000,
    };
    let liquidity_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

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

    // ALTER RESERVE

    config.market_price_feed = Pubkey::new_unique();
    config.irm = Pubkey::new_unique();
    config.liquidation_bonus_bps += 1;
    config.max_borrow_ltv_bps += 1;
    config.fully_unhealthy_ltv_bps += 1;
    config.partial_liquidation_factor_bps += 1;
    config.partly_unhealthy_ltv_bps += 1;

    fees_config.curator_borrow_fee_rate_bps += 1;
    fees_config.curator_performance_fee_rate_bps += 1;
    config.fees = fees_config;

    info!("alter reserve with incorrect authority");
    let result = alter_reserve(
        &mut ctx,
        reserve_pubkey,
        pool_pubkey,
        &owner_keypair,
        curator_pubkey,
        config,
        RESERVE_MODE_BORROW_DISABLED,
    )
    .await;

    assert!(result.is_err());
}
