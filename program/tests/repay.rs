#![cfg(feature = "test-bpf")]

use solana_program::instruction::AccountMeta;
use std::str::FromStr;

use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use super_lendy::instruction::{RefreshReserve, SuperLendyInstruction};
use super_lendy::{MAX_AMOUNT, SUPER_LENDY_ID};
use texture_common::account::PodAccount;
use texture_common::math::Decimal;
use tracing::info;

use super_lendy::pda::{find_liquidity_supply, find_lp_token_mint};
use super_lendy::state::position::Position;
use super_lendy::state::reserve::Reserve;

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    borrow, deposit_liquidity, lock_collateral, refresh_position, refresh_position_ix, repay,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, init_program_test, init_token_accounts,
    lender_keypair, texture_config_keypair, Runner, LAMPORTS, LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#2bd879e54cbb460db335228c001ed1b7
#[tokio::test]
async fn repay_success() {
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

    info!("================ Deposit SOL end =================");

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1000 * LAMPORTS_PER_USDC;

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

    info!("================ Deposit USDC end =================");

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    info!("================ Lock collateral start =================");
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

    // BORROW 1 SOL AFTER LOCK DEPOSITED COLLATERAL

    info!("================ Lock collateral end =================");

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
    let amount = LAMPORTS_PER_SOL;

    info!("================ Borrow start =================");

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
        amount,
        1,
    )
    .await
    .expect("borrow");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    info!(
        "borrowed_liquidity.borrowed_amount_wads() before refresh {}",
        borrowed_liquidity.borrowed_amount().unwrap()
    );

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    // 10 SLOT LATER

    let mut slot = 10_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    // REPAY

    info!("repay");
    repay(
        &mut ctx,
        position_pubkey,
        reserve_sol1_pubkey,
        &borrower_keypair,
        dest_borrower_liq_wallet_sol,
        MAX_AMOUNT,
    )
    .await
    .expect("repay");

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

    // CHECK NO DEBT
    assert_eq!(position.borrowed_value().unwrap(), Decimal::ZERO);

    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    assert_eq!(borrowed_liquidity.borrowed_amount().unwrap(), Decimal::ZERO);

    assert_eq!(reserve.liquidity.borrowed_amount().unwrap(), Decimal::ZERO);

    // 10 SLOT LATER

    slot += 10;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

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

    // CHECK NO DEBT
    assert_eq!(position.borrowed_value().unwrap(), Decimal::ZERO);

    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    assert_eq!(borrowed_liquidity.borrowed_amount().unwrap(), Decimal::ZERO);

    assert_eq!(reserve.liquidity.borrowed_amount().unwrap(), Decimal::ZERO);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#4504e22e742f4f078cdc0461d2ecb511
#[tokio::test]
async fn repay_incorrect_accounts() {
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

    // DEPOSIT INITIAL LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

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

    info!("================ Deposit SOL end =================");

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1000 * LAMPORTS_PER_USDC;

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

    info!("================ Deposit USDC end =================");

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    info!("================ Lock collateral start =================");
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

    // BORROW 1 SOL AFTER LOCK DEPOSITED COLLATERAL

    info!("================ Lock collateral end =================");

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
    let amount = LAMPORTS_PER_SOL;

    info!("================ Borrow start =================");

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
        amount,
        1,
    )
    .await
    .expect("borrow");

    let position_acc = get_account(&mut ctx.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    info!(
        "borrowed_liquidity.borrowed_amount_wads() before refresh {}",
        borrowed_liquidity.borrowed_amount().unwrap()
    );

    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    // REPAY source=reserve_liquidity_supply

    let reserve_liquidity_supply = find_liquidity_supply(&reserve_sol1_pubkey).0;

    info!("repay source=reserve_liquidity_supply");
    let result = repay(
        &mut ctx,
        position_pubkey,
        reserve_sol1_pubkey,
        &borrower_keypair,
        reserve_liquidity_supply,
        MAX_AMOUNT,
    )
    .await;

    assert!(result.is_err());

    // REPAY WITH INCORRECT reserve_liquidity_supply. TRANSFER TO USER WALLET

    info!("repay with incorrect reserve_liquidity_supply, transfer to user wallet");

    let accounts = vec![
        AccountMeta::new(position_pubkey, false),
        AccountMeta::new(dest_borrower_liq_wallet_sol, false),
        AccountMeta::new(source_lender_liq_wallet_sol, false),
        AccountMeta::new(borrower_pubkey, true),
        AccountMeta::new(reserve_sol1_pubkey, false),
        AccountMeta::new(spl_token::ID, false),
    ];

    let (mut ixs, _) = refresh_position_ix(&mut ctx, position_pubkey).await;

    ixs.push(
        RefreshReserve {
            reserve: reserve_sol1_pubkey,
            market_price_feed: sol_price_feed,
            irm,
        }
        .into_instruction(),
    );
    let ix = SuperLendyInstruction::Repay { amount: MAX_AMOUNT };
    ixs.push(solana_program::instruction::Instruction::new_with_borsh(
        SUPER_LENDY_ID,
        &ix,
        accounts,
    ));
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

    // REPAY WITH INCORRECT reserve_liquidity_supply. TRANSFER TO OTHER RESERVE

    let reserve_sol2_liquidity_supply = find_liquidity_supply(&reserve_sol2_pubkey).0;

    info!("repay with incorrect reserve_liquidity_supply, transfer to other reserve");

    let accounts = vec![
        AccountMeta::new(position_pubkey, false),
        AccountMeta::new(dest_borrower_liq_wallet_sol, false),
        AccountMeta::new(reserve_sol2_liquidity_supply, false),
        AccountMeta::new(borrower_pubkey, true),
        AccountMeta::new(reserve_sol1_pubkey, false),
        AccountMeta::new(spl_token::ID, false),
    ];

    let (mut ixs, _) = refresh_position_ix(&mut ctx, position_pubkey).await;

    ixs.push(
        RefreshReserve {
            reserve: reserve_sol1_pubkey,
            market_price_feed: sol_price_feed,
            irm,
        }
        .into_instruction(),
    );
    let ix = SuperLendyInstruction::Repay { amount: MAX_AMOUNT };
    ixs.push(solana_program::instruction::Instruction::new_with_borsh(
        SUPER_LENDY_ID,
        &ix,
        accounts,
    ));
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
