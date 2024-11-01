use std::{error::Error, vec};

use cosmwasm_std_old::Uint128;
use local_interchaintest::utils::{
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, REBALANCER_ARTIFACTS_PATH,
    VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR,
};
use rand::{distributions::Alphanumeric, Rng};
use rebalancer_auction_package::Pair;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .send_with_local_cache(REBALANCER_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)
        .unwrap();

    // create the denoms
    let usdc_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let newt_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&usdc_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));
    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&newt_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let usdc_denom = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(usdc_subdenom)
        .get();
    let newt_denom = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(newt_subdenom)
        .get();

    // init auctions manager
    let mut auctions_manager = test_ctx
        .get_contract()
        .contract("auctions_manager")
        .get_cw();
    let auction = test_ctx.get_contract().contract("auction").get_cw();

    let auctions_manager_init_msg = rebalancer_auction_manager::msg::InstantiateMsg {
        auction_code_id: auction.code_id.unwrap(),
        min_auction_amount: vec![
            (
                "untrn".to_string(),
                rebalancer_auction_package::states::MinAmount {
                    send: Uint128::one(),
                    start_auction: Uint128::one(),
                },
            ),
            (
                usdc_denom.to_string(),
                rebalancer_auction_package::states::MinAmount {
                    send: Uint128::one(),
                    start_auction: Uint128::one(),
                },
            ),
            (
                newt_denom.to_string(),
                rebalancer_auction_package::states::MinAmount {
                    send: Uint128::one(),
                    start_auction: Uint128::one(),
                },
            ),
        ],
        server_addr: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
    };

    let auctions_manager_addr = auctions_manager.instantiate(
        DEFAULT_KEY,
        &serde_json::to_string(&auctions_manager_init_msg).unwrap(),
        "auctions manager",
        None,
        "",
    )?;

    // init oracle
    let mut oracle = test_ctx.get_contract().contract("oracle").get_cw();
    let oracle_msg = rebalancer_oracle::msg::InstantiateMsg {
        auctions_manager_addr: auctions_manager_addr.address.clone(),
        seconds_allow_manual_change: 5,
        seconds_auction_prices_fresh: 60 * 60 * 24,
    };
    let oracle_addr = oracle.instantiate(
        DEFAULT_KEY,
        &serde_json::to_string(&oracle_msg).unwrap(),
        "oracle",
        None,
        "",
    )?;

    // add oracle address to the auctions manager
    let add_oracle_msg = rebalancer_auction_manager::msg::ExecuteMsg::Admin(Box::new(
        rebalancer_auction_manager::msg::AdminMsgs::UpdateOracle {
            oracle_addr: oracle_addr.address,
        },
    ));
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&add_oracle_msg).unwrap(),
        "",
    )?;

    // init auction for each pair (we have 3 tokens, untrn, "usdc", "newt")
    let base_auction_strategy = rebalancer_auction_package::AuctionStrategy {
        start_price_perc: 10000,
        end_price_perc: 9999,
    };

    // pairs
    let ntrn_usdc_pair = Pair("untrn".to_string(), usdc_denom.to_string());
    let usdc_ntrn_pair = Pair(usdc_denom.to_string(), "untrn".to_string());
    let ntrn_newt_pair = Pair("untrn".to_string(), newt_denom.to_string());
    let newt_ntrn_pair = Pair(newt_denom.to_string(), "untrn".to_string());
    let usdc_newt_pair = Pair(usdc_denom.to_string(), newt_denom.to_string());
    let newt_usdc_pair = Pair(newt_denom.to_string(), usdc_denom.to_string());

    // ntrn - usdc
    let ntrn_usdc_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: ntrn_usdc_pair.clone(),
        auction_strategy: base_auction_strategy.clone(),
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_auction_manager::msg::ExecuteMsg::Admin(
            Box::new(rebalancer_auction_manager::msg::AdminMsgs::NewAuction {
                msg: ntrn_usdc_init_msg,
                label: "ntrn_usdc".to_string(),
                min_amount: None,
            }),
        ))
        .unwrap(),
        "",
    )?;

    // usdc - ntrn
    let usdc_ntrn_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: usdc_ntrn_pair.clone(),
        auction_strategy: base_auction_strategy.clone(),
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_auction_manager::msg::ExecuteMsg::Admin(
            Box::new(rebalancer_auction_manager::msg::AdminMsgs::NewAuction {
                msg: usdc_ntrn_init_msg,
                label: "usdc_ntrn".to_string(),
                min_amount: None,
            }),
        ))
        .unwrap(),
        "",
    )?;

    // ntrn - newt
    let ntrn_newt_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: ntrn_newt_pair.clone(),
        auction_strategy: base_auction_strategy.clone(),
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_auction_manager::msg::ExecuteMsg::Admin(
            Box::new(rebalancer_auction_manager::msg::AdminMsgs::NewAuction {
                msg: ntrn_newt_init_msg,
                label: "ntrn_newt".to_string(),
                min_amount: None,
            }),
        ))
        .unwrap(),
        "",
    )?;

    // newt - ntrn
    let newt_ntrn_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: newt_ntrn_pair.clone(),
        auction_strategy: base_auction_strategy.clone(),
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_auction_manager::msg::ExecuteMsg::Admin(
            Box::new(rebalancer_auction_manager::msg::AdminMsgs::NewAuction {
                msg: newt_ntrn_init_msg,
                label: "newt_ntrn".to_string(),
                min_amount: None,
            }),
        ))
        .unwrap(),
        "",
    )?;

    // usdc - newt
    let usdc_newt_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: usdc_newt_pair.clone(),
        auction_strategy: base_auction_strategy.clone(),
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_auction_manager::msg::ExecuteMsg::Admin(
            Box::new(rebalancer_auction_manager::msg::AdminMsgs::NewAuction {
                msg: usdc_newt_init_msg,
                label: "usdc_newt".to_string(),
                min_amount: None,
            }),
        ))
        .unwrap(),
        "",
    )?;

    // newt - usdc
    let newt_usdc_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: newt_usdc_pair.clone(),
        auction_strategy: base_auction_strategy,
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    auctions_manager.execute(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_auction_manager::msg::ExecuteMsg::Admin(
            Box::new(rebalancer_auction_manager::msg::AdminMsgs::NewAuction {
                msg: newt_usdc_init_msg,
                label: "newt_usdc".to_string(),
                min_amount: None,
            }),
        ))
        .unwrap(),
        "",
    )?;

    // update prices on the oracle
    let usdc_price = cosmwasm_std_old::Decimal::from_atomics(1000000u128, 0).unwrap(); // 1$
    let ntrn_price = cosmwasm_std_old::Decimal::from_atomics(2000000u128, 0).unwrap(); // 2$
    let newt_price = cosmwasm_std_old::Decimal::from_atomics(3000000u128, 0).unwrap(); // 3$

    // ntrn_usdc price
    let oracle_price_update_msg = rebalancer_oracle::msg::ExecuteMsg::ManualPriceUpdate {
        pair: ntrn_usdc_pair,
        price: ntrn_price / usdc_price,
    };
    oracle
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&oracle_price_update_msg).unwrap(),
            "",
        )
        .unwrap();

    // usdc_ntrn price
    let oracle_price_update_msg = rebalancer_oracle::msg::ExecuteMsg::ManualPriceUpdate {
        pair: usdc_ntrn_pair,
        price: usdc_price / ntrn_price,
    };
    oracle
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&oracle_price_update_msg).unwrap(),
            "",
        )
        .unwrap();

    // ntrn_newt price
    let oracle_price_update_msg = rebalancer_oracle::msg::ExecuteMsg::ManualPriceUpdate {
        pair: ntrn_newt_pair,
        price: ntrn_price / newt_price,
    };
    oracle
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&oracle_price_update_msg).unwrap(),
            "",
        )
        .unwrap();

    // newt_ntrn price
    let oracle_price_update_msg = rebalancer_oracle::msg::ExecuteMsg::ManualPriceUpdate {
        pair: newt_ntrn_pair,
        price: newt_price / ntrn_price,
    };
    oracle
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&oracle_price_update_msg).unwrap(),
            "",
        )
        .unwrap();

    // newt_usdc price
    let oracle_price_update_msg = rebalancer_oracle::msg::ExecuteMsg::ManualPriceUpdate {
        pair: newt_usdc_pair,
        price: newt_price / usdc_price,
    };
    oracle
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&oracle_price_update_msg).unwrap(),
            "",
        )
        .unwrap();

    // usdc_newt price
    let oracle_price_update_msg = rebalancer_oracle::msg::ExecuteMsg::ManualPriceUpdate {
        pair: usdc_newt_pair,
        price: usdc_price / newt_price,
    };
    oracle
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&oracle_price_update_msg).unwrap(),
            "",
        )
        .unwrap();

    // init services manager
    let mut services_manager = test_ctx
        .get_contract()
        .contract("services_manager")
        .get_cw();
    let services_manager_init_msg = rebalancer_services_manager::msg::InstantiateMsg {
        whitelisted_code_ids: vec![],
    };
    let services_manager_addr = services_manager.instantiate(
        DEFAULT_KEY,
        &serde_json::to_string(&services_manager_init_msg).unwrap(),
        "services manager",
        None,
        "",
    )?;

    // init the rebalancer
    let mut rebalancer = test_ctx.get_contract().contract("rebalancer").get_cw();
    let rebalancer_init_msg = rebalancer_rebalancer::msg::InstantiateMsg {
        denom_whitelist: vec![
            "untrn".to_string(),
            usdc_denom.to_string(),
            newt_denom.to_string(),
        ],
        base_denom_whitelist: vec![],
        services_manager_addr: services_manager_addr.address,
        cycle_start: cosmwasm_std_old::Timestamp::from_seconds(0),
        auctions_manager_addr: auctions_manager_addr.address,
        cycle_period: Some(1),
        fees: rebalancer_package::services::rebalancer::ServiceFeeConfig {
            denom: "untrn".to_string(),
            register_fee: cosmwasm_std_old::Uint128::one(),
            resume_fee: cosmwasm_std_old::Uint128::zero(),
        },
    };
    let rebalancer_addr = rebalancer.instantiate(
        DEFAULT_KEY,
        &serde_json::to_string(&rebalancer_init_msg).unwrap(),
        "rebalancer",
        None,
        "",
    )?;

    // register the rebalancer to the manager
    let register_rebalancer_msg =
        rebalancer_package::msgs::core_execute::ServicesManagerExecuteMsg::Admin(
            rebalancer_package::msgs::core_execute::ServicesManagerAdminMsg::AddService {
                name: rebalancer_package::services::ValenceServices::Rebalancer,
                addr: rebalancer_addr.address,
            },
        );
    services_manager
        .execute(
            DEFAULT_KEY,
            &serde_json::to_string(&register_rebalancer_msg).unwrap(),
            "",
        )
        .unwrap();
    // init MM bidder accounts with all tokens to bid that will buy from the auctions

    // TODO: update the account id whitelisted on the manager
    Ok(())
}