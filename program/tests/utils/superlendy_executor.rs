use chrono::Utc;
use price_proxy::instruction::WritePrice;
use price_proxy::state::price_feed::PriceFeed;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::create_account;
use solana_program_test::{BanksClientError, ProgramTestBanksClientExt, ProgramTestContext};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use texture_common::account::PodAccount;
use texture_common::math::Decimal;

use super_lendy::instruction::{
    AlterCurator, AlterPool, AlterReserve, AlterTextureConfig, ApplyConfigProposal, Borrow,
    ClaimCuratorPerformanceFees, ClaimReward, ClaimTexturePerformanceFees, CreateCurator,
    CreatePool, CreatePosition, CreateReserve, CreateTextureConfig, DepositLiquidity,
    InitRewardSupply, Liquidate, LockCollateral, ProposeConfig, RefreshPosition, RefreshReserve,
    Repay, SetRewardRules, UnlockCollateral, WithdrawLiquidity, WithdrawReward, WriteOffBadDebt,
};
use super_lendy::state::curator::{Curator, CuratorParams};
use super_lendy::state::pool::{Pool, PoolParams};
use super_lendy::state::position::{Position, BORROW_MEMO_LEN, COLLATERAL_MEMO_LEN};
use super_lendy::state::reserve::{
    ConfigProposal, Reserve, ReserveConfig, RewardRules, RESERVE_MODE_NORMAL,
};
use super_lendy::state::texture_cfg::{TextureConfig, TextureConfigParams};
use super_lendy::{SUPER_LENDY_ID, TEXTURE_CONFIG_ID};

use crate::utils::{get_account, price_feed_authority};

pub async fn create_texture_config(
    context: &mut ProgramTestContext,
    owner: &Keypair,
    texture_config: &Keypair,
    params: TextureConfigParams,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let rent = context.banks_client.get_rent().await.expect("get rent");
    let texture_config_lamports = rent.minimum_balance(TextureConfig::SIZE);

    let create_ix = create_account(
        &owner.pubkey(),
        &TEXTURE_CONFIG_ID,
        texture_config_lamports,
        TextureConfig::SIZE as u64,
        &SUPER_LENDY_ID,
    );

    let tx = Transaction::new_signed_with_payer(
        &[
            create_ix,
            CreateTextureConfig {
                owner: owner.pubkey(),
                params,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, owner, texture_config],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn alter_texture_config(
    context: &mut ProgramTestContext,
    owner: &Keypair,
    params: TextureConfigParams,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[AlterTextureConfig {
            owner: owner.pubkey(),
            params,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, owner],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn create_curator(
    context: &mut ProgramTestContext,
    curator: &Keypair,
    admin: &Keypair,
    global_config_owner: &Keypair,
    params: CuratorParams,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let rent = context.banks_client.get_rent().await.expect("get rent");
    let curator_lamports = rent.minimum_balance(Curator::SIZE);

    let create_ix = create_account(
        &admin.pubkey(),
        &curator.pubkey(),
        curator_lamports,
        Curator::SIZE as u64,
        &SUPER_LENDY_ID,
    );

    let tx = Transaction::new_signed_with_payer(
        &[
            create_ix,
            CreateCurator {
                curator: curator.pubkey(),
                global_config_owner: global_config_owner.pubkey(),
                params,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, admin, curator, global_config_owner],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn alter_curator(
    context: &mut ProgramTestContext,
    curator: Pubkey,
    owner: &Keypair,
    params: CuratorParams,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[AlterCurator {
            curator,
            owner: owner.pubkey(),
            params,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, owner],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn create_pool(
    context: &mut ProgramTestContext,
    pool: &Keypair,
    curator_pools_authority: &Keypair,
    curator: Pubkey,
    params: PoolParams,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let rent = context.banks_client.get_rent().await.expect("get rent");
    let pool_lamports = rent.minimum_balance(Pool::SIZE);

    let create_ix = create_account(
        &curator_pools_authority.pubkey(),
        &pool.pubkey(),
        pool_lamports,
        Pool::SIZE as u64,
        &SUPER_LENDY_ID,
    );

    let tx = Transaction::new_signed_with_payer(
        &[
            create_ix,
            CreatePool {
                pool: pool.pubkey(),
                curator_pools_authority: curator_pools_authority.pubkey(),
                curator,
                params,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, pool, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn alter_pool(
    context: &mut ProgramTestContext,
    pool: Pubkey,
    curator_pools_authority: &Keypair,
    curator: Pubkey,
    params: PoolParams,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[AlterPool {
            pool,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator,
            params,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn create_reserve(
    context: &mut ProgramTestContext,
    reserve: &Keypair,
    pool: Pubkey,
    curator_pools_authority: &Keypair,
    curator: Pubkey,
    liquidity_mint: Pubkey,
    market_price_feed: Pubkey,
    params: ReserveConfig,
    reserve_type: u8,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let rent = context.banks_client.get_rent().await.expect("get rent");
    let reserve_lamports = rent.minimum_balance(Reserve::SIZE);

    let create_ix = create_account(
        &curator_pools_authority.pubkey(),
        &reserve.pubkey(),
        reserve_lamports,
        Reserve::SIZE as u64,
        &SUPER_LENDY_ID,
    );

    let liquidity_mint_acc = get_account(&mut context.banks_client, liquidity_mint)
        .await
        .expect("get Mint");
    let liquidity_token_program = liquidity_mint_acc.owner;

    let tx = Transaction::new_signed_with_payer(
        &[
            create_ix,
            CreateReserve {
                reserve: reserve.pubkey(),
                pool,
                curator_pools_authority: curator_pools_authority.pubkey(),
                curator,
                liquidity_mint,
                market_price_feed,
                params,
                reserve_type,
                liquidity_token_program,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, reserve, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn alter_reserve(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    pool: Pubkey,
    curator_pools_authority: &Keypair,
    curator: Pubkey,
    params: ReserveConfig,
    mode: u8,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[AlterReserve {
            reserve,
            pool,
            market_price_feed: params.market_price_feed,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator,
            params,
            mode,
            flash_loans_enabled: 0,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn deposit_liquidity(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    market_price_feed: Pubkey,
    irm: Pubkey,
    authority: &Keypair,
    source_liquidity_wallet: Pubkey,
    destination_lp_wallet: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let liquidity_wallet_acc = get_account(&mut context.banks_client, source_liquidity_wallet)
        .await
        .expect("get wallet");
    let liquidity_token_program = liquidity_wallet_acc.owner;

    let tx = Transaction::new_signed_with_payer(
        &[
            RefreshReserve {
                reserve,
                market_price_feed,
                irm,
            }
            .into_instruction(),
            DepositLiquidity {
                authority: authority.pubkey(),
                source_liquidity_wallet,
                destination_lp_wallet,
                reserve,
                liquidity_mint: liquidity_mint_from_reserve(context, reserve).await?,
                amount,
                liquidity_token_program,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, authority],
        blockhash,
    );
    update_prices(context, &[reserve]).await;
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn withdraw_liquidity(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    market_price_feed: Pubkey,
    irm: Pubkey,
    authority: &Keypair,
    destination_liquidity_wallet: Pubkey,
    source_lp_wallet: Pubkey,
    lp_amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let liquidity_wallet_acc = get_account(&mut context.banks_client, destination_liquidity_wallet)
        .await
        .expect("get wallet");
    let liquidity_token_program = liquidity_wallet_acc.owner;

    let tx = Transaction::new_signed_with_payer(
        &[
            RefreshReserve {
                reserve,
                market_price_feed,
                irm,
            }
            .into_instruction(),
            WithdrawLiquidity {
                authority: authority.pubkey(),
                source_lp_wallet,
                reserve,
                liquidity_mint: liquidity_mint_from_reserve(context, reserve).await?,
                destination_liquidity_wallet,
                lp_amount,
                liquidity_token_program,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, authority],
        blockhash,
    );
    update_prices(context, &[reserve]).await;
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn lock_collateral(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    market_price_feed: Pubkey,
    irm: Pubkey,
    position: Pubkey,
    owner: &Keypair,
    source_lp_wallet: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[
            RefreshReserve {
                reserve,
                market_price_feed,
                irm,
            }
            .into_instruction(),
            LockCollateral {
                position,
                source_lp_wallet,
                owner: owner.pubkey(),
                reserve,
                amount,
                memo: [0; COLLATERAL_MEMO_LEN],
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, owner],
        blockhash,
    );
    update_prices(context, &[reserve]).await;
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn unlock_collateral(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    market_price_feed: Pubkey,
    irm: Pubkey,
    position: Pubkey,
    owner: &Keypair,
    destination_lp_wallet: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[
            RefreshReserve {
                reserve,
                market_price_feed,
                irm,
            }
            .into_instruction(),
            UnlockCollateral {
                position,
                owner: owner.pubkey(),
                reserve,
                amount,
                destination_lp_wallet,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, owner],
        blockhash,
    );
    update_prices(context, &[reserve]).await;
    context.banks_client.process_transaction(tx).await
}

pub async fn create_position(
    context: &mut ProgramTestContext,
    position_kp: &Keypair,
    pool: Pubkey,
    owner: &Keypair,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let rent = context.banks_client.get_rent().await.expect("get rent");
    let init_lamports = rent.minimum_balance(Position::SIZE);

    let create_ix = create_account(
        &owner.pubkey(),
        &position_kp.pubkey(),
        init_lamports,
        Position::SIZE as u64,
        &SUPER_LENDY_ID,
    );

    let tx = Transaction::new_signed_with_payer(
        &[
            create_ix,
            CreatePosition {
                position: position_kp.pubkey(),
                pool,
                owner: owner.pubkey(),
                position_type: 0,
            }
            .into_instruction(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, owner, position_kp],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn refresh_reserves_ix(
    context: &mut ProgramTestContext,
    reserves: &[Pubkey],
) -> Vec<Instruction> {
    let mut ixs = Vec::new();

    for reserve in reserves {
        let reserve_acc = get_account(&mut context.banks_client, *reserve)
            .await
            .expect("get Reserve");
        let unpacked_reserve =
            Reserve::try_from_bytes(&reserve_acc.data).expect("unpacking Reserve");

        let refresh_reserve = RefreshReserve {
            reserve: *reserve,
            market_price_feed: unpacked_reserve.config.market_price_feed,
            irm: unpacked_reserve.config.irm,
        }
        .into_instruction();

        ixs.push(refresh_reserve);
    }

    ixs
}

pub async fn refresh_reserve(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    market_price_feed: Pubkey,
    irm: Pubkey,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("get latest blockhash");

    let ixs = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(800_000),
        RefreshReserve {
            reserve,
            market_price_feed,
            irm,
        }
        .into_instruction(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn refresh_position_ix(
    context: &mut ProgramTestContext,
    position_pubkey: Pubkey,
) -> (Vec<Instruction>, /* all reserves*/ Vec<Pubkey>) {
    let position_acc = get_account(&mut context.banks_client, position_pubkey)
        .await
        .expect("get position");
    let position = Position::try_from_bytes(&position_acc.data).expect("unpacking Position");

    let deposits_reserves: Vec<Pubkey> = position
        .collateral
        .iter()
        .filter_map(|dep| {
            if dep.deposited_amount > 0 {
                Some(dep.deposit_reserve)
            } else {
                None
            }
        })
        .collect();

    let borrows_reserves: Vec<Pubkey> = position
        .borrows
        .iter()
        .filter_map(|bor| {
            if bor.borrowed_amount().unwrap_or_default() > Decimal::ZERO {
                Some(bor.borrow_reserve)
            } else {
                None
            }
        })
        .collect();

    let ix = RefreshPosition {
        position: position_pubkey,
        deposits: deposits_reserves.clone(),
        borrows: borrows_reserves.clone(),
    }
    .into_instruction();

    let mut refresh_deposits = refresh_reserves_ix(context, &deposits_reserves).await;
    let refresh_borrows = refresh_reserves_ix(context, &borrows_reserves).await;

    refresh_deposits.extend(refresh_borrows.iter().cloned());
    refresh_deposits.push(ix);

    let mut all_reserves = deposits_reserves;
    all_reserves.extend(borrows_reserves);

    (refresh_deposits, all_reserves)
}

pub async fn refresh_position(
    context: &mut ProgramTestContext,
    position: Pubkey,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let (mut ixs, reserves) = refresh_position_ix(context, position).await;

    ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(800_000));
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        blockhash,
    );
    update_prices(context, &reserves).await;
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn borrow(
    context: &mut ProgramTestContext,
    position: Pubkey,
    reserve: Pubkey,
    market_price_feed: Pubkey,
    irm: Pubkey,
    pool: Pubkey,
    borrower: &Keypair,
    curator: Pubkey,
    curator_fee_receiver: Pubkey,
    texture_fee_receiver: Pubkey,
    destination_liquidity_wallet: Pubkey,
    amount: u64,
    slippage_limit: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let (mut ixs, mut reserves) = refresh_position_ix(context, position).await;

    ixs.push(
        RefreshReserve {
            reserve,
            market_price_feed,
            irm,
        }
        .into_instruction(),
    );

    ixs.push(
        Borrow {
            position,
            destination_liquidity_wallet,
            curator_fee_receiver,
            borrower: borrower.pubkey(),
            reserve,
            pool,
            curator,
            texture_fee_receiver,
            liquidity_mint: liquidity_mint_from_reserve(context, reserve).await?,
            token_program: spl_token::id(),
            amount,
            slippage_limit,
            memo: [0; BORROW_MEMO_LEN],
        }
        .into_instruction(),
    );

    reserves.push(reserve);
    update_prices(context, &reserves).await;

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer, borrower],
        blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn liquidity_mint_from_reserve(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
) -> Result<Pubkey, BanksClientError> {
    let reserve_account = context
        .banks_client
        .get_account(reserve)
        .await?
        .ok_or(BanksClientError::ClientError("no reserve account"))?;

    let reserve = Reserve::try_from_bytes(&reserve_account.data)
        .map_err(|_| BanksClientError::ClientError("no reserve account"))?;

    Ok(reserve.liquidity.mint)
}

pub async fn repay(
    context: &mut ProgramTestContext,
    position: Pubkey,
    reserve: Pubkey,
    user_authority: &Keypair,
    source_liquidity_wallet: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let (mut ixs, reserves) = refresh_position_ix(context, position).await;

    ixs.push(
        Repay {
            position,
            source_liquidity_wallet,
            reserve,
            amount,
            user_authority: user_authority.pubkey(),
            token_program: spl_token::id(),
            liquidity_mint: liquidity_mint_from_reserve(context, reserve).await?,
        }
        .into_instruction(),
    );

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer, user_authority],
        blockhash,
    );
    update_prices(context, &reserves).await;
    context.banks_client.process_transaction(tx).await
}

pub async fn write_price(
    context: &mut ProgramTestContext,
    price_feed: Pubkey,
    authority: &Keypair,
    price: Decimal,
    price_timestamp: i64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[WritePrice {
            price_feed,
            authority: authority.pubkey(),
            price,
            price_timestamp,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn update_prices(context: &mut ProgramTestContext, reserves: &[Pubkey]) {
    let authority = price_feed_authority();
    let now = Utc::now().timestamp();
    for reserve in reserves {
        let reserve_acc = get_account(&mut context.banks_client, *reserve)
            .await
            .expect("get Reserve");
        let unpacked_reserve =
            Reserve::try_from_bytes(&reserve_acc.data).expect("unpacking Reserve");
        let price_feed_key = unpacked_reserve.config.market_price_feed;
        let price_feed_acc = get_account(&mut context.banks_client, price_feed_key)
            .await
            .expect("get PriceFeed");
        let price_feed =
            PriceFeed::try_from_bytes(&price_feed_acc.data).expect("unpacking PriceFeed");
        write_price(
            context,
            price_feed_key,
            &authority,
            price_feed.try_price().unwrap(),
            now - 3, // -3 is to avoid appearing in the future
        )
        .await
        .expect("update price feed")
    }
}

pub async fn claim_curator_performance_fees(
    context: &mut ProgramTestContext,
    curator: Pubkey,
    reserve: Pubkey,
    pool: Pubkey,
    fee_receiver: Pubkey,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let mut ixs = refresh_reserves_ix(context, &[reserve]).await;

    ixs.push(
        ClaimCuratorPerformanceFees {
            reserve,
            pool,
            curator,
            fee_receiver,
            liquidity_mint: liquidity_mint_from_reserve(context, reserve).await?,
            token_program: spl_token::id(),
        }
        .into_instruction(),
    );

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        blockhash,
    );
    update_prices(context, &[reserve]).await;
    context.banks_client.process_transaction(tx).await
}

pub async fn claim_texture_performance_fees(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    fee_receiver: Pubkey,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let mut ixs = refresh_reserves_ix(context, &[reserve]).await;

    ixs.push(
        ClaimTexturePerformanceFees {
            reserve,
            fee_receiver,
            liquidity_mint: liquidity_mint_from_reserve(context, reserve).await?,
            token_program: spl_token::id(),
        }
        .into_instruction(),
    );

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        blockhash,
    );
    update_prices(context, &[reserve]).await;
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn liquidate(
    context: &mut ProgramTestContext,
    repayment_source_wallet: Pubkey,
    destination_lp_wallet: Pubkey,
    principal_reserve: Pubkey,
    collateral_reserve: Pubkey,
    position: Pubkey,
    liquidator: &Keypair,
    liquidity_amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let (mut ixs, reserves) = refresh_position_ix(context, position).await;

    ixs.push(
        Liquidate {
            repayment_source_wallet,
            destination_lp_wallet,
            principal_reserve,
            collateral_reserve,
            position,
            liquidator: liquidator.pubkey(),
            principal_reserve_liquidity_mint: liquidity_mint_from_reserve(
                context,
                principal_reserve,
            )
            .await?,
            principal_token_program: spl_token::id(),
            liquidity_amount,
        }
        .into_instruction(),
    );

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer, liquidator],
        blockhash,
    );
    update_prices(context, &reserves).await;
    context.banks_client.process_transaction(tx).await
}

pub async fn write_off_bad_debt(
    context: &mut ProgramTestContext,
    position: Pubkey,
    pool: Pubkey,
    curator: Pubkey,
    curator_pools_authority: &Keypair,
    reserve: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let (mut ixs, reserves) = refresh_position_ix(context, position).await;

    ixs.push(
        WriteOffBadDebt {
            pool,
            position,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator,
            reserve,
            amount,
        }
        .into_instruction(),
    );

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    update_prices(context, &reserves).await;
    context.banks_client.process_transaction(tx).await
}

pub async fn set_reward_rules(
    context: &mut ProgramTestContext,
    reserve: Pubkey,
    pool: Pubkey,
    curator: Pubkey,
    curator_pools_authority: &Keypair,
    rules: RewardRules,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let reward_mints = rules.rules.iter().map(|rule| rule.reward_mint).collect();

    let tx = Transaction::new_signed_with_payer(
        &[SetRewardRules {
            reserve,
            pool,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator,
            reward_mints,
            rules,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn init_reward_supply(
    context: &mut ProgramTestContext,
    reward_mint: Pubkey,
    pool: Pubkey,
    curator: Pubkey,
    curator_pools_authority: &Keypair,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[InitRewardSupply {
            reward_mint,
            pool,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator,
            token_program: spl_token::id(),
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn claim_reward(
    context: &mut ProgramTestContext,
    reward_mint: Pubkey,
    pool: Pubkey,
    position: Pubkey,
    position_owner: &Keypair,
    destination_wallet: Pubkey,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[ClaimReward {
            position,
            destination_wallet,
            reward_mint,
            pool,
            position_owner: position_owner.pubkey(),
            token_program: spl_token::id(),
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, position_owner],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn withdraw_reward(
    context: &mut ProgramTestContext,
    reward_mint: Pubkey,
    pool: Pubkey,
    curator: Pubkey,
    curator_pools_authority: &Keypair,
    destination_wallet: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[WithdrawReward {
            destination_wallet,
            reward_mint,
            pool,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator,
            token_program: spl_token::id(),
            amount,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn enable_flash_loans(
    context: &mut ProgramTestContext,
    reserve_keys: Vec<Pubkey>,
    pool_pubkey: Pubkey,
    curator_pubkey: Pubkey,
    curator_pools_authority: &Keypair,
) -> Result<(), BanksClientError> {
    let mut ixs = vec![];
    for reserve_key in reserve_keys {
        let reserve_acc = get_account(&mut context.banks_client, reserve_key)
            .await
            .expect("get position");
        let reserve = Reserve::try_from_bytes(&reserve_acc.data).expect("cast reserve data");
        let params = reserve.config;

        ixs.push(
            AlterReserve {
                reserve: reserve_key,
                pool: pool_pubkey,
                market_price_feed: params.market_price_feed,
                curator_pools_authority: curator_pools_authority.pubkey(),
                curator: curator_pubkey,
                params,
                mode: RESERVE_MODE_NORMAL,
                flash_loans_enabled: 1,
            }
            .into_instruction(),
        )
    }

    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

#[allow(clippy::too_many_arguments)]
pub async fn propose_config(
    context: &mut ProgramTestContext,
    pool_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    market_price_feed_pubkey: Pubkey,
    curator_pubkey: Pubkey,
    curator_pools_authority: &Keypair,
    index: u8,
    proposal: ConfigProposal,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let tx = Transaction::new_signed_with_payer(
        &[ProposeConfig {
            reserve: reserve_pubkey,
            pool: pool_pubkey,
            market_price_feed: market_price_feed_pubkey,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator: curator_pubkey,
            index,
            proposal,
        }
        .into_instruction()],
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

pub async fn apply_proposal(
    context: &mut ProgramTestContext,
    pool_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    market_price_feed_pubkey: Pubkey,
    curator_pubkey: Pubkey,
    curator_pools_authority: &Keypair,
    index: u8,
) -> Result<(), BanksClientError> {
    let blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .expect("get latest blockhash");

    let mut ixs = refresh_reserves_ix(context, &[reserve_pubkey]).await;
    ixs.push(
        ApplyConfigProposal {
            reserve: reserve_pubkey,
            pool: pool_pubkey,
            market_price_feed: market_price_feed_pubkey,
            curator_pools_authority: curator_pools_authority.pubkey(),
            curator: curator_pubkey,
            index,
        }
        .into_instruction(),
    );

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer, curator_pools_authority],
        blockhash,
    );
    update_prices(context, &[reserve_pubkey]).await;
    context.banks_client.process_transaction(tx).await
}
