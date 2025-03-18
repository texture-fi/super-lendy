#![cfg(feature = "test-bpf")]

use std::str::FromStr;

use bytemuck::Zeroable;
use price_proxy::state::utils::str_to_array;
use solana_program::instruction::AccountMeta;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use texture_common::account::PodAccount;
use texture_common::math::{CheckedAdd, CheckedDiv, CheckedMul, Decimal};
use tracing::info;

use super_lendy::instruction::{RefreshReserve, SuperLendyInstruction};
use super_lendy::pda::{find_liquidity_supply, find_lp_token_mint, find_program_authority};
use super_lendy::state::pool::PoolParams;
use super_lendy::state::position::{Position, BORROW_MEMO_LEN};
use super_lendy::state::reserve::{
    FeeCalculation, Reserve, ReserveConfig, ReserveFeesConfig, RESERVE_TYPE_NORMAL,
};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfig, TextureConfigParams};
use super_lendy::state::SLOTS_PER_YEAR;
use super_lendy::{MAX_AMOUNT, SUPER_LENDY_ID, TEXTURE_CONFIG_ID};

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    alter_reserve, alter_texture_config, borrow, create_pool, create_position, create_reserve,
    deposit_liquidity, lock_collateral, refresh_position, refresh_position_ix,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, get_token_account, init_program_test,
    init_token_accounts, lender_keypair, texture_config_keypair, Runner, LAMPORTS,
    LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#21e8f679153e4cdc88b56d61dfa75c63
#[tokio::test]
async fn borrow_success() {
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

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1000 * LAMPORTS_PER_USDC;

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

    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    info!("lock {} collateral lp", deposit_usdc_amount);
    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        position_borrower_pubkey,
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // CHECK deposited_value = deposit_amount * lp_exchange_rate
    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(1001, 0).unwrap()
    );
    // CHECK allowed_borrow_value = deposit_amount * lp_exchange_rate * %max_borrow_ltv
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(9009, 1).unwrap()
    );

    // BORROW 1 SOL AFTER LOCK DEPOSITED COLLATERAL

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

    info!("borrow {} SOL after lock deposited collateral", amount);
    borrow(
        &mut ctx,
        position_borrower_pubkey,
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

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // 100 + 1%_curator_fee + 30%_texture_fee
        Decimal::from_i128_with_scale(131, 0).unwrap()
    );
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    assert_eq!(
        borrowed_liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(1310000000, 9).unwrap()
    );

    // CHECK remaining_borrow_value=0
    let remaining_borrow_value = position.remaining_borrow_value().unwrap();
    assert_eq!(
        remaining_borrow_value,
        Decimal::from_i128_with_scale(7699, 1).unwrap()
    );

    // TRY TO BORROW AMOUNT GREATER WHEN remaining_borrow_value

    info!("borrow amount greater when remaining_borrow_value");
    let result = borrow(
        &mut ctx,
        position_borrower_pubkey,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_sol,
        9 * LAMPORTS_PER_SOL,
        1,
    )
    .await;
    assert!(result.is_err());

    // BORROW AMOUNT GREATER WHEN allowed_borrow_value

    // try to borrow 1000 USD, when allowed_borrow_value ~= 900 - 100 USD
    let amount = 10 * LAMPORTS_PER_SOL;
    info!("borrow amount greater when allowed_borrow_value");
    let result = borrow(
        &mut ctx,
        position_borrower_pubkey,
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
    .await;
    assert!(result.is_err());

    // DEPOSIT 100 SOL INTO SOL2 RESERVE AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_borrower_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_sol =
        get_associated_token_address(&borrower_pubkey, &liquidity_sol_mint);
    let deposit_sol_amount = 10 * LAMPORTS_PER_SOL;

    info!("deposit {} into SOL2 reserve", deposit_sol_amount);
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

    info!("lock {} collateral lp", deposit_sol_amount);
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
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh position");

    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // CHECK deposited_value = (deposit_amount_usdc * lp_exchange_rate) + (deposit_amount_sol * lp_exchange_rate)
    assert_eq!(
        position.deposited_value().unwrap(),
        Decimal::from_i128_with_scale(1001, 0)
            .unwrap()
            .checked_add(Decimal::from_i128_with_scale(1000, 0).unwrap())
            .unwrap()
    );
    // CHECK allowed_borrow_value = 900 (at USDC, mul %max_borrow_ltv) + 500 (at SOL2, mul %max_borrow_ltv) = 1400
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(9009, 1)
            .unwrap()
            .checked_add(Decimal::from_i128_with_scale(500, 0).unwrap())
            .unwrap()
    );

    // BORROW MAX borrow_available AMOUNT

    let amount = MAX_AMOUNT;
    info!("borrow max borrow_available amount");
    borrow(
        &mut ctx,
        position_borrower_pubkey,
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

    info!("refresh position");
    refresh_position(&mut ctx, position_borrower_pubkey)
        .await
        .expect("refresh_position");

    let position_acc = get_account(&mut ctx.banks_client, position_borrower_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("cast position data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(borrowed, Decimal::from_i128_with_scale(14009, 1).unwrap());

    // CHECK remaining_borrow_value=0
    assert_eq!(position.remaining_borrow_value().unwrap(), Decimal::ZERO);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#13953e171798462b91761fca5d956d07
#[tokio::test]
async fn borrow_fees_success() {
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
        1_000 * LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1000 * LAMPORTS_PER_USDC;

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
        Decimal::from_i128_with_scale(1001, 0).unwrap()
    );
    // CHECK allowed_borrow_value = deposit_amount * lp_exchange_rate * %max_borrow_ltv
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(9009, 1).unwrap()
    );

    // BORROW 1 SOL AFTER LOCK DEPOSITED COLLATERAL

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

    let borrower_sol_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc0 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc0 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

    info!("borrow {} SOL after lock deposited collateral", amount);
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

    let borrower_sol_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc1 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc1 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

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
    let texture_config_acc = get_account(&mut ctx.banks_client, texture_config_keypair.pubkey())
        .await
        .expect("get position");
    let texture_config =
        TextureConfig::try_from_bytes(&texture_config_acc.data).expect("cast reserve data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // 100 + 1%_curator_fee + 30%_texture_fee
        Decimal::from_i128_with_scale(131, 0).unwrap()
    );
    // CHECK BorrowedLiquidity amount
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .expect("find_borrowed_liquidity")
        .0;
    let borrowed_amount_wads = borrowed_liquidity.borrowed_amount().unwrap();

    // User borrowed 1_000_000_000 for himself + 400_000_000 for fees. Solana time is not winded
    // between Borrow and RefreshPosition. Thus no interest accrued yet.
    assert_eq!(
        borrowed_amount_wads.round_to_decimals(9),
        Decimal::from_lamports(1_310_000_000, 9).unwrap()
    );

    // CHECK BORROWER BALANCE
    assert_eq!(
        borrower_sol_token_acc1.amount,
        borrower_sol_token_acc0.amount + amount
    );

    // CHECK FEE TRANSFERS
    let (curator_fee, texture_fee) = reserve
        .config
        .fees
        .calculate_borrow_fees(
            Decimal::from_lamports(amount, 9).unwrap(),
            9,
            texture_config.borrow_fee_rate_bps,
            FeeCalculation::Exclusive,
        )
        .expect("calculate borrow fees");
    assert_eq!(curator_fee, 10_000_000);
    assert_eq!(texture_fee, 300_000_000);

    assert_eq!(
        curator_fee_token_acc1.amount,
        curator_fee_token_acc0.amount + curator_fee
    );
    assert_eq!(
        texture_fee_token_acc1.amount,
        texture_fee_token_acc0.amount + texture_fee
    );

    // 1 SOLANA YEAR LATER

    info!("wrap to slot {}", SLOTS_PER_YEAR);
    ctx.warp_to_slot(SLOTS_PER_YEAR).expect("warp_to_slot"); // solana_year = 63072000 slots

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let slot_interest_rate = Decimal::from_i128_with_scale(4, 1) // borrow_rate = 40% fixed
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(SLOTS_PER_YEAR)
        .unwrap();

    let curator_performance_fee = reserve.liquidity.curator_performance_fee().unwrap();
    let texture_performance_fee = reserve.liquidity.texture_performance_fee().unwrap();

    // CHECK PERFORMANCE FEES
    assert_eq!(
        curator_performance_fee.round_to_decimals(9),
        Decimal::from_i128_with_scale(128858068, 9).unwrap()
    );
    assert_eq!(
        texture_performance_fee.round_to_decimals(9),
        Decimal::from_i128_with_scale(257716136, 9).unwrap()
    );

    let exp_borrowed_amount = Decimal::from_i128_with_scale(131, 2) // borrowed_amount = receive_amount + fees
        .unwrap()
        .checked_mul(compounded_interest_rate)
        .unwrap()
        .round_to_decimals(6);
    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    assert_eq!(borrowed_amount.round_to_decimals(6), exp_borrowed_amount);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#8597be2e8e8b4363aecaf2e5afc1d7ad
#[tokio::test]
async fn borrow_lp_exchange_rate_success() {
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

    // ALTER reserve.borrow_fee & reserve.performance_fee to zero

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
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

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1000 * LAMPORTS_PER_USDC;

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
        Decimal::from_i128_with_scale(1001, 0).unwrap()
    );
    // CHECK allowed_borrow_value = deposit_amount * lp_exchange_rate * %max_borrow_ltv
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(9009, 1).unwrap()
    );

    // BORROW 1 SOL AFTER LOCK DEPOSITED COLLATERAL

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

    let borrower_sol_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc0 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc0 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

    info!("borrow {} SOL after lock deposited collateral", amount);
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

    let borrower_sol_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc1 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc1 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

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
    let texture_config_acc = get_account(&mut ctx.banks_client, texture_config_keypair.pubkey())
        .await
        .expect("get position");
    let texture_config =
        TextureConfig::try_from_bytes(&texture_config_acc.data).expect("cast reserve data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // 100 + 0%_curator_fee + 0%_texture_fee
        Decimal::from_i128_with_scale(100, 0).unwrap()
    );
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .unwrap()
        .0;
    assert_eq!(
        borrowed_liquidity.borrowed_amount().unwrap(),
        Decimal::from_lamports(1000000000, 9).unwrap()
    );

    // CHECK BORROWER BALANCE
    assert_eq!(
        borrower_sol_token_acc1.amount,
        borrower_sol_token_acc0.amount + amount
    );

    // CHECK FEE TRANSFERS
    let (curator_fee, texture_fee) = reserve
        .config
        .fees
        .calculate_borrow_fees(
            Decimal::from_i128_with_scale(amount as i128, 0).unwrap(),
            9,
            texture_config.borrow_fee_rate_bps,
            FeeCalculation::Exclusive,
        )
        .expect("calculate borrow fees");
    assert_eq!(curator_fee, 0);
    assert_eq!(texture_fee, 0);

    assert_eq!(curator_fee_token_acc1.amount, curator_fee_token_acc0.amount);
    assert_eq!(texture_fee_token_acc1.amount, texture_fee_token_acc0.amount);

    // 1 SLOT LATER

    let slot = 2_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let slot_interest_rate = Decimal::from_i128_with_scale(4, 1) // borrow_rate = 40% fixed
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(1)
        .unwrap();

    let curator_performance_fee = reserve.liquidity.curator_performance_fee().unwrap();
    let texture_performance_fee = reserve.liquidity.texture_performance_fee().unwrap();

    // CHECK performance_fees=0
    assert_eq!(curator_performance_fee, Decimal::ZERO);
    assert_eq!(texture_performance_fee, Decimal::ZERO);

    let exp_borrowed_amount = Decimal::ONE
        .checked_mul(compounded_interest_rate)
        .unwrap()
        .round_to_decimals(9);
    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    assert_eq!(borrowed_amount.round_to_decimals(9), exp_borrowed_amount);

    let lp_exchange_rate0 = reserve.lp_exchange_rate().unwrap();
    let lp_total_supply = Decimal::from_lamports(1_000 * LAMPORTS_PER_SOL, 9).unwrap(); // received when lender deposited 1_000 SOL
    let total_liquidity = Decimal::from_lamports(999 * LAMPORTS_PER_SOL, 9) // cause borrowed 1 SOL
        .unwrap()
        .checked_add(borrowed_amount) // debt
        .unwrap();

    // CHECK LP EXCHANGE RATE
    assert_eq!(
        lp_exchange_rate0.0,
        lp_total_supply.checked_div(total_liquidity).unwrap()
    );

    // 1 SOLANA YEAR LATER

    info!("wrap to slot {}", SLOTS_PER_YEAR);
    ctx.warp_to_slot(SLOTS_PER_YEAR + slot)
        .expect("warp_to_slot"); // solana_year = 63072000 slots

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let slot_interest_rate = Decimal::from_i128_with_scale(4, 1) // borrow_rate = 40% fixed
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(SLOTS_PER_YEAR)
        .unwrap();
    let exp_borrowed_amount = borrowed_amount // debt of past period
        .checked_mul(compounded_interest_rate)
        .unwrap();

    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    assert_eq!(
        borrowed_amount.round_to_decimals(9),
        exp_borrowed_amount.round_to_decimals(9)
    );

    let lp_exchange_rate1 = reserve.lp_exchange_rate().unwrap();
    let lp_total_supply = Decimal::from_lamports(1_000 * LAMPORTS_PER_SOL, 9).unwrap(); // received when lender deposited 1_000 SOL
    let total_liquidity = Decimal::from_lamports(999 * LAMPORTS_PER_SOL, 9) // cause borrowed 1 SOL
        .unwrap()
        .checked_add(borrowed_amount) // debt
        .unwrap();

    // CHECK LP EXCHANGE RATE
    assert_eq!(
        lp_exchange_rate1.0,
        lp_total_supply.checked_div(total_liquidity).unwrap()
    );

    // CHECK lp_exchange_rate0 > lp_exchange_rate1. LP increased
    assert!(lp_exchange_rate0.0 > lp_exchange_rate1.0)
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#ab20e873239944aba4e447c7be70fb6c
#[tokio::test]
async fn borrow_check_interest_success() {
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

    // ALTER reserve.borrow_fee & reserve.performance_fee to zero

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
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

    // 1 SOLANA YEAR LATER

    info!("wrap to slot {}", SLOTS_PER_YEAR);
    ctx.warp_to_slot(SLOTS_PER_YEAR).expect("warp_to_slot"); // solana_year = 63072000 slots

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

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

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 1000 * LAMPORTS_PER_USDC;

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
        .expect("refresh_position");

    // BORROW 1 SOL AFTER LOCK DEPOSITED COLLATERAL

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

    info!("borrow {} SOL after lock deposited collateral", amount);
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
        .expect("get reserve");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // 100 + 0%_curator_fee + 0%_texture_fee
        Decimal::from_i128_with_scale(100, 0).unwrap()
    );

    // CHECK BorrowedLiquidity amount
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .expect("find_borrowed_liquidity")
        .0;
    let position_borrowed_amount_wads = borrowed_liquidity.borrowed_amount().unwrap();

    // User borrowed 1_000_000_000 for himself. Solana time is not winded
    // between Borrow and RefreshPosition. Thus no interest accrued yet.
    assert_eq!(
        position_borrowed_amount_wads,
        Decimal::from_lamports(1_000_000_000, 9).unwrap()
    );

    // CHECK position_borrowed_amount == reserve_borrowed_amount
    let reserve_borrowed_amount_wads = reserve.liquidity.borrowed_amount().unwrap();
    assert_eq!(position_borrowed_amount_wads, reserve_borrowed_amount_wads);

    // 1 SOLANA YEAR LATER

    info!("wrap to slot {}", SLOTS_PER_YEAR * 2);
    ctx.warp_to_slot(SLOTS_PER_YEAR * 2).expect("warp_to_slot"); // solana_year = 63072000 slots

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let slot_interest_rate = Decimal::from_i128_with_scale(4, 1) // borrow_rate = 40% fixed
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(SLOTS_PER_YEAR)
        .unwrap();
    let exp_borrowed_amount = Decimal::ONE
        .checked_mul(compounded_interest_rate)
        .unwrap()
        .round_to_decimals(9);

    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    info!("repayment = borrow + interest = {}", borrowed_amount);
    assert_eq!(borrowed_amount.round_to_decimals(9), exp_borrowed_amount);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#fe0a4a8c29d54e939ad388c8bf8b856d
#[tokio::test]
async fn borrow_incorrect_accounts() {
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
    let pool1_keypair = Keypair::new();
    let pool1_pubkey = pool1_keypair.pubkey();

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
        &pool1_keypair,
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

    // BORROW BY USER NOT ASSOCIATED WITH POSITION

    let dest_borrower_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);
    let texture_fee_receiver =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_sol_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_sol_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("borrow by user not associated wit position");
    let result = borrow(
        &mut ctx,
        position_pubkey,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        pool1_pubkey,
        &lender_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_sol,
        LAMPORTS_PER_SOL,
        1,
    )
    .await;

    assert!(result.is_err());

    // CREATE POOL2

    let pool2_keypair = Keypair::new();
    let reserve_sol2_keypair = Keypair::new();
    let reserve_sol2_pubkey = reserve_sol2_keypair.pubkey();

    let pool_params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        &mut ctx,
        &pool2_keypair,
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        pool_params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE SOL2

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 100,
        curator_performance_fee_rate_bps: 2000,
        _padding: Zeroable::zeroed(),
    };
    let reserve_config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 6500,
        fully_unhealthy_ltv_bps: 7000,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 10_000 * LAMPORTS_PER_SOL,
        max_borrow_utilization_bps: 1000,
        price_stale_threshold_sec: 1,
        max_withdraw_utilization_bps: 9000,
    };

    info!("create reserve sol borrow enabled");
    create_reserve(
        &mut ctx,
        &reserve_sol2_keypair,
        pool2_keypair.pubkey(),
        &pool_authority_keypair,
        curator_keypair.pubkey(),
        liquidity_sol_mint,
        sol_price_feed,
        reserve_config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve_sol1");

    let dest_borrower_liq_wallet_sol =
        get_associated_token_address(&borrower_pubkey, &liquidity_sol_mint);

    // BORROW WITH INCORRECT reserve_liquidity_supply

    info!("borrow with incorrect reserve_liquidity_supply");

    // choose reserve_liquidity_supply from other reserve
    let reserve_liquidity_supply = find_liquidity_supply(&reserve_sol2_pubkey).0;
    let program_authority = find_program_authority().0;
    let accounts = vec![
        AccountMeta::new(position_pubkey, false),
        AccountMeta::new(reserve_liquidity_supply, false),
        AccountMeta::new(dest_borrower_liq_wallet_sol, false),
        AccountMeta::new(curator_fee_receiver, false),
        AccountMeta::new(borrower_pubkey, true),
        AccountMeta::new(reserve_sol1_pubkey, false),
        AccountMeta::new(pool1_pubkey, false),
        AccountMeta::new(curator_pubkey, false),
        AccountMeta::new(texture_fee_receiver, false),
        AccountMeta::new(TEXTURE_CONFIG_ID, false),
        AccountMeta::new(program_authority, false),
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
    let ix = SuperLendyInstruction::Borrow {
        amount: LAMPORTS_PER_SOL,
        slippage_limit: 1,
        memo: [0; BORROW_MEMO_LEN],
    };
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

    // BORROW WITH INCORRECT POSITION

    let position_pool2_kp = Keypair::new();
    create_position(
        &mut ctx,
        &position_pool2_kp,
        pool2_keypair.pubkey(),
        &borrower_keypair,
    )
    .await
    .expect("create_position");

    info!("borrow with incorrect position");

    let reserve_liquidity_supply = find_liquidity_supply(&reserve_sol1_pubkey).0;
    let program_authority = find_program_authority().0;
    let accounts = vec![
        AccountMeta::new(position_pool2_kp.pubkey(), false),
        AccountMeta::new(reserve_liquidity_supply, false),
        AccountMeta::new(dest_borrower_liq_wallet_sol, false),
        AccountMeta::new(curator_fee_receiver, false),
        AccountMeta::new(borrower_pubkey, true),
        AccountMeta::new(reserve_sol1_pubkey, false),
        AccountMeta::new(pool1_pubkey, false),
        AccountMeta::new(curator_pubkey, false),
        AccountMeta::new(texture_fee_receiver, false),
        AccountMeta::new(TEXTURE_CONFIG_ID, false),
        AccountMeta::new(program_authority, false),
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
    let ix = SuperLendyInstruction::Borrow {
        amount: LAMPORTS_PER_SOL,
        slippage_limit: 1,
        memo: [0; BORROW_MEMO_LEN],
    };
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

    // BORROW WITH INCORRECT destination_liquidity_wallet

    info!("borrow with incorrect destination_liquidity_wallet");
    let result = borrow(
        &mut ctx,
        position_pubkey,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        pool1_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        reserve_liquidity_supply,
        LAMPORTS_PER_SOL,
        1,
    )
    .await;

    assert!(result.is_err());

    // BORROW WITH reserve_liquidity_supply = dest_borrower_liq_wallet_sol

    info!("borrow with reserve_liquidity_supply = dest_borrower_liq_wallet_sol");
    let program_authority = find_program_authority().0;
    let accounts = vec![
        AccountMeta::new(position_pubkey, false),
        AccountMeta::new(dest_borrower_liq_wallet_sol, false),
        AccountMeta::new(dest_borrower_liq_wallet_sol, false),
        AccountMeta::new(curator_fee_receiver, false),
        AccountMeta::new(borrower_pubkey, true),
        AccountMeta::new(reserve_sol1_pubkey, false),
        AccountMeta::new(pool1_pubkey, false),
        AccountMeta::new(curator_pubkey, false),
        AccountMeta::new(texture_fee_receiver, false),
        AccountMeta::new(TEXTURE_CONFIG_ID, false),
        AccountMeta::new(program_authority, false),
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
    let ix = SuperLendyInstruction::Borrow {
        amount: LAMPORTS_PER_SOL,
        slippage_limit: 1,
        memo: [0; BORROW_MEMO_LEN],
    };
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

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#a4d002d950ac42f580f50c0cae89ae5f
#[tokio::test]
async fn borrow_limits() {
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

    // ALTER reserve_sol1.max_total_liquidity & reserve_usdc.max_utilization. Set fees to zero

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
    let max_total_liquidity = 100000;
    params.max_total_liquidity = max_total_liquidity;
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

    // DEPOSIT LIQUIDITY GREATER WHEN THRESHOLD

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit liquidity greater when threshold");
    let result = deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        max_total_liquidity + 1,
    )
    .await;

    assert!(result.is_err());

    // DEPOSIT MAX TOTAL LIQUIDITY

    info!("deposit max_total_liquidity");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        max_total_liquidity,
    )
    .await
    .expect("deposit_liquidity");

    // DEPOSIT 1 LIQUIDITY

    info!("deposit 1 liquidity");
    let result = deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        1,
    )
    .await;

    assert!(result.is_err());

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_amount = 1_000 * LAMPORTS_PER_USDC;

    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_usdc,
        dest_borrower_lp_wallet_usdc,
        deposit_amount,
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
        deposit_amount,
    )
    .await
    .expect("lock_collateral");

    // BORROW 490 USDC TO SET UTILIZATION TO 0.49%

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

    info!("borrow 490 usdc to set utilization to 49%");
    borrow(
        &mut ctx,
        position_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        490 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // BORROW 510 USDC TO INCREASE max_utilization_bps

    info!("borrow 510 usdc to increase max_utilization_bps");
    let result = borrow(
        &mut ctx,
        position_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        20 * LAMPORTS_PER_USDC,
        1,
    )
    .await;

    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#5f16ce76d02a4d3f8e0188337fa171ee
#[tokio::test]
async fn borrow_reserve_type() {
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
    let reserve_sol1_pubkey = reserve_sol1_keypair.pubkey(); // RESERVE_TYPE_BORROW_ONLY
    let reserve_sol2_keypair = Keypair::new();
    let reserve_sol2_pubkey = reserve_sol2_keypair.pubkey(); // RESERVE_TYPE_BORROW_DISABLED
    let reserve_usdc_keypair = Keypair::new();
    let reserve_usdc_pubkey = reserve_usdc_keypair.pubkey(); // RESERVE_TYPE_NORMAL

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

    // ALTER reserve.borrow_fee & reserve.performance_fee to zero

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
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

    // DEPOSIT INITIAL LIQUIDITY TO SOL2 RESERVE & LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_sol2_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    deposit_liquidity(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        LAMPORTS_PER_SOL,
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

    refresh_position(&mut ctx, lender_position_kp.pubkey())
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        lender_position_kp.pubkey(),
        &lender_keypair,
        dest_lender_lp_wallet_sol,
        LAMPORTS_PER_SOL,
    )
    .await
    .expect("lock_collateral");

    // DEPOSIT USDC & LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 100_000_000;

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

    refresh_position(&mut ctx, borrower_position_keypair.pubkey())
        .await
        .expect("refresh position");

    lock_collateral(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        borrower_position_keypair.pubkey(),
        &borrower_keypair,
        dest_borrower_lp_wallet_usdc,
        deposit_usdc_amount,
    )
    .await
    .expect("lock_collateral");

    // TRY TO BORROW FROM SOL2 RESERVE WITH BORROW_DISABLE MODE

    let dest_borrower_liq_wallet_sol =
        get_associated_token_address(&borrower_pubkey, &liquidity_sol_mint);
    let texture_fee_receiver_sol =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_sol_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver_sol =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_sol_mint)
            .await
            .expect("create curator fee receiver ata");

    info!("try to borrow from sol2 reserve");
    let result = borrow(
        &mut ctx,
        position_borrower_pubkey,
        reserve_sol2_pubkey,
        sol_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver_sol,
        texture_fee_receiver_sol,
        dest_borrower_liq_wallet_sol,
        LAMPORTS_PER_SOL,
        1,
    )
    .await;

    assert!(result.is_err());

    // BORROW FROM USDC RESERVE WITH LOCKING COLLATERAL FROM SOL2 RESERVE

    let dest_lender_liq_wallet_usdc =
        get_associated_token_address(&lender_pubkey, &liquidity_usdc_mint);
    let texture_fee_receiver_usdc =
        create_associated_token_account(&mut ctx, &texture_owner_keypair, &liquidity_usdc_mint)
            .await
            .expect("create texture fee receiver ata");
    let curator_fee_receiver_usdc =
        create_associated_token_account(&mut ctx, &pool_authority_keypair, &liquidity_usdc_mint)
            .await
            .expect("create curator fee receiver ata");

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh_position");

    info!("borrow from usdc reserve with locking collateral from sol2 reserve");
    borrow(
        &mut ctx,
        position_lender_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &lender_keypair,
        curator_pubkey,
        curator_fee_receiver_usdc,
        texture_fee_receiver_usdc,
        dest_lender_liq_wallet_usdc,
        10_000_000,
        1,
    )
    .await
    .expect("borrow");

    // DEPOSIT LIQUIDITY TO SOL1 RESERVE

    let lp_mint = find_lp_token_mint(&reserve_sol1_pubkey).0;
    let dest_lender_lp_wallet_sol =
        create_associated_token_account(&mut ctx, &lender_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_lender_liq_wallet_sol =
        get_associated_token_address(&lender_pubkey, &liquidity_sol_mint);

    info!("deposit liquidity to sol1 reserve");
    deposit_liquidity(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        &lender_keypair,
        source_lender_liq_wallet_sol,
        dest_lender_lp_wallet_sol,
        LAMPORTS_PER_SOL,
    )
    .await
    .expect("deposit_liquidity");

    // TRY TO LOCK COLLATERAL IN SOL1 RESERVE WITH BORROW_ONLY MODE

    refresh_position(&mut ctx, position_lender_pubkey)
        .await
        .expect("refresh position");

    info!("try to lock collateral in sol1 reserve with BORROW_ONLY mode");
    let result = lock_collateral(
        &mut ctx,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        position_lender_pubkey,
        &lender_keypair,
        dest_lender_lp_wallet_sol,
        LAMPORTS_PER_SOL,
    )
    .await;

    assert!(result.is_err());

    // BORROW FROM SOL1 RESERVE

    info!("borrow from sol1 reserve");
    borrow(
        &mut ctx,
        position_borrower_pubkey,
        reserve_sol1_pubkey,
        sol_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver_sol,
        texture_fee_receiver_sol,
        dest_borrower_liq_wallet_sol,
        100_000_000,
        1,
    )
    .await
    .expect("borrow");
}

#[tokio::test]
async fn borrow_irm() {
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
    add_price_feed_acc(&mut runner, "sol-usd").await;
    // 1 USDC = 1.001 USD
    let usdc_price_feed = add_price_feed_acc(&mut runner, "usdc-usd").await;

    let irm = add_curve_acc(&mut runner, "curve1-acc").await;

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

    // Set fees to zero

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let mut params = reserve.config;
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

    // DEPOSIT 1000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_amount = 1_000 * LAMPORTS_PER_USDC;

    deposit_liquidity(
        &mut ctx,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        &borrower_keypair,
        source_borrower_liq_wallet_usdc,
        dest_borrower_lp_wallet_usdc,
        deposit_amount,
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
        deposit_amount,
    )
    .await
    .expect("lock_collateral");

    // BORROW 10 USDC TO SET UTILIZATION TO 1%

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

    info!("borrow 10 usdc to set utilization to 1%");
    borrow(
        &mut ctx,
        position_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        10 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // 1 SLOT LATER

    let mut slot = 2_u64;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();
    assert_eq!(
        reserve.liquidity.borrow_rate().unwrap(),
        Decimal::from_i128_with_scale(21, 2).unwrap()
    );

    // BORROW MORE 90 USDC TO SET UTILIZATION TO 10%

    info!("borrow more 90 usdc to set utilization to 10%");
    borrow(
        &mut ctx,
        position_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        90 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // 1 SLOT LATER

    slot += 1;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(
        reserve
            .liquidity
            .borrow_rate()
            .unwrap()
            .round_to_decimals(4),
        Decimal::from_i128_with_scale(3000, 4).unwrap()
    );

    // CHECK INTEREST ACCRUED 1 SLOT

    let slot_interest_rate = Decimal::from_i128_with_scale(3, 1) // borrow_rate = 30%
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(1)
        .unwrap();
    let exp_borrowed_amount = Decimal::from_i128_with_scale(90, 0) // borrowed_amount * interest_rate
        .unwrap()
        .checked_add(borrowed_amount)
        .unwrap()
        .checked_mul(compounded_interest_rate)
        .unwrap()
        .round_to_decimals(9);

    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();
    assert_eq!(borrowed_amount.round_to_decimals(9), exp_borrowed_amount);

    // BORROW MORE 150 USDC TO SET UTILIZATION TO 25%

    info!("borrow more 150 usdc to set utilization to 25%");
    borrow(
        &mut ctx,
        position_pubkey,
        reserve_usdc_pubkey,
        usdc_price_feed,
        irm,
        pool_pubkey,
        &borrower_keypair,
        curator_pubkey,
        curator_fee_receiver,
        texture_fee_receiver,
        dest_borrower_liq_wallet_usdc,
        150 * LAMPORTS_PER_USDC,
        1,
    )
    .await
    .expect("borrow");

    // 1 SLOT LATER

    slot += 1;
    info!("wrap to slot {}", slot);
    ctx.warp_to_slot(slot).expect("warp_to_slot");

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_usdc_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    assert_eq!(
        reserve
            .liquidity
            .borrow_rate()
            .unwrap()
            .round_to_decimals(4),
        Decimal::from_i128_with_scale(4000, 4).unwrap()
    );

    // CHECK INTEREST ACCRUED 1 SLOT

    let slot_interest_rate = Decimal::from_i128_with_scale(4, 1) // borrow_rate = 40%
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(1)
        .unwrap();
    let exp_borrowed_amount = Decimal::from_i128_with_scale(150, 0) // borrowed_amount * interest_rate
        .unwrap()
        .checked_add(borrowed_amount)
        .unwrap()
        .checked_mul(compounded_interest_rate)
        .unwrap()
        .round_to_decimals(8);

    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();
    assert_eq!(borrowed_amount.round_to_decimals(8), exp_borrowed_amount);
}

// Reserve can only give 5 SOL (out of 10) because of max_borrow_utilization_bps = 50%
// While deposited collateral allows greater borrow amount.
#[tokio::test]
async fn borrow_max_amount_limited_by_reserve() {
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

    // DEPOSIT 2000 USDC AND LOCK COLLATERAL

    let lp_mint = find_lp_token_mint(&reserve_usdc_pubkey).0;
    let dest_borrower_lp_wallet_usdc =
        create_associated_token_account(&mut ctx, &borrower_keypair, &lp_mint)
            .await
            .expect("create lp ata");
    let source_borrower_liq_wallet_usdc =
        get_associated_token_address(&borrower_pubkey, &liquidity_usdc_mint);
    let deposit_usdc_amount = 2_000 * LAMPORTS_PER_USDC;

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
        Decimal::from_i128_with_scale(2002, 0).unwrap()
    );
    // CHECK allowed_borrow_value = deposit_amount * lp_exchange_rate * %max_borrow_ltv
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(18018, 1).unwrap()
    );

    // BORROW maximum SOLs contract can give

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

    let borrower_sol_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc0 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc0 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

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

    let amount = LAMPORTS_PER_SOL;

    let borrower_sol_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc1 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc1 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

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
    let texture_config_acc = get_account(&mut ctx.banks_client, texture_config_keypair.pubkey())
        .await
        .expect("get position");
    let texture_config =
        TextureConfig::try_from_bytes(&texture_config_acc.data).expect("cast reserve data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // 500 - including 1%_curator_fee + 30%_texture_fee.
        Decimal::from_i128_with_scale(500, 0).unwrap()
    );
    // CHECK BorrowedLiquidity amount
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .expect("find_borrowed_liquidity")
        .0;
    let borrowed_amount = borrowed_liquidity.borrowed_amount().unwrap();

    // User borrowed 5 SOLs - 3.81 for himself and rest to pay Texture and Curator fees. Solana time is not winded
    // between Borrow and RefreshPosition. Thus no interest accrued yet.
    assert_eq!(
        borrowed_amount,
        Decimal::from_lamports(5_000_000_000, 9).unwrap()
    );

    // CHECK BORROWER BALANCE
    assert_eq!(
        borrower_sol_token_acc1.amount,
        borrower_sol_token_acc0.amount + 3816793893
    );

    // CHECK FEE TRANSFERS
    let (curator_fee, texture_fee) = reserve
        .config
        .fees
        .calculate_borrow_fees(
            Decimal::from_lamports(amount, 9).unwrap(),
            9,
            texture_config.borrow_fee_rate_bps,
            FeeCalculation::Exclusive,
        )
        .expect("calculate borrow fees");
    assert_eq!(curator_fee, 10_000_000);
    assert_eq!(texture_fee, 300_000_000);

    // Curator borrow fee is 1% from borrowed amount i.e. from 3816793893
    assert_eq!(
        curator_fee_token_acc1.amount,
        curator_fee_token_acc0.amount + 38167939
    );
    // Texture borrow fee is 30% i.e. 3816793893 * 0.3 = 1145038168
    assert_eq!(
        texture_fee_token_acc1.amount,
        texture_fee_token_acc0.amount + 1145038168
    );

    // 1 SOLANA YEAR LATER

    info!("wrap to slot {}", SLOTS_PER_YEAR);
    ctx.warp_to_slot(SLOTS_PER_YEAR).expect("warp_to_slot"); // solana_year = 63072000 slots

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");

    let slot_interest_rate = Decimal::from_i128_with_scale(4, 1) // borrow_rate = 40% fixed
        .unwrap()
        .checked_div(Decimal::from_i128_with_scale(SLOTS_PER_YEAR as i128, 0).unwrap())
        .unwrap();
    let compounded_interest_rate = slot_interest_rate
        .checked_add(Decimal::ONE)
        .unwrap()
        .checked_pow(SLOTS_PER_YEAR)
        .unwrap();

    let curator_performance_fee = reserve
        .liquidity
        .curator_performance_fee()
        .unwrap()
        .round_to_decimals(9);
    let texture_performance_fee = reserve
        .liquidity
        .texture_performance_fee()
        .unwrap()
        .round_to_decimals(9);

    // CHECK PERFORMANCE FEES
    assert_eq!(
        curator_performance_fee,
        Decimal::from_i128_with_scale(491824686, 9).unwrap()
    );
    assert_eq!(
        texture_performance_fee,
        Decimal::from_i128_with_scale(983649373, 9).unwrap()
    );

    let exp_borrowed_amount = Decimal::from_i128_with_scale(500, 2) // borrowed_amount = receive_amount + fees
        .unwrap()
        .checked_mul(compounded_interest_rate)
        .unwrap()
        .round_to_decimals(6);
    let borrowed_amount = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    assert_eq!(borrowed_amount.round_to_decimals(6), exp_borrowed_amount);
}

// This test operates on big amounts to test overflow conditions as well as u64:max as special value
// for Borrow IX.
// Reserve can give 5_000_000 SOL (out of 10_000_000) because of max_borrow_utilization_bps = 50%
// While deposited collateral allows to borrow just 1_000_000 SOL.
#[tokio::test]
async fn borrow_max_amount_limited_by_position() {
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
        1_000_000 * LAMPORTS_PER_SOL, // 1M SOLs ! worth 100M $
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
    let deposit_usdc_amount = 10_000_000 * LAMPORTS_PER_USDC; // 10M $

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
        Decimal::from_i128_with_scale(100100000, 1).unwrap()
    );
    // CHECK allowed_borrow_value = deposit_amount * lp_exchange_rate * %max_borrow_ltv
    assert_eq!(
        position.allowed_borrow_value().unwrap(),
        Decimal::from_i128_with_scale(900900000, 2).unwrap()
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

    let borrower_sol_token_acc0 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc0 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc0 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

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

    let borrower_sol_token_acc1 =
        get_token_account(&mut ctx.banks_client, dest_borrower_liq_wallet_sol)
            .await
            .expect("get token acc");
    let curator_fee_token_acc1 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc1 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

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
    let texture_config_acc = get_account(&mut ctx.banks_client, texture_config_keypair.pubkey())
        .await
        .expect("get position");
    let texture_config =
        TextureConfig::try_from_bytes(&texture_config_acc.data).expect("cast reserve data");

    // CHECK POSITION BORROWED VALUE
    let borrowed = position.borrowed_value().unwrap();
    assert_eq!(
        borrowed,
        // Position's allowed borrow value is 9_009_000 USD. We asked max borrow and it is also 9_009_000 in value!
        Decimal::from_i128_with_scale(900900000, 2).unwrap()
    );
    // CHECK BorrowedLiquidity amount
    let borrowed_liquidity = position
        .find_borrowed_liquidity(reserve_sol1_pubkey)
        .expect("find_borrowed_liquidity")
        .0;
    let borrowed_amount = borrowed_liquidity.borrowed_amount().unwrap();

    assert_eq!(
        borrowed_amount,
        Decimal::from_lamports(90_090 * LAMPORTS_PER_SOL, 9).unwrap()
    );

    // CHECK BORROWER BALANCE
    assert_eq!(
        borrower_sol_token_acc1.amount,
        borrower_sol_token_acc0.amount + 68_770_992_366_412 // 68_770 SOL
    );

    // CHECK FEE TRANSFERS
    let (curator_fee, texture_fee) = reserve
        .config
        .fees
        .calculate_borrow_fees(
            Decimal::from_lamports(68_770_992_366_412, 9).unwrap(),
            9,
            texture_config.borrow_fee_rate_bps,
            FeeCalculation::Exclusive,
        )
        .expect("calculate borrow fees");
    assert_eq!(curator_fee, 687_709_923_664);
    assert_eq!(texture_fee, 20_631_297_709_924);

    assert_eq!(
        curator_fee_token_acc1.amount,
        curator_fee_token_acc0.amount + curator_fee
    );
    assert_eq!(
        texture_fee_token_acc1.amount,
        texture_fee_token_acc0.amount + texture_fee
    );
}
