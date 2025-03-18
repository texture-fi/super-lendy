use crate::app::App;

use price_proxy::instruction::WritePrice;
use price_proxy::state::price_feed::{PriceFeed, PriceFeedSource};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use texture_common::account::PodAccount;
use texture_common::math::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Decimal};

use super_lendy::instruction::{
    Borrow, CreatePosition, DepositLiquidity, LockCollateral, RefreshPosition, RefreshReserve,
    Version,
};
use super_lendy::pda::find_lp_token_mint;
use super_lendy::state::curator::Curator;
use super_lendy::state::pool::Pool;
use super_lendy::state::position::{
    Position, BORROW_MEMO_LEN, COLLATERAL_MEMO_LEN, POSITION_TYPE_CLASSIC,
};
use super_lendy::state::reserve::{FeeCalculation, Reserve};
use super_lendy::state::texture_cfg::TextureConfig;
use super_lendy::{SUPER_LENDY_ID, TEXTURE_CONFIG_ID};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionCfg {
    instances: u8,
    #[serde_as(as = "DisplayFromStr")]
    pool: Pubkey,
    deposits: Vec<PositionDeposit>,
    borrows: Vec<PositionBorrow>,
    health: PositionHealth,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionDeposit {
    #[serde_as(as = "DisplayFromStr")]
    reserve: Pubkey,
    amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionBorrow {
    #[serde_as(as = "DisplayFromStr")]
    reserve: Pubkey,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionHealth {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "partly-unhealthy")]
    PartlyUnhealthy,
    #[serde(rename = "fully-unhealthy")]
    FullyUnhealthy,
}

pub async fn price_feed_from_reserve(app: &App, reserve_key: &Pubkey) -> (Pubkey, PriceFeed) {
    let reserve_data = app
        .rpc
        .get_account_data(reserve_key)
        .await
        .expect("getting Reserve account");
    let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");
    let price_feed_data = app
        .rpc
        .get_account_data(&reserve.config.market_price_feed)
        .await
        .expect("getting PriceFeed account");
    (
        reserve.config.market_price_feed,
        *PriceFeed::try_from_bytes(&price_feed_data).expect("unpacking PriceFeed"),
    )
}

pub async fn position_after_refresh(app: &App, position_key: Pubkey) -> Position {
    let refresh_position_info = app.refresh_position_ix(position_key).await;
    app.process_transaction(refresh_position_info.0, &[&app.authority], 5)
        .await
        .expect("Sending TX");
    let position_data = app
        .rpc
        .get_account_data(&position_key)
        .await
        .expect("getting Position account");
    *Position::try_from_bytes(&position_data).expect("unpacking Position")
}

pub async fn create_position(app: &App, position_keypair: &Keypair, pool: Pubkey) {
    let position_key = position_keypair.pubkey();

    let create_position_ixs = vec![
        system_instruction::create_account(
            &app.authority.pubkey(),
            &position_keypair.pubkey(),
            app.rpc
                .get_minimum_balance_for_rent_exemption(Position::SIZE)
                .await
                .expect("getting rent"),
            Position::SIZE as u64,
            &SUPER_LENDY_ID,
        ),
        CreatePosition {
            position: position_keypair.pubkey(),
            pool,
            owner: app.authority.pubkey(),
            position_type: POSITION_TYPE_CLASSIC,
        }
        .into_instruction(),
        RefreshPosition {
            position: position_key,
            deposits: vec![],
            borrows: vec![],
        }
        .into_instruction(),
        Version { no_error: true }.into_instruction(),
    ];

    app.process_transaction(
        create_position_ixs,
        &vec![position_keypair, &app.authority],
        5,
    )
    .await
    .expect("Sending create ATA TX");
    println!("Created position: {}", position_key);
}

pub async fn deposit(
    app: &App,
    position_key: Pubkey,
    collateral_reserve_key: Pubkey,
    pool_key: Pubkey,
    deposit_amount: u64,
) {
    let collateral_reserve_data = app
        .rpc
        .get_account_data(&collateral_reserve_key)
        .await
        .expect("getting Reserve account");
    let collateral_reserve =
        Reserve::try_from_bytes(&collateral_reserve_data).expect("unpacking Reserve");
    if collateral_reserve.pool != pool_key {
        println!(
            "pool key mismatch: key from collateral_reserve.pool is {}, key from config is {}",
            collateral_reserve.pool, pool_key
        );
        return;
    }

    let lp_mint = find_lp_token_mint(&collateral_reserve_key);

    let source_liquidity_wallet = get_associated_token_address_with_program_id(
        &app.authority.pubkey(),
        &collateral_reserve.liquidity.mint,
        &app.token_program_by_mint(&collateral_reserve.liquidity.mint)
            .await,
    );
    if !app
        .account_exists(&source_liquidity_wallet)
        .await
        .expect("check source_liquidity_wallet existance")
    {
        println!(
            "Creating wallet {} for liquidity tokens.",
            source_liquidity_wallet
        );

        let mint_account = app
            .rpc
            .get_account(&collateral_reserve.liquidity.mint)
            .await
            .expect("getting mint account");

        let ix = create_associated_token_account(
            &app.authority.pubkey(),
            &app.authority.pubkey(),
            &collateral_reserve.liquidity.mint,
            &mint_account.owner,
        );

        let version = Version { no_error: true }.into_instruction();

        app.process_transaction(vec![ix, version], &vec![&app.authority], 5)
            .await
            .expect("Sending create ATA TX");
    }

    let destination_lp_wallet = get_associated_token_address(&app.authority.pubkey(), &lp_mint.0);
    if !app
        .account_exists(&destination_lp_wallet)
        .await
        .expect("check destination_lp_wallet existance")
    {
        println!("Creating wallet {} for LP tokens.", destination_lp_wallet);

        let lp_mint_account = app
            .rpc
            .get_account(&lp_mint.0)
            .await
            .expect("getting LP mint account");

        let ix = create_associated_token_account(
            &app.authority.pubkey(),
            &app.authority.pubkey(),
            &lp_mint.0,
            &lp_mint_account.owner,
        );

        app.process_transaction(vec![ix], &vec![&app.authority], 5)
            .await
            .expect("Sending create ATA TX");
    }

    let mut ixs = app.refresh_reserves_ix(&[collateral_reserve_key]).await;
    ixs.push(
        DepositLiquidity {
            reserve: collateral_reserve_key,
            authority: app.authority.pubkey(),
            source_liquidity_wallet,
            destination_lp_wallet,
            amount: deposit_amount,
            liquidity_mint: collateral_reserve.liquidity.mint,
            liquidity_token_program: app
                .token_program_by_mint(&collateral_reserve.liquidity.mint)
                .await,
        }
        .into_instruction(),
    );

    let refresh_position_info = app.refresh_position_ix(position_key).await;
    let mut refresh_position_ixs = refresh_position_info.0;
    if !refresh_position_info
        .1
        .iter()
        .any(|key| key == &collateral_reserve_key)
    {
        let refresh_reserve = RefreshReserve {
            reserve: collateral_reserve_key,
            market_price_feed: collateral_reserve.config.market_price_feed,
            irm: collateral_reserve.config.irm,
        }
        .into_instruction();
        refresh_position_ixs.push(refresh_reserve);
    }
    ixs.extend(refresh_position_ixs);

    ixs.extend(vec![
        LockCollateral {
            position: position_key,
            reserve: collateral_reserve_key,
            source_lp_wallet: destination_lp_wallet,
            owner: app.authority.pubkey(),
            amount: deposit_amount,
            memo: [0; COLLATERAL_MEMO_LEN],
        }
        .into_instruction(),
        Version { no_error: true }.into_instruction(),
    ]);

    app.process_transaction(ixs, &[&app.authority], 5)
        .await
        .expect("Sending TX");
    println!(
        "Deposit & Lock {} into {} reserve",
        deposit_amount, collateral_reserve_key
    );
}

pub async fn borrow(
    app: &App,
    borrow_reserve_key: Pubkey,
    pool_key: Pubkey,
    position_key: Pubkey,
    value_to_borrow: Decimal,
) {
    let borrow_reserve_data = app
        .rpc
        .get_account_data(&borrow_reserve_key)
        .await
        .expect("getting Reserve account");
    let borrow_reserve = Reserve::try_from_bytes(&borrow_reserve_data).expect("unpacking Reserve");
    if borrow_reserve.pool != pool_key {
        println!(
            "pool key mismatch: key from borrow_reserve.pool is {}, key from config is {}",
            borrow_reserve.pool, pool_key
        );
        return;
    }

    let destination_liquidity_wallet = get_associated_token_address_with_program_id(
        &app.authority.pubkey(),
        &borrow_reserve.liquidity.mint,
        &app.token_program_by_mint(&borrow_reserve.liquidity.mint)
            .await,
    );
    if !app
        .account_exists(&destination_liquidity_wallet)
        .await
        .expect("check destination_liquidity_wallet existance")
    {
        println!(
            "Creating wallet {} for liquidity tokens.",
            destination_liquidity_wallet
        );

        let mint_account = app
            .rpc
            .get_account(&borrow_reserve.liquidity.mint)
            .await
            .expect("getting mint account");

        let ix = create_associated_token_account(
            &app.authority.pubkey(),
            &app.authority.pubkey(),
            &borrow_reserve.liquidity.mint,
            &mint_account.owner,
        );

        let version = Version { no_error: true }.into_instruction();

        app.process_transaction(vec![ix, version], &vec![&app.authority], 5)
            .await
            .expect("Sending create ATA TX");
    }

    let cfg_data = app
        .rpc
        .get_account_data(&TEXTURE_CONFIG_ID)
        .await
        .expect("getting Pool account");
    let cfg = TextureConfig::try_from_bytes(&cfg_data).expect("unpacking Global Config");

    let texture_fee_receiver = get_associated_token_address_with_program_id(
        &cfg.fees_authority,
        &borrow_reserve.liquidity.mint,
        &app.token_program_by_mint(&borrow_reserve.liquidity.mint)
            .await,
    );

    let pool_data = app
        .rpc
        .get_account_data(&borrow_reserve.pool)
        .await
        .expect("getting Pool account");
    let pool = Pool::try_from_bytes(&pool_data).expect("unpacking Pool");

    let curator_data = app
        .rpc
        .get_account_data(&pool.curator)
        .await
        .expect("getting Curator account");
    let curator = Curator::try_from_bytes(&curator_data).expect("unpacking Curator");

    let curator_fee_receiver = get_associated_token_address_with_program_id(
        &curator.fees_authority,
        &borrow_reserve.liquidity.mint,
        &app.token_program_by_mint(&borrow_reserve.liquidity.mint)
            .await,
    );

    let refresh_borrow_reserve = app.refresh_reserves_ix(&[borrow_reserve_key]).await;
    app.process_transaction(refresh_borrow_reserve, &[&app.authority], 5)
        .await
        .expect("Sending TX");

    let borrow_reserve_data = app
        .rpc
        .get_account_data(&borrow_reserve_key)
        .await
        .expect("getting Reserve account");
    let borrow_reserve = Reserve::try_from_bytes(&borrow_reserve_data).expect("unpacking Reserve");
    let texture_config_data = app
        .rpc
        .get_account_data(&TEXTURE_CONFIG_ID)
        .await
        .expect("getting TextureConfig account");
    let unpacked_texture_config =
        TextureConfig::try_from_bytes(&texture_config_data).expect("unpacking TextureConfig");

    let borrow_amount = value_to_borrow
        .checked_div(borrow_reserve.liquidity.market_price().unwrap())
        .unwrap();
    let (curator_borrow_fee, texture_borrow_fee) = borrow_reserve
        .config
        .fees
        .calculate_borrow_fees(
            borrow_amount,
            borrow_reserve.liquidity.mint_decimals,
            unpacked_texture_config.borrow_fee_rate_bps,
            FeeCalculation::Inclusive,
        )
        .unwrap();
    let amount_ex_fees = borrow_amount
        .to_lamports_floor(borrow_reserve.liquidity.mint_decimals)
        .unwrap()
        .checked_sub(curator_borrow_fee)
        .unwrap()
        .checked_sub(texture_borrow_fee)
        .unwrap();

    let borrow_ix = Borrow {
        position: position_key,
        reserve: borrow_reserve_key,
        pool: borrow_reserve.pool,
        destination_liquidity_wallet,
        curator_fee_receiver,
        texture_fee_receiver,
        borrower: app.authority.pubkey(),
        amount: amount_ex_fees,
        slippage_limit: 1,
        curator: pool.curator,
        memo: [0; BORROW_MEMO_LEN],
        token_program: app
            .token_program_by_mint(&borrow_reserve.liquidity.mint)
            .await,
        liquidity_mint: borrow_reserve.liquidity.mint,
    }
    .into_instruction();

    let refresh_position_info = app.refresh_position_ix(position_key).await;
    let mut borrow_ixs = refresh_position_info.0;
    borrow_ixs.push(borrow_ix);

    app.process_transaction(borrow_ixs, &[&app.authority], 5)
        .await
        .expect("Sending TX");
    println!(
        "Borrow {} from {} reserve",
        borrow_amount, borrow_reserve_key
    );
}

pub async fn gen_unhealthy_positions(
    app: &App,
    position_config: String,
    price_feed_authority: Keypair,
) {
    let text = std::fs::read_to_string(position_config).expect("read position config");
    let position_cfg: PositionCfg = serde_json::from_str(&text).expect("parse position config");

    let mut last_position = None;

    if position_cfg.deposits.is_empty() || position_cfg.borrows.is_empty() {
        println!("Not enough deposit or borrow reserves to create position");
        return;
    }
    for i in 0..position_cfg.instances {
        let position_keypair = Keypair::new();
        let position_key = position_keypair.pubkey();

        println!("----------------Create Position #{}----------------", i);
        create_position(app, &position_keypair, position_cfg.pool).await;

        println!("--------------------- Deposits & Locks ---------------------");
        for position_deposit in position_cfg.deposits.clone() {
            deposit(
                app,
                position_key,
                position_deposit.reserve,
                position_cfg.pool,
                position_deposit.amount,
            )
            .await;
        }

        let position = position_after_refresh(app, position_key).await;
        let allowed_borrow_value = position.allowed_borrow_value().unwrap();
        let value_to_borrow = allowed_borrow_value
            .checked_div(
                Decimal::from_i128_with_scale(position_cfg.borrows.len() as i128, 0).unwrap(),
            )
            .unwrap();

        println!("--------------------- Borrows ---------------------");
        for position_borrow in position_cfg.borrows.clone() {
            borrow(
                app,
                position_borrow.reserve,
                position_cfg.pool,
                position_key,
                value_to_borrow,
            )
            .await;
        }

        last_position = Some(position_key);
    }

    let position = if let Some(last_position) = last_position {
        position_after_refresh(app, last_position).await
    } else {
        return;
    };
    let borrowed_value = position.borrowed_value().unwrap();
    let deposited_value = position.deposited_value().unwrap();

    println!("--------------------- Write Price ---------------------");

    let unhealthy_borrow_value = match position_cfg.health {
        PositionHealth::PartlyUnhealthy => position.partly_unhealthy_borrow_value().unwrap(),
        PositionHealth::FullyUnhealthy => position.fully_unhealthy_borrow_value().unwrap(),
        PositionHealth::Healthy => {
            println!("Position configured as healthy");
            return;
        }
    };
    let unhealthy_ltv = unhealthy_borrow_value.checked_div(deposited_value).unwrap();

    for deposit in &position_cfg.deposits {
        let (price_feed_key, price_feed) = price_feed_from_reserve(app, &deposit.reserve).await;
        if price_feed.source() == PriceFeedSource::OffChain {
            let unhealthy_deposited_value = borrowed_value.checked_div(unhealthy_ltv).unwrap();
            let diff_value = deposited_value
                .checked_sub(unhealthy_deposited_value)
                .unwrap();
            let collateral_reserve_value =
                position.collateral.iter().find_map(|position_collateral| {
                    if position_collateral.deposit_reserve == deposit.reserve {
                        Some(position_collateral.market_value().unwrap())
                    } else {
                        None
                    }
                });
            if let Some(collateral_reserve_value) = collateral_reserve_value {
                if diff_value >= collateral_reserve_value {
                    println!(
                        "Deposit share of reserve {} in deposit value is too small",
                        deposit.reserve
                    );
                    return;
                }
                let reserve_data = app
                    .rpc
                    .get_account_data(&deposit.reserve)
                    .await
                    .expect("getting Reserve account");
                let reserve = Reserve::try_from_bytes(&reserve_data).expect("unpacking Reserve");
                let new_price = collateral_reserve_value
                    .checked_sub(diff_value)
                    .unwrap()
                    .checked_div(
                        Decimal::from_lamports(deposit.amount, reserve.liquidity.mint_decimals)
                            .unwrap(),
                    )
                    .unwrap();
                app.process_transaction(
                    vec![WritePrice {
                        price_feed: price_feed_key,
                        authority: price_feed_authority.pubkey(),
                        price: new_price,
                        price_timestamp: chrono::Utc::now().timestamp(),
                    }
                    .into_instruction()],
                    &[&app.authority, &price_feed_authority],
                    5,
                )
                .await
                .expect("Sending TX");
                return;
            }
        }
    }

    for borrow in &position_cfg.borrows {
        let (price_feed_key, price_feed) = price_feed_from_reserve(app, &borrow.reserve).await;
        if price_feed.source() == PriceFeedSource::OffChain {
            let unhealthy_borrow_value = deposited_value.checked_mul(unhealthy_ltv).unwrap();
            let diff_value = unhealthy_borrow_value.checked_sub(borrowed_value).unwrap();
            let borrow_reserve_value = position.borrows.iter().find_map(|position_borrow| {
                if position_borrow.borrow_reserve == borrow.reserve {
                    Some((
                        position_borrow.borrowed_amount().unwrap(),
                        position_borrow.market_value().unwrap(),
                    ))
                } else {
                    None
                }
            });

            if let Some((borrow_reserve_amount, borrow_reserve_value)) = borrow_reserve_value {
                let new_price = borrow_reserve_value
                    .checked_add(diff_value)
                    .unwrap()
                    .checked_div(borrow_reserve_amount)
                    .unwrap();
                app.process_transaction(
                    vec![WritePrice {
                        price_feed: price_feed_key,
                        authority: price_feed_authority.pubkey(),
                        price: new_price,
                        price_timestamp: chrono::Utc::now().timestamp(),
                    }
                    .into_instruction()],
                    &[&app.authority, &price_feed_authority],
                    5,
                )
                .await
                .expect("Sending TX");
                return;
            }
        }
    }
}
