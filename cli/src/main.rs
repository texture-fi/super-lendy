use std::path::PathBuf;

use anyhow::anyhow;
use bytemuck::Zeroable;
use derive_more::FromStr;
use price_proxy::state::utils::str_to_array;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use structopt::StructOpt;

use sup::app::{read_lut_config, App};
use super_lendy::pda::find_lp_token_mint;
use super_lendy::state::curator::{CuratorParams, CURATOR_NAME_MAX_LEN};
use super_lendy::state::pool::{PoolParams, CURRENCY_SYMBOL_MAX_LEN, POOL_NAME_MAX_LEN};
use super_lendy::state::reserve::{
    ReserveConfig, ReserveFeesConfig, RESERVE_TYPE_NORMAL, RESERVE_TYPE_NOT_A_COLLATERAL,
    RESERVE_TYPE_PROTECTED_COLLATERAL,
};
use super_lendy::state::texture_cfg::{ReserveTimelock, TextureConfigParams};

#[derive(FromStr)]
struct KeypairPath(PathBuf);

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct SuperLendyArgs {
    /// URL of RPC Solana interface.
    #[structopt(
        long,
        short,
        default_value = "http://localhost:8899",
        env = "SOLANA_RPC"
    )]
    pub url: String,

    /// Keypair to use for signing instructions and to define Lender / Borrower role.
    #[structopt(long, short = "k", default_value)]
    pub authority: KeypairPath,

    /// When provided Sup will produce Base58 TX on std out. Payer in TX and authority (e.g. curator's
    /// authority) will be set to provided Pubkey. Produced Base58 string can be copied in to Squads and
    /// signed by Multisig. If it is proper authority (e.g. curator of the pool) then TX will be executed.
    #[structopt(long, short = "m")]
    pub multisig: Option<Pubkey>,

    /// Priority fee in microlamports. For priority_rate=1 you pay 0.2 (1) priority lamports for one ix, for 10_000 - 2_000.
    #[structopt(long)]
    priority_fee: Option<u64>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    /// Done once. Who call this command becomes Global Texture Config owner. So call it as Rooty
    CreateTextureConfig {
        /// Keypair for gLoBanTpd5VuvyCpYjvYNudFREwLqFy418fGuuXUJfX
        #[structopt(long)]
        keypair: KeypairPath,
        /// Texture performance fees for all pools/reserves
        #[structopt(long)]
        performance_fee_rate_bps: u16,
        /// Texture loan origination fees for all pools/reserves
        #[structopt(long)]
        borrow_fee_rate_bps: u16,
        #[structopt(long)]
        fees_authority: Pubkey,
        #[structopt(long)]
        market_price_feed_lock_sec: u32,
        #[structopt(long)]
        irm_lock_sec: u32,
        #[structopt(long)]
        liquidation_bonus_lock_sec: u32,
        #[structopt(long)]
        unhealthy_ltv_lock_sec: u32,
        #[structopt(long)]
        partial_liquidation_factor_lock_sec: u32,
        #[structopt(long)]
        max_total_liquidity_lock_sec: u32,
        #[structopt(long)]
        max_borrow_ltv_lock_sec: u32,
        #[structopt(long)]
        max_borrow_utilization_lock_sec: u32,
        #[structopt(long)]
        price_stale_threshold_lock_sec: u32,
        #[structopt(long)]
        max_withdraw_utilization_lock_sec: u32,
        #[structopt(long)]
        fees_lock_sec: u32,
    },
    /// Change global Texture config. Only config owner allowed to do this.
    AlterTextureConfig {
        /// Texture performance fees for all pools/reserves
        #[structopt(long)]
        performance_fee_rate_bps: Option<u16>,
        /// Texture loan origination fees for all pools/reserves
        #[structopt(long)]
        borrow_fee_rate_bps: Option<u16>,
        #[structopt(long)]
        performance_fee_authority: Option<Pubkey>,
        #[structopt(long)]
        market_price_feed_lock_sec: Option<u32>,
        #[structopt(long)]
        irm_lock_sec: Option<u32>,
        #[structopt(long)]
        liquidation_bonus_lock_sec: Option<u32>,
        #[structopt(long)]
        unhealthy_ltv_lock_sec: Option<u32>,
        #[structopt(long)]
        partial_liquidation_factor_lock_sec: Option<u32>,
        #[structopt(long)]
        max_total_liquidity_lock_sec: Option<u32>,
        #[structopt(long)]
        max_borrow_ltv_lock_sec: Option<u32>,
        #[structopt(long)]
        max_borrow_utilization_lock_sec: Option<u32>,
        #[structopt(long)]
        price_stale_threshold_lock_sec: Option<u32>,
        #[structopt(long)]
        max_withdraw_utilization_lock_sec: Option<u32>,
        #[structopt(long)]
        fees_lock_sec: Option<u32>,
    },
    /// Transfer Texture Global Config ownership to new authority. This command must be executed
    /// with authority of current Config owner. New authority also must sign thus it should be
    /// keypair.
    TransferTextureConfigOwnership {
        /// Owner address to set for Curator account
        #[structopt(long)]
        new_owner: KeypairPath,
    },
    /// Create Curator account. Must be called under authority of Global Texture config owner.
    CreateCurator {
        /// Curator's name
        #[structopt(long)]
        name: String,
        #[structopt(long)]
        logo_url: String,
        #[structopt(long)]
        website_url: String,
        /// Owner address to set for Curator account
        #[structopt(long)]
        owner: Pubkey,
        /// Curator's fee authority
        #[structopt(long)]
        fees_authority: Pubkey,
        /// Curator's Pools authority - can create Pools, Reserves and configure them
        #[structopt(long)]
        pools_authority: Pubkey,
        /// Curator's Vaults authority - can create Vaults and configure them
        #[structopt(long)]
        vaults_authority: Pubkey,
    },
    /// Change existing Curator account. Must be called by owner of the Curator account.
    AlterCurator {
        /// Curator account to change
        #[structopt(long)]
        curator: Pubkey,
        /// Curator's name
        #[structopt(long)]
        name: Option<String>,
        #[structopt(long)]
        logo_url: Option<String>,
        #[structopt(long)]
        website_url: Option<String>,
        /// Current owner may reassign ownership to someone else
        #[structopt(long)]
        owner: Option<Pubkey>,
        /// Curator's fee authority
        #[structopt(long)]
        fees_authority: Option<Pubkey>,
        /// Curator's Pools authority - can create Pools, Reserves and configure them
        #[structopt(long)]
        pools_authority: Option<Pubkey>,
        /// Curator's Vaults authority - can create Vaults and configure them
        #[structopt(long)]
        vaults_authority: Option<Pubkey>,
    },
    Curators,
    /// Show global Texture config
    TextureConfig,
    /// Creates Pool account. Must be called with curator.pools_authority authority
    CreatePool {
        /// Pool name
        #[structopt(long)]
        name: String,
        /// Market price currency symbol
        #[structopt(long)]
        market_price_currency_symbol: String,
        /// Curator account
        #[structopt(long)]
        curator: Pubkey,
    },
    /// Changes specified existing Pool account. Must be called with curator.pools_authority authority
    AlterPool {
        /// Address of the Pool to change
        #[structopt(long)]
        pool: Pubkey,
        /// Pool name
        #[structopt(long)]
        name: Option<String>,
        /// Market price currency symbol
        #[structopt(long)]
        market_price_currency_symbol: Option<String>,

        #[structopt(long)]
        visible: Option<bool>,
    },
    /// Print all configured pairs
    Pools {
        /// Curator account owned the Pool.
        #[structopt(long)]
        curator: Option<Pubkey>,
        /// Pool address
        #[structopt(long)]
        pool: Option<Pubkey>,
    },
    /// Get contract version
    ContractVersion {},
    /// Command must be run as Curator.pools_authority
    CreateReserve {
        /// Curator account which will own the Pool.
        #[structopt(long)]
        curator: Pubkey,
        /// Pool address to create Reserve for
        #[structopt(long)]
        pool: Pubkey,
        /// Creates Reserve to serve as protected collateral. Disable borrows from that Reserve FOREVER!
        #[structopt(long)]
        protected_collateral: bool,
        /// This reserve can only be used to Borrow but NOT as collateral.
        #[structopt(long)]
        not_a_collateral: bool,
        /// Mint address of the token which will be used as Liquidity in created reserve
        #[structopt(long)]
        liquidity_mint: Pubkey,
        /// PriceProxy account which gives market price for liquidity_mint tokens
        #[structopt(long)]
        market_price_feed: Pubkey,
        /// Interest rate model (IRM) account
        #[structopt(long)]
        irm: Pubkey,
        /// Bonus a liquidator gets when repaying part of an unhealthy position, as a basis points - bps (0.01%)
        #[structopt(long)]
        liquidation_bonus_bps: u16,
        /// Loan to value ratio at which position can be liquidated via partial liquidation.
        #[structopt(long)]
        partly_unhealthy_ltv_bps: u16,
        /// Collateral percentage (in basis points) which can be liquidated in one operation
        #[structopt(long)]
        partial_liquidation_factor_bps: u16,
        /// LTV after which full liquidation is allowed
        #[structopt(long)]
        fully_unhealthy_ltv_bps: u16,
        #[structopt(long)]
        curator_borrow_fee_bps: u16,
        #[structopt(long)]
        curator_performance_fee_bps: u16,
        /// Max LTV (of some position) till which this reserve will give to borrow when used as collateral.
        #[structopt(long)]
        max_borrow_ltv_bps: u16,
        /// Max utilization after which this reserve stops giving borrows. Though it is possible to withdraw
        /// liquidity from it making utilization even higher.
        #[structopt(long)]
        max_borrow_utilization_bps: u16,
        /// Max utilization after which this reserve stops withdrawls.
        #[structopt(long)]
        max_withdraw_utilization_bps: u16,
        /// Max liquidity Reserve can accept and hold
        #[structopt(long)]
        max_total_liquidity: u64,
        /// Maximum market price age (in seconds) to be accepted by the contract.
        #[structopt(long, default_value = "1")]
        price_stale_threshold_sec: u32,
    },
    Reserves {
        /// Reserve address
        #[structopt(long)]
        reserve: Option<Pubkey>,
        /// Pool that reserve belongs to
        #[structopt(long)]
        pool: Option<Pubkey>,
        /// Reserve liquidity mint address
        #[structopt(long)]
        mint: Option<Pubkey>,
    },
    AlterReserve {
        /// Reserve to change
        #[structopt(long)]
        reserve: Pubkey,
        /// PriceProxy account which gives market price for liquidity_mint tokens
        #[structopt(long)]
        market_price_feed: Option<Pubkey>,
        /// Interest rate model (IRM) account
        #[structopt(long)]
        irm: Option<Pubkey>,
        /// Bonus a liquidator gets when repaying part of an unhealthy position, as a basis points - bps (0.01%)
        #[structopt(long)]
        liquidation_bonus_bps: Option<u16>,
        /// Loan to value ratio at which position can be liquidated via partial liquidation.
        #[structopt(long)]
        partly_unhealthy_ltv_bps: Option<u16>,
        /// Collateral percentage (in basis points) which can be liquidated in one operation
        #[structopt(long)]
        partial_liquidation_factor_bps: Option<u16>,
        /// LTV after which full liquidation is allowed
        #[structopt(long)]
        fully_unhealthy_ltv_bps: Option<u16>,
        #[structopt(long)]
        curator_borrow_fee_bps: Option<u16>,
        #[structopt(long)]
        curator_performance_fee_bps: Option<u16>,
        /// Max utilization after which this pool stops giving borrows. Though it is possible to withdraw
        /// liquidity from it making utilization even bigger.
        #[structopt(long)]
        max_borrow_utilization_bps: Option<u16>,
        /// Max utilization after which this reserve stops withdrawls.
        #[structopt(long)]
        max_withdraw_utilization_bps: Option<u16>,
        /// Max liquidity Reserve can accept and hold
        #[structopt(long)]
        max_total_liquidity: Option<u64>,
        /// Max LTV (of some position) till which this reserve will give to borrow when used as collateral.
        #[structopt(long)]
        max_borrow_ltv_bps: Option<u16>,
        /// Maximum market price age (in seconds) to be accepted by the contract.
        #[structopt(long)]
        price_stale_threshold_sec: Option<u32>,
        /// 0 - RESERVE_MODE_NORMAL - enable all Reserve functionality
        /// 1 - RESERVE_MODE_BORROW_DISABLED - disable Borrow
        /// 2 - RESERVE_MODE_RETAIN_LIQUIDITY - disable Borrow, Unlock, Withdraw
        #[structopt(long)]
        mode: Option<u8>,
        /// Enables/disables flash loans for that Reserve
        #[structopt(long)]
        flash_loans_enabled: Option<bool>,
    },
    DeleteReserve {
        /// Reserve to delete
        #[structopt(long)]
        reserve: Pubkey,
    },
    RefreshReserve {
        /// Reserve to refresh
        #[structopt(long)]
        reserve: Pubkey,
    },
    ProposeConfig {
        /// Reserve to change
        #[structopt(long)]
        reserve: Pubkey,
        /// Index to store this proposal to
        #[structopt(long)]
        index: u8,
        /// PriceProxy account which gives market price for liquidity_mint tokens
        #[structopt(long)]
        market_price_feed: Option<Pubkey>,
        /// Interest rate model (IRM) account
        #[structopt(long)]
        irm: Option<Pubkey>,
        /// Bonus a liquidator gets when repaying part of an unhealthy position, as a basis points - bps (0.01%)
        #[structopt(long)]
        liquidation_bonus_bps: Option<u16>,
        /// Loan to value ratio at which position can be liquidated via partial liquidation.
        #[structopt(long)]
        partly_unhealthy_ltv_bps: Option<u16>,
        /// Collateral percentage (in basis points) which can be liquidated in one operation
        #[structopt(long)]
        partial_liquidation_factor_bps: Option<u16>,
        /// LTV after which full liquidation is allowed
        #[structopt(long)]
        fully_unhealthy_ltv_bps: Option<u16>,
        #[structopt(long)]
        curator_borrow_fee_bps: Option<u16>,
        #[structopt(long)]
        curator_performance_fee_bps: Option<u16>,
        /// Max utilization after which this pool stops giving borrows. Though it is possible to withdraw
        /// liquidity from it making utilization even bigger.
        #[structopt(long)]
        max_borrow_utilization_bps: Option<u16>,
        /// Max utilization after which this reserve stops withdrawls.
        #[structopt(long)]
        max_withdraw_utilization_bps: Option<u16>,
        /// Max liquidity Reserve can accept and hold
        #[structopt(long)]
        max_total_liquidity: Option<u64>,
        /// Max LTV (of some position) till which this reserve will give to borrow when used as collateral.
        #[structopt(long)]
        max_borrow_ltv_bps: Option<u16>,
        /// Maximum market price age (in seconds) to be accepted by the contract.
        #[structopt(long)]
        price_stale_threshold_sec: Option<u32>,
    },
    /// Clear (delete, deactivate) config change proposal
    ClearConfigProposal {
        /// Reserve
        #[structopt(long)]
        reserve: Pubkey,
        /// Index of the proposal to clear
        #[structopt(long)]
        index: u8,
    },
    ApplyConfigProposal {
        /// Reserve to apply proposal for
        #[structopt(long)]
        reserve: Pubkey,
        /// Proposal index (see at the output of `reserves` subcommand)
        #[structopt(long)]
        index: u8,
    },
    /// Deposit liquidity in to Reserve
    Deposit {
        /// Reserve to deposit to
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount of liquidity (in smallest units) to deposit
        #[structopt(long)]
        amount: u64,
    },
    /// Withdraw liquidity from Reserve
    Withdraw {
        /// Reserve to withdraw from
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount of LP tokens (in smallest units) to change for liquidity
        #[structopt(long)]
        lp_amount: u64,
    },
    /// Create user position. Required before locking collateral and borrowing.
    CreatePosition {
        /// Pool to create user position in
        #[structopt(long)]
        pool: Pubkey,
        /// Create position of `long-short` type - change UI interpretation of it
        #[structopt(long)]
        long_short: bool,
    },
    ClosePosition {
        /// Position to close
        #[structopt(long)]
        position: Option<Pubkey>,
        /// Close all position by pool
        #[structopt(long)]
        pool: Option<Pubkey>,
    },
    /// Refresh position (all operational values). Also refreshes relevant Reserves based on fresh
    /// market prices.
    RefreshPosition {
        /// Position to update
        #[structopt(long)]
        position: Pubkey,
    },
    /// List all existing users positions
    Positions {
        /// Position address
        #[structopt(long)]
        position: Option<Pubkey>,
        /// Pool address
        #[structopt(long)]
        pool: Option<Pubkey>,
        /// Position owner
        #[structopt(long)]
        owner: Option<Pubkey>,
    },
    /// Place LP tokens as collateral
    LockCollateral {
        /// Position to lock collateral on
        #[structopt(long)]
        position: Pubkey,
        /// Reserve key to lock collateral from (i.e. use LP tokens from given Reserve as collateral)
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount of LP tokens to lock as collateral
        #[structopt(long)]
        amount: u64,
        /// Memo data which is stored in to Collateral record
        #[structopt(long)]
        memo: Option<String>,
    },
    /// Get back LP tokens previously locked as collateral
    UnlockCollateral {
        /// Position to unlock collateral on
        #[structopt(long)]
        position: Pubkey,
        /// Reserve key to unlock collateral from
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount of LP tokens to unlock
        #[structopt(long)]
        amount: u64,
    },
    /// Borrow liquidity against previously deposited collateral
    Borrow {
        /// Position to borrow to
        #[structopt(long)]
        position: Pubkey,
        /// Reserve key to borrow liquidity from
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount of liquidity tokens to borrow. Default value is u64::MAX which means - use max
        /// available borrowing power.
        #[structopt(long, default_value = "u64::MAX")]
        amount: u64,
        /// Slippage - min. tokens amount borrower wants to receive when using max borrowing power
        #[structopt(long, default_value = "1")]
        slippage: u64,
        /// Memo data which is stored in to BorrowedLiquidity record
        #[structopt(long)]
        memo: Option<String>,
    },
    /// Repay debt
    Repay {
        /// Position to repay to
        #[structopt(long)]
        position: Pubkey,
        /// Reserve key to repay liquidity to
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount of liquidity tokens to repay. If not specified then full repay for the Reserve will be done.
        #[structopt(long)]
        amount: Option<u64>,
    },
    /// Claim Curator's performance fees. Anyone can call this command.
    ClaimCuratorFee {
        /// Reserve claim fees from
        #[structopt(long)]
        reserve: Pubkey,
    },
    /// Claim Texture performance fee. Authority from GlobalConfig.performance_fee_authority must sign.
    ClaimTextureFee {
        /// Reserve claim fees from
        #[structopt(long)]
        reserve: Pubkey,
    },
    /// Liquidate unhealthy position
    Liquidate {
        /// Position to liquidate
        #[structopt(long)]
        position: Pubkey,
        /// Principal reserve - to repay part of the debt. One of position borrowings.
        #[structopt(long)]
        principal_reserve: Pubkey,
        /// Collateral reserve to get collateral from. One of position collateral deposits.
        #[structopt(long)]
        collateral_reserve: Pubkey,
        /// Principal amount to liquidate.
        #[structopt(long)]
        principal_amount: Option<u64>,
    },
    /// Write off (i.e. make it acknowledged loss of the whole Reserve) bad debt.
    /// Call it with Curator / Pool owner authority.
    WriteOffBadDebt {
        /// Position to write off bad debt from
        #[structopt(long)]
        position: Pubkey,
        /// Principal reserve to write off debt in. One of position borrowings.
        #[structopt(long)]
        reserve: Pubkey,
        /// Amount in principal token to write off.
        #[structopt(long)]
        amount: Option<u64>,
    },
    /// Init SPL token account to hold reward tokens for given pool. Call it from pools mgmt authority.
    InitRewardSupply {
        /// Pool to init reward supply for
        #[structopt(long)]
        pool: Pubkey,
        #[structopt(long)]
        reward_mint: Pubkey,
    },
    /// Set reward rule in a particular Reserve
    SetRewardRule {
        /// Reserve to set reward rule in
        #[structopt(long)]
        reserve: Pubkey,
        /// Rule index (starting from 0) to set
        #[structopt(long)]
        index: usize,
        /// Rule name
        #[structopt(long)]
        name: String,
        /// Reward token mint
        #[structopt(long)]
        reward_mint: Pubkey,
        /// Set it is you want rule to be applied to deposits
        #[structopt(long)]
        deposits: bool,
        /// Set it is you want rule to be applied to borrows
        #[structopt(long)]
        borrows: bool,
        /// This is amount of reward token to give per each deposited/borrowed liquidity token per slot
        /// E.g. value 0.001 means that per each deposited token User will earn 1 reward token each
        /// 1000 slots. So if he deposit 1 token and will hold this deposit 2000 slots he will earn 2
        /// reward tokens.
        #[structopt(long)]
        rate: f64,
    },
    /// Claim reward.
    ClaimReward {
        /// Position to claim rewards from
        #[structopt(long)]
        position: Pubkey,
        /// Pool (translates to user position under the hood) to claim reward from
        #[structopt(long)]
        pool: Pubkey,
        /// Mint of the reward token to claim
        #[structopt(long)]
        reward_mint: Pubkey,
    },
    /// Prints address of reward supply SPL wallet
    RewardSupplyAddr {
        /// Pool
        #[structopt(long)]
        pool: Pubkey,
        /// Mint of the reward token
        #[structopt(long)]
        reward_mint: Pubkey,
    },
    /// Used to deposit more reward tokens on contract managed rewards supply
    DepositReward {
        /// Pool
        #[structopt(long)]
        pool: Pubkey,
        /// Mint of the reward token
        #[structopt(long)]
        reward_mint: Pubkey,
        #[structopt(long)]
        amount: u64,
    },
    /// Used to withdraw reward tokens from reward supply. Call it with Pool curator authority
    WithdrawReward {
        /// Pool
        #[structopt(long)]
        pool: Pubkey,
        /// Mint of the reward token
        #[structopt(long)]
        reward_mint: Pubkey,
        #[structopt(long)]
        amount: u64,
    },
    /// Prints all existing SPL token account with rewards for given Pool
    RewardsBalances {
        /// Pool
        #[structopt(long)]
        pool: Pubkey,
    },
    /// Prints Liquidity Provider's token mint for given Reserve
    LpMint {
        #[structopt(long)]
        reserve: Pubkey,
    },
    /// Get FlashLoan and put borrowed money on to wallet A while repaying from ATA wallet. Wallet A
    /// created automatically by this command.
    FlashTest {
        #[structopt(long)]
        reserve: Pubkey,
        #[structopt(long)]
        amount: u64,
    },
    PutTokenStandard {
        #[structopt(long)]
        input_cfg: String,
        #[structopt(long)]
        out_cfg: String,
    },
    ExecuteMultisig {
        /// Multisig authority to be used in TX signing
        #[structopt(long)]
        multisig: KeypairPath,
        /// Base58 coded TX body
        #[structopt(long)]
        tx: String,
    },
    /// Creates and updates LUTs. Gives current LUT config as input and produces
    /// updated one.
    MakeLut {
        /// Path to current config file
        #[structopt(long)]
        config: Option<String>,
    },
    /// Show info and addresses from LUT
    Lut {
        #[structopt(long)]
        lut: Pubkey,
    },
    /// Generate unhealthy positions to liquidate
    GenUnhealthyPositions {
        #[structopt(long)]
        position_config: String,
        #[structopt(long)]
        price_feed_authority: KeypairPath,
    },
    /// Creates all ATA SPL Token accounts for specified `curator` account. This is NOT curator authority
    /// but Curator account owned by SuperLendy and listed by `curators` command.
    MakeFeesAta {
        #[structopt(long)]
        curator: Pubkey,
    },
    /// The command creates/updates metadata for LP token in Solana and generates json metadata file.
    /// Call it with Curator Pools Authority
    SetLpMetadata {
        /// Reserve address to set LP metadata for
        #[structopt(long)]
        reserve: Pubkey,
        /// Name of the LP token (will be shown in wallets)
        #[structopt(long)]
        name: String,
        /// Short symbol of LP token (may be used by trading platforms)
        #[structopt(long)]
        symbol: String,
        /// URI of the metadata json file. Defaults to https://texture.finance/cimg/tokens/{}.svg
        #[structopt(long)]
        uri: Option<String>,
    },
    /// Shows LP metadata for all SuperLendy LP tokens. Output may be narrowed down to one `reserve`
    LpMetadata {
        /// Reserve address to view LP metadata
        #[structopt(long)]
        reserve: Option<Pubkey>,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opt: SuperLendyArgs = SuperLendyArgs::from_args();

    let keypair = read_keypair_file(opt.authority.0)
        .map_err(|err| anyhow!("reading authority keypair: {}", err))
        .unwrap();
    let client = RpcClient::new_with_commitment(opt.url.clone(), CommitmentConfig::confirmed());

    let app = App {
        rpc: client,
        url: opt.url,
        authority: keypair,
        priority_fee: opt.priority_fee,
        multisig: opt.multisig,
    };

    match opt.cmd {
        Command::CreateTextureConfig {
            keypair: global_cfg_keypair,
            performance_fee_rate_bps,
            borrow_fee_rate_bps,
            fees_authority,
            market_price_feed_lock_sec,
            irm_lock_sec,
            liquidation_bonus_lock_sec,
            unhealthy_ltv_lock_sec,
            partial_liquidation_factor_lock_sec,
            max_total_liquidity_lock_sec,
            max_borrow_ltv_lock_sec,
            max_borrow_utilization_lock_sec,
            price_stale_threshold_lock_sec,
            max_withdraw_utilization_lock_sec,
            fees_lock_sec,
        } => {
            let params = TextureConfigParams {
                borrow_fee_rate_bps,
                performance_fee_rate_bps,
                fees_authority,
                reserve_timelock: ReserveTimelock {
                    market_price_feed_lock_sec,
                    irm_lock_sec,
                    liquidation_bonus_lock_sec,
                    unhealthy_ltv_lock_sec,
                    partial_liquidation_factor_lock_sec,
                    max_total_liquidity_lock_sec,
                    max_borrow_ltv_lock_sec,
                    max_borrow_utilization_lock_sec,
                    price_stale_threshold_lock_sec,
                    max_withdraw_utilization_lock_sec,
                    fees_lock_sec,
                    _padding: 0,
                },
            };

            let keypair = read_keypair_file(global_cfg_keypair.0)
                .map_err(|err| anyhow!("reading global_cfg_keypair keypair: {}", err))
                .unwrap();

            app.create_texture_config(params, keypair).await;
        }
        Command::TransferTextureConfigOwnership { new_owner } => {
            let keypair = read_keypair_file(new_owner.0)
                .map_err(|err| anyhow!("reading new_owner keypair: {}", err))
                .unwrap();

            app.transfer_texture_config_ownership(keypair).await;
        }
        Command::AlterTextureConfig {
            performance_fee_rate_bps,
            borrow_fee_rate_bps,
            performance_fee_authority,
            market_price_feed_lock_sec,
            irm_lock_sec,
            liquidation_bonus_lock_sec,
            unhealthy_ltv_lock_sec,
            partial_liquidation_factor_lock_sec,
            max_total_liquidity_lock_sec,
            max_borrow_ltv_lock_sec,
            max_borrow_utilization_lock_sec,
            price_stale_threshold_lock_sec,
            max_withdraw_utilization_lock_sec,
            fees_lock_sec,
        } => {
            app.alter_texture_config(
                performance_fee_authority,
                performance_fee_rate_bps,
                borrow_fee_rate_bps,
                market_price_feed_lock_sec,
                irm_lock_sec,
                liquidation_bonus_lock_sec,
                unhealthy_ltv_lock_sec,
                partial_liquidation_factor_lock_sec,
                max_total_liquidity_lock_sec,
                max_borrow_ltv_lock_sec,
                max_borrow_utilization_lock_sec,
                price_stale_threshold_lock_sec,
                max_withdraw_utilization_lock_sec,
                fees_lock_sec,
            )
            .await;
        }
        Command::TextureConfig => app.show_texture_config().await,
        Command::CreateCurator {
            name,
            logo_url,
            website_url,
            owner,
            fees_authority,
            pools_authority,
            vaults_authority,
        } => {
            if name.len() > CURATOR_NAME_MAX_LEN {
                println!(
                    "Curator name is too long. {} symbols max.",
                    CURATOR_NAME_MAX_LEN
                );
                return;
            }

            let to_zero_padded = |str: String| {
                let mut bytes_zero_ended = [0; CURATOR_NAME_MAX_LEN];
                let bytes = str.as_bytes();
                bytes_zero_ended[..bytes.len()].copy_from_slice(bytes);
                bytes_zero_ended
            };

            let params = CuratorParams {
                owner,
                fees_authority,
                pools_authority,
                vaults_authority,
                name: to_zero_padded(name),
                logo_url: to_zero_padded(logo_url),
                website_url: to_zero_padded(website_url),
            };

            app.create_curator(params).await
        }
        Command::Curators => app.list_curators().await,
        Command::AlterCurator {
            curator,
            name,
            logo_url,
            website_url,
            owner,
            fees_authority,
            pools_authority,
            vaults_authority,
        } => {
            app.alter_curator(
                curator,
                name,
                logo_url,
                website_url,
                owner,
                fees_authority,
                pools_authority,
                vaults_authority,
            )
            .await
        }
        Command::CreatePool {
            name,
            market_price_currency_symbol,
            curator,
        } => {
            if name.len() > POOL_NAME_MAX_LEN {
                println!("Pool name is too long. {} max.", POOL_NAME_MAX_LEN);
                return;
            }

            if market_price_currency_symbol.len() > CURRENCY_SYMBOL_MAX_LEN {
                println!(
                    "Currency symbol is too long. {} max.",
                    CURRENCY_SYMBOL_MAX_LEN
                );
                return;
            }

            let params = PoolParams {
                name: str_to_array(&name),
                market_price_currency_symbol: str_to_array(&market_price_currency_symbol),
                visible: 0,
            };

            app.create_pool(curator, params).await;
        }
        Command::AlterPool {
            pool,
            name,
            market_price_currency_symbol,
            visible,
        } => {
            app.alter_pool(pool, name, market_price_currency_symbol, visible)
                .await;
        }
        Command::Pools { pool, curator } => {
            app.list_pools(pool, curator).await;
        }
        Command::ContractVersion {} => {
            app.contract_version().await;
        }
        Command::CreateReserve {
            curator,
            pool,
            protected_collateral,
            not_a_collateral,
            liquidity_mint,
            market_price_feed,
            irm,
            liquidation_bonus_bps,
            partly_unhealthy_ltv_bps,
            partial_liquidation_factor_bps,
            fully_unhealthy_ltv_bps,
            curator_borrow_fee_bps,
            curator_performance_fee_bps,
            max_borrow_ltv_bps,
            max_borrow_utilization_bps,
            max_withdraw_utilization_bps,
            max_total_liquidity,
            price_stale_threshold_sec,
        } => {
            let config = ReserveConfig {
                market_price_feed,
                irm,
                liquidation_bonus_bps,
                max_borrow_ltv_bps,
                partly_unhealthy_ltv_bps,
                partial_liquidation_factor_bps,
                fully_unhealthy_ltv_bps,
                fees: ReserveFeesConfig {
                    curator_borrow_fee_rate_bps: curator_borrow_fee_bps,
                    curator_performance_fee_rate_bps: curator_performance_fee_bps,
                    _padding: Zeroable::zeroed(),
                },
                _padding: Zeroable::zeroed(),
                max_total_liquidity,
                max_borrow_utilization_bps,
                price_stale_threshold_sec,
                max_withdraw_utilization_bps,
            };

            if protected_collateral && not_a_collateral {
                println!(
                    "Choose either --protected-collateral OR --not-a-collateral reserve mode!"
                );
                return;
            }

            let reserve_type = if protected_collateral {
                RESERVE_TYPE_PROTECTED_COLLATERAL
            } else if not_a_collateral {
                RESERVE_TYPE_NOT_A_COLLATERAL
            } else {
                RESERVE_TYPE_NORMAL
            };

            app.create_reserve(
                curator,
                pool,
                liquidity_mint,
                market_price_feed,
                config,
                reserve_type,
            )
            .await;
        }
        Command::Reserves {
            reserve,
            pool,
            mint,
        } => {
            app.list_reserves(reserve, pool, mint).await;
        }
        Command::AlterReserve {
            reserve,
            market_price_feed,
            irm,
            liquidation_bonus_bps,
            partly_unhealthy_ltv_bps,
            partial_liquidation_factor_bps,
            fully_unhealthy_ltv_bps,
            curator_borrow_fee_bps,
            curator_performance_fee_bps,
            max_borrow_utilization_bps,
            max_withdraw_utilization_bps,
            max_total_liquidity,
            max_borrow_ltv_bps,
            price_stale_threshold_sec,
            mode,
            flash_loans_enabled,
        } => {
            app.alter_reserve(
                reserve,
                market_price_feed,
                irm,
                liquidation_bonus_bps,
                partly_unhealthy_ltv_bps,
                partial_liquidation_factor_bps,
                fully_unhealthy_ltv_bps,
                curator_borrow_fee_bps,
                curator_performance_fee_bps,
                max_borrow_utilization_bps,
                max_withdraw_utilization_bps,
                max_total_liquidity,
                max_borrow_ltv_bps,
                price_stale_threshold_sec,
                mode,
                flash_loans_enabled,
            )
            .await;
        }
        Command::ProposeConfig {
            reserve,
            index,
            market_price_feed,
            irm,
            liquidation_bonus_bps,
            partly_unhealthy_ltv_bps,
            partial_liquidation_factor_bps,
            fully_unhealthy_ltv_bps,
            curator_borrow_fee_bps,
            curator_performance_fee_bps,
            max_borrow_utilization_bps,
            max_withdraw_utilization_bps,
            max_total_liquidity,
            max_borrow_ltv_bps,
            price_stale_threshold_sec,
        } => {
            app.propose_config_change(
                reserve,
                index,
                market_price_feed,
                irm,
                liquidation_bonus_bps,
                partly_unhealthy_ltv_bps,
                partial_liquidation_factor_bps,
                fully_unhealthy_ltv_bps,
                curator_borrow_fee_bps,
                curator_performance_fee_bps,
                max_borrow_utilization_bps,
                max_withdraw_utilization_bps,
                max_total_liquidity,
                max_borrow_ltv_bps,
                price_stale_threshold_sec,
            )
            .await;
        }
        Command::ClearConfigProposal { reserve, index } => {
            app.clear_proposed_config_change(reserve, index).await;
        }
        Command::ApplyConfigProposal { reserve, index } => {
            app.apply_config_proposal(reserve, index).await;
        }
        Command::DeleteReserve { reserve } => {
            app.delete_reserve(reserve).await;
        }
        Command::RefreshReserve { reserve } => {
            app.refresh_reserve(reserve).await;
        }
        Command::Deposit { reserve, amount } => {
            app.deposit(reserve, amount).await;
        }
        Command::Withdraw { reserve, lp_amount } => {
            app.withdraw(reserve, lp_amount).await;
        }
        Command::CreatePosition { pool, long_short } => {
            app.create_position(pool, long_short).await;
        }
        Command::ClosePosition { position, pool } => {
            app.close_position(position, pool).await;
        }
        Command::RefreshPosition { position } => app.refresh_position(position).await,
        Command::Positions {
            position,
            owner,
            pool,
        } => app.list_positions(position, owner, pool).await,
        Command::LockCollateral {
            position,
            reserve,
            amount,
            memo,
        } => {
            app.lock(position, reserve, amount, memo).await;
        }
        Command::UnlockCollateral {
            position,
            reserve,
            amount,
        } => {
            app.unlock(position, reserve, amount).await;
        }
        Command::Borrow {
            position,
            reserve,
            amount,
            slippage,
            memo,
        } => {
            app.borrow(position, reserve, amount, slippage, memo).await;
        }
        Command::Repay {
            position,
            reserve,
            amount,
        } => {
            app.repay(position, reserve, amount).await;
        }
        Command::ClaimCuratorFee { reserve } => {
            app.claim_curator_perf_fee(reserve).await;
        }
        Command::ClaimTextureFee { reserve } => {
            app.claim_texture_perf_fee(reserve).await;
        }
        Command::Liquidate {
            position,
            principal_reserve,
            collateral_reserve,
            principal_amount,
        } => {
            app.liquidate(
                position,
                principal_reserve,
                collateral_reserve,
                principal_amount,
            )
            .await;
        }
        Command::WriteOffBadDebt {
            position,
            reserve,
            amount,
        } => {
            app.write_off_bad_debt(position, reserve, amount).await;
        }
        Command::SetRewardRule {
            reserve,
            index,
            name,
            reward_mint,
            deposits,
            borrows,
            rate,
        } => {
            app.set_reward_rule(reserve, index, name, reward_mint, deposits, borrows, rate)
                .await
        }
        Command::InitRewardSupply { pool, reward_mint } => {
            app.init_reward_supply(pool, reward_mint).await
        }
        Command::ClaimReward {
            position,
            pool,
            reward_mint,
        } => app.claim_reward(position, pool, reward_mint).await,
        Command::RewardSupplyAddr { pool, reward_mint } => {
            app.reward_supply_addr(pool, reward_mint)
        }
        Command::DepositReward {
            pool,
            reward_mint,
            amount,
        } => app.deposit_reward(pool, reward_mint, amount).await,
        Command::WithdrawReward {
            pool,
            reward_mint,
            amount,
        } => app.withdraw_reward(pool, reward_mint, amount).await,
        Command::RewardsBalances { pool } => app.list_rewards_balances(pool).await,
        Command::LpMint { reserve } => {
            let mint = find_lp_token_mint(&reserve);
            println!("{}", mint.0);
        }
        Command::FlashTest { reserve, amount } => {
            app.flash_test(reserve, amount).await;
        }
        Command::PutTokenStandard { input_cfg, out_cfg } => {
            app.put_tokens_standards(input_cfg, out_cfg).await;
        }
        Command::ExecuteMultisig { multisig, tx } => {
            let keypair = read_keypair_file(multisig.0)
                .map_err(|err| anyhow!("reading multisig keypair: {}", err))
                .unwrap();
            app.execute_miltisig(keypair, tx).await;
        }
        Command::MakeLut { config } => {
            let parsed_config = if let Some(path) = config {
                read_lut_config(&path).expect("reading LUT config")
            } else {
                Vec::new()
            };

            let new_config = app.create_or_update_luts(parsed_config).await;

            tokio::fs::write(
                "new_lut_config.json",
                serde_json::to_string_pretty(&new_config).expect("to json"),
            )
            .await
            .expect("writing new config");
            println!("New config written to new_lut_config.json");
        }
        Command::Lut { lut } => {
            app.show_lut(&lut).await;
        }
        Command::GenUnhealthyPositions {
            position_config,
            price_feed_authority,
        } => {
            let keypair = read_keypair_file(price_feed_authority.0)
                .map_err(|err| anyhow!("reading price_feed_authority keypair: {}", err))
                .unwrap();
            app.gen_unhealthy_positions(position_config, keypair).await;
        }
        Command::MakeFeesAta { curator } => {
            app.make_fees_ata(&curator).await;
        }
        Command::SetLpMetadata {
            reserve,
            name,
            symbol,
            uri,
        } => {
            app.set_lp_metadata(reserve, name, symbol, uri).await;
        }
        Command::LpMetadata { reserve } => {
            app.show_lp_metadata(reserve).await;
        }
    }
}

impl Default for KeypairPath {
    fn default() -> Self {
        let mut path = dirs_next::home_dir().expect("home dir");
        path.extend([".config", "solana", "id.json"]);
        Self(path)
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for KeypairPath {
    fn to_string(&self) -> String {
        self.0.to_str().expect("non unicode").to_string()
    }
}
