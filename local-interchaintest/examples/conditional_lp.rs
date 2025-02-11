use cosmwasm_std::Uint128;
use cosmwasm_std::{to_json_binary, Binary, Coin, Uint64};
use cosmwasm_std_old::Coin as BankCoin;
use local_interchaintest::utils::base_account::{approve_library, create_base_accounts};
use local_interchaintest::utils::NTRN_DENOM;
use local_interchaintest::utils::{
    authorization::{set_up_authorization_and_processor, set_up_external_domain_with_polytone},
    base_account::create_storage_accounts,
    icq::{generate_icq_relayer_config, start_icq_relayer},
    osmosis::gamm::setup_gamm_pool,
    processor::confirm_remote_domain_processor_queue_length,
    GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_OSMOSIS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::{
    errors::LocalError,
    modules::{
        bank,
        cosmwasm::{contract_execute, contract_instantiate, contract_query},
    },
    types::TransactionResponse,
};
use log::info;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use valence_authorization_utils::authorization_message::ParamRestriction;
use valence_authorization_utils::domain::Domain;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_generic_ibc_transfer_library::msg::IbcTransferAmount;
use valence_library_utils::denoms::UncheckedDenom;
use valence_library_utils::LibraryAccountType;
use valence_middleware_asserter::msg::Predicate;
use valence_middleware_utils::canonical_types::pools::xyk::XykPoolQuery;
use valence_middleware_utils::type_registry::types::RegistryInstantiateMsg;
use valence_neutron_ic_querier::msg::{FunctionMsgs, LibraryConfig, QueryDefinition};

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_ID, OSMOSIS_CHAIN_NAME,
    OSMOSIS_CHAIN_PREFIX,
};

const TARGET_QUERY_LABEL: &str = "gamm_pool";
const SINGLE_SIDE_LP_AMOUNT: u128 = 1_000_000;
const PROVIDE_LIQUIDITY_LABEL: &str = "provide_liquidity";
const REGISTER_KV_QUERY_LABEL: &str = "register_kv_query";
const DEREGISTER_KV_QUERY_LABEL: &str = "deregister_kv_query";
const CONDITIONAL_IBC_FORWARDING_LABEL: &str = "conditional_ibc_forwarding";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let current_dir = env::current_dir()?;
    let salt = hex::encode(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );

    // spin up the test context
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

    // set up the authorization and processor contracts on neutron
    let (authorization_contract_address, neutron_processor_address) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    info!("setting up external domain with polytone...");
    let processor_on_osmosis = set_up_external_domain_with_polytone(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        OSMOSIS_CHAIN_ID,
        OSMOSIS_CHAIN_ADMIN_ADDR,
        OSMOSIS_CHAIN_DENOM,
        OSMOSIS_CHAIN_PREFIX,
        LOCAL_CODE_ID_CACHE_PATH_OSMOSIS,
        "neutron-osmosis",
        salt,
        &authorization_contract_address,
    )?;
    info!("processor on osmosis: {:?}", processor_on_osmosis);

    // upload all contracts relevant to both osmosis and neutron
    upload_contracts(current_dir, &mut test_ctx)?;

    // do the osmosis side setup:
    // - create a OSMO/NTRN xyk pool
    // - instantiate the gamm lper library
    // - create base i/o accounts for the gamm lper
    // - approves the gamm lper library for the input account
    let (osmo_gamm_lper_addr, osmo_input_acc_addr, osmo_output_acc_addr, pool_id) =
        osmosis_setup(&mut test_ctx, &processor_on_osmosis)?;

    // do the neutron side setup:
    // - spin up the interchain query relayer sidecar and insert it into the docker network
    // - set up the middleware (broker, asserter, type registry)
    // - instantiate ibc forwarder and ic querier libraries
    // - set up storage account for the ic querier and approve it
    // - set up input account for the ibc transfer library and approve it
    let (asserter_addr, neutron_storage_account, ic_querier_addr, ibc_forwarder) = neutron_setup(
        &mut test_ctx,
        &neutron_processor_address,
        pool_id,
        &osmo_input_acc_addr,
    )?;

    // set up the authorization entries for the following functions:
    // - (ntrn) register_kv_query
    // - (ntrn) deregister_kv_query
    // - (ntrn) [assert, ibc_transfer]
    // - (osmo) provide_liquidity
    create_authorizations(
        &mut test_ctx,
        &authorization_contract_address,
        ic_querier_addr.to_string(),
        asserter_addr,
        osmo_gamm_lper_addr,
        ibc_forwarder,
        neutron_storage_account.to_string(),
        ntrn_on_osmo_denom.to_string(),
    )?;

    // setup is now done. the program flow begins.

    // 1. KV query registration
    {
        info!("sending kv query registration message to authorizations");
        let kv_query_registration_message = ProcessorMessage::CosmwasmExecuteMsg {
            msg: Binary::from(serde_json::to_vec(
                &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                    FunctionMsgs::RegisterKvQuery {
                        target_query: TARGET_QUERY_LABEL.to_string(),
                    },
                ),
            )?),
        };

        let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                label: REGISTER_KV_QUERY_LABEL.to_string(),
                messages: vec![kv_query_registration_message],
                ttl: None,
            },
        );

        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &authorization_contract_address,
            DEFAULT_KEY,
            &serde_json::to_string(&send_msg)?,
            GAS_FLAGS,
        )?;
        std::thread::sleep(std::time::Duration::from_secs(3));

        info!("Ticking processor on neutron to register the KV query...");
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &neutron_processor_address,
            DEFAULT_KEY,
            &serde_json::to_string(
                &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                    valence_processor_utils::msg::PermissionlessMsg::Tick {},
                ),
            )?,
            "--gas=auto --gas-adjustment=5.0 --fees=5000000untrn",
        )?;
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // 2. Conditional IBC forwarding
    {
        // we configure the assertion message which will:
        // - 1. let a = the amount of ntrn tokens in the pool
        // - 2. let b = 150_000_000
        // - 3. evaluate: a < b ? return ok for true, err for false
        let assertion_message_binary = Binary::from(serde_json::to_vec(
            &valence_middleware_asserter::msg::ExecuteMsg::Assert {
                a: valence_middleware_asserter::msg::AssertionValue::Variable(
                    valence_middleware_asserter::msg::QueryInfo {
                        storage_account: neutron_storage_account.to_string(),
                        storage_slot_key: TARGET_QUERY_LABEL.to_string(),
                        query: to_json_binary(&XykPoolQuery::GetPoolAssetAmount {
                            target_denom: ntrn_on_osmo_denom.to_string(),
                        })?,
                    },
                ),
                predicate: valence_middleware_asserter::msg::Predicate::LT,
                b: valence_middleware_asserter::msg::AssertionValue::Constant(
                    valence_middleware_utils::type_registry::queries::ValencePrimitive::Uint128(
                        Uint128::new(150_000_000),
                    ),
                ),
            },
        )?);

        let assertion_message = ProcessorMessage::CosmwasmExecuteMsg {
            msg: assertion_message_binary,
        };

        // then configure the ibc transfer message which will
        // route the funds (1_000_000untrn) from the neutron ibc forwarder
        // library input account to the osmosis gamm lper input account
        let ibc_transfer_message = ProcessorMessage::CosmwasmExecuteMsg {
            msg: Binary::from(serde_json::to_vec(
                &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                    valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
                ),
            )?),
        };

        // we send the messages to the processor in order of [assert, transfer].
        // because the authorization is configured to do atomic subroutines, it will
        // be processed as follows:
        // 1. execute the assertion which returns Ok or Err
        // - Err -> error out and exit
        // - Ok  -> proceed to second message
        // 2. execute the ibc transfer
        let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                label: CONDITIONAL_IBC_FORWARDING_LABEL.to_string(),
                messages: vec![assertion_message, ibc_transfer_message],
                ttl: None,
            },
        );

        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &authorization_contract_address,
            DEFAULT_KEY,
            &serde_json::to_string(&send_msg)?,
            &format!("{GAS_FLAGS} --fees=1000000untrn"),
        )?;

        std::thread::sleep(std::time::Duration::from_secs(3));

        info!("Ticking processor on neutron to perform the assertion & forward the funds to osmosis input account...");
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &neutron_processor_address,
            DEFAULT_KEY,
            &serde_json::to_string(
                &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                    valence_processor_utils::msg::PermissionlessMsg::Tick {},
                ),
            )?,
            GAS_FLAGS,
        )?;
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    // 3. LP provision on osmosis
    {
        info!("routing LP subroutine instructions to osmosis processsor...");
        let lp_message = ProcessorMessage::CosmwasmExecuteMsg {
            msg: Binary::from(serde_json::to_vec(
                &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                    valence_osmosis_gamm_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                        expected_spot_price: None,
                        asset: ntrn_on_osmo_denom.to_string(),
                        limit: Uint128::new(SINGLE_SIDE_LP_AMOUNT),
                    },
                ),
            )?),
        };
        let provide_liquidity_msg =
            valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                    label: PROVIDE_LIQUIDITY_LABEL.to_string(),
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
            &processor_on_osmosis,
            1,
        );

        std::thread::sleep(std::time::Duration::from_secs(3));

        let osmo_input_acc_balance_pre_lp = bank::get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &osmo_input_acc_addr,
        );
        let osmo_output_acc_balance_pre_lp = bank::get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &osmo_output_acc_addr,
        );

        // before we tick the osmosis processor we assert the following:
        // - the osmosis input account has some neutron
        // - the osmosis output account is empty
        assert!(osmo_output_acc_balance_pre_lp.is_empty());
        assert!(osmo_input_acc_balance_pre_lp.len() == 1);
        assert!(osmo_input_acc_balance_pre_lp[0].amount.u128() == 100000000);
        assert!(osmo_input_acc_balance_pre_lp[0].denom == ntrn_on_osmo_denom);

        info!("Ticking processor on osmosis to provide liquidity...");
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &processor_on_osmosis,
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

        confirm_remote_domain_processor_queue_length(
            &mut test_ctx,
            OSMOSIS_CHAIN_NAME,
            &processor_on_osmosis,
            0,
        );

        let osmo_input_acc_balance = bank::get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &osmo_input_acc_addr,
        );
        let osmo_output_acc_balance = bank::get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            &osmo_output_acc_addr,
        );

        info!("osmo_input_acc_balance: {:?}", osmo_input_acc_balance);
        info!("osmo_output_acc_balance: {:?}", osmo_output_acc_balance);

        assert!(osmo_output_acc_balance.len() == 1);
        assert!(osmo_output_acc_balance[0].denom == "gamm/pool/1");
        assert!(osmo_output_acc_balance[0].amount.u128() > 0u128);
        assert!(
            osmo_input_acc_balance[0].amount.u128()
                == osmo_input_acc_balance_pre_lp[0].amount.u128() - SINGLE_SIDE_LP_AMOUNT
        );
    }

    info!("liquidity provision success!");

    Ok(())
}

fn osmosis_setup(
    test_ctx: &mut TestContext,
    processor_on_osmosis: &str,
) -> Result<(String, String, String, u64), Box<dyn Error>> {
    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();
    let pool_id = setup_gamm_pool(test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    let osmosis_base_acc_code_id = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    let osmo_base_accounts = create_base_accounts(
        test_ctx,
        DEFAULT_KEY,
        OSMOSIS_CHAIN_NAME,
        osmosis_base_acc_code_id,
        OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        vec![processor_on_osmosis.to_string()],
        2,
        Some(Coin::new(1000000u128, OSMOSIS_CHAIN_DENOM)),
    );
    let osmo_input_acc_addr = osmo_base_accounts.first().unwrap();
    let osmo_output_acc_addr = osmo_base_accounts.get(1).unwrap();
    info!("osmo_input_acc_addr: {osmo_input_acc_addr}");
    info!("osmo_output_acc_addr: {osmo_output_acc_addr}");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let osmo_gamm_lper_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_osmosis_gamm_lper::msg::LibraryConfig,
    > {
        owner: OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        processor: processor_on_osmosis.to_string(),
        config: valence_osmosis_gamm_lper::msg::LibraryConfig::new(
            LibraryAccountType::Addr(osmo_input_acc_addr.to_string()),
            LibraryAccountType::Addr(osmo_output_acc_addr.to_string()),
            valence_osmosis_gamm_lper::msg::LiquidityProviderConfig {
                pool_id,
                asset_data: valence_library_utils::liquidity_utils::AssetData {
                    asset1: ntrn_on_osmo_denom.to_string(),
                    asset2: OSMOSIS_CHAIN_DENOM.to_string(),
                },
            },
        ),
    };

    let gamm_lper_code_id = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("valence_osmosis_gamm_lper")
        .get_cw()
        .code_id
        .unwrap();

    let osmo_gamm_lper_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        gamm_lper_code_id,
        &serde_json::to_string(&osmo_gamm_lper_instantiate_msg)?,
        "osmo_gamm_lper_lib",
        Some(OSMOSIS_CHAIN_ADMIN_ADDR),
        &format!("{GAS_FLAGS} --fees=100000uosmo"),
    )?;

    let osmo_gamm_lper_addr = osmo_gamm_lper_lib.address.to_string();

    std::thread::sleep(std::time::Duration::from_secs(1));
    info!("osmo_gamm_lper_addr: {osmo_gamm_lper_addr}");

    info!("approving gamm lper lib on the osmo input acc");
    approve_library(
        test_ctx,
        OSMOSIS_CHAIN_NAME,
        DEFAULT_KEY,
        osmo_input_acc_addr,
        osmo_gamm_lper_addr.to_string(),
        Some(format!("{GAS_FLAGS} --fees=100000uosmo")),
    );

    Ok((
        osmo_gamm_lper_addr,
        osmo_input_acc_addr.to_string(),
        osmo_output_acc_addr.to_string(),
        pool_id,
    ))
}

fn neutron_setup(
    test_ctx: &mut TestContext,
    neutron_processor_address: &str,
    pool_id: u64,
    osmo_input_acc: &str,
) -> Result<(String, String, String, String), Box<dyn Error>> {
    let current_dir = env::current_dir()?;
    let ntrn_to_osmo_connection_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    // with test context set up, we can generate the .env file for the icq relayer
    generate_icq_relayer_config(
        test_ctx,
        current_dir.clone(),
        OSMOSIS_CHAIN_NAME.to_string(),
    )?;

    // start the icq relayer. this runs in detached mode so we need
    // to manually kill it before each run for now.
    start_icq_relayer()?;

    info!("sleeping for 10 to allow icq relayer to start...");
    std::thread::sleep(Duration::from_secs(10));

    let (broker_addr, asserter_addr, _) = setup_middleware(test_ctx)?;

    // set up the storage account
    info!("setting up storage accounts...");
    let storage_acc_code_id = test_ctx
        .get_contract()
        .contract("valence_storage_account")
        .get_cw()
        .code_id
        .unwrap();

    let storage_accounts = create_storage_accounts(
        test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        storage_acc_code_id,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        1,
        None,
    );
    let neutron_storage_account = storage_accounts.first().unwrap();
    info!("neutron storage account address: {neutron_storage_account}",);

    // set up the IC querier
    let neutron_ic_querier_lib_code_id = test_ctx
        .get_contract()
        .contract("valence_neutron_ic_querier")
        .get_cw()
        .code_id
        .unwrap();

    let icq_lib_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<LibraryConfig> {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: neutron_processor_address.to_string(),
        config: LibraryConfig::new(
            LibraryAccountType::Addr(neutron_storage_account.to_string()),
            valence_neutron_ic_querier::msg::QuerierConfig {
                broker_addr: broker_addr.to_string(),
                connection_id: ntrn_to_osmo_connection_id,
            },
            BTreeMap::from_iter(vec![(
                TARGET_QUERY_LABEL.to_string(),
                QueryDefinition {
                    registry_version: None,
                    type_url: osmosis_std::types::osmosis::gamm::v1beta1::Pool::TYPE_URL
                        .to_string(),
                    update_period: Uint64::new(5),
                    params: BTreeMap::from([("pool_id".to_string(), to_json_binary(&pool_id)?)]),
                    query_id: None,
                },
            )]),
        ),
    };
    let ic_querier_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ic_querier_lib_code_id,
        &serde_json::to_string(&icq_lib_instantiate_msg)?,
        "icq_querier_lib",
        Some(NEUTRON_CHAIN_ADMIN_ADDR),
        "",
    )?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    info!("ic_querier lib address: {}", ic_querier_lib.address);

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &ic_querier_lib.address.to_string(),
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: 1_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("approving IC querier lib on the storage account");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        neutron_storage_account,
        ic_querier_lib.address.to_string(),
        None,
    );

    let neutron_base_acc_code_id = test_ctx
        .get_contract()
        .src(NEUTRON_CHAIN_NAME)
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    let neutron_base_accounts = create_base_accounts(
        test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        neutron_base_acc_code_id,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![neutron_processor_address.to_string()],
        1,
        Some(Coin::new(1000000u128, NEUTRON_CHAIN_DENOM)),
    );
    let neutron_input_acc_addr = neutron_base_accounts.first().unwrap();

    // Get the code id
    let code_id_ibc_transfer_lib = test_ctx
        .get_contract()
        .contract("valence_neutron_ibc_transfer_library")
        .get_cw()
        .code_id
        .unwrap();

    info!("Creating IBC transfer library contract");
    let transfer_amount = 100_000_000u128;
    let ntrn_osmo_path = &(
        NEUTRON_CHAIN_NAME.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
    );
    let ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_neutron_ibc_transfer_library::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: neutron_processor_address.to_string(),
        config: valence_neutron_ibc_transfer_library::msg::LibraryConfig::new(
            LibraryAccountType::Addr(neutron_input_acc_addr.clone()),
            osmo_input_acc.to_string(),
            UncheckedDenom::Native(NTRN_DENOM.to_string()),
            IbcTransferAmount::FixedAmount(transfer_amount.into()),
            "".to_owned(),
            valence_neutron_ibc_transfer_library::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .transfer_channel_ids
                    .get(ntrn_osmo_path)
                    .unwrap()
                    .clone(),
                ibc_transfer_timeout: Some(600u64.into()),
            },
        ),
    };

    let ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_ibc_transfer_lib,
        &serde_json::to_string(&ibc_transfer_instantiate_msg)?,
        "ibc_transfer",
        None,
        "",
    )?;

    info!("IBC Transfer library: {}", ibc_transfer.address.clone());

    // Approve the library for the base account
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        neutron_input_acc_addr,
        ibc_transfer.address.clone(),
        None,
    );

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &neutron_input_acc_addr.to_string(),
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: 100_000_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok((
        asserter_addr,
        neutron_storage_account.to_string(),
        ic_querier_lib.address.to_string(),
        ibc_transfer.address,
    ))
}

fn create_authorizations(
    test_ctx: &mut TestContext,
    authorization_contract_address: &str,
    ic_querier: String,
    asserter: String,
    gamm_lper: String,
    ibc_forwarder: String,
    storage_account: String,
    denom: String,
) -> Result<(), Box<dyn Error>> {
    let register_kvq_authorization = AuthorizationBuilder::new()
        .with_label(REGISTER_KV_QUERY_LABEL)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(ic_querier.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let deregister_kvq_authorization = AuthorizationBuilder::new()
        .with_label(DEREGISTER_KV_QUERY_LABEL)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(ic_querier.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let conditional_ibc_forwarding_authorization = AuthorizationBuilder::new()
        .with_label(CONDITIONAL_IBC_FORWARDING_LABEL)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(asserter.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "assert".to_string(),
                                // we apply param restrictions to ensure that funds get forwarded
                                // to the destination domain under the following conditions
                                params_restrictions: Some(vec![
                                    ParamRestriction::MustBeValue(
                                        vec!["assert".to_string(), "a".to_string()],
                                        Binary::from(serde_json::to_vec(&json!(
                                            valence_middleware_asserter::msg::AssertionValue::Variable(
                                                valence_middleware_asserter::msg::QueryInfo {
                                                    storage_account,
                                                    storage_slot_key: TARGET_QUERY_LABEL.to_string(),
                                                    query: to_json_binary(&XykPoolQuery::GetPoolAssetAmount {
                                                        target_denom: denom.to_string(),
                                                    })?,
                                                },
                                            )
                                        ))?)
                                    ),
                                    ParamRestriction::MustBeValue(
                                        vec!["assert".to_string(), "predicate".to_string()],
                                        Binary::from(serde_json::to_vec(&json!(
                                            valence_middleware_asserter::msg::Predicate::LT
                                        ))?)
                                    ),
                                    ParamRestriction::MustBeValue(
                                        vec!["assert".to_string(), "b".to_string()],
                                        Binary::from(serde_json::to_vec(&json!(
                                            valence_middleware_asserter::msg::AssertionValue::Constant(
                                                valence_middleware_utils::type_registry::queries::ValencePrimitive::Uint128(
                                                    Uint128::new(150_000_000),
                                                ),
                                            )
                                        ))?)
                                    ),
                                ]),
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(ibc_forwarder))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let osmosis_lp_authorization = AuthorizationBuilder::new()
        .with_label(PROVIDE_LIQUIDITY_LABEL)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
                        .with_contract_address(LibraryAccountType::Addr(gamm_lper))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let authorizations = vec![
        register_kvq_authorization,
        deregister_kvq_authorization,
        conditional_ibc_forwarding_authorization,
        osmosis_lp_authorization,
    ];

    info!("Creating execute authorization...");
    let create_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations { authorizations },
    );

    std::thread::sleep(std::time::Duration::from_secs(3));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorization_contract_address.to_string().as_str(),
        DEFAULT_KEY,
        &serde_json::to_string(&create_authorization)?,
        &format!("{GAS_FLAGS} --fees=100000untrn"),
    )?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let query_authorizations_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            authorization_contract_address,
            &serde_json::to_string(
                &valence_authorization_utils::msg::QueryMsg::Authorizations {
                    start_after: None,
                    limit: None,
                },
            )?,
        )["data"]
            .clone(),
    )?;
    info!(
        "authorizations created: {:?}",
        query_authorizations_response.as_array().unwrap()
    );

    assert!(query_authorizations_response.as_array().unwrap().len() == 4);

    info!("Authorizations created!");

    Ok(())
}

pub fn set_type_registry(
    test_ctx: &TestContext,
    broker: String,
    type_registry_addr: String,
    type_registry_version: String,
) -> Result<TransactionResponse, LocalError> {
    let set_registry_msg = valence_middleware_broker::msg::ExecuteMsg::SetRegistry {
        version: type_registry_version.to_string(),
        address: type_registry_addr,
    };

    let stringified_msg = serde_json::to_string(&set_registry_msg)
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?;

    info!("registering type registry v.{type_registry_version}");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &broker,
        DEFAULT_KEY,
        &stringified_msg,
        "--amount 1000000untrn --gas 50000000",
    )
}

fn upload_contracts(
    current_dir: PathBuf,
    test_ctx: &mut TestContext,
) -> Result<(), Box<dyn Error>> {
    info!("uploading contracts to neutron & osmosis...");
    let current_dir = current_dir.display().to_string();
    let osmosis_type_registry_middleware_path =
        format!("{current_dir}/artifacts/valence_middleware_osmosis.wasm");
    let osmosis_middleware_broker_path =
        format!("{current_dir}/artifacts/valence_middleware_broker.wasm",);
    let icq_lib_local_path = format!("{current_dir}/artifacts/valence_neutron_ic_querier.wasm",);
    let storage_acc_path = format!("{current_dir}/artifacts/valence_storage_account.wasm",);
    let base_acc_path = format!("{current_dir}/artifacts/valence_base_account.wasm",);
    let asserter_path = format!("{current_dir}/artifacts/valence_middleware_asserter.wasm",);
    let neutron_ibc_forwarder_path =
        format!("{current_dir}/artifacts/valence_neutron_ibc_transfer_library.wasm",);
    let gamm_lper_contract_path =
        format!("{current_dir}/artifacts/valence_osmosis_gamm_lper.wasm",);

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&base_acc_path)?;
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&gamm_lper_contract_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&icq_lib_local_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&osmosis_type_registry_middleware_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&osmosis_middleware_broker_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&storage_acc_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&asserter_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&neutron_ibc_forwarder_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&base_acc_path)?;

    Ok(())
}

fn setup_middleware(
    test_ctx: &mut TestContext,
) -> Result<(String, String, String), Box<dyn Error>> {
    info!("setting up the middleware...");
    let type_registry_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_osmosis")
        .get_cw()
        .code_id
        .unwrap();
    let asserter_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_asserter")
        .get_cw()
        .code_id
        .unwrap();
    let broker_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_broker")
        .get_cw()
        .code_id
        .unwrap();

    let type_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        type_registry_code_id,
        &serde_json::to_string(&RegistryInstantiateMsg {})?,
        "type_registry",
        None,
        "",
    )?
    .address;
    info!("type_registry_contract address: {type_registry_contract}",);
    std::thread::sleep(Duration::from_secs(1));
    let asserter_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        asserter_code_id,
        &serde_json::to_string(&valence_middleware_asserter::msg::InstantiateMsg {})?,
        "asserter",
        None,
        "",
    )?
    .address;

    info!("asserter_contract address: {asserter_contract}");
    std::thread::sleep(Duration::from_secs(1));
    let broker_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        broker_code_id,
        &serde_json::to_string(&valence_middleware_broker::msg::InstantiateMsg {})?,
        "broker",
        None,
        "",
    )?
    .address;
    info!("middleware broker address: {broker_contract}");
    std::thread::sleep(Duration::from_secs(1));

    let resp = set_type_registry(
        test_ctx,
        broker_contract.to_string(),
        type_registry_contract.to_string(),
        "26.0.0".to_string(),
    )?;
    std::thread::sleep(Duration::from_secs(2));
    info!("added type registry response: {:?}", resp.tx_hash.unwrap());

    Ok((broker_contract, asserter_contract, type_registry_contract))
}
