use std::{env, error::Error, time::Duration};

use cosmwasm_std::{instantiate2_address, Api, HexBinary};
use cosmwasm_std_old::Uint64;
use localic_std::{
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query, CosmWasm},
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
};
use log::info;
use serde_json::Value;
use valence_authorization_utils::{
    callback::{ExecutionResult, ProcessorCallbackInfo},
    msg::{CosmwasmBridgeInfo, ExternalDomainInfo, PermissionedMsg, PolytoneConnectorsInfo},
};
use valence_processor_utils::msg::PolytoneContracts;

use crate::utils::{polytone::salt_for_proxy, LOCAL_CODE_ID_CACHE_PATH_NEUTRON};

use super::{relayer::restart_relayer, POLYTONE_ARTIFACTS_PATH};
const MAX_ATTEMPTS: u64 = 50;

/// Sets up the authorization contract with its processor on a domain
pub fn set_up_authorization_and_processor(
    test_ctx: &mut TestContext,
    salt: String,
) -> Result<(String, String), Box<dyn Error>> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    // Upload the authorization contract to Neutron and the processor to both Neutron and Juno
    let current_dir = env::current_dir()?;

    let authorization_contract_path = format!(
        "{}/artifacts/valence_authorization.wasm",
        current_dir.display()
    );

    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&authorization_contract_path)?;
    uploader.send_single_contract(&processor_contract_path)?;

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
        processor_on_main_domain.address.clone()
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
        processor: processor_on_main_domain.address.clone(),
    };

    test_ctx
        .build_tx_instantiate2()
        .with_label("authorization")
        .with_code_id(authorization_code_id)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(authorization_instantiate_msg).unwrap())
        .send()
        .unwrap();

    info!(
        "Authorization contract address: {}",
        predicted_authorization_contract_address.clone()
    );

    Ok((
        predicted_authorization_contract_address,
        processor_on_main_domain.address,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn set_up_external_domain_with_polytone(
    test_ctx: &mut TestContext,
    chain_name: &str,
    chain_id: &str,
    chain_admin_addr: &str,
    chain_denom: &str,
    chain_prefix: &str,
    local_cache: &str,
    path: &str,
    salt: String,
    authorization_contract: &str,
) -> Result<String, Box<dyn Error>> {
    info!("Uploading polytone contracts to neutron");
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .send_with_local_cache(POLYTONE_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)
        .unwrap();

    info!("Uploading polytone contracts to {}", chain_name);
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(chain_name)
        .send_with_local_cache(POLYTONE_ARTIFACTS_PATH, local_cache)
        .unwrap();

    let mut polytone_note_on_neutron = test_ctx.get_contract().contract("polytone_note").get_cw();

    let mut polytone_voice_on_neutron = test_ctx.get_contract().contract("polytone_voice").get_cw();

    let polytone_proxy_on_neutron = test_ctx.get_contract().contract("polytone_proxy").get_cw();

    let mut polytone_note_on_external_domain = test_ctx
        .get_contract()
        .src(chain_name)
        .contract("polytone_note")
        .get_cw();

    let mut polytone_voice_on_external_domain = test_ctx
        .get_contract()
        .src(chain_name)
        .contract("polytone_voice")
        .get_cw();

    let polytone_proxy_on_external_domain = test_ctx
        .get_contract()
        .src(chain_name)
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

    let external_domain_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_external_domain.code_id.unwrap()),
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
            &format!(
                "--gas=auto --gas-adjustment=3.0 --fees {}{}",
                5_000_000, NEUTRON_CHAIN_DENOM
            ),
        )
        .unwrap()
        .address;
    info!("Polytone Note on Neutron: {polytone_note_on_neutron_address}",);

    let polytone_voice_on_neutron_address = polytone_voice_on_neutron
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&neutron_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-neutron",
            None,
            &format!(
                "--gas=auto --gas-adjustment=3.0 --fees {}{}",
                5_000_000, NEUTRON_CHAIN_DENOM
            ),
        )
        .unwrap()
        .address;
    info!("Polytone Voice on Neutron: {polytone_voice_on_neutron_address}",);

    let polytone_note_on_external_domain_address = polytone_note_on_external_domain
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-external-domain",
            None,
            &format!("--fees {}{}", 500_000, chain_denom),
        )
        .unwrap()
        .address;
    info!("Polytone Note on {chain_name}: {polytone_note_on_external_domain_address}",);

    let polytone_voice_on_external_domain_address = polytone_voice_on_external_domain
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&external_domain_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-external-domain",
            None,
            &format!("--fees {}{}", 500_000, chain_denom),
        )
        .unwrap()
        .address;
    info!("Polytone Voice on {chain_name}: {polytone_voice_on_external_domain_address}",);

    info!("Creating WASM connections...");

    let relayer = Relayer::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );

    polytone_note_on_neutron
        .create_wasm_connection(
            &relayer,
            path,
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(chain_name),
                None,
                None,
                Some(polytone_voice_on_external_domain_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    polytone_voice_on_neutron
        .create_wasm_connection(
            &relayer,
            path,
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(chain_name),
                None,
                None,
                Some(polytone_note_on_external_domain_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    // Give some time to make sure the channels are open
    std::thread::sleep(Duration::from_secs(15));

    // Get the connection ids so that we can predict the proxy addresses
    let neutron_channels = relayer.get_channels(NEUTRON_CHAIN_ID).unwrap();

    let connection_id_neutron_to_external_domain = neutron_channels
        .iter()
        .find_map(|neutron_channel| {
            if neutron_channel.port_id
                == format!("wasm.{}", polytone_note_on_neutron_address.clone())
            {
                info!(
                    "there are {} connection hops from neutron to other domain",
                    neutron_channel.connection_hops.len()
                );
                neutron_channel.connection_hops.first().cloned()
            } else {
                None
            }
        })
        .unwrap();
    info!(
        "Connection ID of Wasm connection Neutron to {chain_name}: {connection_id_neutron_to_external_domain}"
    );

    let external_domain_channels = relayer.get_channels(chain_id).unwrap();

    let connection_id_external_domain_to_neutron = external_domain_channels
        .iter()
        .find_map(|external_domain_channel| {
            if external_domain_channel.port_id
                == format!("wasm.{}", polytone_note_on_external_domain_address.clone())
            {
                info!(
                    "there are {} connection hops from other domain to neutron",
                    external_domain_channel.connection_hops.len()
                );
                external_domain_channel.connection_hops.first().cloned()
            } else {
                None
            }
        })
        .unwrap();
    info!(
        "Connection ID of Wasm connection {chain_name} to Neutron: {connection_id_external_domain_to_neutron}"
    );

    let salt_for_proxy_on_external_domain = salt_for_proxy(
        &connection_id_external_domain_to_neutron,
        &format!("wasm.{}", polytone_note_on_neutron_address.clone()),
        authorization_contract,
    );

    let external_proxy_code = polytone_proxy_on_external_domain.code_id.unwrap();

    // Predict the address the proxy on external_domain for the authorization module
    let predicted_proxy_address_on_external_domain = predict_remote_contract_address(
        test_ctx,
        external_proxy_code,
        chain_name,
        chain_prefix,
        &polytone_voice_on_external_domain_address,
        &salt_for_proxy_on_external_domain,
    )
    .unwrap();
    info!("Predicted proxy address on {chain_name}: {predicted_proxy_address_on_external_domain}");

    info!("Uploading processor contract to {chain_name}...");

    // Upload the processor contract to the chain
    let current_dir = env::current_dir()?;

    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(chain_name)
        .send_single_contract(&processor_contract_path)?;

    let external_processor_code_id = test_ctx
        .get_contract()
        .src(chain_name)
        .contract("valence_processor")
        .get_cw()
        .clone()
        .code_id
        .unwrap();

    // To predict the proxy address on neutron for the processor on external_domain we need to first predict the processor address
    let predicted_processor_on_external_domain_address = predict_remote_contract_address(
        test_ctx,
        external_processor_code_id,
        chain_name,
        chain_prefix,
        chain_admin_addr,
        hex::decode(&salt).unwrap().as_slice(),
    )
    .unwrap();
    info!(
        "Predicted external domain processor addr: {predicted_processor_on_external_domain_address}"
    );

    // Let's now predict the proxy
    let salt_for_proxy_on_neutron = salt_for_proxy(
        &connection_id_neutron_to_external_domain,
        &format!("wasm.{}", polytone_note_on_external_domain_address),
        &predicted_processor_on_external_domain_address,
    );
    let predicted_proxy_address_on_neutron = test_ctx
        .get_built_contract_address()
        .src(NEUTRON_CHAIN_NAME)
        .creator(&polytone_voice_on_neutron_address.clone())
        .contract("polytone_proxy")
        .salt_hex_encoded(&hex::encode(salt_for_proxy_on_neutron))
        .get();

    info!("Predicted proxy address on Neutron: {predicted_proxy_address_on_neutron}",);

    let timeout_seconds = 300;
    // Instantiate the processor on the external domain
    let processor_instantiate_msg = valence_processor_utils::msg::InstantiateMsg {
        authorization_contract: authorization_contract.to_string(),
        polytone_contracts: Some(PolytoneContracts {
            polytone_proxy_address: predicted_proxy_address_on_external_domain.clone(),
            polytone_note_address: polytone_note_on_external_domain_address.clone(),
            timeout_seconds,
        }),
    };
    std::thread::sleep(Duration::from_secs(3));

    let extra_flags = format!(
        "--gas=auto --gas-adjustment=3.0 --fees {}{}",
        5_000_000, chain_denom
    );

    test_ctx
        .build_tx_instantiate2()
        .with_chain_name(chain_name)
        .with_label("valence_processor")
        .with_code_id(external_processor_code_id)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(processor_instantiate_msg)?)
        .with_flags(&extra_flags)
        .send()
        .unwrap();

    info!("Processor instantiated on {chain_name}!");

    info!("Adding external domain to the authorization contract...");
    let add_external_domain_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        PermissionedMsg::AddExternalDomains {
            external_domains: vec![ExternalDomainInfo {
                name: chain_name.to_string(),
                execution_environment:
                    valence_authorization_utils::msg::ExecutionEnvironmentInfo::Cosmwasm(
                        CosmwasmBridgeInfo::Polytone(PolytoneConnectorsInfo {
                            polytone_note: valence_authorization_utils::msg::PolytoneNoteInfo {
                                address: polytone_note_on_neutron_address.clone(),
                                timeout_seconds,
                            },
                            polytone_proxy: predicted_proxy_address_on_neutron.clone(),
                        }),
                    ),
                processor: predicted_processor_on_external_domain_address.clone(),
            }],
        },
    );
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorization_contract,
        DEFAULT_KEY,
        &serde_json::to_string(&add_external_domain_msg).unwrap(),
        &format!(
            "--gas=auto --gas-adjustment=3.0 --fees {}{}",
            5_000_000, NEUTRON_CHAIN_DENOM
        ),
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(5));

    Ok(predicted_processor_on_external_domain_address)
}

fn predict_remote_contract_address(
    test_ctx: &TestContext,
    code_id: u64,
    chain_name: &str,
    chain_prefix: &str,
    creator_addr: &str,
    salt: &[u8],
) -> Result<String, Box<dyn Error>> {
    let resp = test_ctx
        .get_request_builder()
        .get_request_builder(chain_name)
        .query(&format!("q wasm code-info {code_id}"), false);

    let checksum = if let Some(data_hash) = resp["data_hash"].as_str() {
        HexBinary::from_hex(data_hash).unwrap()
    } else {
        panic!("failed to get data hash from response: {:?}", resp);
    };
    let mock_api = valence_program_manager::mock_api::MockApi::new(chain_prefix.to_string());
    let canonical_creator = mock_api.addr_canonicalize(creator_addr)?;

    let canonical_predicted_proxy_address_on_external_domain =
        instantiate2_address(&checksum, &canonical_creator, salt)?;

    let predicted_address_on_external_domain = mock_api
        .addr_humanize(&canonical_predicted_proxy_address_on_external_domain)
        .unwrap();

    Ok(predicted_address_on_external_domain.to_string())
}

/// queries the authorization contract processor callbacks and tries to confirm that
/// the processor callback with `execution_id` execution_result is `Success`.
/// retries for 10 times with a 5 second sleep in between. fails after 10 retries.
pub fn confirm_authorizations_callback_state(
    test_ctx: &mut TestContext,
    authorization_domain: &str,
    authorization_addr: &str,
    execution_id: u64,
) -> Result<(), Box<dyn Error>> {
    let mut tries = 0;
    loop {
        let query_processor_callbacks_response: Value = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(authorization_domain),
                authorization_addr,
                &serde_json::to_string(
                    &valence_authorization_utils::msg::QueryMsg::ProcessorCallbacks {
                        start_after: None,
                        limit: None,
                    },
                )?,
            )["data"]
                .clone(),
        )?;

        info!(
            "{authorization_domain} authorization mod processor callbacks: {:?}",
            query_processor_callbacks_response
        );

        if query_processor_callbacks_response.is_null() {
            info!("No authorization callbacks not found yet...");
        } else {
            let processor_callback_infos: Vec<ProcessorCallbackInfo> =
                serde_json::from_value(query_processor_callbacks_response)?;

            let callback_by_id = processor_callback_infos
                .iter()
                .find(|x| x.execution_id == execution_id);

            info!(
                "processor callback #{execution_id} info: {:?}",
                callback_by_id
            );

            if let Some(cb) = callback_by_id {
                match cb.execution_result {
                    valence_authorization_utils::callback::ExecutionResult::Success => {
                        info!("callback #{execution_id} execution = success!");
                        return Ok(());
                    }
                    _ => {
                        info!(
                            "callback #{execution_id} execution result: {:?}",
                            cb.execution_result
                        );
                    }
                }
            }
        }

        tries += 1;
        if tries == 10 {
            return Err("Batch not found after 10 tries".into());
        } else {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}

/// Helper function to verify authorization execution result in a certain amount of tries
pub fn verify_authorization_execution_result(
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
            (
                ExecutionResult::PartiallyExecuted(val1, _),
                ExecutionResult::PartiallyExecuted(val2, _),
            ) => val1 == val2,
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
            restart_relayer(test_ctx);
        }

        if attempts > MAX_ATTEMPTS {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(15));
    }
}
