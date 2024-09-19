use std::{
    env,
    error::Error,
    time::{Duration, SystemTime},
};

use cosmwasm_std::{Binary, Uint128};
use cosmwasm_std_polytone::Uint64;
use cw_utils::Expiration;
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
use serde_json::json;
use valence_authorization::error::ContractError;
use valence_authorization_utils::{
    action::AtomicAction,
    authorization::{
        ActionsConfig, AtomicActionsConfig, AuthorizationDuration, AuthorizationInfo,
        AuthorizationModeInfo, PermissionTypeInfo, Priority,
    },
    authorization_message::{Message, MessageDetails, MessageType},
    callback::{ExecutionResult, ProcessorCallbackInfo},
    domain::{Connector, Domain, ExternalDomain, PolytoneProxyState},
    msg::{
        CallbackProxy, Connector as AuthorizationConnector, ExternalDomainInfo, PermissionedMsg,
        ProcessorMessage,
    },
};
use valence_local_interchaintest_utils::{
    polytone::salt_for_proxy, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_JUNO,
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, POLYTONE_ARTIFACTS_PATH,
    VALENCE_ARTIFACTS_PATH,
};
use valence_processor_utils::{
    callback::{PendingPolytoneCallbackInfo, PolytoneCallbackState},
    msg::PolytoneContracts,
    processor::{Config, MessageBatch, ProcessorDomain},
};

const TIMEOUT_SECONDS: u64 = 5;
const MAX_ATTEMPTS: u64 = 25;
const USER_ADDRESS: &str = "neutron1kljf09rj77uxeu5lye7muejx6ajsu55cuw2mws";
const USER_KEY: &str = "acc1";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
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
        .send_with_local_cache(POLYTONE_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)
        .unwrap();

    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_with_local_cache(POLYTONE_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_JUNO)
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
    // We'll use the current time as a salt so we can run this test multiple times locally without getting conflicts
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

    // Instantiate the authorization contract now, we will add the external domains later
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

    // Get the connection ids so that we can predict the proxy addresses
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

    // Predict the address the proxy on juno for the authorization module
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

    assert_eq!(remote_address, predicted_proxy_address_on_juno);

    let remote_address: String = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(JUNO_CHAIN_NAME),
            &polytone_note_on_juno_address,
            &serde_json::to_string(&polytone_note::msg::QueryMsg::RemoteAddress {
                local_address: predicted_processor_on_juno_address.clone(),
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    assert_eq!(remote_address, predicted_proxy_address_on_neutron);
    info!("Predicted and created addresses match!");

    // Let's test the action creation and execution / retrying

    // First we are going to try to add an authorization with an action for an invalid domain, which should fail
    let mut action = AtomicAction {
        domain: Domain::External("osmosis".to_string()),
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "any".to_string(),
                params_restrictions: None,
            },
        },
        // We don't care about the execution result so we will just make it fail when ticking the processor
        contract_address: "any".to_string(),
    };
    let mut authorization = AuthorizationInfo {
        label: "label".to_string(),
        mode: AuthorizationModeInfo::Permissioned(PermissionTypeInfo::WithCallLimit(vec![(
            USER_ADDRESS.to_string(),
            Uint128::new(2),
        )])),
        not_before: Expiration::Never {},
        duration: AuthorizationDuration::Forever,
        max_concurrent_executions: Some(2),
        actions_config: ActionsConfig::Atomic(AtomicActionsConfig {
            actions: vec![action.clone()],
            retry_logic: None,
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
        ContractError::DomainIsNotRegistered("osmosis".to_string())
            .to_string()
            .as_str()
    ));

    info!("Creating a valid authorization...");

    action.domain = Domain::External("juno".to_string());
    authorization.actions_config = ActionsConfig::Atomic(AtomicActionsConfig {
        actions: vec![action.clone()],
        retry_logic: None,
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

    // Now that it's created, the user will send the message twice, once without TTL and once with it, so after the timeout only the one with non-expired TTL can be retried
    let msg = Binary::from(serde_json::to_vec(&json!({"any": {}})).unwrap());

    info!("Stopping relayer to force timeouts...");
    test_ctx.stop_relayer();

    info!("Sending the messages without TTL...");
    let flags = format!("--amount 1{} {}", tokenfactory_token, GAS_FLAGS);
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY,
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

    info!("Sending the messages with TTL...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                    label: "label".to_string(),
                    messages: vec![ProcessorMessage::CosmwasmExecuteMsg { msg }],
                    ttl: Some(Expiration::Never {}),
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
    test_ctx.start_relayer();

    // Verify that both messages are in timeout state
    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        0,
        &ExecutionResult::Timeout,
    );

    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        1,
        &ExecutionResult::Timeout,
    );

    info!("Both messages correctly timed out");

    info!("Retrying resending the message without TTL...");
    let error = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY,
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
        ContractError::Unauthorized(
            valence_authorization::error::UnauthorizedReason::TtlExpired {}
        )
        .to_string()
        .as_str()
    ));

    info!("Retrying resending the message with TTL...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &predicted_authorization_contract_address.clone(),
        USER_KEY,
        &serde_json::to_string(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::RetryMsgs { execution_id: 1 },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // This should bridge and enqueue into the processor
    info!("Querying the batch from the processor...");
    let mut attempts = 0;
    let mut batches;
    loop {
        attempts += 1;
        batches = get_processor_queue_items(
            &mut test_ctx,
            &predicted_processor_on_juno_address,
            Priority::Medium,
        );

        if batches.len() == 1 {
            info!("Batch found!");
            break;
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));
    }

    assert_eq!(batches[0].id, 1);

    info!("Stopping the relayer again before ticking the processor to force a timeout...");
    test_ctx.stop_relayer();

    info!("Ticking the processor to trigger sending the callback...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &predicted_processor_on_juno_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // Wait enough time to force the time out
    std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));

    info!("Restarting the relayer...");
    test_ctx.start_relayer();

    // The polytone callback in the processor should have timed out
    info!("Querying the callback from the processor...");
    let mut attempts = 0;
    let mut callback_info;
    loop {
        attempts += 1;
        callback_info = get_processor_pending_polytone_callback(
            &mut test_ctx,
            &predicted_processor_on_juno_address,
            1,
        );

        if callback_info.state.eq(&PolytoneCallbackState::TimedOut) {
            info!("Callback successfully timed out!");
            break;
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));
    }

    // Now we should be able to retry the callback permissionlessly
    info!("Retrying the callback...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &predicted_processor_on_juno_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::RetryCallback { execution_id: 1 },
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Querying the execution result on the authorization contract...");
    verify_authorization_execution_result(
        &mut test_ctx,
        &predicted_authorization_contract_address,
        1,
        &ExecutionResult::Rejected("anything".to_string()),
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
                    .get_request_builder(JUNO_CHAIN_NAME),
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
            test_ctx.stop_relayer();
            test_ctx.start_relayer();
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

        match &external_domains.first().unwrap().connector {
            Connector::PolytoneNote { state, .. } => {
                if state.eq(expected_state) {
                    info!("Target state reached!");
                    break;
                } else {
                    info!("Waiting for the right state, current state: {:?}", state);
                }
            }
        }

        if attempts % 5 == 0 {
            // Sometimes the relayer doesn't pick up the changes, so we restart it
            test_ctx.stop_relayer();
            test_ctx.start_relayer();
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));
    }
}

fn verify_authorization_execution_result(
    test_ctx: &mut TestContext,
    authorization_address: &str,
    execution_id: u64,
    expected_result: &ExecutionResult,
) {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let callback_info: ProcessorCallbackInfo = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                authorization_address,
                &serde_json::to_string(
                    &valence_authorization_utils::msg::QueryMsg::ProcessorCallback { execution_id },
                )
                .unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        let result_matches = match (expected_result, &callback_info.execution_result) {
            (ExecutionResult::Rejected(_), ExecutionResult::Rejected(_)) => true,
            _ => callback_info.execution_result.eq(expected_result),
        };

        if result_matches {
            info!("Target execution result reached!");
            break;
        } else {
            info!(
                "Waiting for the right execution result, current execution result: {:?}",
                callback_info.execution_result
            );
        }

        if attempts % 5 == 0 {
            // Sometimes the relayer doesn't pick up the changes, so we restart it
            test_ctx.stop_relayer();
            test_ctx.start_relayer();
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(TIMEOUT_SECONDS));
    }
}

fn get_processor_queue_items(
    test_ctx: &mut TestContext,
    processor_address: &str,
    priority: Priority,
) -> Vec<MessageBatch> {
    serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(JUNO_CHAIN_NAME),
            processor_address,
            &serde_json::to_string(&valence_processor_utils::msg::QueryMsg::GetQueue {
                from: None,
                to: None,
                priority,
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap()
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
                .get_request_builder(JUNO_CHAIN_NAME),
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
