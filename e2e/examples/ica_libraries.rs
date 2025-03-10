use std::{env, error::Error, time::Duration};

use cosmwasm_std::{Binary, Uint128, Uint64};
use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate, contract_query};
use localic_utils::{
    types::config::ConfigChain, utils::test_context::TestContext, ConfigChainBuilder,
    TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR,
    NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_account_utils::ica::{IcaState, RemoteDomainInfo};
use valence_chain_client_utils::{cosmos::base_client::BaseClient, noble::NobleClient};
use valence_e2e::utils::{
    parse::get_grpc_address_and_port_from_logs, relayer::restart_relayer, ADMIN_MNEMONIC,
    GAS_FLAGS, LOGS_FILE_PATH, NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID,
    NOBLE_CHAIN_NAME, NOBLE_CHAIN_PREFIX, VALENCE_ARTIFACTS_PATH,
};
use valence_library_utils::LibraryAccountType;

const UUSDC_DENOM: &str = "uusdc";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_chain(ConfigChain {
            denom: NOBLE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: NOBLE_CHAIN_ID.to_string(),
            chain_name: NOBLE_CHAIN_NAME.to_string(),
            chain_prefix: NOBLE_CHAIN_PREFIX.to_string(),
            admin_addr: NOBLE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, NOBLE_CHAIN_NAME)
        .build()?;

    let rt = tokio::runtime::Runtime::new()?;
    // Get the grpc url and the port for the noble chain
    let (grpc_url, grpc_port) = get_grpc_address_and_port_from_logs(NOBLE_CHAIN_ID)?;

    let noble_client = rt.block_on(async {
        NobleClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NOBLE_CHAIN_ID,
            NOBLE_CHAIN_DENOM,
        )
        .await
        .unwrap()
    });

    // Set up our noble environment to allow for testing on domain_id 0 and with USDC as the bridging denom
    rt.block_on(noble_client.set_up_test_environment(NOBLE_CHAIN_ADMIN_ADDR, 0, UUSDC_DENOM));

    // Upload the ICA account and the CCTP transfer contract
    let current_dir = env::current_dir()?;
    let mut uploader = test_ctx.build_tx_upload_contracts();

    let valence_ica = format!(
        "{}/artifacts/valence_interchain_account.wasm",
        current_dir.display()
    );

    let ica_cctp_transfer = format!(
        "{}/artifacts/valence_ica_cctp_transfer.wasm",
        current_dir.display()
    );

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&valence_ica)?;

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&ica_cctp_transfer)?;

    // Get the code ids
    let code_id_valence_ica = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_interchain_account")
        .unwrap();

    info!("Instantiating the ICA contract...");
    let timeout_seconds = 90;
    let ica_instantiate_msg = valence_account_utils::ica::InstantiateMsg {
        admin: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        approved_libraries: vec![],
        remote_domain_information: RemoteDomainInfo {
            connection_id: test_ctx
                .get_connections()
                .src(NEUTRON_CHAIN_NAME)
                .dest(NOBLE_CHAIN_NAME)
                .get(),
            ica_timeout_seconds: Uint64::new(timeout_seconds),
        },
    };

    let valence_ica = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_valence_ica,
        &serde_json::to_string(&ica_instantiate_msg)?,
        "valence_ica",
        None,
        "",
    )?;
    info!(
        "ICA contract instantiated. Address: {}",
        valence_ica.address
    );

    // Let's test that trying to register the ICA without funds fails
    let error = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &valence_ica.address,
        DEFAULT_KEY,
        &serde_json::to_string(&valence_account_utils::ica::ExecuteMsg::RegisterIca {}).unwrap(),
        GAS_FLAGS,
    )
    .unwrap_err();

    assert!(error.to_string().contains(
        valence_interchain_account::error::ContractError::NotEnoughBalanceForIcaRegistration
            .to_string()
            .as_str()
    ));

    // Let's do it again but this time with enough funds to verify that the ICA is registered
    info!("Registering the ICA with enough funds...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &valence_ica.address,
        DEFAULT_KEY,
        &serde_json::to_string(&valence_account_utils::ica::ExecuteMsg::RegisterIca {}).unwrap(),
        &format!("{} --amount=100000000{}", GAS_FLAGS, NEUTRON_CHAIN_DENOM),
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    // We want to check that it's in state created
    poll_for_ica_state(&mut test_ctx, &valence_ica.address, |state| {
        matches!(state, IcaState::Created(_))
    });

    // Get the remote address
    let ica_state: IcaState = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &valence_ica.address,
            &serde_json::to_string(&valence_account_utils::ica::QueryMsg::IcaState {}).unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let remote_address = match ica_state {
        IcaState::Created(ica_info) => ica_info.address,
        _ => {
            unreachable!("Expected IcaState::Created variant");
        }
    };
    info!("Remote address created: {}", remote_address);

    // Let's fund the ICA account with some uusdc by minting to it
    info!("Minting uusdc to the ICA account...");
    let amount_to_transfer = 10000000;
    rt.block_on(async {
        let tx_response = noble_client
            .mint_fiat(
                NOBLE_CHAIN_ADMIN_ADDR,
                &remote_address,
                &amount_to_transfer.to_string(),
                UUSDC_DENOM,
            )
            .await
            .unwrap();
        noble_client.poll_for_tx(&tx_response.hash).await.unwrap();
        info!(
            "Minted {} to {}: {:?}",
            UUSDC_DENOM, &remote_address, tx_response
        );
    });

    // Verify that the funds were successfully minted
    let balance = rt
        .block_on(noble_client.query_balance(&remote_address, UUSDC_DENOM))
        .unwrap();
    assert_eq!(balance, amount_to_transfer);

    info!("Instantiating the ICA CCTP transfer contract...");
    let code_id_ica_ccpt_transfer = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_ica_cctp_transfer")
        .unwrap();

    let ica_cctp_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_ica_cctp_transfer::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: valence_ica_cctp_transfer::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(valence_ica.address.clone()),
            amount: Uint128::new(amount_to_transfer),
            denom: UUSDC_DENOM.to_string(),
            destination_domain_id: 0,
            mint_recipient: Binary::from(&[0x01; 32]),
        },
    };

    let ica_cctp_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_ica_ccpt_transfer,
        &serde_json::to_string(&ica_cctp_transfer_instantiate_msg)?,
        "valence_ica_cctp_transfer",
        None,
        "",
    )?;
    info!(
        "ICA CCTP transfer contract instantiated. Address: {}",
        ica_cctp_transfer.address
    );

    info!("Approving the ICA CCTP transfer library...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &valence_ica.address,
        DEFAULT_KEY,
        &serde_json::to_string(&valence_account_utils::ica::ExecuteMsg::ApproveLibrary {
            library: ica_cctp_transfer.address.clone(),
        })
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    // Trigger the transfer
    let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
        valence_ica_cctp_transfer::msg::FunctionMsgs::Transfer {},
    );

    info!("Executing remote CCTP transfer...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &ica_cctp_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(15));

    // Verify that the funds were successfully burned
    let balance = rt
        .block_on(noble_client.query_balance(&remote_address, UUSDC_DENOM))
        .unwrap();
    assert_eq!(balance, 0);
    info!("Funds successfully burned! ICA CCTP Transfer library test passed!");

    // Now we are going to force a timeout to verify that timeouts and channel recreations are working
    info!("Forcing a timeout to test channel closing...");
    test_ctx.stop_relayer();

    // Send the message again
    info!("Executing remote CCTP transfer that will timeout...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &ica_cctp_transfer.address,
        DEFAULT_KEY,
        &serde_json::to_string(&transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    // Wait for the timeout to pass
    std::thread::sleep(Duration::from_secs(timeout_seconds + 1));

    // Restart the relayer
    restart_relayer(&mut test_ctx);

    // Verify that ICA state is updated after receiving a timeout
    poll_for_ica_state(&mut test_ctx, &valence_ica.address, |state| {
        matches!(state, IcaState::Closed)
    });

    // Verify that we can recreate the channel
    info!("Recreating the channel...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &valence_ica.address,
        DEFAULT_KEY,
        &serde_json::to_string(&valence_account_utils::ica::ExecuteMsg::RegisterIca {}).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    // Verify that the channel is recreated
    poll_for_ica_state(&mut test_ctx, &valence_ica.address, |state| {
        matches!(state, IcaState::Created(_))
    });

    info!("All ICA tests passed!");

    Ok(())
}

fn poll_for_ica_state<F>(test_ctx: &mut TestContext, addr: &str, expected: F)
where
    F: Fn(&IcaState) -> bool,
{
    let mut attempts = 0;
    loop {
        attempts += 1;
        let ica_state: IcaState = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                addr,
                &serde_json::to_string(&valence_account_utils::ica::QueryMsg::IcaState {}).unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        if expected(&ica_state) {
            info!("Target ICA state reached!");
            break;
        } else {
            info!(
                "Waiting for the right ICA state, current state: {:?}",
                ica_state
            );
        }

        if attempts % 5 == 0 {
            restart_relayer(test_ctx);
        }

        if attempts > 60 {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}
