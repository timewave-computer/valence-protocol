use std::{collections::HashMap, error::Error, time::Duration};

use cosmwasm_std::Binary;
use cosmwasm_std_old::Uint64;
use local_interchaintest::utils::{
    manager::{
        get_global_config, setup_manager, use_manager_init, OSMOSIS_GAMM_LPER_NAME,
        OSMOSIS_GAMM_LWER_NAME, POLYTONE_NOTE_NAME, POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME,
    },
    processor::get_processor_queue_items,
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_OSMO_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};

use localic_std::{
    modules::{
        bank,
        cosmwasm::{contract_execute, contract_query, CosmWasm},
    },
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    GAIA_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM,
    OSMOSIS_CHAIN_ID, OSMOSIS_CHAIN_NAME,
};
use log::info;
use serde_json::Value;
use valence_authorization_utils::{
    authorization::Priority,
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    callback::ProcessorCallbackInfo,
    domain::Domain,
    msg::ProcessorMessage,
};
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    bridge::{Bridge, PolytoneSingleChainInfo},
    library::{LibraryConfig, LibraryInfo},
    program_config_builder::ProgramConfigBuilder,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let pool_id = setup_gamm_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_OSMO_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            OSMOSIS_GAMM_LPER_NAME,
            OSMOSIS_GAMM_LWER_NAME,
            POLYTONE_NOTE_NAME,
            POLYTONE_VOICE_NAME,
            POLYTONE_PROXY_NAME,
        ],
    )?;

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let osmo_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(OSMOSIS_CHAIN_NAME.to_string());
    let ntrn_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let gamm_input_acc_info = AccountInfo::new(
        "gamm_input".to_string(),
        &osmo_domain,
        AccountType::default(),
    );
    let gamm_output_acc_info = AccountInfo::new(
        "gamm_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );
    let final_output_acc_info = AccountInfo::new(
        "final_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    let gamm_input_acc = builder.add_account(gamm_input_acc_info);
    let gamm_output_acc = builder.add_account(gamm_output_acc_info);
    let final_output_acc = builder.add_account(final_output_acc_info);

    info!("gamm input acc: {:?}", gamm_input_acc);
    info!("gamm output acc: {:?}", gamm_output_acc);
    info!("final output acc: {:?}", final_output_acc);

    let gamm_lper_config = valence_osmosis_gamm_lper::msg::LibraryConfig {
        input_addr: gamm_input_acc.clone(),
        output_addr: gamm_output_acc.clone(),
        lp_config: valence_osmosis_gamm_lper::msg::LiquidityProviderConfig {
            pool_id,
            pool_asset_2: OSMOSIS_CHAIN_DENOM.to_string(),
            pool_asset_1: ntrn_on_osmo_denom.to_string(),
        },
    };

    let gamm_lwer_config = valence_osmosis_gamm_withdrawer::msg::LibraryConfig {
        input_addr: gamm_output_acc.clone(),
        output_addr: final_output_acc.clone(),
        lw_config: valence_osmosis_gamm_withdrawer::msg::LiquidityWithdrawerConfig { pool_id },
    };

    let gamm_lper_library = builder.add_library(LibraryInfo::new(
        "test_gamm_lp".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisGammLper(gamm_lper_config.clone()),
    ));

    let gamm_lwer_library = builder.add_library(LibraryInfo::new(
        "test_gamm_lw".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisGammWithdrawer(gamm_lwer_config.clone()),
    ));

    // establish the input_acc -> lper_lib -> output_acc link
    builder.add_link(
        &gamm_lper_library,
        vec![&gamm_input_acc],
        vec![&gamm_output_acc],
    );
    // establish the output_acc -> lwer_lib -> final_output_acc link
    builder.add_link(
        &gamm_lwer_library,
        vec![&gamm_output_acc],
        vec![&final_output_acc],
    );

    let gamm_lper_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(gamm_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let gamm_lwer_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(gamm_lwer_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("provide_liquidity")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(gamm_lper_function)
                    .build(),
            )
            .build(),
    );
    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("withdraw_liquidity")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(gamm_lwer_function)
                    .build(),
            )
            .build(),
    );

    let mut program_config = builder.build();

    setup_polytone(&mut test_ctx)?;

    use_manager_init(&mut program_config)?;
    info!("manager initialized successfully!");

    let input_acc_addr = program_config
        .get_account(gamm_input_acc)?
        .addr
        .clone()
        .unwrap();
    let output_acc_addr = program_config
        .get_account(gamm_output_acc)?
        .addr
        .clone()
        .unwrap();
    let final_output_acc_addr = program_config
        .get_account(final_output_acc)?
        .addr
        .clone()
        .unwrap();

    info!("input_acc_addr: {input_acc_addr}");
    info!("output_acc_addr: {output_acc_addr}");
    info!("final_output_acc_addr: {final_output_acc_addr}");

    let input_acc_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &input_acc_addr,
    );
    let output_acc_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &output_acc_addr,
    );
    let final_output_acc_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &final_output_acc_addr,
    );
    info!("input_acc_balances: {:?}", input_acc_balances);
    info!("output_acc_balances: {:?}", output_acc_balances);
    info!("final_output_acc_balances: {:?}", final_output_acc_balances);

    info!("funding the input account...");
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        &input_acc_addr,
        &[
            cosmwasm_std_old::Coin {
                denom: ntrn_on_osmo_denom.to_string(),
                amount: 1_000_000u128.into(),
            },
            cosmwasm_std_old::Coin {
                denom: OSMOSIS_CHAIN_DENOM.to_string(),
                amount: 1_000_000u128.into(),
            },
        ],
        &cosmwasm_std_old::Coin {
            denom: OSMOSIS_CHAIN_DENOM.to_string(),
            amount: 1_000_000u128.into(),
        },
    )
    .unwrap();

    std::thread::sleep(Duration::from_secs(3));

    let input_acc_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &input_acc_addr,
    );
    info!("input_acc_balances: {:?}", input_acc_balances);

    // Get authorization and processor contract addresses
    let authorization_contract_address =
        program_config.authorization_data.authorization_addr.clone();
    let osmo_processor_contract_address = program_config
        .get_processor_addr(&osmo_domain.to_string())
        .unwrap();
    let ntrn_processor_contract_address = program_config
        .get_processor_addr(&ntrn_domain.to_string())
        .unwrap();

    info!("authorization contract address: {authorization_contract_address}");
    info!("osmo processor contract address: {osmo_processor_contract_address}");
    info!("ntrn processor contract address: {ntrn_processor_contract_address}");

    let lp_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(
            serde_json::to_vec(
                &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                    valence_osmosis_gamm_lper::msg::FunctionMsgs::ProvideDoubleSidedLiquidity {
                        expected_spot_price: None,
                    },
                ),
            )
            .unwrap(),
        ),
    };
    let provide_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "provide_liquidity".to_string(),
            messages: vec![lp_message],
            ttl: None,
        },
    );

    let enqueue_resp = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&provide_liquidity_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    info!("enqueue authorizations response: {:?}", enqueue_resp);

    info!("provide_liquidity_msg sent to the authorization contract!");

    confirm_remote_domain_processor_queue_length(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &osmo_processor_contract_address,
        1,
    );

    info!("Ticking osmo processor...");
    let resp = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_processor_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        &format!(
            "--gas=auto --gas-adjustment=3.0 --fees {}{}",
            5_000_000, OSMOSIS_CHAIN_DENOM
        ),
    )
    .unwrap();
    info!("osmo processor tick response: {:?}", resp);

    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("asserting that providing liquidity worked...");
    let input_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &input_acc_addr,
    );
    let output_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &output_acc_addr,
    );
    info!("input acc bal: {:?}", input_acc_bal);
    info!("output acc bal: {:?}", output_acc_bal);

    assert_eq!(input_acc_bal.len(), 0);
    assert_eq!(output_acc_bal.len(), 1);
    assert_eq!(output_acc_bal[0].denom, "gamm/pool/1".to_string());

    info!("confirmed liquidity provision!");
    info!("asserting authorizations callbacks state sync...");
    let mut tries = 0;
    loop {
        let query_processor_callbacks_response: Value = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &authorization_contract_address,
                &serde_json::to_string(
                    &valence_authorization_utils::msg::QueryMsg::ProcessorCallbacks {
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        info!(
            "neutron processor callbacks response: {:?}",
            query_processor_callbacks_response
        );

        if query_processor_callbacks_response.is_null() {
            info!("No authorization callbacks not found yet...");
        } else {
            info!("Callbacks found!");
            let processor_callback_info: Vec<ProcessorCallbackInfo> =
                serde_json::from_value(query_processor_callbacks_response).unwrap();
            info!(
                "processor callback info on authorizations: {:?}",
                processor_callback_info
            );

            match processor_callback_info[0].execution_result {
                valence_authorization_utils::callback::ExecutionResult::Success => {
                    info!("authorizations module callback result is success!");
                    break;
                }
                _ => {
                    info!(
                        "Callback state: {:?}",
                        processor_callback_info[0].execution_result
                    );
                }
            };
        }

        tries += 1;
        if tries == 10 {
            panic!("Batch not found after 10 tries");
        } else {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    let lw_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(
            serde_json::to_vec(
                &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                    valence_osmosis_gamm_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {},
                ),
            )
            .unwrap(),
        ),
    };
    let withdraw_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "withdraw_liquidity".to_string(),
            messages: vec![lw_message],
            ttl: None,
        },
    );

    let enqueue_resp = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&withdraw_liquidity_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    info!("enqueue authorizations response: {:?}", enqueue_resp);

    info!("withdraw_liquidity_msg sent to the authorization contract!");

    confirm_remote_domain_processor_queue_length(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &osmo_processor_contract_address,
        1,
    );

    info!("Ticking osmo processor...");
    let resp = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_processor_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        &format!(
            "--gas=auto --gas-adjustment=3.0 --fees {}{}",
            5_000_000, OSMOSIS_CHAIN_DENOM
        ),
    )
    .unwrap();
    info!("osmo processor tick response: {:?}", resp);

    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("asserting that withdrawing liquidity worked...");
    let output_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &output_acc_addr,
    );
    let final_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &final_output_acc_addr,
    );
    info!("output acc bal: {:?}", output_acc_bal);
    info!("final acc bal: {:?}", final_acc_bal);

    assert_eq!(output_acc_bal.len(), 0);
    assert_eq!(final_acc_bal.len(), 2);

    info!("asserting authorizations callbacks state sync...");
    let mut tries = 0;
    loop {
        let query_processor_callbacks_response: Value = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                &authorization_contract_address,
                &serde_json::to_string(
                    &valence_authorization_utils::msg::QueryMsg::ProcessorCallbacks {
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        info!(
            "neutron processor callbacks response: {:?}",
            query_processor_callbacks_response
        );

        if query_processor_callbacks_response.is_null() {
            info!("No authorization callbacks not found yet...");
        } else {
            info!("Callbacks found!");
            let processor_callback_info: Vec<ProcessorCallbackInfo> =
                serde_json::from_value(query_processor_callbacks_response).unwrap();
            info!(
                "processor callback info on authorizations: {:?}",
                processor_callback_info
            );

            match processor_callback_info.len() {
                2 => {
                    match processor_callback_info[1].execution_result {
                        valence_authorization_utils::callback::ExecutionResult::Success => {
                            info!("authorizations module callback result is success!");
                            break;
                        }
                        _ => {
                            info!(
                                "Callback state: {:?}",
                                processor_callback_info[1].execution_result
                            );
                        }
                    };
                }
                _ => {
                    info!(
                        "Callback state: {:?}",
                        processor_callback_info[1].execution_result
                    );
                }
            }
        }

        tries += 1;
        if tries == 10 {
            panic!("Batch not found after 10 tries");
        } else {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    Ok(())
}

fn setup_polytone(test_ctx: &mut TestContext) -> Result<(), Box<dyn Error>> {
    // Before setting up the external domains and the processor on the external domain, we are going to set up polytone and predict the proxy addresses on both sides
    let mut polytone_note_on_neutron = test_ctx
        .get_contract()
        .contract(POLYTONE_NOTE_NAME)
        .get_cw();

    let mut polytone_voice_on_neutron = test_ctx
        .get_contract()
        .contract(POLYTONE_VOICE_NAME)
        .get_cw();

    let polytone_proxy_on_neutron = test_ctx
        .get_contract()
        .contract(POLYTONE_PROXY_NAME)
        .get_cw();

    let mut polytone_note_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract(POLYTONE_NOTE_NAME)
        .get_cw();

    let mut polytone_voice_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract(POLYTONE_VOICE_NAME)
        .get_cw();

    let polytone_proxy_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract(POLYTONE_PROXY_NAME)
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

    let osmo_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_osmosis.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    info!("Instantiating polytone contracts on both domains");
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

    info!("Polytone Note on Neutron: {polytone_note_on_neutron_address}");
    std::thread::sleep(std::time::Duration::from_secs(2));

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

    info!("Polytone voice on neutron: {polytone_voice_on_neutron_address}",);
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_note_on_osmo_address = polytone_note_on_osmosis
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-osmosis",
            None,
            &format!("--fees {}{}", 5000, OSMOSIS_CHAIN_DENOM),
        )
        .unwrap()
        .address;

    info!("polytone note on osmosis: {polytone_note_on_osmo_address}");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_voice_on_osmo_address = polytone_voice_on_osmosis
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&osmo_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-osmosis",
            None,
            &format!("--fees {}{}", 5000, OSMOSIS_CHAIN_DENOM),
        )
        .unwrap()
        .address;
    info!("Polytone Voice on osmo: {polytone_voice_on_osmo_address}");

    std::thread::sleep(std::time::Duration::from_secs(2));
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
                Some(polytone_voice_on_osmo_address.clone()),
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
                Some(polytone_note_on_osmo_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    // Give some time to make sure the channels are open
    std::thread::sleep(Duration::from_secs(15));

    // Get the connection ids so that we can predict the proxy addresses
    let neutron_channels = relayer.get_channels(NEUTRON_CHAIN_ID).unwrap();

    let neutron_to_osmo_polytone_channel = neutron_channels
        .iter()
        .find_map(|neutron_channel| {
            if neutron_channel.port_id
                == format!("wasm.{}", polytone_note_on_neutron_address.clone())
            {
                Some(neutron_channel.clone())
            } else {
                None
            }
        })
        .unwrap();

    let osmo_channels = relayer.get_channels(OSMOSIS_CHAIN_ID).unwrap();

    let osmo_to_neutron_polytone_channel = osmo_channels
        .iter()
        .find_map(|osmo_channel| {
            if osmo_channel.port_id == format!("wasm.{}", polytone_note_on_osmo_address.clone()) {
                Some(osmo_channel.clone())
            } else {
                None
            }
        })
        .unwrap();

    let osmo_polytone_info = PolytoneSingleChainInfo {
        voice_addr: polytone_voice_on_osmo_address,
        note_addr: polytone_note_on_osmo_address,
        other_note_port: neutron_to_osmo_polytone_channel.port_id,
        connection_id: osmo_to_neutron_polytone_channel
            .connection_hops
            .first()
            .cloned()
            .unwrap(),
        channel_id: osmo_to_neutron_polytone_channel.channel_id,
    };
    let neutron_polytone_info = PolytoneSingleChainInfo {
        voice_addr: polytone_voice_on_neutron_address,
        note_addr: polytone_note_on_neutron_address,
        other_note_port: osmo_to_neutron_polytone_channel.port_id,
        connection_id: neutron_to_osmo_polytone_channel
            .connection_hops
            .first()
            .cloned()
            .unwrap(),
        channel_id: neutron_to_osmo_polytone_channel.channel_id,
    };

    let osmo_to_neutron_polytone_bridge_info: HashMap<String, PolytoneSingleChainInfo> =
        HashMap::from([
            (NEUTRON_CHAIN_NAME.to_string(), neutron_polytone_info),
            (OSMOSIS_CHAIN_NAME.to_string(), osmo_polytone_info),
        ]);

    let mut neutron_bridge_map: HashMap<String, Bridge> = HashMap::new();
    neutron_bridge_map.insert(
        OSMOSIS_CHAIN_NAME.to_string(),
        Bridge::Polytone(osmo_to_neutron_polytone_bridge_info),
    );

    let mut gc = get_global_config();

    gc.bridges
        .insert(NEUTRON_CHAIN_NAME.to_string(), neutron_bridge_map);

    Ok(())
}

fn setup_gamm_pool(
    test_ctx: &mut TestContext,
    denom_1: &str,
    denom_2: &str,
) -> Result<u64, Box<dyn Error>> {
    info!("transferring 1000 neutron tokens to osmo admin addr for pool creation...");
    test_ctx
        .build_tx_transfer()
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .with_amount(1_000_000_000u128)
        .with_recipient(OSMOSIS_CHAIN_ADMIN_ADDR)
        .with_denom(NEUTRON_CHAIN_DENOM)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        OSMOSIS_CHAIN_ADMIN_ADDR,
    );
    info!("osmosis chain admin addr balances: {:?}", token_balances);

    test_ctx
        .build_tx_create_osmo_pool()
        .with_weight(denom_1, 1)
        .with_weight(denom_2, 1)
        .with_initial_deposit(denom_1, 100_000_000)
        .with_initial_deposit(denom_2, 100_000_000)
        .send()?;

    // Get its id
    let pool_id = test_ctx
        .get_osmo_pool()
        .denoms(denom_1.into(), denom_2.to_string())
        .get_u64();

    info!("Gamm pool id: {:?}", pool_id);

    Ok(pool_id)
}

fn confirm_remote_domain_processor_queue_length(
    test_ctx: &mut TestContext,
    processor_domain: &str,
    processor_addr: &str,
    len: usize,
) {
    let mut tries = 0;
    loop {
        let items =
            get_processor_queue_items(test_ctx, processor_domain, processor_addr, Priority::Medium);
        println!("Items on {processor_domain}: {:?}", items);

        info!("processor queue length: {len}");

        if items.len() == len {
            break;
        } else if tries > 10 {
            panic!("Batch not found after 10 tries");
        }

        tries += 1;
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
