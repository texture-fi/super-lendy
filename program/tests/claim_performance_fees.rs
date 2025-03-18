#![cfg(feature = "test-bpf")]

use std::str::FromStr;

use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use texture_common::account::PodAccount;
use texture_common::math::{CheckedAdd, CheckedDiv, CheckedMul, Decimal};
use tracing::info;

use super_lendy::pda::find_lp_token_mint;
use super_lendy::state::position::Position;
use super_lendy::state::reserve::{FeeCalculation, Reserve};
use super_lendy::state::texture_cfg::TextureConfig;
use super_lendy::state::SLOTS_PER_YEAR;

use crate::utils::setup_super_lendy::setup_lendy_env;
use crate::utils::superlendy_executor::{
    borrow, claim_curator_performance_fees, claim_texture_performance_fees, deposit_liquidity,
    lock_collateral, refresh_position,
};
use crate::utils::{
    add_curve_acc, add_price_feed_acc, admin_keypair, borrow_keypair,
    create_associated_token_account, get_account, get_token_account, init_program_test,
    init_token_accounts, lender_keypair, texture_config_keypair, Runner, LAMPORTS,
    LAMPORTS_PER_USDC,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#bba77513db424d9b9afa5320846f510e
#[tokio::test]
async fn claim_success() {
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
        borrowed_amount_wads,
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
    let lp_exchange_rate0 = reserve.lp_exchange_rate().unwrap();

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
    let borrowed_amount0 = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    assert_eq!(borrowed_amount0.round_to_decimals(6), exp_borrowed_amount);

    // CLAIM PERFORMANCE FEES

    let curator_fee_token_acc0 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc0 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");
    let total_liquidity0 = reserve.liquidity.total_liquidity().unwrap();

    info!("claim curator performance fees");
    claim_curator_performance_fees(
        &mut ctx,
        curator_pubkey,
        reserve_sol1_pubkey,
        pool_pubkey,
        curator_fee_receiver,
    )
    .await
    .expect("claim_curator_performance_fees");

    info!("claim texture performance fees");
    claim_texture_performance_fees(&mut ctx, reserve_sol1_pubkey, texture_fee_receiver)
        .await
        .expect("claim_texture_performance_fees");

    let curator_fee_token_acc1 = get_token_account(&mut ctx.banks_client, curator_fee_receiver)
        .await
        .expect("get token acc");
    let texture_fee_token_acc1 = get_token_account(&mut ctx.banks_client, texture_fee_receiver)
        .await
        .expect("get token acc");

    // CHECK TRANSFER FEES
    assert_eq!(
        curator_fee_token_acc1.amount,
        curator_fee_token_acc0.amount
            + reserve
                .liquidity
                .curator_performance_fee()
                .unwrap()
                .to_lamports_floor(9)
                .unwrap()
    );
    assert_eq!(
        texture_fee_token_acc1.amount,
        texture_fee_token_acc0.amount
            + reserve
                .liquidity
                .texture_performance_fee()
                .unwrap()
                .to_lamports_floor(9)
                .unwrap()
    );

    info!("refresh position");
    refresh_position(&mut ctx, position_pubkey)
        .await
        .expect("refresh_position");

    let reserve_acc = get_account(&mut ctx.banks_client, reserve_sol1_pubkey)
        .await
        .expect("get position");
    let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
    let lp_exchange_rate1 = reserve.lp_exchange_rate().unwrap();
    let borrowed_amount1 = reserve.liquidity.borrowed_amount().unwrap();
    let total_liquidity1 = reserve.liquidity.total_liquidity().unwrap();

    // CHECK PERFORMANCE FEES RESET TO ZERO
    assert_eq!(
        reserve
            .liquidity
            .curator_performance_fee()
            .unwrap()
            .floor()
            .unwrap(),
        0
    );
    assert_eq!(
        reserve
            .liquidity
            .texture_performance_fee()
            .unwrap()
            .floor()
            .unwrap(),
        0
    );

    // CHECK borrowed_amount & lp_exchange_rate NOT CHANGED
    assert_eq!(borrowed_amount1, borrowed_amount0);
    assert_eq!(lp_exchange_rate1.0, lp_exchange_rate0.0);
    assert_eq!(total_liquidity1, total_liquidity0);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#bba77513db424d9b9afa5320846f510e
#[tokio::test]
async fn claim_incorrect_fee_receiver() {
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
        borrowed_amount_wads,
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
    let borrowed_amount0 = reserve.liquidity.borrowed_amount().unwrap();

    // CHECK REPAYMENT TO RESERVE
    assert_eq!(borrowed_amount0.round_to_decimals(6), exp_borrowed_amount);

    // TRY TO CLAIM WITH INCORRECT FEE RECEIVER

    info!("try to claim with incorrect fee receiver");
    let result = claim_curator_performance_fees(
        &mut ctx,
        curator_pubkey,
        reserve_sol1_pubkey,
        pool_pubkey,
        dest_borrower_liq_wallet_sol,
    )
    .await;
    assert!(result.is_err());

    let result =
        claim_texture_performance_fees(&mut ctx, reserve_sol1_pubkey, dest_borrower_liq_wallet_sol)
            .await;
    assert!(result.is_err());
}
