#![cfg(feature = "test-bpf")]

use bytemuck::Zeroable;
use std::str::FromStr;

use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use texture_common::account::PodAccount;
use texture_common::math::Decimal;
use tracing::info;

use super_lendy::instruction::{
    Borrow, DepositLiquidity, FlashBorrow, FlashRepay, LockCollateral, RefreshReserve, Repay,
    Version, WithdrawLiquidity,
};
use super_lendy::pda::{find_liquidity_supply, find_lp_token_mint};
use super_lendy::state::position::Position;
use super_lendy::state::reserve::{Reserve, ReserveFeesConfig, RESERVE_MODE_NORMAL};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};
use super_lendy::state::SLOTS_PER_YEAR;

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    alter_reserve, alter_texture_config, borrow, create_position, deposit_liquidity,
    enable_flash_loans, lock_collateral, refresh_position, refresh_reserve,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, get_token_account, init_program_test,
    init_token_accounts, lender_keypair, texture_config_keypair, Runner, LAMPORTS,
    LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#115b5734a22080549ceefcd5463a8125
#[tokio::test]
async fn flash_success() {
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

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1000 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // TRANSFER AMOUNT FROM WALLET1 TO WALLET2 BY FLASH LOAN

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            liquidity_mint: liquidity_usdc_mint,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: spl_token::id(),
            amount: reserve_liquidity,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("sending tx");
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4
#[tokio::test]
async fn flash_withdraw_limit_success() {
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

    // Set reserve_usdc.max_withdraw_utilization to 10%

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_withdraw_utilization_bps = 1000; // 10%
    params.max_withdraw_utilization_bps = max_withdraw_utilization_bps;
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };

    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_borrow_ltv_bps = 9900; // 99%
    params.max_borrow_ltv_bps = max_borrow_ltv_bps;
    params.partly_unhealthy_ltv_bps = max_borrow_ltv_bps + 1;
    params.fully_unhealthy_ltv_bps = max_borrow_ltv_bps + 2;

    alter_reserve(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol2_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO sol2 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        1000 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("deposit_liquidity");

    // BORROW 100 USDC to increase utilization by 10%

    let position_lender_kp = Keypair::new();
    create_position(
        &mut ctx,
        &position_lender_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_kp.pubkey(),
        &lender_keypair,
        lender_lp_wallet_sol,
        1_001_000_000, // 1.001 SOL = 100 USDC = 100.1 USD
    )
    .await
    .expect("lock_collateral");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow 99 USDC to increase utilization by 9.9%");
    borrow(
        &mut ctx,
        position_lender_kp.pubkey(),
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_keypair.pubkey(),
        &lender_keypair,
        curator_keypair.pubkey(),
        curator_fee_receiver,
        texture_fee_receiver,
        lender_liq_wallet_usdc,
        99 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // TRY TO TRANSFER 902 USDC FROM WALLET1 TO WALLET2 BY FLASH LOAN

    let mut ixs = vec![];

    let amount = 902 * LAMPORTS_PER_USDC;
    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: spl_token::id(),
            amount,
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // TRANSFER 901 USDC FROM WALLET1 TO WALLET2 BY FLASH LOAN

    let mut ixs = vec![];

    let amount = 901 * LAMPORTS_PER_USDC;
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: spl_token::id(),
            amount,
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#115b5734a2208043ad4af448fd3dcbba
#[tokio::test]
async fn flash_not_affect_interest() {
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

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_borrow_utilization_bps = 10000; // 100%
    let max_withdraw_utilization_bps = 10000; // 100%
    params.max_borrow_utilization_bps = max_borrow_utilization_bps;
    params.max_withdraw_utilization_bps = max_withdraw_utilization_bps;
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };

    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_borrow_ltv_bps = 9900; // 99%
    params.max_borrow_ltv_bps = max_borrow_ltv_bps;
    params.partly_unhealthy_ltv_bps = max_borrow_ltv_bps + 1;
    params.fully_unhealthy_ltv_bps = max_borrow_ltv_bps + 2;

    alter_reserve(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol2_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO sol2 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        1000 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("deposit_liquidity");

    // BORROW 100 USDC

    let position_lender_kp = Keypair::new();
    create_position(
        &mut ctx,
        &position_lender_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_kp.pubkey(),
        &lender_keypair,
        lender_lp_wallet_sol,
        3 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("lock_collateral");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow 99 USDC");
    borrow(
        &mut ctx,
        position_lender_kp.pubkey(),
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_keypair.pubkey(),
        &lender_keypair,
        curator_keypair.pubkey(),
        curator_fee_receiver,
        texture_fee_receiver,
        lender_liq_wallet_usdc,
        99 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // 1 SOLANA YEAR LATER

    let slot = SLOTS_PER_YEAR;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot"); // solana_year = 63072000 slots

    refresh_reserve(&mut ctx, reserve_usdc_pubkey, usdc_price_feed, irm)
        .await
        .expect("refresh_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let cumulative_borrow_rate0 = reserve.liquidity.cumulative_borrow_rate().unwrap();

    assert_eq!(
        cumulative_borrow_rate0,
        Decimal::from_i128_with_scale(1491824686287962198, 18).unwrap() // 1.4918
    );

    // TRANSFER 800 USDC FROM WALLET1 TO WALLET2 BY FLASH LOAN

    let mut ixs = vec![];

    let amount = 800 * LAMPORTS_PER_USDC;
    let refresh_ix = RefreshReserve {
        reserve: reserve_usdc_pubkey,
        market_price_feed: usdc_price_feed,
        irm,
    }
    .into_instruction();

    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: spl_token::id(),
            amount,
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(refresh_ix.clone());
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(refresh_ix);
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");

    // CHECK BORROW RATE NOT CHANGED

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let cumulative_borrow_rate1 = reserve.liquidity.cumulative_borrow_rate().unwrap();

    assert_eq!(cumulative_borrow_rate0, cumulative_borrow_rate1);

    // TRANSFER 800 USDC FROM WALLET1 TO WALLET2 BY FLASH LOAN

    let mut ixs = vec![];

    let amount = 800 * LAMPORTS_PER_USDC;
    let refresh_ix = RefreshReserve {
        reserve: reserve_usdc_pubkey,
        market_price_feed: usdc_price_feed,
        irm,
    }
    .into_instruction();

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
            amount,
        }
        .into_instruction(),
    );
    ixs.push(
        Borrow {
            position: position_lender_kp.pubkey(),
            destination_liquidity_wallet: lender_liq_wallet_usdc,
            curator_fee_receiver,
            reserve: reserve_usdc_pubkey,
            pool: pool_keypair.pubkey(),
            curator: curator_keypair.pubkey(),
            texture_fee_receiver,
            amount: 50 * LAMPORTS_PER_USDC,
            slippage_limit: 0,
            memo: [1; 32],
            borrower: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(refresh_ix.clone());
    ixs.push(
        WithdrawLiquidity {
            authority: borrower_pubkey,
            destination_liquidity_wallet: borrower_liq_wallet_usdc,
            source_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            lp_amount: 10 * LAMPORTS_PER_USDC,
            liquidity_mint: liquidity_usdc_mint,
            liquidity_token_program: spl_token::id(),
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(refresh_ix);
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");

    // CHECK BORROW RATE NOT CHANGED

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let cumulative_borrow_rate2 = reserve.liquidity.cumulative_borrow_rate().unwrap();

    assert_eq!(cumulative_borrow_rate0, cumulative_borrow_rate2)
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#115b5734a22080f38779ef923b426754
#[tokio::test]
async fn flash_forbidden() {
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

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    let mut ixs = vec![];

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1000 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // TRY TO DONATE BY FLASH REPAY

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());

    // TRY TO MAKE FLASH BORROW ONLY

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());

    // TRY TO MULTIPLE FLASH BORROW WITH OTHER IXs

    let mut ixs = vec![];

    let amount = 100 * LAMPORTS_PER_USDC;
    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        spl_token::instruction::transfer(
            &spl_token::ID,
            &borrower_liq_wallet_usdc,
            &lender_liq_wallet_usdc,
            &borrower_pubkey,
            &[],
            10 * LAMPORTS_PER_USDC,
        )
        .expect("build ix"),
    );
    ixs.push(Version { no_error: true }.into_instruction());
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        spl_token::instruction::transfer(
            &spl_token::ID,
            &borrower_liq_wallet_usdc,
            &lender_liq_wallet_usdc,
            &borrower_pubkey,
            &[],
            10 * LAMPORTS_PER_USDC,
        )
        .expect("build ix"),
    );
    ixs.push(Version { no_error: true }.into_instruction());
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());

    // TRY TO MULTIPLE FLASH BORROW

    let mut ixs = vec![];

    let amount = 100 * LAMPORTS_PER_USDC;
    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());

    // TRY TO MAKE FLASH LOAN WITH DIFFERENT AMOUNTS

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 101 * LAMPORTS_PER_USDC,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());

    // TRY TO MAKE FLASH LOAN WITH INVALID SEQUENCE

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());

    // TRY TO MAKE FLASH LOAN WITH STALE RESERVE

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;

    assert!(result.is_err());
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#118b5734a22080ff876ddaf0cd40cfb6
#[tokio::test]
async fn flash_multiple_success() {
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

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1000 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // MAKE MULTIPLE FLASH LOAN

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let borrower_liq_wallet_sol =
        get_associated_token_address(&borrower_pubkey, &liquidity_sol_mint);

    let lender_usdc_token_acc0 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_usdc)
        .await
        .expect("get token acc");
    let lender_sol_token_acc0 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_sol)
        .await
        .expect("get token acc");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity0 = reserve.liquidity.available_amount;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_sol_liquidity0 = reserve.liquidity.available_amount;

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        RefreshReserve {
            reserve: reserve_sol1_pubkey,
            market_price_feed: sol_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_sol1_pubkey,
            destination_wallet: lender_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_sol1_pubkey,
            source_wallet: borrower_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("sending tx");

    // CHECK BALANCES

    let lender_usdc_token_acc1 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_usdc)
        .await
        .expect("get token acc");
    let lender_sol_token_acc1 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_sol)
        .await
        .expect("get token acc");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity1 = reserve.liquidity.available_amount;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_sol_liquidity1 = reserve.liquidity.available_amount;

    assert_eq!(
        lender_usdc_token_acc1.amount,
        lender_usdc_token_acc0.amount + (100 * LAMPORTS_PER_USDC),
    );
    assert_eq!(
        lender_sol_token_acc1.amount,
        lender_sol_token_acc0.amount + (LAMPORTS_PER_SOL),
    );
    assert_eq!(reserve_usdc_liquidity1, reserve_usdc_liquidity0,);
    assert_eq!(reserve_sol_liquidity1, reserve_sol_liquidity0,);

    // MAKE MILTIPLE FLASH LOAN WITH MULTIPLE SIGNERS

    let mut ixs = vec![];

    let lender_usdc_token_acc0 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_usdc)
        .await
        .expect("get token acc");
    let borrower_sol_token_acc0 = get_token_account(&mut ctx.banks_client, borrower_liq_wallet_sol)
        .await
        .expect("get token acc");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity0 = reserve.liquidity.available_amount;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_sol_liquidity0 = reserve.liquidity.available_amount;

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        RefreshReserve {
            reserve: reserve_sol1_pubkey,
            market_price_feed: sol_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_sol1_pubkey,
            destination_wallet: borrower_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_sol1_pubkey,
            source_wallet: lender_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            user_transfer_authority: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("sending tx");

    let lender_usdc_token_acc1 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_usdc)
        .await
        .expect("get token acc");
    let borrower_sol_token_acc1 = get_token_account(&mut ctx.banks_client, borrower_liq_wallet_sol)
        .await
        .expect("get token acc");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity1 = reserve.liquidity.available_amount;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_sol_liquidity1 = reserve.liquidity.available_amount;

    assert_eq!(
        lender_usdc_token_acc1.amount,
        lender_usdc_token_acc0.amount + (100 * LAMPORTS_PER_USDC),
    );
    assert_eq!(
        borrower_sol_token_acc1.amount,
        borrower_sol_token_acc0.amount + (LAMPORTS_PER_SOL),
    );
    assert_eq!(reserve_usdc_liquidity1, reserve_usdc_liquidity0);
    assert_eq!(reserve_sol_liquidity1, reserve_sol_liquidity0);

    // MAKE MILTIPLE FLASH LOAN FROM ONE RESERVE WITH MULTIPLE SIGNERS

    ixs = vec![];

    let lender_usdc_token_acc0 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_usdc)
        .await
        .expect("get token acc");
    let borrower_sol_token_acc0 = get_token_account(&mut ctx.banks_client, borrower_liq_wallet_sol)
        .await
        .expect("get token acc");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity0 = reserve.liquidity.available_amount;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_sol_liquidity0 = reserve.liquidity.available_amount;

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        RefreshReserve {
            reserve: reserve_sol1_pubkey,
            market_price_feed: sol_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_sol1_pubkey,
            destination_wallet: borrower_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_sol1_pubkey,
            source_wallet: lender_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            user_transfer_authority: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_sol1_pubkey,
            destination_wallet: borrower_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_sol1_pubkey,
            source_wallet: lender_liq_wallet_sol,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: LAMPORTS_PER_SOL,
            user_transfer_authority: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_sol_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: 100 * LAMPORTS_PER_USDC,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("sending tx");

    let lender_usdc_token_acc1 = get_token_account(&mut ctx.banks_client, lender_liq_wallet_usdc)
        .await
        .expect("get token acc");
    let borrower_sol_token_acc1 = get_token_account(&mut ctx.banks_client, borrower_liq_wallet_sol)
        .await
        .expect("get token acc");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity1 = reserve.liquidity.available_amount;
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_sol_liquidity1 = reserve.liquidity.available_amount;

    assert_eq!(
        lender_usdc_token_acc1.amount,
        lender_usdc_token_acc0.amount + (200 * LAMPORTS_PER_USDC),
    );
    assert_eq!(
        borrower_sol_token_acc1.amount,
        borrower_sol_token_acc0.amount + (2 * LAMPORTS_PER_SOL),
    );
    assert_eq!(reserve_usdc_liquidity1, reserve_usdc_liquidity0);
    assert_eq!(reserve_sol_liquidity1, reserve_sol_liquidity0);
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#115b5734a220801cb3f3e0a89923a695
#[tokio::test]
async fn deposit_inside_flash() {
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

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1000 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // MAKE DEPOSIT INSIDE FLASH LOAN

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity0 = reserve.liquidity.available_amount;

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        DepositLiquidity {
            authority: borrower_pubkey,
            source_liquidity_wallet: borrower_liq_wallet_usdc,
            destination_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            liquidity_mint: liquidity_usdc_mint,
            amount: 100 * LAMPORTS_PER_USDC,
            liquidity_token_program: spl_token::id(),
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("sending tx");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity1 = reserve.liquidity.available_amount;

    assert_eq!(
        reserve_usdc_liquidity1,
        reserve_usdc_liquidity0 + (100 * LAMPORTS_PER_USDC)
    );

    // MAKE DEPOSIT INSIDE FLASH LOAN SOURCE RESERVE LIQUIDITY SUPPLY

    let mut ixs = vec![];

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let source_liquidity_wallet = find_liquidity_supply(&reserve_usdc_pubkey).0;
    ixs.push(
        DepositLiquidity {
            authority: borrower_pubkey,
            source_liquidity_wallet,
            destination_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            amount: 100 * LAMPORTS_PER_USDC,
            liquidity_token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#118b5734a22080b9ac2be267cf5787bc
#[tokio::test]
async fn withdraw_inside_flash() {
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

    // Set reserve_usdc.max_withdraw_utilization to 100%

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_withdraw_utilization_bps = 10000; // 100%
    params.max_withdraw_utilization_bps = max_withdraw_utilization_bps;
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };

    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1100 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // TRY TO WITHDRAW INSIDE FLASH LOAN INSUFFICIENT FUNDS

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);

    let flash_borrow_amount = 1050 * LAMPORTS_PER_USDC;
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        WithdrawLiquidity {
            authority: borrower_pubkey,
            destination_liquidity_wallet: borrower_liq_wallet_usdc,
            source_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            lp_amount: 100 * LAMPORTS_PER_USDC,
            liquidity_token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // WITHDRAW INSIDE FLASH LOAN

    let mut ixs = vec![];

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity0 = reserve.liquidity.available_amount;

    let flash_borrow_amount = 1050 * LAMPORTS_PER_USDC;
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        WithdrawLiquidity {
            authority: borrower_pubkey,
            destination_liquidity_wallet: borrower_liq_wallet_usdc,
            source_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            lp_amount: 50 * LAMPORTS_PER_USDC,
            liquidity_token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let reserve_usdc_liquidity1 = reserve.liquidity.available_amount;

    assert_eq!(
        reserve_usdc_liquidity1,
        reserve_usdc_liquidity0 - (50 * LAMPORTS_PER_USDC)
    )
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#11cb5734a22080d3adb1f52c17bd8f05
#[tokio::test]
async fn flash_incorrect_wallets() {
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

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1000 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // TRY TO FLASH REPAY SOURCE LIQUIDITY SUPPLY

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let source_liquidity_wallet = find_liquidity_supply(&reserve_usdc_pubkey).0;

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: source_liquidity_wallet,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // TRY TO FLASH BORROW DESTINATION LIQUIDITY SUPPLY

    ixs.clear();

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: source_liquidity_wallet,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: reserve_liquidity,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#115b5734a22080bf87c5e9885ea9bd31
#[tokio::test]
async fn lock_collateral_inside_flash() {
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

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol1_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    let reserve_liquidity = 1000 * LAMPORTS_PER_USDC;
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        reserve_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // LOCK COLLATERAL INSIDE FLASH LOAN

    refresh_reserve(&mut ctx, reserve_usdc_pubkey, usdc_price_feed, irm)
        .await
        .expect("refresh_reserve");
    refresh_position(&mut ctx, borrower_position_keypair.pubkey())
        .await
        .expect("refresh_position");

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let flash_borrow_amount = 100 * LAMPORTS_PER_USDC;

    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        LockCollateral {
            position: borrower_position_keypair.pubkey(),
            source_lp_wallet: borrower_lp_wallet_usdc,
            owner: borrower_pubkey,
            reserve: reserve_usdc_pubkey,
            amount: flash_borrow_amount,
            memo: [1; 24],
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#115b5734a220803ca7b3c7e22616e820
#[tokio::test]
async fn borrow_inside_flash() {
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

    // Set reserve_usdc.max_withdraw_utilization to 50%

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_borrow_utilization_bps = 5000; // 50%
    let max_withdraw_utilization_bps = 5500; // 55%
    params.max_borrow_utilization_bps = max_borrow_utilization_bps;
    params.max_withdraw_utilization_bps = max_withdraw_utilization_bps;
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };

    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_borrow_ltv_bps = 9900; // 99%
    params.max_borrow_ltv_bps = max_borrow_ltv_bps;
    params.partly_unhealthy_ltv_bps = max_borrow_ltv_bps + 1;
    params.fully_unhealthy_ltv_bps = max_borrow_ltv_bps + 2;

    alter_reserve(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol2_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO sol2 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        1000 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("deposit_liquidity");

    // BORROW 400 USDC to increase utilization by 40%

    let position_lender_kp = Keypair::new();
    create_position(
        &mut ctx,
        &position_lender_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_kp.pubkey(),
        &lender_keypair,
        lender_lp_wallet_sol,
        10_010_000_000, // 10.01 SOL = 1000 USDC = 1001 USD
    )
    .await
    .expect("lock_collateral");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow 400 USDC to increase utilization by 40%");
    borrow(
        &mut ctx,
        position_lender_kp.pubkey(),
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_keypair.pubkey(),
        &lender_keypair,
        curator_keypair.pubkey(),
        curator_fee_receiver,
        texture_fee_receiver,
        lender_liq_wallet_usdc,
        400 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // CHECK UTILIZATION = 40%

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(
        reserve.liquidity.utilization_rate().unwrap(),
        Decimal::from_i128_with_scale(4, 1).unwrap()
    );

    // BORROW 50 USDC INSIDE FLASH LOAN

    let mut ixs = vec![];

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let flash_borrow_amount = 50 * LAMPORTS_PER_USDC;

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount, // utilization increased to 45%
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        Borrow {
            position: position_lender_kp.pubkey(),
            destination_liquidity_wallet: lender_liq_wallet_usdc,
            curator_fee_receiver,
            reserve: reserve_usdc_pubkey,
            pool: pool_keypair.pubkey(),
            curator: curator_keypair.pubkey(),
            texture_fee_receiver,
            amount: flash_borrow_amount, // utilization increased to 50%
            slippage_limit: 0,
            memo: [1; 32],
            borrower: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount, // utilization decreased to 45%
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");

    // CHECK BORROWED AMOUNT

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(
        reserve.liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(450 * LAMPORTS_PER_USDC, 6).unwrap()
    );
    assert_eq!(reserve.liquidity.available_amount, 550 * LAMPORTS_PER_USDC);

    // TRY TO BORROW WHEN RESERVE IS STALE

    ixs.clear();

    ixs.push(
        Borrow {
            position: position_lender_kp.pubkey(),
            destination_liquidity_wallet: lender_liq_wallet_usdc,
            curator_fee_receiver,
            reserve: reserve_usdc_pubkey,
            pool: pool_keypair.pubkey(),
            curator: curator_keypair.pubkey(),
            texture_fee_receiver,
            amount: flash_borrow_amount,
            slippage_limit: 0,
            memo: [1; 32],
            borrower: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &lender_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // TRY TO BORROW 11 USDC INSIDE FLASH LOAN TO INCREASE UTILIZATION TO 50.1%

    let mut ixs = vec![];

    let flash_borrow_amount = 40 * LAMPORTS_PER_USDC;

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount, // utilization increased to 49%
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        Borrow {
            position: position_lender_kp.pubkey(),
            destination_liquidity_wallet: lender_liq_wallet_usdc,
            curator_fee_receiver,
            reserve: reserve_usdc_pubkey,
            pool: pool_keypair.pubkey(),
            curator: curator_keypair.pubkey(),
            texture_fee_receiver,
            amount: 11 * LAMPORTS_PER_USDC, // utilization increased to 50.1%
            slippage_limit: 0,
            memo: [1; 32],
            borrower: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // CHECK BORROWED AMOUNT

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(
        reserve.liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(450 * LAMPORTS_PER_USDC, 6).unwrap()
    );
    assert_eq!(reserve.liquidity.available_amount, 550 * LAMPORTS_PER_USDC);

    // BORROW & WITHDRAW INSIDE FLASH LOAN, MAX_WITHDRAW_UTILIZATION EXCEEDED

    let mut ixs = vec![];

    let flash_borrow_amount = 25 * LAMPORTS_PER_USDC;

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount, // utilization increased to 47,5%
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        Borrow {
            position: position_lender_kp.pubkey(),
            destination_liquidity_wallet: lender_liq_wallet_usdc,
            curator_fee_receiver,
            reserve: reserve_usdc_pubkey,
            pool: pool_keypair.pubkey(),
            curator: curator_keypair.pubkey(),
            texture_fee_receiver,
            amount: flash_borrow_amount, // utilization increased to 50%
            slippage_limit: 0,
            memo: [1; 32],
            borrower: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        WithdrawLiquidity {
            authority: borrower_pubkey,
            destination_liquidity_wallet: borrower_liq_wallet_usdc,
            source_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            lp_amount: 100 * LAMPORTS_PER_USDC, // utilization increased to 55,5%
            liquidity_mint: liquidity_usdc_mint,
            liquidity_token_program: spl_token::id(),
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    info!("try to exceed max_withdraw_utilization");
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // BORROW & WITHDRAW INSIDE FLASH LOAN

    let mut ixs = vec![];

    let flash_borrow_amount = 25 * LAMPORTS_PER_USDC;

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount, // utilization increased to 47,5%
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        Borrow {
            position: position_lender_kp.pubkey(),
            destination_liquidity_wallet: lender_liq_wallet_usdc,
            curator_fee_receiver,
            reserve: reserve_usdc_pubkey,
            pool: pool_keypair.pubkey(),
            curator: curator_keypair.pubkey(),
            texture_fee_receiver,
            amount: flash_borrow_amount, // utilization increased to 50%
            slippage_limit: 0,
            memo: [1; 32],
            borrower: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        WithdrawLiquidity {
            authority: borrower_pubkey,
            destination_liquidity_wallet: borrower_liq_wallet_usdc,
            source_lp_wallet: borrower_lp_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            lp_amount: 50 * LAMPORTS_PER_USDC, // utilization increased to ~53% - ok, max_withdraw_utilization=55%
            liquidity_token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount, // utilization decreased to 50%
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");

    // CHECK BORROWED AMOUNT

    refresh_reserve(&mut ctx, reserve_usdc_pubkey, usdc_price_feed, irm)
        .await
        .expect("refresh_reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(
        reserve.liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(475 * LAMPORTS_PER_USDC, 6).unwrap()
    );
    assert_eq!(reserve.liquidity.available_amount, 475 * LAMPORTS_PER_USDC);
    assert_eq!(
        reserve.liquidity.utilization_rate().unwrap(),
        Decimal::from_i128_with_scale(5, 1).unwrap() // 50%
    );
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#11bb5734a2208049859bdf97f76c2b47
#[tokio::test]
async fn repay_inside_flash() {
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

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_borrow_utilization_bps = 10000; // 100%
    let max_withdraw_utilization_bps = 10000; // 100%
    params.max_borrow_utilization_bps = max_borrow_utilization_bps;
    params.max_withdraw_utilization_bps = max_withdraw_utilization_bps;
    params.fees = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 0,
        curator_performance_fee_rate_bps: 0,
        _padding: Zeroable::zeroed(),
    };

    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        params,
        RESERVE_MODE_NORMAL,
    )
    .await
    .expect("alter_reserve");

    // Enable flash loans for reserves

    enable_flash_loans(
        &mut ctx,
        vec![reserve_sol2_pubkey, reserve_usdc_pubkey],
        pool_keypair.pubkey(),
        curator_keypair.pubkey(),
        &pool_authority_keypair,
    )
    .await
    .expect("enable_flash_loans");

    // DEPOSIT INITIAL LIQUIDITY TO sol2 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let lender_lp_wallet_sol = create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
        .await
        .expect("create lp ata");
    let lender_liq_wallet_sol = get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit sol initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        lender_liq_wallet_sol,
        lender_lp_wallet_sol,
        100 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT INITIAL LIQUIDITY TO USDC RESERVE

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);

    info!("deposit usdc initial liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        borrower_liq_wallet_usdc,
        borrower_lp_wallet_usdc,
        1000 * LAMPORTS_PER_USDC,
    )
    .await
    .expect("deposit_liquidity");

    // BORROW 400 USDC

    let position_lender_kp = Keypair::new();
    create_position(
        &mut ctx,
        &position_lender_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_kp.pubkey(),
        &lender_keypair,
        lender_lp_wallet_sol,
        4_005_000_000, // 4.005 SOL = 400 USDC = 400,4 USD
    )
    .await
    .expect("lock_collateral");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh position");

    let lender_liq_wallet_usdc = get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow 200 USDC");
    borrow(
        &mut ctx,
        position_lender_kp.pubkey(),
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_keypair.pubkey(),
        &lender_keypair,
        curator_keypair.pubkey(),
        curator_fee_receiver,
        texture_fee_receiver,
        lender_liq_wallet_usdc,
        200 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // TRY TO REPAY INSIDE FLASH LOAN FROM RESERVE LIQUIDITY SUPPLY

    let mut ixs = vec![];

    let flash_borrow_amount = 500 * LAMPORTS_PER_USDC;

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let source_liquidity_wallet = find_liquidity_supply(&reserve_usdc_pubkey).0;
    ixs.push(
        Repay {
            position: position_lender_kp.pubkey(),
            source_liquidity_wallet,
            reserve: reserve_usdc_pubkey,
            amount: 200 * LAMPORTS_PER_USDC,
            user_authority: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());

    // REPAY INSIDE FLASH LOAN

    let mut ixs = vec![];

    let flash_borrow_amount = 500 * LAMPORTS_PER_USDC;

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    ixs.push(
        RefreshReserve {
            reserve: reserve_usdc_pubkey,
            market_price_feed: usdc_price_feed,
            irm,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashBorrow {
            reserve: reserve_usdc_pubkey,
            destination_wallet: lender_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        Repay {
            position: position_lender_kp.pubkey(),
            source_liquidity_wallet: lender_liq_wallet_usdc,
            reserve: reserve_usdc_pubkey,
            amount: 200 * LAMPORTS_PER_USDC,
            user_authority: lender_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    ixs.push(
        FlashRepay {
            reserve: reserve_usdc_pubkey,
            source_wallet: borrower_liq_wallet_usdc,
            sysvar_instructions: solana_program::sysvar::instructions::id(),
            amount: flash_borrow_amount,
            user_transfer_authority: borrower_pubkey,
            token_program: spl_token::id(),
            liquidity_mint: liquidity_usdc_mint,
        }
        .into_instruction(),
    );
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &borrower_keypair, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("send tx");

    refresh_position(&mut ctx, position_lender_kp.pubkey())
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_kp.pubkey())
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(position.borrowed_value, 0);
    assert_eq!(reserve.liquidity.available_amount, 1000 * LAMPORTS_PER_USDC)
}
