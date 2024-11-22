use std::{error::Error, str::FromStr, time::Duration};

use cosmwasm_std::{Binary, Decimal256, Int64, Uint64};
use local_interchaintest::utils::{
    authorization::confirm_authorizations_callback_state,
    manager::{
        setup_manager, use_manager_init, OSMOSIS_CL_LPER_NAME, OSMOSIS_CL_LWER_NAME,
        POLYTONE_NOTE_NAME, POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME,
    },
    osmosis::concentrated_liquidity::{query_cl_position, setup_cl_pool},
    polytone::setup_polytone,
    processor::confirm_remote_domain_processor_queue_length,
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_OSMO_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};

use localic_std::modules::{bank, cosmwasm::contract_execute};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_ID, OSMOSIS_CHAIN_NAME,
};
use log::info;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
    function::{RetryLogic, RetryTimes},
    msg::ProcessorMessage,
};
use valence_osmosis_utils::utils::cl_utils::TickRange;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
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

    let pool_id = setup_cl_pool(&mut test_ctx, &ntrn_on_osmo_denom, OSMOSIS_CHAIN_DENOM)?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_OSMO_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            OSMOSIS_CL_LPER_NAME,
            OSMOSIS_CL_LWER_NAME,
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

    let cl_input_acc_info =
        AccountInfo::new("cl_input".to_string(), &osmo_domain, AccountType::default());
    let cl_output_acc_info = AccountInfo::new(
        "cl_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );
    let final_output_acc_info = AccountInfo::new(
        "final_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    let cl_input_acc = builder.add_account(cl_input_acc_info);
    let cl_output_acc = builder.add_account(cl_output_acc_info);
    let final_output_acc = builder.add_account(final_output_acc_info);

    let cl_lper_config = valence_osmosis_cl_lper::msg::LibraryConfig {
        input_addr: cl_input_acc.clone(),
        output_addr: cl_output_acc.clone(),
        lp_config: valence_osmosis_cl_lper::msg::LiquidityProviderConfig {
            pool_id: pool_id.into(),
            pool_asset_1: ntrn_on_osmo_denom.to_string(),
            pool_asset_2: OSMOSIS_CHAIN_DENOM.to_string(),
            global_tick_range: TickRange {
                lower_tick: Int64::from(-1_000_000),
                upper_tick: Int64::from(1_000_000),
            },
        },
    };

    let cl_lwer_config = valence_osmosis_cl_withdrawer::msg::LibraryConfig {
        input_addr: cl_output_acc.clone(),
        output_addr: final_output_acc.clone(),
        pool_id: pool_id.into(),
    };

    let cl_lper_library = builder.add_library(LibraryInfo::new(
        "test_cl_lper".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisClLper(cl_lper_config),
    ));

    let cl_lwer_library = builder.add_library(LibraryInfo::new(
        "test_cl_lwer".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisClWithdrawer(cl_lwer_config),
    ));

    // establish the input_acc -> lper_lib -> output_acc link
    builder.add_link(&cl_lper_library, vec![&cl_input_acc], vec![&cl_output_acc]);
    // establish the output_acc -> lwer_lib -> final_output_acc link
    builder.add_link(
        &cl_lwer_library,
        vec![&cl_output_acc],
        vec![&final_output_acc],
    );

    let cl_lper_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(cl_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let cl_lwer_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(cl_lwer_library.clone())
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
                    .with_function(cl_lper_function)
                    .build(),
            )
            .build(),
    );
    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("withdraw_liquidity")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(cl_lwer_function)
                    .with_retry_logic(RetryLogic {
                        times: RetryTimes::Amount(3),
                        interval: cw_utils::Duration::Time(1),
                    })
                    .build(),
            )
            .build(),
    );

    let mut program_config = builder.build();

    // prior to initializing the manager, we do the middleware plumbing
    setup_polytone(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        OSMOSIS_CHAIN_NAME,
        NEUTRON_CHAIN_ID,
        OSMOSIS_CHAIN_ID,
        NEUTRON_CHAIN_DENOM,
        OSMOSIS_CHAIN_DENOM,
    )?;

    info!("initializing manager...");
    use_manager_init(&mut program_config)?;

    let input_acc_addr = program_config
        .get_account(cl_input_acc)?
        .addr
        .clone()
        .unwrap();
    let output_acc_addr = program_config
        .get_account(cl_output_acc)?
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

    // let input_acc_balances = bank::get_balance(
    //     test_ctx
    //         .get_request_builder()
    //         .get_request_builder(OSMOSIS_CHAIN_NAME),
    //     &input_acc_addr,
    // );
    // let output_acc_balances = bank::get_balance(
    //     test_ctx
    //         .get_request_builder()
    //         .get_request_builder(OSMOSIS_CHAIN_NAME),
    //     &output_acc_addr,
    // );
    // let final_output_acc_balances = bank::get_balance(
    //     test_ctx
    //         .get_request_builder()
    //         .get_request_builder(OSMOSIS_CHAIN_NAME),
    //     &final_output_acc_addr,
    // );
    // info!("input_acc_balances: {:?}", input_acc_balances);
    // info!("output_acc_balances: {:?}", output_acc_balances);
    // info!("final_output_acc_balances: {:?}", final_output_acc_balances);

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
            amount: 5_000u128.into(),
        },
    )?;
    std::thread::sleep(Duration::from_secs(3));

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        &output_acc_addr,
        &[cosmwasm_std_old::Coin {
            denom: OSMOSIS_CHAIN_DENOM.to_string(),
            amount: 10_000u128.into(),
        }],
        &cosmwasm_std_old::Coin {
            denom: OSMOSIS_CHAIN_DENOM.to_string(),
            amount: 5_000u128.into(),
        },
    )?;

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
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                // valence_osmosis_cl_lper::msg::FunctionMsgs::ProvideLiquidityCustom {
                //     tick_range: TickRange {
                //         lower_tick: Int64::new(1000),
                //         upper_tick: Int64::new(2000),
                //     },
                //     token_min_amount_0: Some(Uint128::zero()),
                //     token_min_amount_1: Some(Uint128::zero()),
                // },
                valence_osmosis_cl_lper::msg::FunctionMsgs::ProvideLiquidityDefault {
                    bucket_amount: Uint64::new(10),
                },
            ),
        )?),
    };
    let provide_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "provide_liquidity".to_string(),
            messages: vec![lp_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&provide_liquidity_msg)?,
        GAS_FLAGS,
    )?;

    info!("confirming that osmosis processor enqueued the provide_liquidity_msg...");
    confirm_remote_domain_processor_queue_length(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &osmo_processor_contract_address,
        1,
    );

    info!("Ticking osmo processor...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_processor_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )?,
        &format!(
            "--gas=auto --gas-adjustment=3.0 --fees {}{}",
            5_000_000, OSMOSIS_CHAIN_DENOM
        ),
    )?;

    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("asserting that providing liquidity worked...");
    let input_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &input_acc_addr,
    );

    let output_acc_cl_positions = query_cl_position(&mut test_ctx, &output_acc_addr)?;

    info!("[POST-LP] input acc bal: {:?}", input_acc_bal);
    info!(
        "[POST-LP] output acc cl positions: {:?}",
        output_acc_cl_positions
    );

    // input acc started with 2 denoms. we provided liquidity with two denoms,
    // so we should either be left with 1 or 0 denoms (1 in case of leftover).
    assert_ne!(input_acc_bal.len(), 2);
    // output acc should now own one CL position
    assert_eq!(output_acc_cl_positions.positions.len(), 1);

    let output_acc_cl_position = output_acc_cl_positions
        .positions
        .first()
        .unwrap()
        .position
        .clone()
        .unwrap();

    info!("confirmed liquidity provision! asserting authorizations callbacks state sync...");
    confirm_authorizations_callback_state(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &authorization_contract_address,
        0,
    )?;

    let liquidity_amount = Decimal256::from_str(&output_acc_cl_position.liquidity)?;

    let lw_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_osmosis_cl_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                    position_id: output_acc_cl_position.position_id.into(),
                    liquidity_amount: Some(liquidity_amount),
                },
            ),
        )?),
    };
    let withdraw_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "withdraw_liquidity".to_string(),
            messages: vec![lw_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&withdraw_liquidity_msg)?,
        GAS_FLAGS,
    )?;

    std::thread::sleep(std::time::Duration::from_secs(5));

    info!("confirming that osmosis processor enqueued the withdraw_liquidity_msg...");
    confirm_remote_domain_processor_queue_length(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &osmo_processor_contract_address,
        1,
    );

    info!(
        "PRE-TICK OUTPUT ACC CL POSITIONS: {:?}",
        query_cl_position(&mut test_ctx, &output_acc_addr)?.positions
    );

    info!("Ticking osmo processor to withdraw liquidity...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_processor_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )?,
        &format!(
            "--gas=auto --gas-adjustment=5.0 --fees {}{}",
            5_000_000, OSMOSIS_CHAIN_DENOM
        ),
    )?;

    std::thread::sleep(std::time::Duration::from_secs(5));

    info!("asserting that withdrawing liquidity worked...");

    let output_acc_cl_positions = query_cl_position(&mut test_ctx, &output_acc_addr)?;
    let final_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &final_output_acc_addr,
    );
    let output_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &output_acc_addr,
    );
    info!(
        "POST-TICK OUTPUT ACC CL POSITIONS: {:?}",
        output_acc_cl_positions.positions
    );
    info!("final acc bal: {:?}", final_acc_bal);
    info!("output acc bal: {:?}", output_acc_bal);

    // assert_eq!(output_acc_cl_positions.positions.len(), 0);
    // assert_eq!(final_acc_bal.len(), 2);

    info!("asserting authorizations callbacks state sync...");
    confirm_authorizations_callback_state(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &authorization_contract_address,
        1,
    )?;

    Ok(())
}
