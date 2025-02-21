use std::{error::Error, str::FromStr, time::Duration};
// import e2e test utilities
use cosmwasm_std::{Binary, Decimal256, Int64, Uint64};
use valence_e2e::utils::{
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
use valence_authorization_utils::msg::ProcessorMessage;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up the Neutron and Osmosis chains with a transfer channel
    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    // Get the IBC denom of NTRN on Osmosis
    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    // Setup the CL pool on Osmosis
    let pool_id = setup_cl_pool(&mut test_ctx, &ntrn_on_osmo_denom, OSMOSIS_CHAIN_DENOM)?;

    // Provide environment context to config
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

    // Configure the Polytone connection. Polytone is required to transport
    // program messages from one domain to the other. Note the token denoms are
    // required as there is a fee associated with instantiating Polytone on each chain.
    setup_polytone(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        OSMOSIS_CHAIN_NAME,
        NEUTRON_CHAIN_ID,
        OSMOSIS_CHAIN_ID,
        NEUTRON_CHAIN_DENOM,
        OSMOSIS_CHAIN_DENOM,
    )?;

    let ntrn_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let osmo_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(OSMOSIS_CHAIN_NAME.to_string());

    // Build the program configuration
    let mut program_config = osmo_cl_example::osmo_cl::my_osmosis_cl_program(
        osmo_domain.clone(),
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(), // owner
        pool_id,                              // target pool id
        &ntrn_on_osmo_denom,                  // denom_1
        OSMOSIS_CHAIN_DENOM,                  // denom_2
        Int64::from(-1_000_000),              // lower tick
        Int64::from(1_000_000),               // upper tick
    )?;

    info!("Initialize the program...");
    use_manager_init(&mut program_config)?;

    // Retrieved the instantiated accounts from the chain
    let input_acc_addr = program_config.get_account(0u64)?.addr.clone().unwrap();
    let output_acc_addr = program_config.get_account(1u64)?.addr.clone().unwrap();
    let final_output_acc_addr = program_config.get_account(2u64)?.addr.clone().unwrap();

    // log addresses of the input accounts
    info!("input_acc_addr: {input_acc_addr}");
    info!("output_acc_addr: {output_acc_addr}");
    info!("final_output_acc_addr: {final_output_acc_addr}");

    // Fund the input account on Osmosis with NTRN and OSMO
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

    // Log the balances of the input account
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

    // Create the provide liquidity message
    let lp_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
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

    // execute provide liquidity message
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

    // get the CL position
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

    // create withdraw liquidity message
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

    // execute withdraw liquidity
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

    assert_eq!(output_acc_cl_positions.positions.len(), 0);
    assert_eq!(final_acc_bal.len(), 2);

    info!("asserting authorizations callbacks state sync...");
    confirm_authorizations_callback_state(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &authorization_contract_address,
        1,
    )?;

    Ok(())
}