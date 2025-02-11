use std::error::Error;

use alloy::sol;
use cosmwasm_std_old::HexBinary;
use hpl_interface::core::mailbox::DispatchMsg;
use localic_std::modules::cosmwasm::contract_execute;
use localic_utils::{
    utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_e2e::utils::{
    ethereum::set_up_anvil_container,
    hyperlane::{set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts, set_up_hyperlane},
    DEFAULT_ANVIL_RPC_ENDPOINT, GAS_FLAGS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(set_up_anvil_container())?;

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
    // Deploy all Hyperlane contracts on Ethereum
    let eth_hyperlane_contracts = set_up_eth_hyperlane_contracts(&eth, 1)?;

    set_up_hyperlane(
        "hyperlane-net",
        vec!["localneutron-1-val-0-neutronic", "anvil"],
        "neutron",
        "ethereum",
        &neutron_hyperlane_contracts,
        &eth_hyperlane_contracts,
    )?;

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

    let test_recipient_address = eth.send_transaction(tx)?.contract_address.unwrap();

    // Remove "0x" prefix if present and ensure proper hex formatting
    let address_hex = test_recipient_address
        .to_string()
        .trim_start_matches("0x")
        .to_string();
    // Pad to 32 bytes (64 hex characters) because mailboxes expect 32 bytes addresses with leading zeros
    let padded_recipient = format!("{:0>64}", address_hex);
    let msg_body = HexBinary::from_hex(&hex::encode("Hello my friend!"))?;

    let dispatch_msg = hpl_interface::core::mailbox::ExecuteMsg::Dispatch(DispatchMsg {
        dest_domain: 1,
        recipient_addr: HexBinary::from_hex(&padded_recipient)?,
        msg_body: msg_body.clone(),
        hook: None,
        metadata: None,
    });

    // Execute dispatch on mailbox
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &neutron_hyperlane_contracts.mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&dispatch_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(10));

    // Check that it was relayed and updated on the Ethereum side
    let test_recipient = TestRecipient::new(test_recipient_address, &eth.provider);
    let builder = test_recipient.lastData();
    let last_data = eth.rt.block_on(async { builder.call().await })?._0;
    assert_eq!(last_data.to_vec(), msg_body);

    info!("Test passed successfully!");

    Ok(())
}
