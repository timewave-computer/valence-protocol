use std::{
    env,
    error::Error,
    ops::{Add, Sub},
};

use cosmwasm_std_old::Coin as BankCoin;
use local_interchaintest::utils::{
    base_account::{approve_service, create_base_accounts},
    GAS_FLAGS, LOGS_FILE_PATH, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, JUNO_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;

use valence_generic_ibc_transfer_service::msg::{IbcTransferAmount, ServiceConfigUpdate};
use valence_neutron_ibc_transfer_service::msg::{FunctionMsgs, ServiceConfig};
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_juno().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, JUNO_CHAIN_NAME)
        .build()?;

    let neutron_on_juno_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NTRN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(JUNO_CHAIN_NAME)
        .get();

    // Let's upload the base account contract to Neutron
    let current_dir = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&base_account_contract_path)?;

    // Get the code id
    let code_id_base_account = test_ctx
        .get_contract()
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    // Create 1 base accounts on Neutron, to be the input account for the IBC transfer service
    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        code_id_base_account,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        1,
    );
    let input_account = base_accounts[0].clone();
    info!("Input account: {:?}", input_account);

    // Send native tokens to the input account
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &input_account,
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: 1_000_000_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let start_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == NTRN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start input balance: {:?}", start_input_balance);

    // We need a normal account on Juno to be the output account for the IBC transfer service
    let output_account = test_ctx.get_chain(JUNO_CHAIN_NAME).admin_addr.clone();
    info!("Output account: {:?}", output_account);

    let start_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == neutron_on_juno_denom)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start output balance: {:?}", start_output_balance);

    info!("Prepare the IBC transfer service contract");
    let ibc_transfer_svc_contract_path = format!(
        "{}/artifacts/valence_neutron_ibc_transfer_service.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&ibc_transfer_svc_contract_path)?;

    // Get the code id
    let code_id_ibc_transfer_svc = test_ctx
        .get_contract()
        .contract("valence_neutron_ibc_transfer_service")
        .get_cw()
        .code_id
        .unwrap();

    info!("Creating IBC transfer service contract");
    let transfer_amount = 100_000_000_000u128;
    let ntrn_juno_path = &(NEUTRON_CHAIN_NAME.to_string(), JUNO_CHAIN_NAME.to_string());
    let ibc_transfer_instantiate_msg = valence_service_utils::msg::InstantiateMsg::<ServiceConfig> {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: ServiceConfig::new(
            ServiceAccountType::Addr(input_account.clone()),
            output_account.clone(),
            UncheckedDenom::Native(NTRN_DENOM.to_string()),
            IbcTransferAmount::FixedAmount(transfer_amount.into()),
            "".to_owned(),
            valence_neutron_ibc_transfer_service::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .transfer_channel_ids
                    .get(ntrn_juno_path)
                    .unwrap()
                    .clone(),
                ibc_transfer_timeout: Some(600u64.into()),
            },
        ),
    };
    info!(
        "IBC Transfer instantiate message: {:?}",
        ibc_transfer_instantiate_msg
    );

    let ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_ibc_transfer_svc,
        &serde_json::to_string(&ibc_transfer_instantiate_msg).unwrap(),
        "ibc_transfer",
        None,
        "",
    )
    .unwrap();

    info!("IBC Transfer service: {}", ibc_transfer.address.clone());

    // Approve the services for the base account
    approve_service(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        ibc_transfer.address.clone(),
    );

    info!("Initiate IBC transfer");
    let ibc_transfer_msg = &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
        FunctionMsgs::IbcTransfer {},
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&ibc_transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the IBC Transfer service!");
    std::thread::sleep(std::time::Duration::from_secs(10));

    let ibc_fee = 2000u128;

    info!("Verifying balances...");
    let end_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == NTRN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_input_balance,
        start_input_balance.sub(transfer_amount).add(ibc_fee / 2)
    );

    let end_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == neutron_on_juno_denom)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_output_balance,
        start_output_balance.add(transfer_amount).sub(ibc_fee)
    );

    // Update config to transfer the input account's full remaining balance
    info!("Update service configuration...");
    let new_config = valence_neutron_ibc_transfer_service::msg::ServiceConfigUpdate {
        input_addr: None,
        output_addr: None,
        denom: None,
        amount: Some(IbcTransferAmount::FullAmount),
        memo: None,
        remote_chain_info: None,
        denom_to_pfm_map: None,
    };
    let upd_cfg_msg =
        valence_service_utils::msg::ExecuteMsg::<FunctionMsgs, ServiceConfigUpdate>::UpdateConfig {
            new_config,
        };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&upd_cfg_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("Initiate IBC transfer");
    let ibc_transfer_msg = &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
        FunctionMsgs::IbcTransfer {},
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&ibc_transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the IBC Transfer service!");
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("Verifying balances...");
    let prev_end_input_balance = end_input_balance;
    let end_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == NTRN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(end_input_balance, ibc_fee / 2);

    let prev_end_output_balance = end_output_balance;
    let end_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == neutron_on_juno_denom)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_output_balance,
        prev_end_output_balance
            .add(prev_end_input_balance)
            .sub(ibc_fee)
    );

    info!("IBC transfer successful!");

    Ok(())
}
