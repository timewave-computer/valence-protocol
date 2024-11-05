use std::{
    collections::BTreeMap,
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
    types::ibc::get_multihop_ibc_denom, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    GAIA_CHAIN_DENOM, GAIA_CHAIN_NAME, JUNO_CHAIN_ADMIN_ADDR, JUNO_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;

use valence_generic_ibc_transfer_service::msg::IbcTransferAmount;
use valence_ibc_utils::types::PacketForwardMiddlewareConfig;
use valence_neutron_ibc_transfer_service::msg::{FunctionMsgs, ServiceConfig};
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

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
        .with_transfer_channels(JUNO_CHAIN_NAME, GAIA_CHAIN_NAME)
        .build()?;

    // Let's upload the base account contract to Neutron
    let current_dir = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&base_account_contract_path)?;

    // Get the code id
    let code_id_base_account = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_base_account")
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

    // Send some NTRN to the input account to pay for the IBC transfer fee
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &input_account,
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: 10_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Send UATOM tokens to Juno
    test_ctx
        .build_tx_transfer()
        .with_chain_name(GAIA_CHAIN_NAME)
        .with_amount(1_000_000_000_000u128)
        .with_recipient(JUNO_CHAIN_ADMIN_ADDR)
        .with_denom(GAIA_CHAIN_DENOM)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Send UATOM tokens from Juno to Neutron
    let atom_on_juno = test_ctx
        .get_ibc_denom()
        .base_denom(GAIA_CHAIN_DENOM.to_owned())
        .src(GAIA_CHAIN_NAME)
        .dest(JUNO_CHAIN_NAME)
        .get();
    info!("Atom on Juno: {:?}", atom_on_juno);

    test_ctx
        .build_tx_transfer()
        .with_chain_name(JUNO_CHAIN_NAME)
        .with_amount(1_000_000_000_000u128)
        .with_recipient(&input_account)
        .with_denom(&atom_on_juno)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let atom_on_neutron_via_juno = get_multihop_ibc_denom(
        GAIA_CHAIN_DENOM,
        vec![
            &test_ctx
                .get_transfer_channels()
                .src(NEUTRON_CHAIN_NAME)
                .dest(JUNO_CHAIN_NAME)
                .get(),
            &test_ctx
                .get_transfer_channels()
                .src(JUNO_CHAIN_NAME)
                .dest(GAIA_CHAIN_NAME)
                .get(),
        ],
    );
    info!("Atom on Neutron via Juno: {:?}", atom_on_neutron_via_juno);

    let start_input_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &input_account,
    )
    .iter()
    .find(|bal| bal.denom == atom_on_neutron_via_juno)
    .map_or(0, |bal| bal.amount.u128());
    info!("Start input balance: {:?}", start_input_balance);

    // We need a normal account on Gaia to be the output account for the IBC transfer service
    let output_account = test_ctx.get_chain(GAIA_CHAIN_NAME).admin_addr.clone();
    info!("Output account: {:?}", output_account);

    let start_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == GAIA_CHAIN_DENOM)
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
    let transfer_amount = 1_000_000_000_000u128;
    let ntrn_juno_path = &(NEUTRON_CHAIN_NAME.to_string(), JUNO_CHAIN_NAME.to_string());
    let ibc_transfer_instantiate_msg = valence_service_utils::msg::InstantiateMsg::<ServiceConfig> {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: ServiceConfig::with_pfm_map(
            ServiceAccountType::Addr(input_account.clone()),
            output_account.clone(),
            UncheckedDenom::Native(atom_on_neutron_via_juno.clone()),
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
            BTreeMap::from([(
                atom_on_neutron_via_juno.clone(),
                PacketForwardMiddlewareConfig {
                    local_to_hop_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(NEUTRON_CHAIN_NAME)
                        .dest(JUNO_CHAIN_NAME)
                        .get(),
                    hop_to_destination_chain_channel_id: test_ctx
                        .get_transfer_channels()
                        .src(JUNO_CHAIN_NAME)
                        .dest(GAIA_CHAIN_NAME)
                        .get(),
                    hop_chain_receiver_address: JUNO_CHAIN_ADMIN_ADDR.to_string(),
                },
            )]),
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
    .find(|bal| bal.denom == atom_on_neutron_via_juno)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_input_balance,
        start_input_balance.sub(transfer_amount).add(ibc_fee)
    );

    let end_output_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(GAIA_CHAIN_NAME),
        &output_account,
    )
    .iter()
    .find(|bal| bal.denom == GAIA_CHAIN_DENOM)
    .map_or(0, |bal| bal.amount.u128());
    assert_eq!(
        end_output_balance,
        start_output_balance.add(transfer_amount).sub(ibc_fee)
    );

    info!("IBC transfer successful!");

    Ok(())
}
