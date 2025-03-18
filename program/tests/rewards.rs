#![cfg(feature = "test-bpf")]

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

use super_lendy::pda::{find_lp_token_mint, find_reward_supply};
use super_lendy::state::position::Position;
use super_lendy::state::reserve::{Reserve, NO_REWARD, REWARD_FOR_BORROW, REWARD_FOR_LIQUIDITY};

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    borrow, claim_reward, create_position, deposit_liquidity, init_reward_supply, lock_collateral,
    refresh_position, set_reward_rules, withdraw_reward,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, get_token_account, init_program_test,
    init_token_accounts, lender_keypair, texture_config_keypair, Runner, LAMPORTS,
    LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#b06e521acac541edb75205f1a182b8cc
#[tokio::test]
async fn reward_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let position_borrower_pubkey = borrower_position_keypair.pubkey();
    let lender_position_kp = Keypair::new();
    let position_lender_pubkey = lender_position_kp.pubkey();

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
    let msol_mint = Pubkey::from_str("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So").unwrap();
    let jitosol_mint = Pubkey::from_str("jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL").unwrap();
    let jupsol_mint = Pubkey::from_str("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").unwrap();
    init_token_accounts(&mut runner, &liquidity_sol_mint);
    init_token_accounts(&mut runner, &liquidity_usdc_mint);
    init_token_accounts(&mut runner, &msol_mint);
    init_token_accounts(&mut runner, &jitosol_mint);
    init_token_accounts(&mut runner, &jupsol_mint);

    let lender_msol_wallet = get_associated_token_address(&lender_pubkey, &msol_mint);
    let borrower_msol_wallet = get_associated_token_address(&borrower_pubkey, &msol_mint);

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

    // INIT mSOL REWARD SUPPLY
    info!("init reward supply");
    init_reward_supply(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
    )
    .await
    .expect("init_reward_supply");
    let msol_reward_supply = find_reward_supply(&pool_pubkey, &msol_mint).0;

    // DEPOSIT REWARD

    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &lender_msol_wallet,
        &msol_reward_supply,
        &lender_pubkey,
        &[],
        10_000 * LAMPORTS_PER_SOL,
    )
    .expect("transfer ix");

    info!("deposit reward");
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("transfer");

    // SET REWARDS RULES FOR SOL RESERVE
    info!("set rewards rules for sol reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let mut new_rules = reserve.reward_rules;

    new_rules.rules[0].reward_mint = msol_mint;
    new_rules.rules[0].name = [0; 7];
    new_rules.rules[0].reason = REWARD_FOR_LIQUIDITY;
    new_rules.rules[0].start_slot = 0;
    new_rules.rules[0]
        .set_rate(Decimal::from_i128_with_scale(1, 3).unwrap())
        .expect("set_rate");

    new_rules.rules[1].reward_mint = jitosol_mint;
    new_rules.rules[1].name = [1; 7];
    new_rules.rules[1].reason = REWARD_FOR_LIQUIDITY;
    new_rules.rules[1].start_slot = 0;
    new_rules.rules[1]
        .set_rate(Decimal::from_i128_with_scale(2, 3).unwrap())
        .expect("set_rate");

    new_rules.rules[2].reward_mint = msol_mint;
    new_rules.rules[2].name = [2; 7];
    new_rules.rules[2].reason = REWARD_FOR_BORROW;
    new_rules.rules[2].start_slot = 0;
    new_rules.rules[2]
        .set_rate(Decimal::from_i128_with_scale(4, 3).unwrap())
        .expect("set_rate");

    set_reward_rules(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // SET REWARDS RULES FOR USDC RESERVE
    info!("set rewards rules for usdc reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let mut new_rules = reserve.reward_rules;

    new_rules.rules[0].reward_mint = jupsol_mint;
    new_rules.rules[0].name = [3; 7];
    new_rules.rules[0].reason = REWARD_FOR_BORROW;
    new_rules.rules[0].start_slot = 0;
    new_rules.rules[0]
        .set_rate(Decimal::from_i128_with_scale(3, 3).unwrap())
        .expect("set_rate");

    new_rules.rules[1].reward_mint = jupsol_mint;
    new_rules.rules[1].name = [4; 7];
    new_rules.rules[1].reason = REWARD_FOR_LIQUIDITY;
    new_rules.rules[1].start_slot = 0;
    new_rules.rules[1]
        .set_rate(Decimal::from_i128_with_scale(3, 3).unwrap())
        .expect("set_rate");

    set_reward_rules(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // DEPOSIT 1000 SOL, 1000 USDC AND LOCK COLLATERAL

    create_position(
        &mut ctx,
        &lender_position_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);
    let deposit_sol_amount = 1_000 * LAMPORTS_PER_SOL;

    info!("deposit 1000 SOL, 1000 USDC & lock collateral by user1");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("lock_collateral");

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_lender_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_usdc =
        get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1_000 * LAMPORTS_PER_USDC;

    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_usdc,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_borrower_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_sol =
        get_associated_token_address(&borrower_pubkey, &liquidity_sol_mint);
    let deposit_sol_amount = 1_000 * LAMPORTS_PER_SOL;

    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("lock_collateral");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_sol,
        dest_borrower_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;
    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position_borrower =
        Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_borrower = position_borrower.rewards;

    // CHECK REWARD RECORDS

    assert_eq!(rewards_lender.rewards[0].reward_mint, msol_mint);
    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_lender.rewards[0].is_vacant());
    assert_eq!(rewards_lender.rewards[1].reward_mint, jitosol_mint);
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_lender.rewards[1].is_vacant());
    assert_eq!(rewards_lender.rewards[2].reward_mint, jupsol_mint);
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_lender.rewards[2].is_vacant());

    assert_eq!(rewards_borrower.rewards[0].reward_mint, msol_mint);
    assert_eq!(
        rewards_borrower.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_borrower.rewards[0].is_vacant());
    assert_eq!(rewards_borrower.rewards[1].reward_mint, jitosol_mint);
    assert_eq!(
        rewards_borrower.rewards[1].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_borrower.rewards[1].is_vacant());

    // 1000 SLOTS LATER

    let slot = 501_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    // Lender deposits and locks another 1000 USDC
    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_usdc,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    let mut slot = 1001_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;
    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position_borrower =
        Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_borrower = position_borrower.rewards;

    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(1_000_000_000_000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(2_000_000_000_000, 9).unwrap()
    ); // jitoSOL

    // Because on 501 slot there was 1000 USDC deposit by Lender he will receive
    // 3000 jupSOL for initial 1000 USDC locked for 1000 slots and also 1500 jupSOL
    // for another 1000 USDC locked for 500 slots.
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::from_lamports(4500_000000000, 9).unwrap()
    ); // jupSOL

    assert_eq!(
        rewards_borrower.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(2000_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_borrower.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(4000_000000000, 9).unwrap()
    ); // jitoSOL

    // Borrow 100 USDC

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
    let borrow_amount = 100 * LAMPORTS_PER_USDC;

    info!("borrow {} usdc by borrower", borrow_amount);
    borrow(
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
        borrow_amount,
        1,
    )
    .await
    .expect("borrow");

    info!("borrow {} usdc by lender", borrow_amount);
    borrow(
        &mut ctx,
        position_lender_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &lender_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        source_lender_liq_wallet_usdc,
        borrow_amount,
        1,
    )
    .await
    .expect("borrow");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;
    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position_borrower =
        Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_borrower = position_borrower.rewards;

    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(1000_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(2000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::from_lamports(4500_000000000, 9).unwrap()
    ); // jupSOL

    assert_eq!(
        rewards_borrower.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(2000_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_borrower.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(4000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_borrower.rewards[2].accrued_amount().unwrap(),
        Decimal::ZERO
    ); // jupSOL

    slot += 500;
    info!("wrap to slot {} to make second borrow", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    // Borrow again same amount as first time. This function also do RefreshPosition prior to Borrow.
    // Because Borrow requires refreshed position. This allows correct rewards calculation before
    // increase of borrowed amount.
    borrow(
        &mut ctx,
        position_lender_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &lender_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        source_lender_liq_wallet_usdc,
        borrow_amount,
        1,
    )
    .await
    .expect("borrow");

    slot += 500;
    info!("wrap to slot {} to make second borrow", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;
    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position_borrower =
        Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_borrower = position_borrower.rewards;

    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(2000_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(4000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_lender.rewards[2]
            .accrued_amount()
            .unwrap()
            .round_to_decimals(9),
        Decimal::from_lamports(11089_502492396, 9).unwrap()
    ); // jupSOL, liquidity + (borrow + interest)

    assert_eq!(
        rewards_borrower.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(4000_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_borrower.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(8000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_borrower.rewards[2]
            .accrued_amount()
            .unwrap()
            .round_to_decimals(9),
        Decimal::from_lamports(393_002492398, 9).unwrap()
    ); // jupSOL liquidity + (borrow + interest)

    // CLAIM REWARDS

    let borrower_msol_token_acc0 = get_token_account(&mut ctx.banks_client, borrower_msol_wallet)
        .await
        .expect("get token acc");
    let lender_msol_token_acc0 = get_token_account(&mut ctx.banks_client, lender_msol_wallet)
        .await
        .expect("get token acc");

    info!("claim borrower rewards");
    claim_reward(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        position_borrower_pubkey,
        &borrower_keypair,
        borrower_msol_wallet,
    )
    .await
    .expect("claim_reward");

    info!("claim lender rewards");
    claim_reward(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        position_lender_pubkey,
        &lender_keypair,
        lender_msol_wallet,
    )
    .await
    .expect("claim_reward");

    let borrower_msol_token_acc1 = get_token_account(&mut ctx.banks_client, borrower_msol_wallet)
        .await
        .expect("get token acc");
    let lender_msol_token_acc1 = get_token_account(&mut ctx.banks_client, lender_msol_wallet)
        .await
        .expect("get token acc");

    // CHECK TRANSFERS
    assert_eq!(
        borrower_msol_token_acc1.amount,
        borrower_msol_token_acc0.amount + 4000_000000000
    );
    assert_eq!(
        lender_msol_token_acc1.amount,
        lender_msol_token_acc0.amount + 2000_000000000
    );

    // CHECK POSITIONS

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;
    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position_borrower =
        Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_borrower = position_borrower.rewards;

    assert_eq!(
        rewards_lender.rewards[0].reward_mint,
        solana_sdk::system_program::ID
    );
    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert_eq!(
        rewards_borrower.rewards[0].reward_mint,
        solana_sdk::system_program::ID
    );
    assert_eq!(
        rewards_borrower.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );

    // CHECK POSITIONS AFTER REFRESH

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;
    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position_borrower =
        Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_borrower = position_borrower.rewards;

    assert_eq!(rewards_lender.rewards[0].reward_mint, msol_mint);
    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_lender.rewards[0].is_vacant());
    assert_eq!(rewards_borrower.rewards[0].reward_mint, msol_mint);
    assert_eq!(
        rewards_borrower.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert!(!rewards_borrower.rewards[0].is_vacant());
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#142f2f3178574e55bacb3aeeecf52f0f
#[tokio::test]
async fn withdraw_reward_success() {
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
    let msol_mint = Pubkey::from_str("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So").unwrap();
    init_token_accounts(&mut runner, &liquidity_sol_mint);
    init_token_accounts(&mut runner, &liquidity_usdc_mint);
    init_token_accounts(&mut runner, &msol_mint);

    let lender_msol_wallet = get_associated_token_address(&lender_pubkey, &msol_mint);

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

    // INIT mSOL REWARD SUPPLY
    info!("init reward supply");
    init_reward_supply(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
    )
    .await
    .expect("init_reward_supply");
    let msol_reward_supply = find_reward_supply(&pool_pubkey, &msol_mint).0;

    // DEPOSIT REWARD

    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &lender_msol_wallet,
        &msol_reward_supply,
        &lender_pubkey,
        &[],
        10_000 * LAMPORTS_PER_SOL,
    )
    .expect("transfer ix");

    info!("deposit reward");
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("transfer");

    // TRY TO WITHDRAW WITH INCORRECT pool_authority

    info!("try to withdraw with incorrect pool_authority");
    let result = withdraw_reward(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        curator_pubkey,
        &lender_keypair,
        lender_msol_wallet,
        5_000 * LAMPORTS_PER_SOL,
    )
    .await;
    assert!(result.is_err());

    // WITHDRAW REWARDS

    let lender_msol_token_acc0 = get_token_account(&mut ctx.banks_client, lender_msol_wallet)
        .await
        .expect("get token acc");
    let amount = 5_000 * LAMPORTS_PER_SOL;

    info!("withdraw rewards");
    withdraw_reward(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        lender_msol_wallet,
        amount,
    )
    .await
    .expect("withdraw_reward");

    let lender_msol_token_acc1 = get_token_account(&mut ctx.banks_client, lender_msol_wallet)
        .await
        .expect("get token acc");

    assert_eq!(
        lender_msol_token_acc1.amount,
        lender_msol_token_acc0.amount + amount
    )
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#b491bacd0485438e897363470016ba85
#[tokio::test]
async fn set_reward_rules_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let lender_position_kp = Keypair::new();
    let position_lender_pubkey = lender_position_kp.pubkey();

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
    let msol_mint = Pubkey::from_str("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So").unwrap();
    let jitosol_mint = Pubkey::from_str("jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL").unwrap();
    let jupsol_mint = Pubkey::from_str("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").unwrap();
    init_token_accounts(&mut runner, &liquidity_sol_mint);
    init_token_accounts(&mut runner, &liquidity_usdc_mint);
    init_token_accounts(&mut runner, &msol_mint);
    init_token_accounts(&mut runner, &jitosol_mint);
    init_token_accounts(&mut runner, &jupsol_mint);

    let lender_msol_wallet = get_associated_token_address(&lender_pubkey, &msol_mint);

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

    // INIT mSOL REWARD SUPPLY
    info!("init reward supply");
    init_reward_supply(
        &mut ctx,
        msol_mint,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
    )
    .await
    .expect("init_reward_supply");
    let msol_reward_supply = find_reward_supply(&pool_pubkey, &msol_mint).0;

    // DEPOSIT REWARD

    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &lender_msol_wallet,
        &msol_reward_supply,
        &lender_pubkey,
        &[],
        10_000 * LAMPORTS_PER_SOL,
    )
    .expect("transfer ix");

    info!("deposit reward");
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &lender_keypair],
        blockhash,
    );
    ctx.banks_client
        .process_transaction(tx)
        .await
        .expect("transfer");

    // SET REWARDS RULES FOR SOL RESERVE
    info!("set rewards rules for sol reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let mut new_rules = reserve.reward_rules;

    new_rules.rules[0].reward_mint = msol_mint;
    new_rules.rules[0].name = [0; 7];
    new_rules.rules[0].reason = REWARD_FOR_LIQUIDITY;
    new_rules.rules[0].start_slot = 0;
    new_rules.rules[0]
        .set_rate(Decimal::from_i128_with_scale(1, 3).unwrap())
        .expect("set_rate");

    new_rules.rules[1].reward_mint = jitosol_mint;
    new_rules.rules[1].name = [1; 7];
    new_rules.rules[1].reason = REWARD_FOR_LIQUIDITY;
    new_rules.rules[1].start_slot = 0;
    new_rules.rules[1]
        .set_rate(Decimal::from_i128_with_scale(2, 3).unwrap())
        .expect("set_rate");

    set_reward_rules(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // SET REWARDS RULES FOR USDC RESERVE
    info!("set rewards rules for usdc reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let mut new_rules = reserve.reward_rules;

    new_rules.rules[0].reward_mint = jupsol_mint;
    new_rules.rules[0].name = [3; 7];
    new_rules.rules[0].reason = REWARD_FOR_BORROW;
    new_rules.rules[0].start_slot = 0;
    new_rules.rules[0]
        .set_rate(Decimal::from_i128_with_scale(3, 3).unwrap())
        .expect("set_rate");

    new_rules.rules[1].reward_mint = jupsol_mint;
    new_rules.rules[1].name = [4; 7];
    new_rules.rules[1].reason = REWARD_FOR_LIQUIDITY;
    new_rules.rules[1].start_slot = 0;
    new_rules.rules[1]
        .set_rate(Decimal::from_i128_with_scale(3, 3).unwrap())
        .expect("set_rate");

    set_reward_rules(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // DEPOSIT 1000 SOL, 1000 USDC AND LOCK COLLATERAL

    create_position(
        &mut ctx,
        &lender_position_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);
    let deposit_sol_amount = 1_000 * LAMPORTS_PER_SOL;

    info!("deposit 1000 SOL, 1000 USDC & lock collateral by user1");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("lock_collateral");

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_lender_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_usdc =
        get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1_000 * LAMPORTS_PER_USDC;

    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_usdc,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;

    // CHECK REWARD RECORDS

    assert_eq!(rewards_lender.rewards[0].reward_mint, msol_mint);
    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert_eq!(rewards_lender.rewards[1].reward_mint, jitosol_mint);
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::ZERO
    );
    assert_eq!(rewards_lender.rewards[2].reward_mint, jupsol_mint);
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::ZERO
    );

    // 1000 SLOTS LATER

    let mut slot = 1001_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;

    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(1000_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(2000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::from_lamports(3000_000000000, 9).unwrap()
    ); // jupSOL

    // SET REWARD RULES FOR SOL RESERVE, SET mSOL RATE TO 0.0001

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let mut new_rules = reserve.reward_rules;
    new_rules.rules[0]
        .set_rate(Decimal::from_i128_with_scale(1, 4).unwrap())
        .expect("set_rate");

    info!("set reward rules for sol reserve, set mSOL rate to 0.0001");
    set_reward_rules(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // 1000 SLOTS LATER

    slot += 1000;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;

    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(1100_000000000, 9).unwrap()
    ); // mSOL
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(4000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::from_lamports(6000_000000000, 9).unwrap()
    ); // jupSOL

    // SET REWARD RULES FOR SOL RESERVE, SET REASON TO NO_REWARD

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let mut new_rules = reserve.reward_rules;
    new_rules.rules[0].reason = NO_REWARD;

    info!("set reward rules for sol reserve, set reason to NO_REWARD");
    set_reward_rules(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // 1000 SLOTS LATER

    slot += 1000;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;

    assert_eq!(
        rewards_lender.rewards[0].accrued_amount().unwrap(),
        Decimal::from_lamports(1100_000000000, 9).unwrap()
    ); // mSOL, no rewards accrued
    assert_eq!(
        rewards_lender.rewards[1].accrued_amount().unwrap(),
        Decimal::from_lamports(6000_000000000, 9).unwrap()
    ); // jitoSOL
    assert_eq!(
        rewards_lender.rewards[2].accrued_amount().unwrap(),
        Decimal::from_lamports(9000_000000000, 9).unwrap()
    ); // jupSOL
}

/// See test description in
/// https://www.notion.so/Super-Lendy-3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#c85894cf5f7046e2b7fe2cc67b03ce65
#[tokio::test]
async fn reward_array_overflow_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let borrower_keypair = borrow_keypair();
    let borrower_pubkey = borrower_keypair.pubkey();
    let lender_keypair = lender_keypair();
    let lender_pubkey = lender_keypair.pubkey();
    let borrower_position_keypair = Keypair::new();
    let lender_position_kp = Keypair::new();
    let position_lender_pubkey = lender_position_kp.pubkey();

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

    let liquidity_sol_mint =
        Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let liquidity_usdc_mint =
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    let irm = add_curve_acc(&mut runner, "const-40-pct-acc").await;

    let reward_mints = vec![
        liquidity_sol_mint,
        liquidity_usdc_mint,
        Pubkey::from_str("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So").unwrap(), // msol
        Pubkey::from_str("jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL").unwrap(), // jitosol
        Pubkey::from_str("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").unwrap(), // jupsol
        Pubkey::from_str("5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm").unwrap(), // inf
        Pubkey::from_str("vSoLxydx6akxyMD9XEcPvGYNGq6Nn66oqVb3UkGkei7").unwrap(), // vsol
        Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap(), // bonk
        Pubkey::from_str("EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm").unwrap(), // wif
        Pubkey::from_str("HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3").unwrap(), // pyth
    ];
    for mint in reward_mints.iter() {
        init_token_accounts(&mut runner, mint);
    }

    // MINT FOR 11th RULE
    let cwif_mint = Pubkey::from_str("7atgF8KQo4wJrD5ATGX7t1V2zVvykPJbFfNeVf1icFv1").unwrap();
    init_token_accounts(&mut runner, &cwif_mint);

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

    // SET 8 (MAX) REWARDS RULES FOR SOL RESERVE
    info!("set 8 (max) rewards rules for sol reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol2_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut new_rules = reserve.reward_rules;

    for (index, mint) in reward_mints[..8].iter().enumerate() {
        let mut rule = new_rules.rules[index];
        rule.reward_mint = *mint;
        rule.name = [(index + 1) as u8; 7];
        rule.reason = REWARD_FOR_LIQUIDITY;
        rule.start_slot = 0;
        rule.set_rate(Decimal::from_i128_with_scale(1, 3).unwrap())
            .expect("set_rate"); // 0.001
        new_rules.rules[index] = rule;
    }

    set_reward_rules(
        &mut ctx,
        reserve_sol2_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // SET 2 REWARDS RULES FOR USDC RESERVE
    info!("set 2 rewards rules for usdc reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut new_rules = reserve.reward_rules;

    for (index, mint) in reward_mints[8..].iter().enumerate() {
        let mut rule = new_rules.rules[index];
        rule.reward_mint = *mint;
        rule.name = [(index + 1) as u8; 7];
        rule.reason = REWARD_FOR_LIQUIDITY;
        rule.start_slot = 0;
        rule.set_rate(Decimal::from_i128_with_scale(1, 3).unwrap())
            .expect("set_rate"); // 0.001
        new_rules.rules[index] = rule;
    }

    set_reward_rules(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // DEPOSIT 1000 SOL, 1000 USDC AND LOCK COLLATERAL

    create_position(
        &mut ctx,
        &lender_position_kp,
        pool_keypair.pubkey(),
        &lender_keypair,
    )
    .await
    .expect("create_position");

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);
    let deposit_sol_amount = 1_000 * LAMPORTS_PER_SOL;

    info!("deposit 1000 SOL, 1000 USDC & lock collateral by user1");
    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_sol,
        deposit_sol_amount,
    )
    .await
    .expect("lock_collateral");

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_lender_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_usdc =
        get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1_000 * LAMPORTS_PER_USDC;

    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_usdc,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("deposit_liquidity");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;

    // CHECK REWARD RECORDS

    for (index, mint) in reward_mints.clone().into_iter().enumerate() {
        assert_eq!(rewards_lender.rewards[index].reward_mint, mint);
        assert_eq!(
            rewards_lender.rewards[index].accrued_amount().unwrap(),
            Decimal::ZERO
        );
        assert!(!rewards_lender.rewards[index].is_vacant());
    }

    // ADD NEW REWARDS RULE FOR USDC RESERVE
    info!("add new rewards rule for usdc reserve");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut new_rules = reserve.reward_rules;

    let mut rule = new_rules.rules[2];
    rule.reward_mint = cwif_mint;
    rule.name = [0; 7];
    rule.reason = REWARD_FOR_LIQUIDITY;
    rule.start_slot = 0;
    rule.set_rate(Decimal::from_i128_with_scale(3, 0).unwrap())
        .expect("set_rate");
    new_rules.rules[2] = rule;

    set_reward_rules(
        &mut ctx,
        reserve_usdc_pubkey,
        pool_pubkey,
        curator_pubkey,
        &pool_authority_keypair,
        new_rules,
    )
    .await
    .expect("set_reward_rules");

    // 1000 SLOTS LATER

    let slot = 1001_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_lender_pubkey)
        .await
        .expect("get position");
    let position_lender = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let rewards_lender = position_lender.rewards;

    assert_eq!(
        position_lender.collateral[0].deposited_amount,
        1_000_000_000_000
    ); // 1000 SOL
    assert_eq!(
        position_lender.collateral[1].deposited_amount,
        1_000_000_000
    ); // 1000 USDC

    // CHECK REWARD RECORDS NOT CHANGED

    for (index, mint) in reward_mints.iter().enumerate() {
        assert_eq!(rewards_lender.rewards[index].reward_mint, *mint);
        // Deposit is 1000 USDC, reward rate is 0.001, slots_elapsed = 1000. Thus rewards are 1000 tokens.
        // Some records are from
        assert_eq!(
            rewards_lender.rewards[index].accrued_amount().unwrap(),
            Decimal::from_lamports(1_000_000_000, 6).unwrap()
        );
    }
}
