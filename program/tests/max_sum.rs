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
use super_lendy::MAX_AMOUNT;
use tracing::info;

use super_lendy::pda::find_lp_token_mint;
use super_lendy::state::curator::CuratorParams;
use super_lendy::state::pool::PoolParams;
use super_lendy::state::reserve::{ReserveConfig, ReserveFeesConfig, RESERVE_TYPE_NORMAL};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};

use crate::utils::superlendy_executor::{
    borrow, create_curator, create_pool, create_position, create_reserve, create_texture_config,
    deposit_liquidity, lock_collateral, refresh_position, repay, unlock_collateral,
    withdraw_liquidity,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, init_program_test, init_token_accounts,
    lender_keypair, texture_config_keypair, Runner, LAMPORTS,
};

pub mod utils;

/// Test IXs with MAX token amount
pub async fn max_sum_success(
    liquidity_mint: Pubkey,
    price_feed: &str,
    decimals: u8,
    amount_in_tokens: u64,
) {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_owner_pubkey = texture_owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();

    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();

    let principal_reserve_keypair = Keypair::new();
    let principal_reserve_pubkey = principal_reserve_keypair.pubkey();
    let collateral_reserve_keypair = Keypair::new();
    let collateral_reserve_pubkey = collateral_reserve_keypair.pubkey();

    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let position_borrower_pubkey = borrower_position_keypair.pubkey();

    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_pubkey, LAMPORTS);
    runner.add_native_wallet(borrower_pubkey, LAMPORTS);
    runner.add_native_wallet(lender_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);

    let principal_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let collateral_mint = liquidity_mint;
    init_token_accounts(&mut runner, &principal_mint);
    init_token_accounts(&mut runner, &collateral_mint);

    let principal_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    let collateral_price_feed = add_price_feed_acc(&mut runner, price_feed).await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: texture_owner_pubkey,
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
    let mut config = ReserveConfig {
        market_price_feed: principal_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 9500,
        fully_unhealthy_ltv_bps: 9700,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 10_000_000_000 * LAMPORTS_PER_SOL,
        max_borrow_utilization_bps: 9900,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 10000,
    };

    create_reserve(
        &mut ctx,
        &principal_reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        principal_mint,
        principal_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    config.market_price_feed = collateral_price_feed;

    create_reserve(
        &mut ctx,
        &collateral_reserve_keypair,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        collateral_mint,
        collateral_price_feed,
        config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve");

    // DEPOSIT INITIAL LIQUIDITY TO PRINCIPAL RESERVE

    let lp_mint = find_lp_token_mint(&principal_reserve_pubkey).0;
    let dest_lender_lp_wallet =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_liquidity_wallet = get_associated_token_address(&lender_pubkey, &principal_mint);

    info!("deposit initial liquidity");
    deposit_liquidity(
        &mut ctx,
        principal_reserve_pubkey,
        principal_price_feed,
        irm,
        &lender_keypair,
        source_liquidity_wallet,
        dest_lender_lp_wallet,
        1_000_000_000 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT LIQUIDITY TO COLLATERAL RESERVE

    let lp_mint = find_lp_token_mint(&collateral_reserve_pubkey).0;
    let destination_lp_wallet =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_liquidity_wallet = get_associated_token_address(&borrower_pubkey, &collateral_mint);

    let lamports_per_token = 10_u32.checked_pow(decimals as u32).unwrap() as u64;
    info!("deposit liquidity");
    deposit_liquidity(
        &mut ctx,
        collateral_reserve_pubkey,
        collateral_price_feed,
        irm,
        &borrower_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        amount_in_tokens * lamports_per_token,
    )
    .await
    .expect("deposit_liquidity");

    // LOCK COLLATERAL

    create_position(
        &mut ctx,
        &borrower_position_keypair,
        pool_keypair.pubkey(),
        &borrower_keypair,
    )
    .await
    .expect("create_position");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    info!("lock collateral");
    lock_collateral(
        &mut ctx,
        collateral_reserve_pubkey,
        collateral_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        destination_lp_wallet,
        MAX_AMOUNT,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    // BORROW

    let mint_acc = get_account(&mut ctx.banks_client, principal_mint)
        .await
        .expect("get mint acc");
    let dest_liquidity_wallet = get_associated_token_address_with_program_id(
        &borrower_pubkey,
        &principal_mint,
        &mint_acc.owner,
    );
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &principal_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &principal_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow");
    borrow(
        &mut ctx,
        position_borrower_pubkey,
        principal_reserve_pubkey,
        principal_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_liquidity_wallet,
        MAX_AMOUNT,
        1,
    )
    .await
    .expect("borrow");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh_position");

    // REPAY

    info!("repay");
    repay(
        &mut ctx,
        position_borrower_pubkey,
        principal_reserve_pubkey,
        &borrower_keypair,
        dest_liquidity_wallet,
        MAX_AMOUNT,
    )
    .await
    .expect("borrow");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh_position");

    // UNLOCK COLLATERAL

    info!("unlock collateral");
    unlock_collateral(
        &mut ctx,
        collateral_reserve_pubkey,
        collateral_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        destination_lp_wallet,
        MAX_AMOUNT,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    // WITHDRAW LIQUIDITY

    info!("withdraw liquidity");
    withdraw_liquidity(
        &mut ctx,
        collateral_reserve_pubkey,
        collateral_price_feed,
        irm,
        &borrower_keypair,
        source_liquidity_wallet,
        destination_lp_wallet,
        MAX_AMOUNT,
    )
    .await
    .expect("withdraw_liquidity");
}

#[tokio::test]
async fn run_mux_sum_success() {
    max_sum_success(
        Pubkey::from_str("BonK1YhkXEGLZzwtcvRTip3gAL9nCeQD7ppZBLXhtTs").unwrap(), // WIF
        "bonk-usd",
        5,
        50_000_000_000,
    )
    .await;

    max_sum_success(
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(), // USDC
        "usdc-usd",
        6,
        50_000_000_000,
    )
    .await;

    max_sum_success(
        Pubkey::from_str("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So").unwrap(), // mSOL
        "msol-usd",
        9,
        500_000_000,
    )
    .await;
}
