use std::str::FromStr;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program_test::{
    find_file, read_file, BanksClient, BanksClientError, ProgramTest, ProgramTestContext,
};
use solana_sdk::account::Account;
use solana_sdk::signature::{read_keypair_file, Keypair};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_token::state::Account as TokenAccount;

use super_lendy::SUPER_LENDY_ID;

pub mod setup_super_lendy;
pub mod superlendy_executor;

pub const LAMPORTS_PER_USDC: u64 = 1_000_000;

pub fn program_name() -> &'static str {
    lazy_static! {
        static ref NAME: String = std::env!("CARGO_PKG_NAME").replace('-', "_");
    }
    &NAME
}

pub fn program_id() -> &'static Pubkey {
    &SUPER_LENDY_ID
}

pub fn admin_keypair() -> Keypair {
    read_keypair_file("../local-test/keys/LSDrcJsWUHMGJ6aQqHeapNftxhftkzR5SFs3YghQe4Y.json")
        .unwrap()
}

pub fn lender_keypair() -> Keypair {
    read_keypair_file("../local-test/keys/U1Enj8mfM6Lybeu3km6hhPyKCbi8Gj5VZTT7wQHYPaJ.json")
        .unwrap()
}

pub fn borrow_keypair() -> Keypair {
    read_keypair_file("../local-test/keys/uborhWCsojScXvx1dM5YaGLj4nPVXUYam57szVdVv8H.json")
        .unwrap()
}

pub fn texture_config_keypair() -> Keypair {
    read_keypair_file("../local-test/keys/gLoBanTpd5VuvyCpYjvYNudFREwLqFy418fGuuXUJfX.json")
        .unwrap()
}

pub fn price_feed_authority() -> Keypair {
    Keypair::from_bytes(&[
        52, 254, 138, 187, 39, 192, 47, 12, 4, 57, 178, 237, 196, 135, 132, 244, 11, 33, 144, 80,
        97, 206, 203, 254, 61, 196, 206, 68, 22, 82, 58, 177, 170, 146, 243, 184, 250, 17, 143, 33,
        84, 157, 252, 68, 214, 251, 10, 26, 48, 55, 182, 3, 189, 230, 147, 82, 74, 195, 46, 54, 62,
        16, 153, 234,
    ])
    .unwrap()
}

pub fn init_program_test() -> ProgramTest {
    tracing_init();

    tracing::info!("init program test...");

    let mut program_test = ProgramTest::default();

    let data = read_file(find_file("super_lendy.so").unwrap());
    program_test.add_account(
        super_lendy::SUPER_LENDY_ID,
        Account {
            lamports: 10 ^ 28,
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    let data = read_file(find_file("price_proxy.so").unwrap());
    program_test.add_account(
        price_proxy::ID,
        Account {
            lamports: 10 ^ 28,
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    let data = read_file(find_file("curvy.so").unwrap());
    program_test.add_account(
        curvy::ID,
        Account {
            lamports: 10 ^ 28,
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );
    program_test
}

pub fn init_token_accounts(runner: &mut ProgramTest, mint: &Pubkey) {
    macro_rules! token_data_path {
        ($data:literal) => {
            format!(
                concat!(
                    std::env!("CARGO_MANIFEST_DIR"),
                    "/../local-test/tokens/{}-",
                    $data,
                    ".json"
                ),
                mint
            )
        };
    }

    let paths = [
        token_data_path!("mint"),
        token_data_path!("borrower-wallet"),
        token_data_path!("lender-wallet"),
    ];

    for path in paths.iter() {
        let account_info: JSONAccountInfo = {
            let file_data = std::fs::read(path).unwrap();
            serde_json::from_slice(&file_data).unwrap()
        };
        runner.add_account_with_base64_data(
            Pubkey::from_str(&account_info.pubkey).unwrap(),
            account_info.account.lamports,
            Pubkey::from_str(&account_info.account.owner).unwrap(),
            &account_info.account.data[0],
        )
    }
}

pub async fn add_price_feed_acc(runner: &mut ProgramTest, file: &str) -> Pubkey {
    let feed_data_path = format!(
        concat!(
            std::env!("CARGO_MANIFEST_DIR"),
            "/../local-test/price-feeds/{}",
            ".json"
        ),
        file
    );

    let account_info: JSONAccountInfo = {
        let file_data = std::fs::read(feed_data_path).unwrap();
        serde_json::from_slice(&file_data).unwrap()
    };

    let price_feed = Pubkey::from_str(&account_info.pubkey).unwrap();
    runner.add_account_with_base64_data(
        price_feed,
        account_info.account.lamports,
        Pubkey::from_str(&account_info.account.owner).unwrap(),
        &account_info.account.data[0],
    );
    price_feed
}

pub async fn add_curve_acc(runner: &mut ProgramTest, file: &str) -> Pubkey {
    let feed_data_path = format!(
        concat!(
            std::env!("CARGO_MANIFEST_DIR"),
            "/../local-test/irm/{}",
            ".json"
        ),
        file
    );
    let account_info: JSONAccountInfo = {
        let file_data = std::fs::read(feed_data_path).unwrap();
        serde_json::from_slice(&file_data).unwrap()
    };

    let curve = Pubkey::from_str(&account_info.pubkey).unwrap();
    runner.add_account_with_base64_data(
        curve,
        account_info.account.lamports,
        Pubkey::from_str(&account_info.account.owner).unwrap(),
        &account_info.account.data[0],
    );
    curve
}

pub async fn get_account(
    banks_client: &mut BanksClient,
    address: Pubkey,
) -> std::io::Result<Account> {
    banks_client
        .get_account(address)
        .await?
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "account not found"))
}

pub async fn get_token_account(
    banks_client: &mut BanksClient,
    address: Pubkey,
) -> std::io::Result<TokenAccount> {
    let acc = get_account(banks_client, address).await?;
    TokenAccount::unpack_from_slice(acc.data.as_ref())
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
}

pub async fn create_associated_token_account(
    context: &mut ProgramTestContext,
    wallet: &Keypair,
    token_mint: &Pubkey,
) -> Result<Pubkey, BanksClientError> {
    let recent_blockhash = context.last_blockhash;

    let tx = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &context.payer.pubkey(),
                &wallet.pubkey(),
                token_mint,
                &spl_token::ID,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        recent_blockhash,
    );

    // connection.send_and_confirm_transaction(&tx)?;
    context.banks_client.process_transaction(tx).await.unwrap();

    Ok(spl_associated_token_account::get_associated_token_address(
        &wallet.pubkey(),
        token_mint,
    ))
}

pub const LAMPORTS: u64 = 1_000_000_000_000_000;

pub trait Runner {
    fn add_account(&mut self, key: Pubkey, account: Account) -> &mut Self;

    fn add_native_wallet(&mut self, key: Pubkey, lamports: u64) -> &mut Self {
        self.add_account(
            key,
            Account {
                lamports,
                data: Vec::new(),
                owner: system_program::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
    }

    fn add_account_with(&mut self, key: Pubkey, owner: Pubkey, f: impl FnOnce() -> Vec<u8>) {
        let data = f();
        let mut account = Account::new(LAMPORTS, data.len(), &owner);
        account.data = data;
        self.add_account(key, account);
    }
}

impl Runner for ProgramTest {
    fn add_account(&mut self, key: Pubkey, account: Account) -> &mut Self {
        self.add_account(key, account);
        self
    }
}

pub fn tracing_init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::fmt::Subscriber;
        use tracing_subscriber::util::SubscriberInitExt;

        let builder = Subscriber::builder();
        let builder = builder.with_max_level(LevelFilter::TRACE);

        let subscriber = builder.finish();
        let subscriber = {
            use std::env;
            use tracing_subscriber::{filter::Targets, layer::SubscriberExt};
            let targets = match env::var("RUST_LOG") {
                Ok(var) => var,
                Err(_) => concat!(
                    "debug",
                    ",solana_program_test=debug",
                    ",solana_runtime::message_processor::stable_log=debug",
                    ",solana_program_runtime=warn",
                    ",solana_program=warn",
                    ",solana_runtime=warn",
                    ",solana_metrics=warn",
                    ",tarpc=error",
                )
                .to_owned(),
            };
            subscriber.with(Targets::from_str(&targets).unwrap())
        };

        subscriber.try_init().unwrap();
    });
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONAccountInfo {
    account: JSONAccount,
    pubkey: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONAccount {
    data: Vec<String>,
    executable: bool,
    lamports: u64,
    owner: String,
    rent_epoch: u16,
    space: Option<u16>,
}
