#![cfg(feature = "test-bpf")]

use std::str::FromStr;

use bytemuck::Zeroable;
use chrono::Utc;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use texture_common::account::PodAccount;
use texture_common::math::{CheckedDiv, Decimal};
use tracing::info;

use super_lendy::pda::find_lp_token_mint;
use super_lendy::state::position::Position;
use super_lendy::state::reserve::{Reserve, ReserveFeesConfig};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};
use super_lendy::MAX_AMOUNT;

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    alter_reserve, alter_texture_config, borrow, create_position, deposit_liquidity, liquidate,
    lock_collateral, refresh_position, write_off_bad_debt, write_price,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, get_token_account, init_program_test,
    init_token_accounts, lender_keypair, price_feed_authority, texture_config_keypair, Runner,
    LAMPORTS, LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#918899eacc0d4c6893f794b52582db71
#[tokio::test]
async fn liquidate_health_position() {
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

    // ALTER LIQUIDATION PARAMS

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    params.max_borrow_ltv_bps = 6000; // 60%
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };
    alter_reserve(
        &mut ctx,
        reserve_sol1_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
        0,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    params.max_borrow_ltv_bps = 6000; // 60%
    params.partly_unhealthy_ltv_bps = 7000; // 70%
    params.fully_unhealthy_ltv_bps = 8000; // 80%
    params.liquidation_bonus_bps = 100; // 1%
    params.partial_liquidation_factor_bps = 2000; // 20%
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };
    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
        0,
    )
    .await
    .expect("alter_reserve");

    // ALTER texture_config.borrow_fee & texture_config.performance_fee to zero

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 0,
        performance_fee_rate_bps: 0,
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
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

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
        1_000 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT 10_000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 10_000 * LAMPORTS_PER_USDC;

    info!("deposit 10_000");

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

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    // BORROW 50 SOL AFTER LOCK COLLATERAL

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
    let borrow_amount = 50 * LAMPORTS_PER_SOL;

    info!(
        "borrow {} SOL after lock deposited collateral",
        borrow_amount
    );
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
        borrow_amount,
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

    // LTV = 50%
    assert_eq!(
        position
            .borrowed_value()
            .unwrap()
            .checked_div(position.deposited_value().unwrap())
            .unwrap()
            .round_to_decimals(2),
        Decimal::from_i128_with_scale(50, 2).unwrap()
    );

    // TRY TO FULL LIQUIDATE HEALTH POSITION

    info!("try to liquidate full borrow amount in health position");
    let result = liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        position_pubkey,
        &borrower_keypair,
        borrow_amount,
    )
    .await;
    assert!(result.is_err());
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#245e6a1fe36b4f10ad8e04b811a5b1d9
#[tokio::test]
async fn liquidate_success() {
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

    // ALTER LIQUIDATION PARAMS

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    params.max_borrow_ltv_bps = 6000; // 60%
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };
    alter_reserve(
        &mut ctx,
        reserve_sol1_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
        0,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    params.max_borrow_ltv_bps = 6000; // 60%
    params.partly_unhealthy_ltv_bps = 7000; // 70%
    params.fully_unhealthy_ltv_bps = 8000; // 80%
    params.liquidation_bonus_bps = 2000; // 20%
    params.partial_liquidation_factor_bps = 2000; // 20%
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };
    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
        0,
    )
    .await
    .expect("alter_reserve");

    // ALTER texture_config.borrow_fee & texture_config.performance_fee to zero

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 0,
        performance_fee_rate_bps: 0,
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
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

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
        1_000 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT 10_000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 10_000 * LAMPORTS_PER_USDC;

    info!("deposit 10_000");

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

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    // BORROW 50 SOL AFTER LOCK COLLATERAL

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
    let borrow_amount = 50 * LAMPORTS_PER_SOL;

    info!(
        "borrow {} SOL after lock deposited collateral",
        borrow_amount
    );
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
        borrow_amount,
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

    // LTV = 50%
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(50, 2).unwrap()
    );

    // RAISE SOL PRICE FROM 100 TO 150 USD

    info!("raise SOL price from 100 to 150");
    let now = Utc::now().timestamp();
    write_price(
        &mut ctx,
        sol_price_feed,
        &price_feed_authority(),
        Decimal::from_i128_with_scale(150, 0).unwrap(),
        now - 3,
    )
    .await
    .expect("update sol price feed");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // LTV = 75%
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(75, 2).unwrap()
    );

    // CHECK PARTLY & FULLY POSITION UNHEALTHY BORROW VALUE
    assert_eq!(
        position.partly_unhealthy_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(7007, 0).unwrap()
    );
    assert_eq!(
        position.fully_unhealthy_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(8008, 0).unwrap()
    );

    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(10010, 0).unwrap() // 10_000 USDC with 1.001 price
    );

    let deposited_collateral = position.find_collateral(reserve_usdc_pubkey).unwrap().0;

    info!(
        "before liquidate deposited_collateral.deposited_amount {}",
        deposited_collateral.deposited_amount
    );
    assert_eq!(deposited_collateral.deposited_amount, 10_000_000_000_u64);

    // TRY TO FULL LIQUIDATE

    info!("try to liquidate full borrow amount");
    let result = liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        position_pubkey,
        &borrower_keypair,
        borrow_amount,
    )
    .await;
    assert!(result.is_err());

    // LIQUIDATE 5 SOL

    let liquidate_amount = 5 * LAMPORTS_PER_SOL;

    let borrower_lp_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    info!("liquidate {} SOL", liquidate_amount);
    liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        position_pubkey,
        &borrower_keypair,
        liquidate_amount,
    )
    .await
    .expect("liquidate");

    let borrower_lp_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    // 5 * 150 * 1.2 = 900 USD / 1.001 = 889 USDC
    assert_eq!(
        borrower_lp_token_acc1.amount,
        borrower_lp_token_acc0.amount + 899_100_899
    );

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;

    assert_eq!(
        borrowed_liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(45 * LAMPORTS_PER_SOL, 9).unwrap()
    );
    assert_eq!(
        position.borrowed_value().unwrap(),
        Decimal::from_i128_with_scale(6750, 0).unwrap() // (50 - 5) * 150 = 6750 USD
    );

    let deposited_collateral = position.find_collateral(reserve_usdc_pubkey).unwrap().0;

    info!(
        "after liquidate deposited_collateral.deposited_amount {}",
        deposited_collateral.deposited_amount
    );

    assert_eq!(
        deposited_collateral.deposited_amount,
        10_000_000_000_u64 - 899_100_899_u64 // , where 10_000_000_000 - initial collateral, 899_100_899 - paid to Liquidator
    );
    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(9110000000101_i128, 9).unwrap() // 10_010 - 900 = 9110 USD
    );

    // LTV ~= 74%. Partially liquidation decreased LTV
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(74, 2).unwrap()
    );

    // RAISE SOL PRICE FROM 150 TO 250 USD

    info!("raise SOL price from 150 to 250");
    let now = Utc::now().timestamp();
    write_price(
        &mut ctx,
        sol_price_feed,
        &price_feed_authority(),
        Decimal::from_i128_with_scale(250, 0).unwrap(),
        now - 3,
    )
    .await
    .expect("update sol price feed");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    assert_eq!(
        position.borrowed_value().unwrap(),
        Decimal::from_i128_with_scale(11250, 0).unwrap() // 45 * 250 = 11_250 USD
    );
    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(9110000000101_i128, 9).unwrap()
    );
    assert_eq!(
        reserve.liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(45 * LAMPORTS_PER_SOL, 9).unwrap()
    );

    // LTV ~= 123%
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(123, 2).unwrap()
    );

    // TRY TO WriteOffBadDebt

    info!("try to write_off_bad_debt");
    let result = write_off_bad_debt(
        &mut ctx,
        position_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        reserve_sol1_pubkey,
        45 * LAMPORTS_PER_SOL,
    )
    .await;
    assert!(result.is_err());

    // LIQUIDATE FULL DEPOSITED AMOUNT

    let borrower_lp_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");
    let borrower_liq_sol_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");

    info!("liquidate full deposited amount");
    liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        position_pubkey,
        &borrower_keypair,
        MAX_AMOUNT,
    )
    .await
    .expect("liquidate");

    let borrower_lp_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    // Here Borrower acted as Liquidator. Which doesn't look realistic but for test it is ok.
    assert_eq!(
        borrower_lp_token_acc1.amount,
        borrower_lp_token_acc0.amount + 9_100_899_101
    );

    let borrower_liq_sol_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");

    // This was SOL amount repayed during liquidation when Borrower acted as Liquidator.
    assert_eq!(
        borrower_liq_sol_acc1.amount,
        borrower_liq_sol_acc0.amount - 30_366_666_668
    );

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    assert_eq!(
        borrowed_liquidity
            .borrowed_amount()
            .unwrap()
            .round_to_decimals(9),
        Decimal::from_lamports(14633333333, 9)
            .unwrap()
            .round_to_decimals(9) // Bad debt
    );
    assert_eq!(
        position.borrowed_value().unwrap().round_to_decimals(0),
        Decimal::from_i128_with_scale(3658, 0).unwrap() // 14,63 * 250 ~= 3658 USD
    );
    assert_eq!(position.deposited_value().unwrap(), Decimal::ZERO);
    assert_eq!(
        reserve.lp_exchange_rate().unwrap().0.round_to_decimals(13),
        Decimal::from_i128_with_scale(9999999999990, 13).unwrap() //TODO: check lp exchange rate after full liquidation
    );

    // WRITE OFF BAD DEBT

    info!("write off bad debt");
    write_off_bad_debt(
        &mut ctx,
        position_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        reserve_sol1_pubkey,
        MAX_AMOUNT,
    )
    .await
    .expect("write_off_bad_debt");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    assert_eq!(
        borrowed_liquidity
            .borrowed_amount()
            .unwrap()
            .round_to_decimals(0),
        Decimal::ZERO
    );
    assert_eq!(
        position.borrowed_value().unwrap().round_to_decimals(0),
        Decimal::ZERO
    );
    // CHECK lp_exchange_rate1 > lp_exchange_rate0. LP decreased
    assert_eq!(
        reserve.lp_exchange_rate().unwrap().0.round_to_decimals(13),
        Decimal::from_i128_with_scale(10148506478116, 13).unwrap()
    );
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#1c3605bce3b5470491df4ab0d7898f50
#[tokio::test]
async fn liquidate_with_lp_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let borrower_position_pubkey = borrower_position_keypair.pubkey();
    let lender_position_kp = Keypair::new();
    let lender_position_pubkey = lender_position_kp.pubkey();

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
    let reserve_sol2_pubkey = reserve_sol2_keypair.pubkey();
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

    // ALTER LIQUIDATION PARAMS

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    params.max_borrow_ltv_bps = 6000; // 60%
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };
    alter_reserve(
        &mut ctx,
        reserve_sol1_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
        0,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    params.max_borrow_ltv_bps = 6000; // 60%
    params.partly_unhealthy_ltv_bps = 7000; // 70%
    params.fully_unhealthy_ltv_bps = 8000; // 80%
    params.liquidation_bonus_bps = 2000; // 20%
    params.partial_liquidation_factor_bps = 2000; // 20%
    params.max_borrow_utilization_bps = 5000; // 50%
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };
    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
        0,
    )
    .await
    .expect("alter_reserve");

    // ALTER texture_config.borrow_fee & texture_config.performance_fee to zero

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 0,
        performance_fee_rate_bps: 0,
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
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit initial liquidity to SOL1 reserve");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        500 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO SOL2 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");

    info!("deposit initial liquidity to SOL2 reserve");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        500 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    create_position(
        &mut ctx,
        &lender_position_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    refresh_position(&mut ctx, lender_position_pubkey)
        .await
        .expect("refresh position");

    info!("lock some liquidity to borrow from usdc reserve");
    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        lender_position_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_sol,
        500 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("lock_collateral");

    // DEPOSIT 10_000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 10_000 * LAMPORTS_PER_USDC;

    info!("deposit 10_000");

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

    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        borrower_position_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    let dest_lender_liq_wallet_usdc =
        get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    // BORROW 3000 USDC TO INCREASE LP EXCHANGE RATE
    borrow(
        &mut ctx,
        lender_position_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &lender_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_lender_liq_wallet_usdc,
        3000 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh position");

    // BORROW 50 SOL AFTER LOCK COLLATERAL

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
    let borrow_amount = 50 * LAMPORTS_PER_SOL;

    info!(
        "borrow {} SOL after lock deposited collateral",
        borrow_amount
    );
    borrow(
        &mut ctx,
        borrower_position_pubkey,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_sol,
        borrow_amount,
        1,
    )
    .await
    .expect("borrow");

    info!("refresh position");
    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, borrower_position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // LTV = 50%
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(50, 2).unwrap()
    );

    // 1 SLOT LATER. RAISE SOL PRICE FROM 100 TO 150 USD

    let mut slot = 2_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("raise SOL price from 100 to 150");
    let now = Utc::now().timestamp();
    write_price(
        &mut ctx,
        sol_price_feed,
        &price_feed_authority(),
        Decimal::from_i128_with_scale(150, 0).unwrap(),
        now - 3,
    )
    .await
    .expect("update sol price feed");

    info!("refresh position");
    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, borrower_position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // LTV = 75%
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(75, 2).unwrap()
    );

    // CHECK PARTLY & FULLY POSITION UNHEALTHY BORROW VALUE
    assert_eq!(
        position.partly_unhealthy_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(7007_000013331430746334, 18).unwrap()
    );
    assert_eq!(
        position.fully_unhealthy_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(8008_000015235920852953, 18).unwrap()
    );

    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(10010_000019044901066191, 18).unwrap() // 10_000 USDC with 1.001 price and LP price
    );

    let deposited_collateral = position.find_collateral(reserve_usdc_pubkey).unwrap().0;

    info!(
        "before liquidate deposited_collateral.deposited_amount {}",
        deposited_collateral.deposited_amount
    );
    assert_eq!(deposited_collateral.deposited_amount, 10_000_000_000_u64);

    // TRY TO FULL LIQUIDATE

    info!("try to liquidate full borrow amount");
    let result = liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        borrower_position_pubkey,
        &borrower_keypair,
        borrow_amount,
    )
    .await;
    assert!(result.is_err());

    // LIQUIDATE 5 SOL

    let liquidate_amount = 5 * LAMPORTS_PER_SOL;

    let borrower_lp_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    info!("liquidate {} SOL", liquidate_amount);
    liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        borrower_position_pubkey,
        &borrower_keypair,
        liquidate_amount,
    )
    .await
    .expect("liquidate");

    let borrower_lp_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    // 5 * 150 * 1.2 = 900 USD / 1.001 = 889 USDC
    assert_eq!(
        borrower_lp_token_acc1.amount,
        borrower_lp_token_acc0.amount + 899_100_897
    );

    info!("refresh position");
    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, borrower_position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;

    assert_eq!(
        borrowed_liquidity
            .borrowed_amount()
            .unwrap()
            .round_to_decimals(9),
        Decimal::from_lamports(45_000000317, 9).unwrap()
    );
    assert_eq!(
        position.borrowed_value().unwrap().round_to_decimals(8),
        Decimal::from_lamports(6750_000047560, 9).unwrap() // (50 - 5) * 150 = 6750 USD * LP price
    );

    let deposited_collateral = position.find_collateral(reserve_usdc_pubkey).unwrap().0;

    info!(
        "after liquidate deposited_collateral.deposited_amount {}",
        deposited_collateral.deposited_amount
    );

    assert_eq!(
        deposited_collateral.deposited_amount,
        10_000_000_000_u64 - 899_100_897_u64 // , where 10_000_000_000 - initial collateral, 899_100_899 - paid to Liquidator
    );
    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(9110_000019435572303002, 18).unwrap() // 10_010 - 900 = 9110 USD
    );

    // LTV ~= 74%. Partially liquidation decreased LTV
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(74, 2).unwrap()
    );

    // 1 SLOT LATER. RAISE SOL PRICE FROM 150 TO 250 USD

    slot += 1;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("raise SOL price from 150 to 250");
    let now = Utc::now().timestamp();
    write_price(
        &mut ctx,
        sol_price_feed,
        &price_feed_authority(),
        Decimal::from_i128_with_scale(250, 0).unwrap(),
        now - 3,
    )
    .await
    .expect("update sol price feed");

    info!("refresh position");
    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, borrower_position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    assert_eq!(
        position.borrowed_value().unwrap().round_to_decimals(8),
        Decimal::from_lamports(11250_000150620, 9).unwrap() // 45 * 250 = 11_250 USD * LP price
    );
    assert_eq!(
        position.deposited_value().unwrap().round_to_decimals(9),
        Decimal::from_lamports(9110_000036768, 9).unwrap()
    );
    assert_eq!(
        reserve
            .liquidity
            .borrowed_amount()
            .unwrap()
            .round_to_decimals(9),
        Decimal::from_lamports(45000000602, 9).unwrap()
    );

    // LTV ~= 123%
    assert_eq!(
        position.ltv().unwrap().round_to_decimals(2),
        Decimal::from_i128_with_scale(123, 2).unwrap()
    );

    // TRY TO WriteOffBadDebt

    info!("try to write_off_bad_debt");
    let result = write_off_bad_debt(
        &mut ctx,
        borrower_position_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        reserve_sol1_pubkey,
        45 * LAMPORTS_PER_SOL,
    )
    .await;
    assert!(result.is_err());

    // LIQUIDATE FULL DEPOSITED AMOUNT

    let borrower_lp_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    info!("liquidate full deposited amount");
    liquidate(
        &mut ctx,
        dest_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_usdc,
        reserve_sol1_pubkey,
        reserve_usdc_pubkey,
        borrower_position_pubkey,
        &borrower_keypair,
        MAX_AMOUNT,
    )
    .await
    .expect("liquidate");

    let borrower_lp_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_lp_wallet_usdc)
            .await
            .expect("get token acc");

    assert_eq!(
        borrower_lp_token_acc1.amount,
        borrower_lp_token_acc0.amount + 9_100_899_103
    );

    info!("refresh position");
    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, borrower_position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    assert_eq!(
        borrowed_liquidity
            .borrowed_amount()
            .unwrap()
            .round_to_decimals(9),
        Decimal::from_lamports(14633333813, 9).unwrap() // Bad debt
    );
    assert_eq!(
        position.borrowed_value().unwrap().round_to_decimals(0),
        Decimal::from_i128_with_scale(3658, 0).unwrap() // 14,63 * 250 ~= 3658 USD
    );
    assert_eq!(position.deposited_value().unwrap(), Decimal::ZERO);
    assert_eq!(
        reserve.lp_exchange_rate().unwrap().0.round_to_decimals(13),
        Decimal::from_i128_with_scale(9999999987935, 13).unwrap()
    );

    // WRITE OFF BAD DEBT

    info!("write off bad debt");
    write_off_bad_debt(
        &mut ctx,
        borrower_position_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        reserve_sol1_pubkey,
        MAX_AMOUNT,
    )
    .await
    .expect("write_off_bad_debt");

    info!("refresh position");
    refresh_position(&mut ctx, borrower_position_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, borrower_position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    assert_eq!(
        borrowed_liquidity
            .borrowed_amount()
            .unwrap()
            .round_to_decimals(0),
        Decimal::ZERO
    );
    assert_eq!(
        position.borrowed_value().unwrap().round_to_decimals(0),
        Decimal::ZERO
    );
    // CHECK lp_exchange_rate1 > lp_exchange_rate0. LP decreased
    assert_eq!(
        reserve.lp_exchange_rate().unwrap().0.round_to_decimals(13),
        Decimal::from_i128_with_scale(1_0301490279643, 13).unwrap()
    );
}
