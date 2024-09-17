use std::{
    env,
    error::Error,
    time::{Duration, SystemTime},
};

use cosmwasm_std::Uint64;
use localic_std::{
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query, CosmWasm},
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    GAIA_CHAIN_NAME, JUNO_CHAIN_ADMIN_ADDR, JUNO_CHAIN_ID, JUNO_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_authorization_utils::{
    domain::{Connector, ExternalDomain, PolytoneProxyState},
    msg::{
        CallbackProxy, Connector as AuthorizationConnector, ExternalDomainInfo, PermissionedMsg,
    },
};
use valence_local_interchaintest_utils::{
    polytone::salt_for_proxy, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_JUNO,
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, POLYTONE_PATH,
};
use valence_processor_utils::{
    msg::PolytoneContracts,
    processor::{Config, ProcessorDomain},
};

const TIMEOUT_SECONDS: u64 = 5;
const MAX_ATTEMPTS: u64 = 25;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir("artifacts")
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_chain(ConfigChainBuilder::default_juno().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, JUNO_CHAIN_NAME)
        .build()?;

    let mut uploader = test_ctx.build_tx_upload_contracts();

    // Upload all Polytone contracts to both Neutron and Juno
    uploader
        .send_with_local_cache(POLYTONE_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)
        .unwrap();

    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_with_local_cache(POLYTONE_PATH, LOCAL_CODE_ID_CACHE_PATH_JUNO)
        .unwrap();

    // Upload the authorization contract to Neutron and the processor to both Neutron and Juno
    let current_dir = env::current_dir()?;

    let authorization_contract_path = format!(
        "{}/artifacts/valence_authorization.wasm",
        current_dir.display()
    );

    info!("{}", authorization_contract_path);

    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&authorization_contract_path)?;
    uploader.send_single_contract(&processor_contract_path)?;

    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_single_contract(&processor_contract_path)?;

    // We need to predict the authorization contract address in advance for the processor contract on the main domain
    // We'll use the current time as a salt so we can run this test multiple times
    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );
    let predicted_authorization_contract_address = test_ctx
        .get_built_contract_address()
        .src(NEUTRON_CHAIN_NAME)
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .contract("valence_authorization")
        .salt_hex_encoded(&salt)
        .get();

    // Now we can instantiate the processor
    let processor_code_id_on_neutron = test_ctx
        .get_contract()
        .contract("valence_processor")
        .get_cw()
        .code_id
        .unwrap();

    let processor_instantiate_msg = valence_processor_utils::msg::InstantiateMsg {
        authorization_contract: predicted_authorization_contract_address.clone(),
        polytone_contracts: None,
    };

    let processor_on_main_domain = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        processor_code_id_on_neutron,
        &serde_json::to_string(&processor_instantiate_msg).unwrap(),
        "processor",
        None,
        "",
    )
    .unwrap();

    info!(
        "Processor on main domain: {}",
        processor_on_main_domain.address
    );

    // Instantiate the authorization contract now, we will add the external domains later on
    let authorization_code_id = test_ctx
        .get_contract()
        .contract("valence_authorization")
        .get_cw()
        .code_id
        .unwrap();

    let authorization_instantiate_msg = valence_authorization_utils::msg::InstantiateMsg {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        sub_owners: vec![],
        processor: processor_on_main_domain.address,
        external_domains: vec![],
    };

    test_ctx
        .build_tx_instantiate2()
        .with_label("authorization")
        .with_code_id(authorization_code_id)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(&authorization_instantiate_msg).unwrap())
        .send()
        .unwrap();

    // Before setting up the external domains and the processor on the external domain, we are going to set up polytone and predict the proxy addresses on both sides
    let mut polytone_note_on_neutron = test_ctx.get_contract().contract("polytone_note").get_cw();

    let mut polytone_voice_on_neutron = test_ctx.get_contract().contract("polytone_voice").get_cw();

    let polytone_proxy_on_neutron = test_ctx.get_contract().contract("polytone_proxy").get_cw();

    let mut polytone_note_on_juno = test_ctx
        .get_contract()
        .src(JUNO_CHAIN_NAME)
        .contract("polytone_note")
        .get_cw();

    let mut polytone_voice_on_juno = test_ctx
        .get_contract()
        .src(JUNO_CHAIN_NAME)
        .contract("polytone_voice")
        .get_cw();

    let polytone_proxy_on_juno = test_ctx
        .get_contract()
        .src(JUNO_CHAIN_NAME)
        .contract("polytone_proxy")
        .get_cw();

    let polytone_note_instantiate_msg = polytone_note::msg::InstantiateMsg {
        pair: None,
        block_max_gas: Uint64::new(3010000),
    };

    let neutron_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_neutron.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    let juno_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_juno.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    info!("Instantiating polytone contracts on both domains...");

    let polytone_note_on_neutron_address = polytone_note_on_neutron
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-neutron",
            None,
            "",
        )
        .unwrap()
        .address;
    info!(
        "Polytone Note on Neutron: {}",
        polytone_note_on_neutron_address
    );

    let polytone_voice_on_neutron_address = polytone_voice_on_neutron
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&neutron_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-neutron",
            None,
            "",
        )
        .unwrap()
        .address;
    info!(
        "Polytone Voice on Neutron: {}",
        polytone_voice_on_neutron_address
    );

    let polytone_note_on_juno_address = polytone_note_on_juno
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-juno",
            None,
            "",
        )
        .unwrap()
        .address;
    info!("Polytone Note on Juno: {}", polytone_note_on_juno_address);

    let polytone_voice_on_juno_address = polytone_voice_on_juno
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&juno_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-juno",
            None,
            "",
        )
        .unwrap()
        .address;
    info!("Polytone Voice on Juno: {}", polytone_voice_on_juno_address);

    info!("Creating WASM connections...");

    let relayer = Relayer::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );

    polytone_note_on_neutron
        .create_wasm_connection(
            &relayer,
            "neutron-juno",
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(JUNO_CHAIN_NAME),
                None,
                None,
                Some(polytone_voice_on_juno_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    polytone_voice_on_neutron
        .create_wasm_connection(
            &relayer,
            "neutron-juno",
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(JUNO_CHAIN_NAME),
                None,
                None,
                Some(polytone_note_on_juno_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    // Get the connection-ids so that we can predict the proxy addresses
    let neutron_channels = relayer.get_channels(NEUTRON_CHAIN_ID).unwrap();

    let connection_id_neutron_to_juno = neutron_channels.iter().find_map(|neutron_channel| {
        if neutron_channel.port_id == format!("wasm.{}", polytone_note_on_neutron_address.clone()) {
            neutron_channel.connection_hops.first().cloned()
        } else {
            None
        }
    });
    info!(
        "Connection ID of Wasm connection Neutron to Juno: {:?}",
        connection_id_neutron_to_juno
    );

    let juno_channels = relayer.get_channels(JUNO_CHAIN_ID).unwrap();

    let connection_id_juno_to_neutron = juno_channels.iter().find_map(|juno_channel| {
        if juno_channel.port_id == format!("wasm.{}", polytone_note_on_juno_address.clone()) {
            juno_channel.connection_hops.first().cloned()
        } else {
            None
        }
    });
    info!(
        "Connection ID of Wasm connection Juno to Neutron: {:?}",
        connection_id_juno_to_neutron
    );

    let salt_for_proxy_on_juno = salt_for_proxy(
        &connection_id_juno_to_neutron.unwrap(),
        &format!("wasm.{}", polytone_note_on_neutron_address.clone()),
        &predicted_authorization_contract_address,
    );

    // Predict the address the proxy on juno that the authorization module will have
    let predicted_proxy_address_on_juno = test_ctx
        .get_built_contract_address()
        .src(JUNO_CHAIN_NAME)
        .creator(&polytone_voice_on_juno_address.clone())
        .contract("polytone_proxy")
        .salt_hex_encoded(&hex::encode(salt_for_proxy_on_juno))
        .get();

    info!(
        "Predicted proxy address on Juno: {}",
        predicted_proxy_address_on_juno
    );

    // To predict the proxy address on neutron for the processor on juno we need to first predict the processor address
    let predicted_processor_on_juno_address = test_ctx
        .get_built_contract_address()
        .src(JUNO_CHAIN_NAME)
        .creator(JUNO_CHAIN_ADMIN_ADDR)
        .contract("valence_processor")
        .salt_hex_encoded(&salt)
        .get();

    // Let's now predict the proxy
    let salt_for_proxy_on_neutron = salt_for_proxy(
        &connection_id_neutron_to_juno.unwrap(),
        &format!(
            "wasm.{}",
            polytone_note_on_juno.contract_addr.clone().unwrap()
        ),
        &predicted_processor_on_juno_address,
    );
    let predicted_proxy_address_on_neutron = test_ctx
        .get_built_contract_address()
        .src(NEUTRON_CHAIN_NAME)
        .creator(&polytone_voice_on_neutron_address.clone())
        .contract("polytone_proxy")
        .salt_hex_encoded(&hex::encode(salt_for_proxy_on_neutron))
        .get();

    info!(
        "Predicted proxy address on Neutron: {}",
        predicted_proxy_address_on_neutron
    );

    // Instantiate the processor on the external domain
    let processor_instantiate_msg = valence_processor_utils::msg::InstantiateMsg {
        authorization_contract: predicted_authorization_contract_address.clone(),
        polytone_contracts: Some(PolytoneContracts {
            polytone_proxy_address: predicted_proxy_address_on_juno.clone(),
            polytone_note_address: polytone_note_on_juno_address.clone(),
            timeout_seconds: TIMEOUT_SECONDS,
        }),
    };

    // Before instantiating the processor and adding the external domain we are going to stop the relayer to force timeouts
    test_ctx.stop_relayer();

    let processor_code_id_on_juno = test_ctx
        .get_contract()
        .src(JUNO_CHAIN_NAME)
        .contract("valence_processor")
        .get_cw()
        .code_id
        .unwrap();

    // Instantiate processor
    test_ctx
        .build_tx_instantiate2()
        .with_chain_name(JUNO_CHAIN_NAME)
        .with_label("processor")
        .with_code_id(processor_code_id_on_juno)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(&processor_instantiate_msg).unwrap())
        .with_flags(GAS_FLAGS)
        .send()
        .unwrap();

    info!("Adding external domain to the authorization contract...");
    let add_external_domain_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        PermissionedMsg::AddExternalDomains {
            external_domains: vec![ExternalDomainInfo {
                name: "juno".to_string(),
                execution_environment:
                    valence_authorization_utils::domain::ExecutionEnvironment::CosmWasm,
                connector: AuthorizationConnector::PolytoneNote {
                    address: polytone_note_on_neutron_address.clone(),
                    timeout_seconds: TIMEOUT_SECONDS,
                },
                processor: predicted_processor_on_juno_address.clone(),
                callback_proxy: CallbackProxy::PolytoneProxy(
                    predicted_proxy_address_on_neutron.clone(),
                ),
            }],
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&add_external_domain_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // Let's make sure that when we start the relayer, the packets will time out
    std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));

    // Start the relayer again
    test_ctx.start_relayer();

    // The proxy creation from the processor should have timed out
    verify_proxy_state_on_processor(
        &mut test_ctx,
        &predicted_processor_on_juno_address,
        &PolytoneProxyState::TimedOut,
    );

    // The proxy creation for the external domain that we added on the authorization contract should have timed out too
    verify_proxy_state_on_authorization(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        &PolytoneProxyState::TimedOut,
    );

    info!("Retrying proxy creation...");
    // If we retry the proxy creation now, it should succeed and it should create the proxy on both domains
    let retry_proxy_creation_msg_on_authorization_contract =
        valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_authorization_utils::msg::PermissionlessMsg::RetryBridgeCreation {
                domain_name: "juno".to_string(),
            },
        );

    let retry_proxy_creation_on_juno_processor =
        valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_processor_utils::msg::PermissionlessMsg::RetryBridgeCreation {},
        );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&retry_proxy_creation_msg_on_authorization_contract).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &predicted_processor_on_juno_address,
        DEFAULT_KEY,
        &serde_json::to_string(&retry_proxy_creation_on_juno_processor).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // Now both proxies should be created
    verify_proxy_state_on_processor(
        &mut test_ctx,
        &predicted_processor_on_juno_address,
        &PolytoneProxyState::Created,
    );

    verify_proxy_state_on_authorization(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        &PolytoneProxyState::Created,
    );

    // Let's verify that the addresses that the voice contract created are the same that we predicted
    let remote_address: String = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &polytone_note_on_neutron_address,
            &serde_json::to_string(&polytone_note::msg::QueryMsg::RemoteAddress {
                local_address: predicted_authorization_contract_address,
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    assert_eq!(remote_address, predicted_proxy_address_on_juno);

    let remote_address: String = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(JUNO_CHAIN_NAME),
            &polytone_note_on_juno_address,
            &serde_json::to_string(&polytone_note::msg::QueryMsg::RemoteAddress {
                local_address: predicted_processor_on_juno_address,
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    assert_eq!(remote_address, predicted_proxy_address_on_neutron);
    info!("Predicted and created addresses match!");

    Ok(())
}

fn verify_proxy_state_on_processor(
    test_ctx: &mut TestContext,
    processor_address: &str,
    expected_state: &PolytoneProxyState,
) {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let config: Config = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(JUNO_CHAIN_NAME),
                &processor_address,
                &serde_json::to_string(&valence_processor_utils::msg::QueryMsg::Config {}).unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        if let ProcessorDomain::External(external) = &config.processor_domain {
            if external.proxy_on_main_domain_state.eq(expected_state) {
                info!("Target state reached!");
                break;
            } else {
                info!("Waiting for the right state");
            }
        } else {
            panic!("The processor domain is not external!");
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));
    }
}

fn verify_proxy_state_on_authorization(
    test_ctx: &mut TestContext,
    authorization_address: &str,
    expected_state: &PolytoneProxyState,
) {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let external_domains: Vec<ExternalDomain> = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &authorization_address,
                &serde_json::to_string(
                    &valence_authorization_utils::msg::QueryMsg::ExternalDomains {
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        match &external_domains.first().unwrap().connector {
            Connector::PolytoneNote { state, .. } => {
                if state.eq(expected_state) {
                    info!("Target state reached!");
                    break;
                } else {
                    info!("Waiting for the right state");
                }
            }
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));
    }
}
