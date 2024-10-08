use std::{env, error::Error, time::SystemTime};

use cosmwasm_std::Binary;
use cosmwasm_std_old::Coin as BankCoin;

use local_interchaintest::utils::{
    authorization::set_up_authorization_and_processor,
    base_account::{approve_service, create_base_accounts},
    processor::tick_processor,
    GAS_FLAGS, LOGS_FILE_PATH, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
use rand::{distributions::Alphanumeric, Rng};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};
use valence_splitter_service::msg::{
    ActionMsgs, ServiceConfig, UncheckedSplitAmount, UncheckedSplitConfig,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // We need to predict the authorization contract address in advance for the processor contract on the main domain
    // We'll use the current time as a salt so we can run this test multiple times locally without getting conflicts
    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );
    // Upload and instantiate authorization and processor on Neutron
    let (authorization_contract_address, processor_contract_address) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

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

    // We are going to create 2 base accounts on Neutron, who'll make a token swap with each other
    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        code_id_base_account,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        2,
    );
    let base_account_1 = base_accounts[0].clone();
    let base_account_2 = base_accounts[1].clone();

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

    let swap_amount = 1_000_000_000;
    // Mint some of each token and send it to the base accounts
    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(swap_amount)
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
            amount: swap_amount.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(swap_amount)
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
            amount: swap_amount.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Now that each account has its token to swap, let's prepare the workflow
    // Let's upload the splitter, which is what we are going to use to do the swap.
    let splitter_contract_path = format!(
        "{}/artifacts/valence_splitter_service.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&splitter_contract_path)?;

    // Get the code id
    let code_id_splitter = test_ctx
        .get_contract()
        .contract("valence_splitter_service")
        .get_cw()
        .code_id
        .unwrap();

    info!("Preparing splitters...");
    let splitter_1_instantiate_msg = valence_service_utils::msg::InstantiateMsg::<ServiceConfig> {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: processor_contract_address.clone(),
        config: ServiceConfig {
            input_addr: base_account_1.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token1.clone()),
                account: ServiceAccountType::AccountAddr(base_account_2.clone()),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
            }],
        },
    };
    let splitter1 = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_splitter,
        &serde_json::to_string(&splitter_1_instantiate_msg).unwrap(),
        "splitter",
        None,
        "",
    )
    .unwrap();

    info!("Splitter 1: {}", splitter1.address.clone());

    let splitter_2_instantiate_msg = valence_service_utils::msg::InstantiateMsg::<ServiceConfig> {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: processor_contract_address.clone(),
        config: ServiceConfig {
            input_addr: base_account_2.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token2.clone()),
                account: ServiceAccountType::AccountAddr(base_account_1.clone()),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
            }],
        },
    };

    let splitter2 = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_splitter,
        &serde_json::to_string(&splitter_2_instantiate_msg).unwrap(),
        "splitter",
        None,
        "",
    )
    .unwrap();

    info!("Splitter 2: {}", splitter2.address.clone());

    // Approve the services for the base accounts
    approve_service(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &base_account_1,
        splitter1.address.clone(),
    );
    approve_service(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &base_account_2,
        splitter2.address.clone(),
    );

    // Now that everything is set up, we need to create the authorization that will do the swap atomically between the 2 base accounts
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("swap")
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(&splitter1.address)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(
                                    vec!["process_action".to_string(), "split".to_string()],
                                )]),
                            },
                        })
                        .build(),
                )
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(&splitter2.address)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(
                                    vec!["process_action".to_string(), "split".to_string()],
                                )]),
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    info!("Creating swap authorization...");
    let create_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations { authorizations },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&create_authorization).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Swap authorization created!");
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Send the messages to the authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(ActionMsgs::Split {}),
        )
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "swap".to_string(),
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
        .any(|balance| balance.denom == token2 && balance.amount.u128() == swap_amount));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &base_account_2,
    );

    assert!(token_balances
        .iter()
        .any(|balance| balance.denom == token1 && balance.amount.u128() == swap_amount));

    info!("Token swap successful!");
    Ok(())
}
