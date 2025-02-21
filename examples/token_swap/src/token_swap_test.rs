use std::error::Error;

use cosmwasm_std::Binary;
use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::{bank, cosmwasm::contract_execute};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};
use log::info;
use rand::{distributions::Alphanumeric, Rng};
use token_swap_example::token_swap::my_atomic_token_swap_program;
use valence_authorization_utils::msg::ProcessorMessage;
use valence_e2e::utils::{
    manager::{setup_manager, use_manager_init, SPLITTER_NAME},
    processor::tick_processor,
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use valence_library_utils::LibraryAccountType;
use valence_splitter_library::msg::FunctionMsgs;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    info!("Create and mint tokens to perform the swap...");
    // We are going to create 2 tokenfactory tokens so that we can test the token swap, one will be given to first account and the second one will be given to the second account
    // We are going to use random subdenoms so that the test can be run multiple times
    let token1_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    let token2_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token1_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token1 = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token1_subdenom)
        .get();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token2_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token2 = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token2_subdenom)
        .get();

    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());
    let swap_amount_token1 = 500_000_000;
    let swap_amount_token2 = 1_000_000_000;

    let mut program_config = my_atomic_token_swap_program(
        neutron_domain.clone(),
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        token1.clone(),
        token2.clone(),
        swap_amount_token1,
        swap_amount_token2,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
    );

    // Verify config is ok before we upload all contracts
    program_config.verify_new_config()?;

    // Setup the contracts and update the global config
    info!("Setup manager...");
    setup_manager(
        &mut test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![SPLITTER_NAME],
    )?;

    // init the program
    info!("Start manager init...");
    use_manager_init(&mut program_config)?;

    // Get all the addresses we need to interact with
    let authorization_contract_address =
        program_config.authorization_data.authorization_addr.clone();
    let processor_contract_address = program_config
        .get_processor_addr(&neutron_domain.to_string())
        .unwrap();
    let base_account_1 = program_config
        .get_account(LibraryAccountType::AccountId(0))
        .unwrap()
        .addr
        .clone()
        .unwrap();
    let base_account_2 = program_config
        .get_account(LibraryAccountType::AccountId(1))
        .unwrap()
        .addr
        .clone()
        .unwrap();

    // Mint some of each token and send it to the base accounts
    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(swap_amount_token1)
        .with_denom(&token1)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &base_account_1,
        &[BankCoin {
            denom: token1.clone(),
            amount: swap_amount_token1.into(),
        }],
        &BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(swap_amount_token2)
        .with_denom(&token2)
        .send()
        .unwrap();

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &base_account_2,
        &[BankCoin {
            denom: token2.clone(),
            amount: swap_amount_token2.into(),
        }],
        &BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Send the messages to the authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                FunctionMsgs::Split {},
            ),
        )
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "atomic_swap".to_string(),
            messages: vec![message.clone(), message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&send_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the authorization contract!");
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor and executing swap...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
        GAS_FLAGS,
    );

    info!("Verifying balances...");
    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &base_account_1,
    );
    assert!(token_balances
        .iter()
        .any(|balance| balance.denom == token2 && balance.amount.u128() == swap_amount_token2));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &base_account_2,
    );

    assert!(token_balances
        .iter()
        .any(|balance| balance.denom == token1 && balance.amount.u128() == swap_amount_token1));

    info!("Token swap successful!");
    Ok(())
}
