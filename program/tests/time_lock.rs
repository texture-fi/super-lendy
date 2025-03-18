#![cfg(feature = "test-bpf")]

use bytemuck::Zeroable;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use super_lendy::state::reserve::{ConfigFields, ConfigProposal, Reserve};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};
use texture_common::account::PodAccount;

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    alter_reserve, alter_texture_config, apply_proposal, propose_config,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair, get_account,
    init_program_test, init_token_accounts, lender_keypair, texture_config_keypair, Runner,
    LAMPORTS,
};

pub mod utils;

pub fn texture_config_params(fees_authority: Pubkey) -> TextureConfigParams {
    TextureConfigParams {
        borrow_fee_rate_bps: 3000,
        performance_fee_rate_bps: 4000,
        fees_authority,
        reserve_timelock: ReserveTimelock {
            market_price_feed_lock_sec: 10,
            irm_lock_sec: 20,
            liquidation_bonus_lock_sec: 30,
            unhealthy_ltv_lock_sec: 40,
            partial_liquidation_factor_lock_sec: 50,
            max_total_liquidity_lock_sec: 60,
            max_borrow_ltv_lock_sec: 70,
            max_borrow_utilization_lock_sec: 80,
            price_stale_threshold_lock_sec: 90,
            max_withdraw_utilization_lock_sec: 100,
            fees_lock_sec: 110,
            _padding: 0,
        },
    }
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#21e8f679153e4cdc88b56d61dfa75c63
#[tokio::test]
async fn change_proposal() {
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
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    // 1 USDC = 1.001 USD
    add_price_feed_acc(&mut runner, "usdc-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;
    let new_irm = add_curve_acc(&mut runner, "curve1-acc").await;
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

    // ALTER TEXTURE CONFIG

    let params = texture_config_params(texture_owner_pubkey);
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut config = reserve.config;
    config.irm = new_irm;

    let config_proposal = ConfigProposal {
        can_be_applied_at: 0,
        change_map: ConfigFields::IRM.bits(),
        config,
    };

    // PROPOSE CONFIG WITH NEW IRM

    propose_config(
        &mut ctx,
        pool_pubkey,
        reserve_usdc_pubkey,
        sol_price_feed,
        curator_pubkey,
        &pool_authority_keypair,
        0,
        config_proposal,
    )
    .await
    .expect("propose_config");

    let mut config = reserve.config;
    config.max_borrow_ltv_bps = 9001;

    let config_proposal = ConfigProposal {
        can_be_applied_at: 0,
        change_map: ConfigFields::MAX_BORROW_LTV.bits(),
        config,
    };

    // PROPOSE CONFIG WITH NEW MAX_BORROW_LTV

    propose_config(
        &mut ctx,
        pool_pubkey,
        reserve_usdc_pubkey,
        sol_price_feed,
        curator_pubkey,
        &pool_authority_keypair,
        0,
        config_proposal,
    )
    .await
    .expect("propose_config");

    // CHECK reserve.proposed_configs

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let config_proposal = reserve.proposed_configs.0.first().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("create timestamp in timing")
        .as_secs();
    let exp_applied = now + 70;
    assert!((exp_applied - config_proposal.can_be_applied_at as u64) <= 1); // may be 1 sec delay
    assert_eq!(config_proposal.config.irm, irm); // old irm
    assert_eq!(config_proposal.config.max_borrow_ltv_bps, 9001); // new max_borrow_ltv_bps
    assert_eq!(
        config_proposal.change_map,
        ConfigFields::MAX_BORROW_LTV.bits()
    );
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#21e8f679153e4cdc88b56d61dfa75c63
#[tokio::test]
async fn remove_proposal() {
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
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    // 1 USDC = 1.001 USD
    add_price_feed_acc(&mut runner, "usdc-usd").await;

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;
    let new_irm = add_curve_acc(&mut runner, "curve1-acc").await;
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

    // ALTER TEXTURE CONFIG

    let params = texture_config_params(texture_owner_pubkey);
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut config = reserve.config;
    config.irm = new_irm;

    let config_proposal = ConfigProposal {
        can_be_applied_at: 0,
        change_map: ConfigFields::IRM.bits(),
        config,
    };

    // PROPOSE CONFIG WITH NEW IRM

    propose_config(
        &mut ctx,
        pool_pubkey,
        reserve_usdc_pubkey,
        sol_price_feed,
        curator_pubkey,
        &pool_authority_keypair,
        0,
        config_proposal,
    )
    .await
    .expect("propose_config");

    // REMOVE PROPOSAL

    let config_proposal = ConfigProposal::zeroed();

    propose_config(
        &mut ctx,
        pool_pubkey,
        reserve_usdc_pubkey,
        sol_price_feed,
        curator_pubkey,
        &pool_authority_keypair,
        0,
        config_proposal,
    )
    .await
    .expect("propose_config");

    // TRY TO APPLY CONFIG WITH ZEROED PROPOSAL

    let result = apply_proposal(
        &mut ctx,
        pool_pubkey,
        reserve_usdc_pubkey,
        sol_price_feed,
        curator_pubkey,
        &pool_authority_keypair,
        0,
    )
    .await;
    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#21e8f679153e4cdc88b56d61dfa75c63
#[tokio::test]
async fn propose_config_with_zeroed_lock_sec() {
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
    let sol_price_feed = add_price_feed_acc(&mut runner, "sol-usd").await;
    // 1 USDC = 1.001 USD
    add_price_feed_acc(&mut runner, "usdc-usd").await;

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

    // ALTER TEXTURE CONFIG

    let mut params = texture_config_params(texture_owner_pubkey);
    params.reserve_timelock.max_borrow_ltv_lock_sec = 0;
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut config = reserve.config;
    config.max_borrow_ltv_bps = 9001;

    let config_proposal = ConfigProposal {
        can_be_applied_at: 0,
        change_map: ConfigFields::MAX_BORROW_LTV.bits(),
        config,
    };

    // TRY TO PROPOSE CONFIG WITH ZEROED LOCK SEC

    let result = propose_config(
        &mut ctx,
        pool_pubkey,
        reserve_usdc_pubkey,
        sol_price_feed,
        curator_pubkey,
        &pool_authority_keypair,
        0,
        config_proposal,
    )
    .await;
    assert!(result.is_err());

    // ALTER reserve.max_borrow_ltv_bps

    alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        config,
        reserve.mode,
    )
    .await
    .expect("alter_reserve");

    // CHECK MAX_BORROW_LTV CHANGED

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(reserve.config.max_borrow_ltv_bps, 9001)
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#21e8f679153e4cdc88b56d61dfa75c63
#[tokio::test]
async fn alter_for_time_locked_param() {
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
    add_price_feed_acc(&mut runner, "usdc-usd").await;

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

    // ALTER TEXTURE CONFIG

    let params = texture_config_params(texture_owner_pubkey);
    alter_texture_config(&mut ctx, &texture_owner_keypair, params)
        .await
        .expect("alter_texture_config");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut config = reserve.config;
    config.max_borrow_ltv_bps = 9001;

    // TRY TO ALTER TIME LOCKED MAX_BORROW_LTV

    let result = alter_reserve(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        config,
        reserve.mode,
    )
    .await;
    assert!(result.is_err())
}
