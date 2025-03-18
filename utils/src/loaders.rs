use std::collections::HashMap;

use anyhow::{Error, Result};
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use texture_common::account::PodAccount;

use super_lendy::state::curator::Curator;
use super_lendy::state::pool::Pool;
use super_lendy::state::position::Position;
use super_lendy::state::reserve::Reserve;
use super_lendy::state::{
    CURATOR_DISCRIMINATOR, POOL_DISCRIMINATOR, POSITION_DISCRIMINATOR, RESERVE_DISCRIMINATOR,
};
use super_lendy::SUPER_LENDY_ID;

pub async fn load_curators(rpc: &RpcClient) -> Result<HashMap<Pubkey, Curator>> {
    let filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, CURATOR_DISCRIMINATOR.to_vec()));

    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig::confirmed()),
        min_context_slot: None,
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config,
        with_context: None,
    };

    let accounts = rpc
        .get_program_accounts_with_config(&SUPER_LENDY_ID, config)
        .await?;

    let mut curators = HashMap::new();
    for (key, account) in &accounts {
        match Curator::try_from_bytes(&account.data) {
            Ok(curator) => {
                curators.insert(*key, *curator);
            }
            Err(err) => {
                return Err(Error::from(err));
            }
        }
    }

    Ok(curators)
}

pub async fn load_pools(rpc: &RpcClient) -> Result<HashMap<Pubkey, Pool>> {
    let filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, POOL_DISCRIMINATOR.to_vec()));

    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig::confirmed()),
        min_context_slot: None,
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config,
        with_context: None,
    };

    let accounts = rpc
        .get_program_accounts_with_config(&SUPER_LENDY_ID, config)
        .await?;

    let mut pools = HashMap::new();
    for (key, account) in &accounts {
        match Pool::try_from_bytes(&account.data) {
            Ok(pool) => {
                pools.insert(*key, *pool);
            }
            Err(err) => {
                return Err(Error::from(err));
            }
        }
    }

    Ok(pools)
}

pub async fn load_reserves(rpc: &RpcClient) -> Result<HashMap<Pubkey, Reserve>> {
    let filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, RESERVE_DISCRIMINATOR.to_vec()));

    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig::confirmed()),
        min_context_slot: None,
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config,
        with_context: None,
    };

    let accounts = rpc
        .get_program_accounts_with_config(&SUPER_LENDY_ID, config)
        .await?;

    let mut reserves = HashMap::new();
    for (key, account) in &accounts {
        match Reserve::try_from_bytes(&account.data) {
            Ok(reserve) => {
                reserves.insert(*key, *reserve);
            }
            Err(err) => {
                return Err(Error::from(err));
            }
        }
    }

    Ok(reserves)
}

pub async fn load_positions(rpc: &RpcClient) -> Result<HashMap<Pubkey, Position>> {
    let filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, POSITION_DISCRIMINATOR.to_vec()));

    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig::confirmed()),
        min_context_slot: None,
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config,
        with_context: None,
    };

    let accounts = rpc
        .get_program_accounts_with_config(&SUPER_LENDY_ID, config)
        .await?;

    let mut positions = HashMap::new();
    for (key, account) in &accounts {
        match Position::try_from_bytes(&account.data) {
            Ok(position) => {
                positions.insert(*key, *position);
            }
            Err(err) => {
                return Err(Error::from(err));
            }
        }
    }

    Ok(positions)
}
