#![cfg(feature = "test-bpf")]

use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use texture_common::account::PodAccount;
use tracing::info;

use super_lendy::instruction::CreateTextureConfig;
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfig, TextureConfigParams};

use crate::utils::superlendy_executor::{alter_texture_config, create_texture_config};
use crate::utils::{get_account, init_program_test, texture_config_keypair, Runner, LAMPORTS};

pub mod utils;

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#7871e4a90866466d8889be9fb6c72046
#[tokio::test]
async fn create_alter_success() {
    let mut runner = init_program_test();

    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let fee_authority_keypair = Keypair::new();
    let fee_authority_pubkey = fee_authority_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    let texture_config_pubkey = texture_config_keypair.pubkey();
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(fee_authority_pubkey, LAMPORTS);

    let mut ctx = runner.start_with_context().await;

    // CREATE TEXTURE CONFIG

    info!("create texture config");
    let mut params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
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
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    macro_rules! validate {
        ($this:ident, $params:ident) => {
            assert_eq!($this.borrow_fee_rate_bps, $params.borrow_fee_rate_bps);
            assert_eq!($this.fees_authority, $params.fees_authority);
            assert_eq!(
                $this.performance_fee_rate_bps,
                $params.performance_fee_rate_bps
            );
            assert_eq!($this.owner, owner_pubkey);
        };
    }

    let config_acc = get_account(&mut ctx.banks_client, texture_config_pubkey)
        .await
        .expect("get curator");
    let config = TextureConfig::try_from_bytes(&config_acc.data).expect("cast config data");
    validate!(config, params);

    // ALTER TEXTURE CONFIG

    info!("alter texture config");
    params.performance_fee_rate_bps += 1;
    params.borrow_fee_rate_bps -= 1;
    params.fees_authority = fee_authority_pubkey;

    alter_texture_config(&mut ctx, &owner_keypair, params)
        .await
        .expect("alter_texture_config");

    let config_acc = get_account(&mut ctx.banks_client, texture_config_pubkey)
        .await
        .expect("get curator");
    let config = TextureConfig::try_from_bytes(&config_acc.data).expect("cast config data");
    validate!(config, params);
}

#[tokio::test]
async fn create_repeated() {
    let mut runner = init_program_test();

    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let other_owner_keypair = Keypair::new();
    let other_owner_pubkey = other_owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(other_owner_pubkey, LAMPORTS);

    let mut ctx = runner.start_with_context().await;

    let params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
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
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    // CREATE REPEATED WITH OTHER OWNER

    info!("create repeated");
    let blockhash = ctx
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[CreateTextureConfig {
            owner: other_owner_pubkey,
            params,
        }
        .into_instruction()],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &other_owner_keypair, &texture_config_keypair],
        blockhash,
    );
    let result = ctx.banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

/// See test description in
/// https://www.notion.so/3fc6f2d034dc4ff194c69d6f549217f8?pvs=4#f6fe7788025948b39e18c9d50876fa8a
#[tokio::test]
async fn alter_incorrect_owner() {
    let mut runner = init_program_test();

    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();
    let other_owner_keypair = Keypair::new();
    let other_owner_pubkey = other_owner_keypair.pubkey();
    let texture_config_keypair = texture_config_keypair();
    runner.add_native_wallet(owner_pubkey, LAMPORTS);
    runner.add_native_wallet(other_owner_pubkey, LAMPORTS);

    let mut ctx = runner.start_with_context().await;

    let mut params = TextureConfigParams {
        borrow_fee_rate_bps: 100,
        performance_fee_rate_bps: 100,
        fees_authority: owner_pubkey,
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
    create_texture_config(&mut ctx, &owner_keypair, &texture_config_keypair, params)
        .await
        .expect("create_texture_config");

    params.performance_fee_rate_bps += 1;

    // ALTER WITH INCORRECT OWNER

    info!("alter with incorrect owner");
    let result = alter_texture_config(&mut ctx, &other_owner_keypair, params).await;
    assert!(result.is_err())
}
