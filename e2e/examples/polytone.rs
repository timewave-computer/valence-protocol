use std::{
    env,
    error::Error,
    time::{Duration, SystemTime},
};

use cosmwasm_std::{Binary, Timestamp, Uint128};
use cosmwasm_std_old::Uint64;
use cw_utils::Expiration;
use localic_std::{
    modules::{
        bank,
        cosmwasm::{contract_execute, contract_query, CosmWasm},
    },
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    GAIA_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_ID, OSMOSIS_CHAIN_NAME,
    OSMOSIS_CHAIN_PREFIX,
};
use log::info;
use serde_json::json;
use valence_authorization::error::ContractError;
use valence_authorization_utils::{
    authorization::{
        AtomicSubroutine, AuthorizationDuration, AuthorizationInfo, AuthorizationModeInfo,
        PermissionTypeInfo, Priority, Subroutine,
    },
    authorization_message::{Message, MessageDetails, MessageType},
    callback::ExecutionResult,
    domain::{CosmwasmBridge, Domain, ExecutionEnvironment, ExternalDomain, PolytoneProxyState},
    function::AtomicFunction,
    msg::{ExternalDomainInfo, PermissionedMsg, PolytoneNoteInfo, ProcessorMessage},
};
use valence_e2e::utils::{
    authorization::{
        predict_remote_contract_address, set_up_authorization_and_processor,
        verify_authorization_execution_result,
    },
    polytone::salt_for_proxy,
    processor::{get_processor_queue_items, tick_processor},
    relayer::restart_relayer,
    GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOCAL_CODE_ID_CACHE_PATH_OSMOSIS, LOGS_FILE_PATH,
    NEUTRON_USER_ADDRESS_1, POLYTONE_ARTIFACTS_PATH, USER_KEY_1, VALENCE_ARTIFACTS_PATH,
};

use valence_library_utils::LibraryAccountType;
use valence_processor_utils::{
    callback::{PendingPolytoneCallbackInfo, PolytoneCallbackState},
    msg::PolytoneContracts,
    processor::{Config, ProcessorDomain},
};

const TIMEOUT_SECONDS: u64 = 15;
const MAX_ATTEMPTS: u64 = 50;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    // Upload the authorization contract to Neutron and the processor to both Neutron and Osmosis

    // We need to predict the authorization contract address in advance for the processor contract on the main domain
    // We'll use the current time as a salt so we can run this test multiple times locally without getting conflicts
    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );
    // Upload and instantiate authorization and processor on Neutron
    let (predicted_authorization_contract_address, _) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    // Upload the processor contract to Osmosis
    let current_dir = env::current_dir()?;
    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&processor_contract_path)?;

    // Upload all Polytone contracts to both Neutron and Osmosis
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .send_with_local_cache(POLYTONE_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)
        .unwrap();

    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_with_local_cache(POLYTONE_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_OSMOSIS)
        .unwrap();

    // Before setting up the external domains and the processor on the external domain, we are going to set up polytone and predict the proxy addresses on both sides
    let mut polytone_note_on_neutron = test_ctx.get_contract().contract("polytone_note").get_cw();

    let mut polytone_voice_on_neutron = test_ctx.get_contract().contract("polytone_voice").get_cw();

    let polytone_proxy_on_neutron = test_ctx.get_contract().contract("polytone_proxy").get_cw();

    let mut polytone_note_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("polytone_note")
        .get_cw();

    let mut polytone_voice_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("polytone_voice")
        .get_cw();

    let polytone_proxy_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
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

    let osmosis_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_osmosis.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    info!("Instantiating polytone contracts on both domains...");
    let osmosis_gas_flags = &format!("{GAS_FLAGS} --fees {}{}", 5_000_000, OSMOSIS_CHAIN_DENOM);

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

    let polytone_note_on_osmosis_address = polytone_note_on_osmosis
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-osmosis",
            None,
            osmosis_gas_flags,
        )
        .unwrap()
        .address;
    info!(
        "Polytone Note on Osmosis: {}",
        polytone_note_on_osmosis_address
    );

    let polytone_voice_on_osmosis_address = polytone_voice_on_osmosis
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&osmosis_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-osmosis",
            None,
            osmosis_gas_flags,
        )
        .unwrap()
        .address;
    info!(
        "Polytone Voice on Osmosis: {}",
        polytone_voice_on_osmosis_address
    );

    info!("Creating WASM connections...");

    let relayer = Relayer::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );

    polytone_note_on_neutron
        .create_wasm_connection(
            &relayer,
            "neutron-osmosis",
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(OSMOSIS_CHAIN_NAME),
                None,
                None,
                Some(polytone_voice_on_osmosis_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    polytone_voice_on_neutron
        .create_wasm_connection(
            &relayer,
            "neutron-osmosis",
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(OSMOSIS_CHAIN_NAME),
                None,
                None,
                Some(polytone_note_on_osmosis_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    // Give some time to make sure the channels are open
    std::thread::sleep(Duration::from_secs(15));

    // Get the connection ids so that we can predict the proxy addresses
    let neutron_channels = relayer.get_channels(NEUTRON_CHAIN_ID).unwrap();

    let connection_id_neutron_to_osmosis = neutron_channels.iter().find_map(|neutron_channel| {
        if neutron_channel.port_id == format!("wasm.{}", polytone_note_on_neutron_address.clone()) {
            neutron_channel.connection_hops.first().cloned()
        } else {
            None
        }
    });
    info!(
        "Connection ID of Wasm connection Neutron to Osmosis: {:?}",
        connection_id_neutron_to_osmosis
    );

    let osmosis_channels = relayer.get_channels(OSMOSIS_CHAIN_ID).unwrap();

    let connection_id_osmosis_to_neutron = osmosis_channels.iter().find_map(|osmosis_channel| {
        if osmosis_channel.port_id == format!("wasm.{}", polytone_note_on_osmosis_address.clone()) {
            osmosis_channel.connection_hops.first().cloned()
        } else {
            None
        }
    });
    info!(
        "Connection ID of Wasm connection Osmosis to Neutron: {:?}",
        connection_id_osmosis_to_neutron
    );

    let salt_for_proxy_on_osmosis = salt_for_proxy(
        &connection_id_osmosis_to_neutron.unwrap(),
        &format!("wasm.{}", polytone_note_on_neutron_address.clone()),
        &predicted_authorization_contract_address,
    );

    // Predict the address the proxy on Osmosis for the authorization module
    let predicted_proxy_address_on_osmosis = predict_remote_contract_address(
        &test_ctx,
        polytone_proxy_on_osmosis.code_id.unwrap(),
        OSMOSIS_CHAIN_NAME,
        OSMOSIS_CHAIN_PREFIX,
        &polytone_voice_on_osmosis_address,
        &salt_for_proxy_on_osmosis,
    )
    .unwrap();

    info!(
        "Predicted proxy address on Osmosis: {}",
        predicted_proxy_address_on_osmosis
    );

    let processor_code_id_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("valence_processor")
        .get_cw()
        .code_id
        .unwrap();

    // To predict the proxy address on neutron for the processor on Osmosis we need to first predict the processor address
    let predicted_processor_on_osmosis_address = predict_remote_contract_address(
        &test_ctx,
        processor_code_id_on_osmosis,
        OSMOSIS_CHAIN_NAME,
        OSMOSIS_CHAIN_PREFIX,
        OSMOSIS_CHAIN_ADMIN_ADDR,
        hex::decode(&salt).unwrap().as_slice(),
    )
    .unwrap();

    info!(
        "Predicted processor address on Osmosis: {}",
        predicted_processor_on_osmosis_address
    );

    // Let's now predict the proxy
    let salt_for_proxy_on_neutron = salt_for_proxy(
        &connection_id_neutron_to_osmosis.unwrap(),
        &format!(
            "wasm.{}",
            polytone_note_on_osmosis.contract_addr.clone().unwrap()
        ),
        &predicted_processor_on_osmosis_address,
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
            polytone_proxy_address: predicted_proxy_address_on_osmosis.clone(),
            polytone_note_address: polytone_note_on_osmosis_address.clone(),
            timeout_seconds: TIMEOUT_SECONDS,
        }),
    };

    // Before instantiating the processor and adding the external domain we are going to stop the relayer to force timeouts
    test_ctx.stop_relayer();

    // Instantiate processor
    test_ctx
        .build_tx_instantiate2()
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .with_label("processor")
        .with_code_id(processor_code_id_on_osmosis)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(processor_instantiate_msg).unwrap())
        .with_flags(osmosis_gas_flags)
        .send()
        .unwrap();

    info!("Adding external domain to the authorization contract...");
    let add_external_domain_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        PermissionedMsg::AddExternalDomains {
            external_domains: vec![ExternalDomainInfo {
                name: "osmosis".to_string(),
                execution_environment:
                    valence_authorization_utils::msg::ExecutionEnvironmentInfo::Cosmwasm(
                        valence_authorization_utils::msg::CosmwasmBridgeInfo::Polytone(
                            valence_authorization_utils::msg::PolytoneConnectorsInfo {
                                polytone_note: PolytoneNoteInfo {
                                    address: polytone_note_on_neutron_address.clone(),
                                    timeout_seconds: TIMEOUT_SECONDS,
                                },
                                polytone_proxy: predicted_proxy_address_on_neutron.clone(),
                            },
                        ),
                    ),
                processor: predicted_processor_on_osmosis_address.clone(),
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
    restart_relayer(&mut test_ctx);

    // The proxy creation from the processor should have timed out
    verify_proxy_state_on_processor(
        &mut test_ctx,
        &predicted_processor_on_osmosis_address,
        &PolytoneProxyState::TimedOut,
    );

    // The proxy creation for the external domain that we added on the authorization contract should have timed out too
    verify_proxy_state_on_authorization(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        &PolytoneProxyState::TimedOut,
    );

    // Stop relayer again
    test_ctx.stop_relayer();

    info!("Retrying proxy creation...");
    // If we retry the proxy creation now, it should update the state to PendingResponse
    let retry_proxy_creation_msg_on_authorization_contract =
        valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_authorization_utils::msg::PermissionlessMsg::RetryBridgeCreation {
                domain_name: "osmosis".to_string(),
            },
        );

    let retry_proxy_creation_on_osmosis_processor =
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
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &predicted_processor_on_osmosis_address,
        DEFAULT_KEY,
        &serde_json::to_string(&retry_proxy_creation_on_osmosis_processor).unwrap(),
        osmosis_gas_flags,
    )
    .unwrap();

    verify_proxy_state_on_processor(
        &mut test_ctx,
        &predicted_processor_on_osmosis_address,
        &PolytoneProxyState::PendingResponse,
    );

    verify_proxy_state_on_authorization(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        &PolytoneProxyState::PendingResponse,
    );

    // Let's make sure that when we start the relayer, the packets will time out again
    std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));

    // Start the relayer again
    restart_relayer(&mut test_ctx);

    // The proxy creation from the processor should have timed out
    verify_proxy_state_on_processor(
        &mut test_ctx,
        &predicted_processor_on_osmosis_address,
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
                domain_name: "osmosis".to_string(),
            },
        );

    let retry_proxy_creation_on_osmosis_processor =
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
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &predicted_processor_on_osmosis_address,
        DEFAULT_KEY,
        &serde_json::to_string(&retry_proxy_creation_on_osmosis_processor).unwrap(),
        osmosis_gas_flags,
    )
    .unwrap();

    // Now both proxies should be created
    verify_proxy_state_on_processor(
        &mut test_ctx,
        &predicted_processor_on_osmosis_address,
        &PolytoneProxyState::Created,
    );

    verify_proxy_state_on_authorization(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        &PolytoneProxyState::Created,
    );

    info!("Verifying that predicted and generated addresses match...");
    let remote_address: String = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &polytone_note_on_neutron_address,
            &serde_json::to_string(&polytone_note::msg::QueryMsg::RemoteAddress {
                local_address: predicted_authorization_contract_address.clone(),
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    assert_eq!(remote_address, predicted_proxy_address_on_osmosis);

    let remote_address: String = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &polytone_note_on_osmosis_address,
            &serde_json::to_string(&polytone_note::msg::QueryMsg::RemoteAddress {
                local_address: predicted_processor_on_osmosis_address.clone(),
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    assert_eq!(remote_address, predicted_proxy_address_on_neutron);
    info!("Predicted and created addresses match!");

    // Let's test the function creation and execution / retrying

    // First we are going to try to add an authorization with an function for an invalid domain, which should fail
    let mut function = AtomicFunction {
        domain: Domain::External("juno".to_string()),
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "any".to_string(),
                params_restrictions: None,
            },
        },
        // We don't care about the execution result so we will just make it fail when ticking the processor
        contract_address: LibraryAccountType::Addr("any".to_string()),
    };
    let mut authorization = AuthorizationInfo {
        label: "label".to_string(),
        mode: AuthorizationModeInfo::Permissioned(PermissionTypeInfo::WithCallLimit(vec![(
            NEUTRON_USER_ADDRESS_1.to_string(),
            Uint128::new(3),
        )])),
        not_before: Expiration::Never {},
        duration: AuthorizationDuration::Forever,
        max_concurrent_executions: Some(3),
        subroutine: Subroutine::Atomic(AtomicSubroutine {
            functions: vec![function.clone()],
            retry_logic: None,
            expiration_time: None,
        }),
        priority: None,
    };
    let tokenfactory_token = format!(
        "factory/{}/label",
        predicted_authorization_contract_address.clone()
    );

    info!("Trying to create an authorization with an invalid external domain...");

    let error = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                PermissionedMsg::CreateAuthorizations {
                    authorizations: vec![authorization.clone()],
                },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::DomainIsNotRegistered("juno".to_string())
            .to_string()
            .as_str()
    ));

    info!("Creating a valid authorization...");

    function.domain = Domain::External("osmosis".to_string());
    authorization.subroutine = Subroutine::Atomic(AtomicSubroutine {
        functions: vec![function.clone()],
        retry_logic: None,
        expiration_time: None,
    });

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                PermissionedMsg::CreateAuthorizations {
                    authorizations: vec![authorization.clone()],
                },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    std::thread::sleep(Duration::from_secs(3));

    // Now that it's created, we will send the message three times:
    // One without TTL, which should return the token when timed out
    // Another one with TTL never, which should time out and be retriable
    // And one with TTL at a future timestamp, which should time out (being retriable at that point), and not be retriable after a while, and the token should be returned when we retry it after TTL expires
    let msg = Binary::from(serde_json::to_vec(&json!({"any": {}})).unwrap());

    info!("Stopping relayer to force timeouts...");
    test_ctx.stop_relayer();

    info!("Sending the messages without TTL...");
    let flags = format!("--amount 1{tokenfactory_token} {GAS_FLAGS}");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                    label: "label".to_string(),
                    messages: vec![ProcessorMessage::CosmwasmExecuteMsg { msg: msg.clone() }],
                    ttl: None,
                },
            ),
        )
        .unwrap(),
        &flags,
    )
    .unwrap();

    std::thread::sleep(Duration::from_secs(3));

    info!("Sending the messages with TTL (and expire = never)...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                    label: "label".to_string(),
                    messages: vec![ProcessorMessage::CosmwasmExecuteMsg { msg: msg.clone() }],
                    ttl: Some(Expiration::Never {}),
                },
            ),
        )
        .unwrap(),
        &flags,
    )
    .unwrap();

    std::thread::sleep(Duration::from_secs(3));

    // Give enough time to timeout just in case relayer is slow (specially on CI)
    let ttl_time = 300;
    info!(
        "Sending the messages with TTL (and {} seconds as expire)...",
        ttl_time
    );
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                    label: "label".to_string(),
                    messages: vec![ProcessorMessage::CosmwasmExecuteMsg { msg: msg.clone() }],
                    ttl: Some(Expiration::AtTime(Timestamp::from_seconds(
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)?
                            .as_secs()
                            + ttl_time,
                    ))),
                },
            ),
        )
        .unwrap(),
        &flags,
    )
    .unwrap();

    // Let's make sure that when we start the relayer, the packets will time out
    std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));

    info!("Restarting the relayer...");
    restart_relayer(&mut test_ctx);

    // Verify that all messages are in timeout state
    // The one without TTL should not be retriable and the two with TTL should be retriable
    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        0,
        &ExecutionResult::Timeout(false),
    );

    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        1,
        &ExecutionResult::Timeout(true),
    );

    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        2,
        &ExecutionResult::Timeout(true),
    );

    info!("All messages correctly timed out");

    info!("Check user balance...");
    // Let's check the balance of the sender, to verify that 1 token was sent back and the others were not because they are still retriable
    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        NEUTRON_USER_ADDRESS_1,
    );

    assert_eq!(
        token_balances
            .iter()
            .find(|coin| coin.denom.eq(&tokenfactory_token))
            .unwrap()
            .amount
            .u128(),
        1,
    );

    info!("Retrying resending the message without TTL...");
    let error = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::RetryMsgs { execution_id: 0 },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(valence_authorization::error::MessageErrorReason::NotRetriable {})
            .to_string()
            .as_str()
    ));

    info!("Retrying resending the message with TTL...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::RetryMsgs { execution_id: 1 },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    std::thread::sleep(Duration::from_secs(3));

    // If we try to retry again, it won't work because it's InProcess again
    let error = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::RetryMsgs { execution_id: 1 },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(valence_authorization::error::MessageErrorReason::NotRetriable {})
            .to_string()
            .as_str()
    ));

    // Make sure the 3rd message will not be retriable after the TTL expires and that the token is correctly sent back
    info!("Waiting for the TTL to expire...");
    std::thread::sleep(Duration::from_secs(ttl_time - TIMEOUT_SECONDS));

    info!("Retrying resending the message with TTL after it expired...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::RetryMsgs { execution_id: 2 },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // Let's check that the execution result is correctly updated
    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        2,
        &ExecutionResult::Timeout(false),
    );

    info!("Check user balance...");
    // Let's also check that the token was sent back correctly
    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        NEUTRON_USER_ADDRESS_1,
    );

    assert_eq!(
        token_balances
            .iter()
            .find(|coin| coin.denom.eq(&tokenfactory_token))
            .unwrap()
            .amount
            .u128(),
        2,
    );

    // This should bridge and enqueue into the processor
    info!("Querying the batch from the processor...");
    let mut attempts = 0;
    let mut batches;
    loop {
        attempts += 1;
        batches = get_processor_queue_items(
            &mut test_ctx,
            OSMOSIS_CHAIN_NAME,
            &predicted_processor_on_osmosis_address,
            Priority::Medium,
        );

        if batches.len() == 1 {
            info!("Batch found!");
            break;
        }

        if attempts % 5 == 0 {
            // Sometimes the relayer doesn't pick up the changes, so we restart it
            restart_relayer(&mut test_ctx);
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(15));
    }

    assert_eq!(batches[0].id, 1);

    info!("Stopping the relayer again before ticking the processor to force a timeout...");
    test_ctx.stop_relayer();

    info!("Ticking the processor to trigger sending the callback...");
    tick_processor(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        DEFAULT_KEY,
        &predicted_processor_on_osmosis_address,
        osmosis_gas_flags,
    );

    // Wait enough time to force the time out
    std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));

    info!("Restarting the relayer...");
    restart_relayer(&mut test_ctx);

    // The polytone callback in the processor should have timed out
    info!("Querying the callback from the processor...");
    let mut attempts = 0;
    let mut callback_info;
    loop {
        attempts += 1;
        callback_info = get_processor_pending_polytone_callback(
            &mut test_ctx,
            &predicted_processor_on_osmosis_address,
            1,
        );

        if callback_info.state.eq(&PolytoneCallbackState::TimedOut) {
            info!("Callback successfully timed out!");
            break;
        }

        if attempts % 5 == 0 {
            // Sometimes the relayer doesn't pick up the changes, so we restart it
            restart_relayer(&mut test_ctx);
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(15));
    }

    // Now we should be able to retry the callback permissionlessly
    info!("Retrying the callback...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &predicted_processor_on_osmosis_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::RetryCallback { execution_id: 1 },
            ),
        )
        .unwrap(),
        osmosis_gas_flags,
    )
    .unwrap();

    info!("Querying the execution result on the authorization contract...");
    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        1,
        &ExecutionResult::Rejected("anything".to_string()),
    );

    // Let's create an authorization that we will force to expire the moment the batch is received by the processor
    info!("Creating an authorization that will expire immediately after the batch is received by the processor...");
    authorization.subroutine = Subroutine::Atomic(AtomicSubroutine {
        functions: vec![function.clone()],
        retry_logic: None,
        expiration_time: Some(5),
    });
    authorization.label = "expire".to_string();
    authorization.mode = AuthorizationModeInfo::Permissionless;

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                PermissionedMsg::CreateAuthorizations {
                    authorizations: vec![authorization.clone()],
                },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    // Stop relayer before sending
    test_ctx.stop_relayer();

    // Send the messages
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY_1,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                    label: "expire".to_string(),
                    messages: vec![ProcessorMessage::CosmwasmExecuteMsg { msg: msg.clone() }],
                    ttl: None,
                },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // Wait for expiration_time to pass
    std::thread::sleep(Duration::from_secs(6));

    // Start the relayer again
    restart_relayer(&mut test_ctx);

    // The message will be received by the processor that will confirm it's already expired and return the callback immediately
    info!("Querying the result in the authorization contract...");
    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        3,
        &ExecutionResult::Expired(0),
    );

    info!("All polytone tests passed!");

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
                    .get_request_builder(OSMOSIS_CHAIN_NAME),
                processor_address,
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
                info!(
                    "Waiting for the right state, current state: {:?}",
                    external.proxy_on_main_domain_state
                );
            }
        } else {
            panic!("The processor domain is not external!");
        }

        if attempts % 5 == 0 {
            // Sometimes the relayer doesn't pick up the changes, so we restart it
            restart_relayer(test_ctx);
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(15));
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
                authorization_address,
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

        match &external_domains.first().unwrap().execution_environment {
            ExecutionEnvironment::Cosmwasm(cosmwasm_bridge) => match cosmwasm_bridge {
                CosmwasmBridge::Polytone(polytone_connectors) => {
                    if polytone_connectors.polytone_note.state.eq(expected_state) {
                        info!("Target state reached!");
                        break;
                    } else {
                        info!(
                            "Waiting for the right state, current state: {:?}",
                            polytone_connectors.polytone_note.state
                        );
                    }
                }
            },
            ExecutionEnvironment::Evm(_, _) => {
                panic!("No polytone proxy state on EVM bridge!")
            }
        }

        if attempts % 5 == 0 {
            // Sometimes the relayer doesn't pick up the changes, so we restart it
            restart_relayer(test_ctx);
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(15));
    }
}

fn get_processor_pending_polytone_callback(
    test_ctx: &mut TestContext,
    processor_address: &str,
    execution_id: u64,
) -> PendingPolytoneCallbackInfo {
    serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            processor_address,
            &serde_json::to_string(
                &valence_processor_utils::msg::QueryMsg::PendingPolytoneCallback { execution_id },
            )
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap()
}
