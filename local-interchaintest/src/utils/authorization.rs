use std::{env, error::Error, time::Duration};

use cosmwasm_std_old::Uint64;
use localic_std::{
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query, CosmWasm},
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_DENOM,
};
use log::info;
use serde_json::Value;
use valence_authorization_utils::{
    callback::ProcessorCallbackInfo,
    msg::{CallbackProxy, Connector, ExternalDomainInfo, PermissionedMsg},
};
use valence_processor_utils::msg::PolytoneContracts;

use crate::utils::{polytone::salt_for_proxy, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON};

use super::POLYTONE_ARTIFACTS_PATH;

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

    info!("{}", authorization_contract_path);

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
    local_cache: &str,
    path: &str,
    salt: String,
    authorization_contract: &str,
) -> Result<String, Box<dyn Error>> {
    // Upload the processor contract to the chain
    let current_dir = env::current_dir()?;
    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());

    info!("Uploading processor contract to {}", chain_name);
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(chain_name)
        .send_single_contract(&processor_contract_path)?;

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
    info!(
        "Polytone Note on Neutron: {}",
        polytone_note_on_neutron_address
    );
    std::thread::sleep(Duration::from_secs(1));
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
    info!(
        "Polytone Voice on Neutron: {}",
        polytone_voice_on_neutron_address
    );

    let polytone_note_on_external_domain_address = polytone_note_on_external_domain
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-external-domain",
            None,
            &format!(
                "--gas=auto --gas-adjustment=3.0 --fees {}{}",
                5_000_000, OSMOSIS_CHAIN_DENOM
            ),
        )
        .unwrap()
        .address;
    info!(
        "Polytone Note on {}: {}",
        chain_name, polytone_note_on_external_domain_address
    );
    std::thread::sleep(Duration::from_secs(1));
    let polytone_voice_on_external_domain_address = polytone_voice_on_external_domain
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&external_domain_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-external-domain",
            None,
            &format!(
                "--gas=auto --gas-adjustment=3.0 --fees {}{}",
                5_000_000, OSMOSIS_CHAIN_DENOM
            ),
        )
        .unwrap()
        .address;
    info!(
        "Polytone Voice on {}: {}",
        chain_name, polytone_voice_on_external_domain_address
    );

    info!("Creating WASM connections...");

    let relayer = Relayer::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );
    std::thread::sleep(Duration::from_secs(1));

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
    std::thread::sleep(Duration::from_secs(1));

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

    let connection_id_neutron_to_external_domain =
        neutron_channels.iter().find_map(|neutron_channel| {
            if neutron_channel.port_id
                == format!("wasm.{}", polytone_note_on_neutron_address.clone())
            {
                neutron_channel.connection_hops.first().cloned()
            } else {
                None
            }
        });
    info!(
        "Connection ID of Wasm connection Neutron to {}: {:?}",
        chain_name, connection_id_neutron_to_external_domain
    );

    let external_domain_channels = relayer.get_channels(chain_id).unwrap();

    let connection_id_external_domain_to_neutron =
        external_domain_channels
            .iter()
            .find_map(|external_domain_channel| {
                if external_domain_channel.port_id
                    == format!("wasm.{}", polytone_note_on_external_domain_address.clone())
                {
                    external_domain_channel.connection_hops.first().cloned()
                } else {
                    None
                }
            });
    info!(
        "Connection ID of Wasm connection {} to Neutron: {:?}",
        chain_name, connection_id_external_domain_to_neutron
    );

    let salt_for_proxy_on_external_domain = salt_for_proxy(
        &connection_id_external_domain_to_neutron.unwrap(),
        &format!("wasm.{}", polytone_note_on_neutron_address.clone()),
        authorization_contract,
    );

    // Predict the address the proxy on external_domain for the authorization module
    let predicted_proxy_address_on_external_domain = test_ctx
        .get_built_contract_address()
        .src(chain_name)
        .creator(&polytone_voice_on_external_domain_address.clone())
        .contract("polytone_proxy")
        .salt_hex_encoded(&hex::encode(salt_for_proxy_on_external_domain))
        .get();

    info!(
        "Predicted proxy address on {}: {}",
        chain_name, predicted_proxy_address_on_external_domain
    );

    // To predict the proxy address on neutron for the processor on external_domain we need to first predict the processor address
    let predicted_processor_on_external_domain_address = test_ctx
        .get_built_contract_address()
        .src(chain_name)
        .creator(chain_admin_addr)
        .contract("valence_processor")
        .salt_hex_encoded(&salt)
        .get();

    // Let's now predict the proxy
    let salt_for_proxy_on_neutron = salt_for_proxy(
        &connection_id_neutron_to_external_domain.unwrap(),
        &format!(
            "wasm.{}",
            polytone_note_on_external_domain
                .contract_addr
                .clone()
                .unwrap()
        ),
        &predicted_processor_on_external_domain_address,
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

    let processor_code_id_on_external_domain = test_ctx
        .get_contract()
        .src(chain_name)
        .contract("valence_processor")
        .get_cw()
        .code_id
        .unwrap();
    info!(
        "processor code id on external domain: {:?}",
        processor_code_id_on_external_domain
    );
    std::thread::sleep(Duration::from_secs(3));

    // Instantiate processor
    test_ctx
        .build_tx_instantiate2()
        .with_chain_name(chain_name)
        .with_label("processor")
        .with_code_id(processor_code_id_on_external_domain)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(processor_instantiate_msg).unwrap())
        .with_flags(&format!("--fees {}{}", 5_000_000, OSMOSIS_CHAIN_DENOM))
        .send()
        .unwrap();

    info!("Processor instantiated on {}!", chain_name);
    info!(
        "Processor address: {}",
        predicted_processor_on_external_domain_address
    );

    info!("Adding external domain to the authorization contract...");
    let add_external_domain_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        PermissionedMsg::AddExternalDomains {
            external_domains: vec![ExternalDomainInfo {
                name: chain_name.to_string(),
                execution_environment:
                    valence_authorization_utils::domain::ExecutionEnvironment::CosmWasm,
                connector: Connector::PolytoneNote {
                    address: polytone_note_on_neutron_address.clone(),
                    timeout_seconds,
                },
                processor: predicted_processor_on_external_domain_address.clone(),
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
        authorization_contract,
        DEFAULT_KEY,
        &serde_json::to_string(&add_external_domain_msg).unwrap(),
        &format!(
            "--gas=auto --gas-adjustment=3.0 --fees {}{}",
            5_000_000, OSMOSIS_CHAIN_DENOM
        ),
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(5));

    Ok(predicted_processor_on_external_domain_address)
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
