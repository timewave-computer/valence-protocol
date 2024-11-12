use std::{
    env,
    error::Error,
    ops::{Add, Sub},
};

use cosmwasm_std::coin;
use cosmwasm_std_old::Coin as BankCoin;
use local_interchaintest::utils::{
    base_account::{approve_library, create_base_accounts},
    GAS_FLAGS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_NAME,
};
use log::info;

use valence_generic_ibc_transfer_library::msg::{FunctionMsgs, IbcTransferAmount, LibraryConfig};
use valence_library_utils::{denoms::UncheckedDenom, LibraryAccountType};

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

    let osmo_on_neutron_denom = test_ctx
        .get_ibc_denom()
        .base_denom(OSMOSIS_CHAIN_DENOM.to_owned())
        .src(OSMOSIS_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    // Let's upload the base account contract to Osmosis
    let current_dir = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&base_account_contract_path)?;

    // Get the code id
    let code_id_base_account = *test_ctx
        .get_chain(OSMOSIS_CHAIN_NAME)
        .contract_codes
        .get("valence_base_account")
        .unwrap();

    // Create 1 base accounts on Osmosis, to be the input account for the IBC transfer library
    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        OSMOSIS_CHAIN_NAME,
        code_id_base_account,
        OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        1,
        Some(coin(2000, OSMOSIS_CHAIN_DENOM)),
    );
    let input_account = base_accounts[0].clone();
    info!("Input account: {:?}", input_account);

    // Send native tokens to the input account
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        &input_account,
        &[BankCoin {
            denom: OSMOSIS_CHAIN_DENOM.to_string(),
            amount: 1_000_000_000_000u128.into(),
        }],
        &BankCoin {
            denom: OSMOSIS_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let start_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == OSMOSIS_CHAIN_DENOM)
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
    .find(|bal| bal.denom == osmo_on_neutron_denom)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start output balance: {:?}", start_output_balance);

    info!("Prepare the IBC transfer library contract");
    let ibc_transfer_svc_contract_path = format!(
        "{}/artifacts/valence_generic_ibc_transfer_library.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&ibc_transfer_svc_contract_path)?;

    // Get the code id
    let code_id_ibc_transfer_svc = *test_ctx
        .get_chain(OSMOSIS_CHAIN_NAME)
        .contract_codes
        .get("valence_generic_ibc_transfer_library")
        .unwrap();

    info!("Creating IBC transfer library contract");
    let transfer_amount = 1_000_000_000u128;
    let ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<LibraryConfig> {
        owner: OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        processor: OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        config: LibraryConfig::new(
            LibraryAccountType::Addr(input_account.clone()),
            output_account.clone(),
            UncheckedDenom::Native(OSMOSIS_CHAIN_DENOM.to_string()),
            IbcTransferAmount::FixedAmount(transfer_amount.into()),
            "".to_owned(),
            valence_generic_ibc_transfer_library::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .get_transfer_channels()
                    .src(OSMOSIS_CHAIN_NAME)
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
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_ibc_transfer_svc,
        &serde_json::to_string(&ibc_transfer_instantiate_msg).unwrap(),
        "ibc_transfer",
        None,
        &format!("--fees {}{}", 5000, OSMOSIS_CHAIN_DENOM),
    )
    .unwrap();

    info!("IBC Transfer library: {}", ibc_transfer.address.clone());

    // Approve the librarys for the base account
    approve_library(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        ibc_transfer.address.clone(),
        Some(format!("--fees {}{}", 5000, OSMOSIS_CHAIN_DENOM)),
    );

    info!("Initiate IBC transfer");
    let ibc_transfer_msg =
        &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(FunctionMsgs::IbcTransfer {});

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &ibc_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&ibc_transfer_msg).unwrap(),
        &format!("--fees {}{} {}", 5000, OSMOSIS_CHAIN_DENOM, GAS_FLAGS),
    )
    .unwrap();

    info!("Messages sent to the IBC Transfer library!");
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("Verifying balances...");
    let end_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == OSMOSIS_CHAIN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(end_input_balance, start_input_balance.sub(transfer_amount));

    let end_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == osmo_on_neutron_denom)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_output_balance,
        start_output_balance.add(transfer_amount)
    );

    info!("IBC transfer successful!");

    Ok(())
}
