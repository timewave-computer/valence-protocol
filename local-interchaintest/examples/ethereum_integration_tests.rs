use std::{
    collections::HashMap,
    env,
    error::Error,
    str::FromStr,
    time::{Duration, SystemTime},
};

use alloy::primitives::{Address, U256};
use alloy_sol_types_encoder::SolValue;
use cosmwasm_std::Empty;
use local_interchaintest::utils::{
    authorization::set_up_authorization_and_processor,
    ethereum::set_up_anvil_container,
    hyperlane::{
        bech32_to_evm_bytes32, set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts,
        set_up_hyperlane,
    },
    solidity_contracts::{BaseAccount, Forwarder, LiteProcessor, MockERC20},
    DEFAULT_ANVIL_RPC_ENDPOINT, ETHEREUM_CHAIN_NAME, ETHEREUM_HYPERLANE_DOMAIN, GAS_FLAGS,
    LOGS_FILE_PATH, NEUTRON_HYPERLANE_DOMAIN, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate};
use localic_utils::{
    utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_authorization_utils::{
    domain::Domain,
    msg::{
        EncoderInfo, EvmBridgeInfo, ExternalDomainInfo, HyperlaneConnectorInfo, PermissionedMsg,
    },
};
use valence_encoder_utils::libraries::forwarder::solidity_types::{
    ForwarderConfig, ForwardingConfig, IntervalType,
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
    let eth_hyperlane_contracts = set_up_eth_hyperlane_contracts(&eth, ETHEREUM_HYPERLANE_DOMAIN)?;

    set_up_hyperlane(
        "hyperlane-net",
        vec!["localneutron-1-val-0-neutronic", "anvil"],
        "neutron",
        "ethereum",
        &neutron_hyperlane_contracts,
        &eth_hyperlane_contracts,
    )?;

    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );

    let (authorization_contract_address, _) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    info!("Setting up encoders ...");
    // Since we are going to send messages to EVM, we need to set up the encoder broker with the evm encoder
    let current_dir = env::current_dir()?;
    let encoder_broker_path = format!(
        "{}/artifacts/valence_encoder_broker.wasm",
        current_dir.display()
    );
    let evm_encoder_path = format!(
        "{}/artifacts/valence_evm_encoder_v1.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&encoder_broker_path)?;
    uploader.send_single_contract(&evm_encoder_path)?;

    let code_id_encoder_broker = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_encoder_broker")
        .unwrap();
    let code_id_evm_encoder = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_evm_encoder_v1")
        .unwrap();

    let evm_encoder = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_evm_encoder,
        &serde_json::to_string(&Empty {}).unwrap(),
        "evm_encoder",
        None,
        "",
    )
    .unwrap();

    let namespace_evm_encoder = "evm_encoder_v1".to_string();
    let encoder_broker = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_encoder_broker,
        &serde_json::to_string(&valence_encoder_broker::msg::InstantiateMsg {
            encoders: HashMap::from([(namespace_evm_encoder.clone(), evm_encoder.address)]),
            owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        })
        .unwrap(),
        "encoder_broker",
        None,
        "",
    )
    .unwrap();
    info!(
        "Encoders set up successfully! Broker address: {}",
        encoder_broker.address
    );

    info!("Setting up Lite Processor on Ethereum");
    let accounts = eth.get_accounts_addresses()?;

    let tx = LiteProcessor::deploy_builder(
        &eth.provider,
        bech32_to_evm_bytes32(&authorization_contract_address)?,
        Address::from_str(&eth_hyperlane_contracts.mailbox)?,
        NEUTRON_HYPERLANE_DOMAIN,
        vec![],
    )
    .into_transaction_request()
    .from(accounts[0]);

    let lite_processor_address = eth.send_transaction(tx)?.contract_address.unwrap();
    info!("Lite Processor deployed at: {}", lite_processor_address);

    info!("Adding EVM external domain to Authorization contract");
    let add_external_evm_domain_msg =
        valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
            PermissionedMsg::AddExternalDomains {
                external_domains: vec![ExternalDomainInfo {
                    name: ETHEREUM_CHAIN_NAME.to_string(),
                    execution_environment:
                        valence_authorization_utils::msg::ExecutionEnvironmentInfo::Evm(
                            EncoderInfo {
                                broker_address: encoder_broker.address,
                                encoder_version: namespace_evm_encoder,
                            },
                            EvmBridgeInfo::Hyperlane(HyperlaneConnectorInfo {
                                mailbox: neutron_hyperlane_contracts.mailbox.to_string(),
                                domain_id: ETHEREUM_HYPERLANE_DOMAIN,
                            }),
                        ),
                    processor: lite_processor_address.to_string(),
                }],
            },
        );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&add_external_evm_domain_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    info!("Test pausing the processor...");
    let pause_processor_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        PermissionedMsg::PauseProcessor {
            domain: Domain::External(ETHEREUM_CHAIN_NAME.to_string()),
        },
    );
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&pause_processor_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(5));

    // Query the processor to verify that it is paused
    let lite_processor = LiteProcessor::new(lite_processor_address, &eth.provider);
    let builder = lite_processor.paused();
    let paused = eth.rt.block_on(async { builder.call().await })?._0;
    assert!(paused);

    info!("Test resuming the processor...");
    let resume_processor_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        PermissionedMsg::ResumeProcessor {
            domain: Domain::External(ETHEREUM_CHAIN_NAME.to_string()),
        },
    );
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&resume_processor_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(5));

    // Query the processor to verify that it is resumed
    let lite_processor = LiteProcessor::new(lite_processor_address, &eth.provider);
    let builder = lite_processor.paused();
    let paused = eth.rt.block_on(async { builder.call().await })?._0;
    assert!(!paused);

    // Let's create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    info!("Deploying base accounts on Ethereum...");
    let base_account_tx = BaseAccount::deploy_builder(&eth.provider, accounts[0], vec![])
        .into_transaction_request()
        .from(accounts[0]);

    let base_account_1 = eth
        .send_transaction(base_account_tx.clone())?
        .contract_address
        .unwrap();
    let base_account_2 = eth
        .send_transaction(base_account_tx.clone())?
        .contract_address
        .unwrap();

    info!("Deploying ERC20s on Ethereum...");
    let token_1_tx =
        MockERC20::deploy_builder(&eth.provider, "Token1".to_string(), "T1".to_string())
            .into_transaction_request()
            .from(accounts[0]);
    let token_1_address = eth.send_transaction(token_1_tx)?.contract_address.unwrap();
    let token_1 = MockERC20::new(token_1_address, &eth.provider);

    let token_2_tx =
        MockERC20::deploy_builder(&eth.provider, "Token2".to_string(), "T2".to_string())
            .into_transaction_request()
            .from(accounts[0]);
    let token_2_address = eth.send_transaction(token_2_tx)?.contract_address.unwrap();
    let token_2 = MockERC20::new(token_2_address, &eth.provider);

    // Let's mint some token1 to base_account_1 and some token2 to base_account_2
    let mint_token1_tx = token_1
        .mint(base_account_1, U256::from(1000))
        .into_transaction_request()
        .from(accounts[0]);
    eth.send_transaction(mint_token1_tx)?;
    let mint_token2_tx = token_2
        .mint(base_account_2, U256::from(1000))
        .into_transaction_request()
        .from(accounts[0]);
    eth.send_transaction(mint_token2_tx)?;

    // Now let's deploy the forwarders
    let forwarder_1_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(&base_account_1.to_string())?,
        outputAccount: alloy_primitives_encoder::Address::from_str(&base_account_2.to_string())?,
        forwardingConfigs: vec![ForwardingConfig {
            tokenAddress: alloy_primitives_encoder::Address::from_str(
                &token_1_address.to_string(),
            )?,
            maxAmount: 1000,
        }],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };
    let forwarder_1_tx = Forwarder::deploy_builder(
        &eth.provider,
        accounts[0],
        lite_processor_address,
        forwarder_1_config.abi_encode().into(),
    )
    .into_transaction_request()
    .from(accounts[0]);
    let forwarder_1_address = eth
        .send_transaction(forwarder_1_tx)?
        .contract_address
        .unwrap();
    info!("Forwarder 1 deployed at: {}", forwarder_1_address);

    let forwarder_2_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(&base_account_2.to_string())?,
        outputAccount: alloy_primitives_encoder::Address::from_str(&base_account_1.to_string())?,
        forwardingConfigs: vec![ForwardingConfig {
            tokenAddress: alloy_primitives_encoder::Address::from_str(
                &token_2_address.to_string(),
            )?,
            maxAmount: 1000,
        }],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };
    let forwarder_2_tx = Forwarder::deploy_builder(
        &eth.provider,
        accounts[0],
        lite_processor_address,
        forwarder_2_config.abi_encode().into(),
    )
    .into_transaction_request()
    .from(accounts[0]);
    let forwarder_2_address = eth
        .send_transaction(forwarder_2_tx)?
        .contract_address
        .unwrap();
    info!("Forwarder 2 deployed at: {}", forwarder_2_address);

    /*// Create a Test Recipient
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
    assert_eq!(last_data.to_vec(), msg_body);*/

    info!("Integration tests passed successfully!");

    Ok(())
}
