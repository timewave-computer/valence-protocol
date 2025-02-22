use std::{error::Error, time::Duration};

use cosmwasm_std::Binary;
use valence_e2e::utils::{
    authorization::confirm_authorizations_callback_state,
    manager::{
        setup_manager, use_manager_init, OSMOSIS_GAMM_LPER_NAME, OSMOSIS_GAMM_LWER_NAME,
        POLYTONE_NOTE_NAME, POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME,
    },
    osmosis::gamm::setup_gamm_pool,
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

#[allow(dead_code)]
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

    // configure the polytone connections (middleware plumbing)
    setup_polytone(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        OSMOSIS_CHAIN_NAME,
        NEUTRON_CHAIN_ID,
        OSMOSIS_CHAIN_ID,
        NEUTRON_CHAIN_DENOM,
        OSMOSIS_CHAIN_DENOM,
    )?;

    let osmo_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(OSMOSIS_CHAIN_NAME.to_string());
    let ntrn_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let mut program_config = osmo_gamm_example::osmo_gamm::my_osmosis_gamm_program(
        osmo_domain.clone(),
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        pool_id,
        &ntrn_on_osmo_denom,
        OSMOSIS_CHAIN_DENOM,
    )?;

    info!("initializing manager...");
    use_manager_init(&mut program_config)?;

    let input_acc_addr = program_config.get_account(0u64)?.addr.clone().unwrap();
    let output_acc_addr = program_config.get_account(1u64)?.addr.clone().unwrap();
    let final_output_acc_addr = program_config.get_account(2u64)?.addr.clone().unwrap();

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
                amount: 1_200_000u128.into(),
            },
        ],
        &cosmwasm_std_old::Coin {
            denom: OSMOSIS_CHAIN_DENOM.to_string(),
            amount: 1_000_000u128.into(),
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
                valence_osmosis_gamm_lper::msg::FunctionMsgs::ProvideDoubleSidedLiquidity {
                    expected_spot_price: None,
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

    info!("confirmed liquidity provision! asserting authorizations callbacks state sync...");
    confirm_authorizations_callback_state(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &authorization_contract_address,
        0,
    )?;

    let lw_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_osmosis_gamm_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                    expected_spot_price: None,
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

    info!("confirming that osmosis processor enqueued the withdraw_liquidity_msg...");
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
    confirm_authorizations_callback_state(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &authorization_contract_address,
        1,
    )?;

    Ok(())
}
