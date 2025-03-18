#![cfg(feature = "test-bpf")]

use bytemuck::Zeroable;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use texture_common::account::PodAccount;
use tracing::info;

use super_lendy::instruction::CreateCurator;
use super_lendy::state::curator::{Curator, CuratorParams};
use super_lendy::state::texture_cfg::TextureConfigParams;

use crate::utils::superlendy_executor::{alter_curator, create_curator, create_texture_config};
use crate::utils::{
    admin_keypair, get_account, init_program_test, texture_config_keypair, Runner, LAMPORTS,
};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#e437f6163ced42c38a007c5707cb943a
#[tokio::test]
async fn create_alter_success() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let fees_authority_keypair = Keypair::new();
    let fees_authority_pubkey = fees_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(fees_authority_pubkey, LAMPORTS);
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

    info!("create curator");
    let mut params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: owner_pubkey,
        pools_authority: owner_pubkey,
        vaults_authority: owner_pubkey,
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

    macro_rules! validate {
        ($this:ident, $params:ident) => {
            assert_eq!($this.owner, $params.owner);
            assert_eq!($this.fees_authority, $params.fees_authority);
            assert_eq!($this.pools_authority, $params.pools_authority);
            assert_eq!($this.vaults_authority, $params.vaults_authority);
            assert_eq!($this.name, $params.name);
            assert_eq!($this.logo_url, $params.logo_url);
            assert_eq!($this.website_url, $params.website_url);
        };
    }

    let curator_acc = get_account(&mut ctx.banks_client, curator_pubkey)
        .await
        .expect("get curator");
    let curator = Curator::try_from_bytes(&curator_acc.data).expect("cast curator data");
    validate!(curator, params);

    // ALTER CURATOR

    info!("alter curator");
    params.owner = fees_authority_pubkey;
    params.fees_authority = fees_authority_pubkey;
    params.pools_authority = fees_authority_pubkey;
    params.vaults_authority = fees_authority_pubkey;
    params.name = [4; 128];
    params.logo_url = [5; 128];
    params.website_url = [6; 128];

    alter_curator(&mut ctx, curator_pubkey, &owner_keypair, params)
        .await
        .expect("alter_curator");

    let curator_acc = get_account(&mut ctx.banks_client, curator_pubkey)
        .await
        .expect("get curator");
    let curator = Curator::try_from_bytes(&curator_acc.data).expect("cast curator data");
    validate!(curator, params);
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#e568358c846645d09ec4367955d9ff58
#[tokio::test]
async fn create_incorrect_admin() {
    let mut runner = init_program_test();

    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
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

    // CREATE CURATOR WITH INCORRECT ADMIN

    info!("create curator with incorrect admin");
    let params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: owner_pubkey,
        pools_authority: owner_pubkey,
        vaults_authority: owner_pubkey,
        name: [1; 128],
        logo_url: [2; 128],
        website_url: [3; 128],
    };

    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");

    let result = std::panic::catch_unwind(|| {
        Transaction::new_signed_with_payer(
            &[CreateCurator {
                curator: curator_pubkey,
                global_config_owner: curator_keypair.pubkey(),
                params,
            }
            .into_instruction()],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &owner_keypair, &curator_keypair],
            blockhash,
        )
    });
    assert!(result.is_err())
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#a7f31abee92446a691bb93e397f89c90
#[tokio::test]
async fn alter_incorrect_owner() {
    let mut runner = init_program_test();

    let admin_keypair = admin_keypair();
    let admin_pubkey = admin_keypair.pubkey();
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let fees_authority_keypair = Keypair::new();
    let fees_authority_pubkey = fees_authority_keypair.pubkey();
    let curator_keypair = Keypair::new();
    let curator_pubkey = curator_keypair.pubkey();
    let texture_owner_keypair = Keypair::new();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(admin_pubkey, LAMPORTS);
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(fees_authority_pubkey, LAMPORTS);
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

    let mut params = CuratorParams {
        owner: owner_pubkey,
        fees_authority: owner_pubkey,
        pools_authority: owner_pubkey,
        vaults_authority: owner_pubkey,
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

    // ALTER CURATOR WITH INCORRECT OWNER

    info!("alter curator with incorrect owner");
    params.owner = fees_authority_pubkey;
    params.fees_authority = fees_authority_pubkey;
    params.pools_authority = fees_authority_pubkey;
    params.vaults_authority = fees_authority_pubkey;
    params.name = [4; 128];
    params.logo_url = [5; 128];
    params.website_url = [6; 128];

    let result = alter_curator(&mut ctx, curator_pubkey, &fees_authority_keypair, params).await;
    assert!(result.is_err())
}
