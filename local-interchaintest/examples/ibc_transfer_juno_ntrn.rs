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
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, JUNO_CHAIN_ADMIN_ADDR,
    JUNO_CHAIN_DENOM, JUNO_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
};
use log::info;

use valence_ibc_transfer_service::msg::{ActionsMsgs, ServiceConfig};
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let ibc_denom = "ibc/4E41ED8F3DCAEA15F4D6ADC6EDD7C04A676160735C9710B904B7BF53525B56D6";
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
        .with_ibc_denom(NEUTRON_CHAIN_NAME, JUNO_CHAIN_NAME, ibc_denom.to_string())
        .build()?;

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

    // Create 1 base accounts on Juno, to be the input account for the IBC transfer service
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

    // Send native tokens to the input account
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        DEFAULT_KEY,
        &input_account,
        &[BankCoin {
            denom: JUNO_CHAIN_DENOM.to_string(),
            amount: 1_000_000_000_000u128.into(),
        }],
        &BankCoin {
            denom: JUNO_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let start_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == ibc_denom)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start input balance: {:?}", start_input_balance);

    // We need a normal account on Neutron to be the output account for the IBC transfer service
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

    info!("Prepare the IBC transfer service contract");
    let ibc_transfer_svc_contract_path = format!(
        "{}/artifacts/valence_ibc_transfer_service.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_single_contract(&ibc_transfer_svc_contract_path)?;

    // Get the code id
    let code_id_ibc_transfer_svc = *test_ctx
        .get_chain(JUNO_CHAIN_NAME)
        .contract_codes
        .get("valence_ibc_transfer_service")
        .unwrap();

    info!("Creating IBC transfer service contract");
    let transfer_amount = 1_000_000_000u128;
    let juno_ntrn_path = &(JUNO_CHAIN_NAME.to_string(), NEUTRON_CHAIN_NAME.to_string());
    let ibc_transfer_instantiate_msg = valence_service_utils::msg::InstantiateMsg::<ServiceConfig> {
        owner: JUNO_CHAIN_ADMIN_ADDR.to_string(),
        processor: JUNO_CHAIN_ADMIN_ADDR.to_string(),
        config: ServiceConfig {
            input_addr: ServiceAccountType::Addr(input_account.clone()),
            output_addr: output_account.clone(),
            denom: UncheckedDenom::Native(ibc_denom.to_string()),
            amount: transfer_amount.into(),
            memo: "".to_owned(),
            remote_chain_info: valence_ibc_transfer_service::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .transfer_channel_ids
                    .get(juno_ntrn_path)
                    .unwrap()
                    .clone(),
                port_id: None,
                denom: NTRN_DENOM.to_string(),
                ibc_transfer_timeout: Some(1000u64.into()),
            },
        },
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
        JUNO_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        ibc_transfer.address.clone(),
    );

    info!("Initiate IBC transfer");
    let ibc_transfer_msg = &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(
        ActionsMsgs::IbcTransfer {},
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

    info!("Messages sent to the IBC Transfer service!");
    std::thread::sleep(std::time::Duration::from_secs(10));

    let ibc_fee = 2000u128;

    info!("Verifying balances...");
    let end_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == ibc_denom)
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
        start_output_balance.add(transfer_amount).sub(ibc_fee)
    );

    info!("IBC transfer successful!");

    Ok(())
}
