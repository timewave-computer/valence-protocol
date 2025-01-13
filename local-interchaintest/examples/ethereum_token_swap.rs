use std::error::Error;

use alloy::sol;
use cosmwasm_std_old::{Coin, Uint128};
use local_interchaintest::utils::{
    ethereum::EthClient,
    hyperlane::{
        bech32_to_hex_address, set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts,
    },
    DEFAULT_ANVIL_RPC_ENDPOINT, HYPERLANE_RELAYER_NEUTRON_WALLET, LOGS_FILE_PATH,
    VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::bank;
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_NAME,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // Upload all Hyperlane contracts to Neutron
    let neutron_hyperlane_contracts = set_up_cw_hyperlane_contracts(&mut test_ctx)?;

    println!("Neutron Mailbox: {:?}", neutron_hyperlane_contracts.mailbox);
    println!("Neutron IGP: {:?}", neutron_hyperlane_contracts.igp);
    println!(
        "Neutron ISM: {:?}",
        neutron_hyperlane_contracts.ism_pausable
    );
    println!(
        "Neutron Hook Merkle: {:?}",
        neutron_hyperlane_contracts.hook_merkle
    );
    println!(
        "Neutron Validator Announce: {:?}",
        neutron_hyperlane_contracts.validator_announce
    );

    println!(
        "Neutron Mailbox hex: {:?}",
        bech32_to_hex_address(&neutron_hyperlane_contracts.mailbox)?
    );
    println!(
        "Neutron IGP hex: {:?}",
        bech32_to_hex_address(&neutron_hyperlane_contracts.igp)?
    );
    println!(
        "Neutron ISM hex: {:?}",
        bech32_to_hex_address(&neutron_hyperlane_contracts.ism_pausable)?
    );
    println!(
        "Neutron Hook Merkle hex: {:?}",
        bech32_to_hex_address(&neutron_hyperlane_contracts.hook_merkle)?
    );
    println!(
        "Neutron Validator Announce hex: {:?}",
        bech32_to_hex_address(&neutron_hyperlane_contracts.validator_announce)?
    );

    let eth_hyperlane_contracts = set_up_eth_hyperlane_contracts(&eth, 1)?;
    println!("Eth Mailbox: {:?}", eth_hyperlane_contracts.mailbox);
    println!("Eth IGP: {:?}", eth_hyperlane_contracts.igp);
    println!("Eth ISM: {:?}", eth_hyperlane_contracts.ism_pausable);
    println!("Eth Hook Merkle: {:?}", eth_hyperlane_contracts.hook_merkle);
    println!(
        "Eth Validator Announce: {:?}",
        eth_hyperlane_contracts.validator_announce
    );

    // Create a Test Recipient
    sol!(
        #[sol(rpc)]
        TestRecipient,
        "./hyperlane/contracts/solidity/TestRecipient.json",
    );

    let accounts = eth.get_accounts_addresses()?;

    let tx = TestRecipient::deploy_builder(&eth.provider)
        .into_transaction_request()
        .from(accounts[0]);

    let test_recipient = eth.send_transaction(tx)?.contract_address.unwrap();
    println!("Test Recipient: {:?}", test_recipient);

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &HYPERLANE_RELAYER_NEUTRON_WALLET,
        &[Coin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: Uint128::new(1000000000),
        }],
        &Coin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok(())
}
