use std::{env,error::Error};
use rand::{distributions::Alphanumeric, Rng};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME, DEFAULT_KEY
};
use localic_std::modules::{
    cosmwasm::{contract_execute, contract_instantiate, contract_query},
};
use serde_json::Value;
use valence_astroport_utils::astroport_native_lp_token::{
     FactoryInstantiateMsg, FactoryQueryMsg, PairConfig, PairType, AssetInfo
};
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config_builder::ProgramConfigBuilder,
    program_config::ProgramConfig,
};
use valence_e2e::utils::{
   NTRN_DENOM, ASTROPORT_PATH, GAS_FLAGS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
   LOCAL_CODE_ID_CACHE_PATH_NEUTRON
};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_astroport_lper::msg::LiquidityProviderConfig;
use valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig;
use valence_library_utils::{
     liquidity_utils::AssetData,
};

// three accounts. let's call them input account, position account, output account
// two libraries. astroport liquidity provider and astroport withdrawer
// two subroutines:
// provide liquidity from the input account and deposit LP tokens into the position account
// withdraw liquidity from the position account and into the output account

/// Write your program using the program builder
pub(crate) fn my_program() -> Result<ProgramConfig, Box<dyn Error>> {

    let mut test_ctx = TestContextBuilder::default()
    .with_unwrap_raw_logs(true)
    .with_api_url(LOCAL_IC_API_URL)
    .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
    .with_chain(ConfigChainBuilder::default_neutron().build()?)
    .with_log_file_path(LOGS_FILE_PATH)
    .build()?;


    let current_dir = env::current_dir()?;
    let astroport_contracts_path = format!("{}/{}", current_dir.display(), ASTROPORT_PATH);

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(&astroport_contracts_path, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)?;

            // Set up the astroport factory and the pool
    let astroport_factory_code_id = test_ctx
    .get_contract()
    .contract("astroport_factory_native")
    .get_cw()
    .code_id
    .unwrap();

    let astroport_pair_native_code_id = test_ctx
    .get_contract()
    .contract("astroport_pair_native")
    .get_cw()
    .code_id
    .unwrap();

    let astroport_token_code_id = test_ctx
    .get_contract()
    .contract("astroport_token")
    .get_cw()
    .code_id
    .unwrap();

    let astroport_factory_instantiate_msg = FactoryInstantiateMsg {
        pair_configs: vec![PairConfig {
            code_id: astroport_pair_native_code_id,
            pair_type: PairType::Xyk {},
            total_fee_bps: 0,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: true,
            permissioned: false,
        }],
        token_code_id: astroport_token_code_id,
        fee_address: None,
        generator_address: None,
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        whitelist_code_id: 0, // This is not needed anymore but still part of API
        coin_registry_address: NEUTRON_CHAIN_ADMIN_ADDR.to_string(), // Passing any address here is fine as long as it's a valid one
        tracker_config: None,
    };

    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_factory_code_id,
        &serde_json::to_string(&astroport_factory_instantiate_msg).unwrap(),
        "processor",
        None,
        "",
    )
    .unwrap();

        // Let's create a token to pair it with NTRN
        let token_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token_subdenom)
        .get();

            // Mint some of the token
        test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(1_000_000_000)
        .with_denom(&token)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let pool_assets = vec![
        AssetInfo::NativeToken {
            denom: NTRN_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: token.clone(),
        },
    ];
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_astroport_utils::astroport_native_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: PairType::Xyk {},
                asset_infos: pool_assets.clone(),
                init_params: None,
            },
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(3));

    let query_pool_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &factory_contract.address.clone(),
            &serde_json::to_string(&FactoryQueryMsg::Pair {
                asset_infos: pool_assets.clone(),
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();
    let pool_addr = query_pool_response["contract_addr"].as_str().unwrap();
    let lp_token = query_pool_response["liquidity_token"].as_str().unwrap();

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let input_account = builder.add_account(AccountInfo::new(
        "input_account".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let liquidity_position_account = builder.add_account(AccountInfo::new(
        "liquidity_position_account".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let output_account = builder.add_account(AccountInfo::new(
        "output_account".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Provide Liquidity

    let lper_config = LiquidityProviderConfig  {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(
            valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
        ),
        asset_data: AssetData {
            asset1: NTRN_DENOM.to_string(),
            asset2: token.clone(),
        },
        max_spread: None,
    };

    let lper_library = builder.add_library(LibraryInfo::new(
        "deploy_liquidity".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportLper(
            valence_astroport_lper::msg::LibraryConfig {
                input_addr: input_account.clone(),
                output_addr: liquidity_position_account.clone(),
                pool_addr: pool_addr.to_string(),
                lp_config: lper_config.clone(),
            },
    ),
    ));

    builder.add_link(&lper_library, vec![&input_account], vec![&liquidity_position_account]);

    let provide_liqudity_function = AtomicFunctionBuilder::new().with_contract_address(lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![
                    ParamRestriction::MustBeIncluded(vec![
                        "process_function".to_string(),
                        "provide_double_sided_liquidity".to_string(),
                    ]),
                ]),
            },
        })
        .build();
    
        builder.add_authorization(
            AuthorizationBuilder::new()
                .with_label("provide_liquidity")
                .with_subroutine(
                    AtomicSubroutineBuilder::new()
                        .with_function(provide_liqudity_function)
                        .build()
                )
                .build()
        );

        // Withdraw Liquidity

        let withdrawer_config = LiquidityWithdrawerConfig  {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {  },
            ),
            asset_data: AssetData {
                asset1: NTRN_DENOM.to_string(),
                asset2: token.clone(),
            },
        };


    let withdrawer_library = builder.add_library(LibraryInfo::new(
        "withdraw_liquidity_position".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportWithdrawer(         
            valence_astroport_withdrawer::msg::LibraryConfig {
            input_addr: liquidity_position_account.clone(),
            output_addr: output_account.clone(),
            pool_addr: pool_addr.to_string(),
            withdrawer_config: withdrawer_config.clone(),
        },),
    ));

        builder.add_link(&withdrawer_library, vec![&liquidity_position_account], vec![&output_account]);


        let withdraw_liqudity_function = AtomicFunctionBuilder::new()
        .with_contract_address(withdrawer_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![
                    ParamRestriction::MustBeIncluded(vec![
                        "process_function".to_string(),
                        "withdraw_liquidity".to_string(),
                    ]),
                ]),
            },
        })
        .build();

    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("withdraw_liquidity")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(withdraw_liqudity_function)
                    .build()
            )
            .build()
    );

   Ok(builder.build())
    
}
