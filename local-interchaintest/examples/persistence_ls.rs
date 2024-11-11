use std::{env, error::Error, time::SystemTime};

use cosmwasm_std::{Binary, CosmosMsg};
use local_interchaintest::utils::{
    authorization::{set_up_authorization_and_processor, set_up_external_domain_with_polytone},
    base_account::create_base_accounts,
    ibc::send_successful_ibc_transfer,
    persistence::{activate_host_zone, register_host_zone},
    processor::{get_processor_queue_items, tick_processor},
    GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_PERSISTENCE, LOGS_FILE_PATH, PERSISTENCE_CHAIN_ADMIN_ADDR,
    PERSISTENCE_CHAIN_DENOM, PERSISTENCE_CHAIN_ID, PERSISTENCE_CHAIN_NAME,
    PERSISTENCE_CHAIN_PREFIX, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{bank, cosmwasm::contract_execute};
use localic_utils::{
    types::config::ConfigChain, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
};
use log::info;
use persistence_std::types::{
    cosmos::base::v1beta1::Coin, pstake::liquidstakeibc::v1beta1::MsgLiquidStake,
};
use valence_authorization_utils::{
    authorization::Priority,
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
    msg::ProcessorMessage,
};
use valence_service_utils::ServiceAccountType;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChain {
            denom: PERSISTENCE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: PERSISTENCE_CHAIN_ID.to_string(),
            chain_name: PERSISTENCE_CHAIN_NAME.to_string(),
            chain_prefix: PERSISTENCE_CHAIN_PREFIX.to_string(),
            admin_addr: PERSISTENCE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, PERSISTENCE_CHAIN_NAME)
        .build()?;

    let channel_id = test_ctx
        .get_transfer_channels()
        .src(PERSISTENCE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let connection_id = test_ctx
        .get_connections()
        .src(PERSISTENCE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let native_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN_NAME).get();

    info!("Registering host zone...");
    register_host_zone(
        test_ctx
            .get_request_builder()
            .get_request_builder(PERSISTENCE_CHAIN_NAME),
        NEUTRON_CHAIN_ID,
        &connection_id,
        &channel_id,
        &native_denom,
        DEFAULT_KEY,
    )?;

    info!("Activating host zone...");
    activate_host_zone(NEUTRON_CHAIN_ID)?;

    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );

    let (authorization_contract_address, _) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    let processor_on_persistence = set_up_external_domain_with_polytone(
        &mut test_ctx,
        PERSISTENCE_CHAIN_NAME,
        PERSISTENCE_CHAIN_ID,
        PERSISTENCE_CHAIN_ADMIN_ADDR,
        LOCAL_CODE_ID_CACHE_PATH_PERSISTENCE,
        "neutron-persistence",
        salt,
        &authorization_contract_address,
    )?;

    // Now that we have the processor on persistence, let's create a base account and approve it
    let current_dir: std::path::PathBuf = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    info!("Uploading base account contract to persistence...");
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(PERSISTENCE_CHAIN_NAME)
        .send_single_contract(&base_account_contract_path)?;

    let base_account_code_id = test_ctx
        .get_contract()
        .src(PERSISTENCE_CHAIN_NAME)
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        PERSISTENCE_CHAIN_NAME,
        base_account_code_id,
        PERSISTENCE_CHAIN_ADMIN_ADDR.to_string(),
        vec![processor_on_persistence.clone()],
        1,
    );
    let persistence_base_account = base_accounts.first().unwrap();

    // Now that everything is set up, let's create the authorization that will be used to liquid stake from the base account
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("execute")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(Domain::External(PERSISTENCE_CHAIN_NAME.to_string()))
                        .with_contract_address(ServiceAccountType::Addr(
                            persistence_base_account.clone(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "execute_msg".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    info!("Creating execute authorization...");
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
    std::thread::sleep(std::time::Duration::from_secs(3));
    info!("Execute authorization created!");

    info!("Send some NTRN to the base account to liquid stake...");
    let amount_to_liquid_stake = 1000000;
    let neutron_on_persistence = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_string())
        .src(NEUTRON_CHAIN_NAME)
        .dest(PERSISTENCE_CHAIN_NAME)
        .get();

    send_successful_ibc_transfer(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        PERSISTENCE_CHAIN_NAME,
        amount_to_liquid_stake,
        NEUTRON_CHAIN_DENOM,
        &neutron_on_persistence,
        persistence_base_account,
        10,
    )?;

    // Everything is ready now to send the liquid staking message and tick the processor
    info!("Send the messages to the authorization contract...");

    let msg_liquid_stake = MsgLiquidStake {
        amount: Some(Coin {
            denom: neutron_on_persistence.clone(),
            amount: amount_to_liquid_stake.to_string(),
        }),
        delegator_address: persistence_base_account.clone(),
    };
    #[allow(deprecated)]
    let liquid_staking_message = CosmosMsg::Stargate {
        type_url: msg_liquid_stake.to_any().type_url,
        value: Binary::from(msg_liquid_stake.to_proto_bytes()),
    };

    let binary = Binary::from(
        serde_json::to_vec(&valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
            msgs: vec![liquid_staking_message],
        })
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "execute".to_string(),
            messages: vec![message],
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
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Waiting for the processor to have the batch...");
    let mut tries = 0;
    loop {
        let items = get_processor_queue_items(
            &mut test_ctx,
            PERSISTENCE_CHAIN_NAME,
            &processor_on_persistence,
            Priority::Medium,
        );
        println!("Items on persistence: {:?}", items);
        tries += 1;
        if !items.is_empty() {
            info!("Batch found!");
            break;
        }
        if tries > 10 {
            panic!("Batch not found after 10 tries");
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    tick_processor(
        &mut test_ctx,
        PERSISTENCE_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_on_persistence,
    );
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Verify that base account liquid staked...");
    let liquid_stake_denom = "stk/untrn";

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(PERSISTENCE_CHAIN_NAME),
        persistence_base_account,
    );
    assert!(token_balances
        .iter()
        .any(|balance| balance.denom == liquid_stake_denom));

    info!("Base account successfully liquid staked!");

    Ok(())
}
