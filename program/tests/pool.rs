#![cfg(feature = "test-bpf")]

use bytemuck::Zeroable;
use price_proxy::state::utils::str_to_array;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use texture_common::account::PodAccount;
use tracing::info;

use super_lendy::state::curator::CuratorParams;
use super_lendy::state::pool::{Pool, PoolParams};
use super_lendy::state::texture_cfg::TextureConfigParams;

use crate::utils::superlendy_executor::{
    alter_pool, create_curator, create_pool, create_texture_config,
};
use crate::utils::{
    admin_keypair, get_account, init_program_test, texture_config_keypair, Runner, LAMPORTS,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#70cc967a68114e01b1cb3330dd066dcf
#[tokio::test]
async fn create_alter_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_keypair.pubkey(), LAMPORTS);

    let mut ctx = runner.start_with_context().await;

    info!("create texture config");

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: Zeroable::zeroed(),
    };
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        params,
    )
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
        &texture_owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    info!("create pool");
    let mut params = PoolParams {
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

    macro_rules! validate {
        ($this:ident, $params:ident) => {
            assert_eq!($this.visible, $params.visible);
            assert_eq!($this.name, $params.name);
            assert_eq!($this.curator, curator_pubkey);
        };
    }

    let pool_acc = get_account(&mut ctx.banks_client, pool_pubkey)
        .await
        .expect("get pool");
    let pool = Pool::try_from_bytes(&pool_acc.data).expect("cast pool data");
    validate!(pool, params);

    // ALTER POOL

    params.visible = 1;
    params.name = [2; 128];

    info!("alter pool");
    alter_pool(
        &mut ctx,
        pool_pubkey,
        &pool_authority_keypair,
        curator_pubkey,
        params,
    )
    .await
    .expect("alter pool");

    let pool_acc = get_account(&mut ctx.banks_client, pool_pubkey)
        .await
        .expect("get pool");
    let pool = Pool::try_from_bytes(&pool_acc.data).expect("cast pool data");
    validate!(pool, params);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#503df7cd118b46c39cf05b09245bcfb6
#[tokio::test]
async fn create_incorrect_pools_authority() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_keypair.pubkey(), LAMPORTS);

    let mut ctx = runner.start_with_context().await;

    info!("create texture config");

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: Zeroable::zeroed(),
    };
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        params,
    )
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
        &texture_owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL WITH INCORRECT AUTHORITY

    info!("create pool with incorrect authority");
    let params = PoolParams {
        name: [1; 128],
        market_price_currency_symbol: str_to_array("USD"),
        visible: 0,
    };

    let result = create_pool(
        &mut ctx,
        &pool_keypair,
        &owner_keypair,
        curator_pubkey,
        params,
    )
    .await;
    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#709d8bbf19724652bc2af02392bb274c
#[tokio::test]
async fn alter_incorrect_pools_authority() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let pool_authority_keypair = Keypair::new();
    let pool_authority_pubkey = pool_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let pool_keypair = Keypair::new();
    let pool_pubkey = pool_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(pool_authority_pubkey, LAMPORTS);
    runner.add_native_wallet(texture_owner_keypair.pubkey(), LAMPORTS);

    let mut ctx = runner.start_with_context().await;

    info!("create texture config");

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
        reserve_timelock: Zeroable::zeroed(),
    };
    create_texture_config(
        &mut ctx,
        &texture_owner_keypair,
        &texture_config_keypair,
        params,
    )
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
        &texture_owner_keypair,
        params,
    )
    .await
    .expect("create_curator");

    // CREATE POOL

    info!("create pool");
    let mut params = PoolParams {
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

    // ALTER POOL WITH INCORRECT AUTHORITY

    info!("alter pool with incorrect authority");
    params.visible = 1;
    params.name = [2; 128];

    let result = alter_pool(
        &mut ctx,
        pool_pubkey,
        &owner_keypair,
        curator_pubkey,
        params,
    )
    .await;

    assert!(result.is_err())
}
