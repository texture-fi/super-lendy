use std::str::FromStr;

use bytemuck::Zeroable;
use price_proxy::state::utils::str_to_array;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use tracing::info;

use super_lendy::state::curator::CuratorParams;
use super_lendy::state::pool::PoolParams;
use super_lendy::state::reserve::{
    ReserveConfig, ReserveFeesConfig, RESERVE_TYPE_NORMAL, RESERVE_TYPE_NOT_A_COLLATERAL,
    RESERVE_TYPE_PROTECTED_COLLATERAL,
};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};

use crate::utils::superlendy_executor::{
    create_curator, create_pool, create_position, create_reserve, create_texture_config,
};

#[allow(clippy::too_many_arguments)]
pub async fn setup_lendy_env(
    ctx: &mut ProgramTestContext,
    admin_keypair: &Keypair,
    borrower_keypair: &Keypair,
    curator_keypair: &Keypair,
    pool_keypair: &Keypair,
    reserve_sol1_keypair: &Keypair,
    reserve_sol2_keypair: &Keypair,
    reserve_usdc_keypair: &Keypair,
    texture_owner_keypair: &Keypair,
    texture_config_keypair: &Keypair,
    pool_authority_keypair: &Keypair,
    borrower_position_keypair: &Keypair,
    irm: Pubkey,
) {
    // 1 SOL = 100 USD
    let sol_price_feed = Pubkey::from_str("2Ds9EtKTqMhQsm71oWyuM3YGVWT6fjPDJ8SKXqsb7c6b").unwrap();
    // 1 USDC = 1.001 USD
    let usdc_price_feed = Pubkey::from_str("3CkekH4QYb3b5U8tSDNNqCsf34c7kMbE7SaCgwqLy5FW").unwrap();

    let liquidity_sol_mint =
        Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let liquidity_usdc_mint =
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

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
        ctx,
        texture_owner_keypair,
        texture_config_keypair,
        texture_params,
    )
    .await
    .expect("create_texture_config");

    // CREATE CURATOR

    let curator_params = CuratorParams {
        owner: texture_owner_keypair.pubkey(),
        fees_authority: pool_authority_keypair.pubkey(),
        pools_authority: pool_authority_keypair.pubkey(),
        vaults_authority: pool_authority_keypair.pubkey(),
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };
    create_curator(
        ctx,
        curator_keypair,
        admin_keypair,
        texture_owner_keypair,
        curator_params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    let pool_params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    create_pool(
        ctx,
        pool_keypair,
        pool_authority_keypair,
        curator_keypair.pubkey(),
        pool_params,
    )
    .await
    .expect("create_pool");

    // CREATE RESERVE SOL1 BORROW ENABLED

    let fees_config = ReserveFeesConfig {
        curator_borrow_fee_rate_bps: 100,       // 1%
        curator_performance_fee_rate_bps: 2000, // 20%
        _padding: Zeroable::zeroed(),
    };
    let mut reserve_config = ReserveConfig {
        market_price_feed: sol_price_feed,
        irm,
        liquidation_bonus_bps: 200,
        max_borrow_ltv_bps: 6000,
        partly_unhealthy_ltv_bps: 9500,
        fully_unhealthy_ltv_bps: 9700,
        partial_liquidation_factor_bps: 2000,
        _padding: Zeroable::zeroed(),
        fees: fees_config,
        max_total_liquidity: 1_000_000_000 * LAMPORTS_PER_SOL,
        max_borrow_utilization_bps: 5000,
        price_stale_threshold_sec: 10000000,
        max_withdraw_utilization_bps: 9000,
    };

    info!("create reserve sol borrow enabled");
    create_reserve(
        ctx,
        reserve_sol1_keypair,
        pool_keypair.pubkey(),
        pool_authority_keypair,
        curator_keypair.pubkey(),
        liquidity_sol_mint,
        sol_price_feed,
        reserve_config,
        RESERVE_TYPE_NOT_A_COLLATERAL,
    )
    .await
    .expect("create_reserve_sol1");

    // CREATE RESERVE SOL2 BORROW DISABLED

    reserve_config.max_borrow_ltv_bps = 5000;

    info!("create reserve sol borrow disabled");
    create_reserve(
        ctx,
        reserve_sol2_keypair,
        pool_keypair.pubkey(),
        pool_authority_keypair,
        curator_keypair.pubkey(),
        liquidity_sol_mint,
        sol_price_feed,
        reserve_config,
        RESERVE_TYPE_PROTECTED_COLLATERAL,
    )
    .await
    .expect("create_reserve_sol2");

    // CREATE RESERVE USDC MODE NORMAL

    reserve_config.market_price_feed = usdc_price_feed;
    reserve_config.max_borrow_ltv_bps = 9000;

    info!("create reserve usdc type normal");
    create_reserve(
        ctx,
        reserve_usdc_keypair,
        pool_keypair.pubkey(),
        pool_authority_keypair,
        curator_keypair.pubkey(),
        liquidity_usdc_mint,
        usdc_price_feed,
        reserve_config,
        RESERVE_TYPE_NORMAL,
    )
    .await
    .expect("create_reserve_usdc");

    // CREATE POSITION FOR BORROWER

    info!("create position for borrower");
    create_position(
        ctx,
        borrower_position_keypair,
        pool_keypair.pubkey(),
        borrower_keypair,
    )
    .await
    .expect("create_position");
}
