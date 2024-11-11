use std::{
    env,
    error::Error,
    ops::{Add, Sub},
};

use cosmwasm_std::Uint128;
use local_interchaintest::utils::{
    base_account::{approve_library, create_base_accounts},
    GAS_FLAGS, LOGS_FILE_PATH, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, JUNO_CHAIN_ADMIN_ADDR, JUNO_CHAIN_NAME,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
};
use log::info;

use valence_generic_ibc_transfer_library::msg::{
    FunctionMsgs, IbcTransferAmount, LibraryConfig, LibraryConfigUpdate,
};
use valence_library_utils::{denoms::UncheckedDenom, LibraryAccountType};

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

    // Let's upload the base account contract to Juno
    let current_dir = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_single_contract(&base_account_contract_path)?;

    // Get the code id
    let code_id_base_account = *test_ctx
        .get_chain(JUNO_CHAIN_NAME)
        .contract_codes
        .get("valence_base_account")
        .unwrap();

    // Create 1 base accounts on Juno, to be the input account for the IBC transfer library
    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        JUNO_CHAIN_NAME,
        code_id_base_account,
        JUNO_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        1,
    );
    let input_account = base_accounts[0].clone();
    info!("Input account: {:?}", input_account);

    // Send NTRN tokens to the input account on Juno
    test_ctx
        .build_tx_transfer()
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .with_amount(1_000_000_000_000u128)
        .with_recipient(&input_account)
        .with_denom(NTRN_DENOM)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let start_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == neutron_on_juno_denom)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start input balance: {:?}", start_input_balance);

    // We need a normal account on Neutron to be the output account for the IBC transfer library
    let output_account = test_ctx.get_chain(NEUTRON_CHAIN_NAME).admin_addr.clone();
    info!("Output account: {:?}", output_account);

    let start_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == NTRN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start output balance: {:?}", start_output_balance);

    info!("Prepare the IBC transfer library contract");
    let ibc_transfer_lib_contract_path = format!(
        "{}/artifacts/valence_generic_ibc_transfer_library.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_single_contract(&ibc_transfer_lib_contract_path)?;

    // Get the code id
    let code_id_ibc_transfer_lib = *test_ctx
        .get_chain(JUNO_CHAIN_NAME)
        .contract_codes
        .get("valence_generic_ibc_transfer_library")
        .unwrap();

    info!("Creating IBC transfer library contract");
    let transfer_amount = 1_000_000_000u128;
    let ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<LibraryConfig> {
        owner: JUNO_CHAIN_ADMIN_ADDR.to_string(),
        processor: JUNO_CHAIN_ADMIN_ADDR.to_string(),
        config: LibraryConfig::new(
            LibraryAccountType::Addr(input_account.clone()),
            output_account.clone(),
            UncheckedDenom::Native(neutron_on_juno_denom.to_string()),
            IbcTransferAmount::FixedAmount(transfer_amount.into()),
            "".to_owned(),
            valence_generic_ibc_transfer_library::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .get_transfer_channels()
                    .src(JUNO_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
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
            .get_request_builder(JUNO_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_ibc_transfer_lib,
        &serde_json::to_string(&ibc_transfer_instantiate_msg).unwrap(),
        "ibc_transfer",
        None,
        "",
    )
    .unwrap();

    info!("IBC Transfer library: {}", ibc_transfer.address.clone());

    // Approve the librarys for the base account
    approve_library(
        &mut test_ctx,
        JUNO_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        ibc_transfer.address.clone(),
    );

    info!("Initiate IBC transfer");
    let ibc_transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
        FunctionMsgs::IbcTransfer {},
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&ibc_transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the IBC Transfer library!");
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("Verifying balances...");
    let end_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == neutron_on_juno_denom)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(end_input_balance, start_input_balance.sub(transfer_amount));

    let end_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == NTRN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_output_balance,
        start_output_balance.add(transfer_amount)
    );

    // Update config to transfer the input account's full remaining balance
    info!("Update library configuration...");
    let new_config = valence_neutron_ibc_transfer_library::msg::LibraryConfigUpdate {
        input_addr: None,
        output_addr: None,
        denom: None,
        amount: Some(IbcTransferAmount::FullAmount),
        memo: None,
        remote_chain_info: None,
        denom_to_pfm_map: None,
    };
    let upd_cfg_msg =
        valence_library_utils::msg::ExecuteMsg::<FunctionMsgs, LibraryConfigUpdate>::UpdateConfig {
            new_config,
        };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&upd_cfg_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("Initiate IBC transfer");
    let ibc_transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
        FunctionMsgs::IbcTransfer {},
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&ibc_transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the IBC Transfer library!");
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("Verifying balances...");
    let prev_end_input_balance = end_input_balance;
    let end_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == neutron_on_juno_denom)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(end_input_balance, Uint128::zero().u128());

    let prev_end_output_balance = end_output_balance;
    let end_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == NTRN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_output_balance,
        prev_end_output_balance.add(prev_end_input_balance)
    );

    info!("IBC transfer successful!");

    Ok(())
}
