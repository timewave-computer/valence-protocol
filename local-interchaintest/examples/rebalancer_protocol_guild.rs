use std::error::Error;

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
        auctions_manager_addr: auctions_manager_addr.address,
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
    let mut auction = test_ctx.get_contract().contract("auction").get_cw();

    // ntrn - usdc
    let ntrn_usdc_init_msg = rebalancer_auction::msg::InstantiateMsg {
        pair: Pair("untrn".to_string(), usdc_denom.to_string()),
        auction_strategy: rebalancer_auction_package::AuctionStrategy {
            start_price_perc: 10000,
            end_price_perc: 9999,
        },
        chain_halt_config: Default::default(),
        price_freshness_strategy: Default::default(),
    };
    //let auction.instantiate(DEFAULT_KEY, msg, label, admin, flags)
    // usdc - ntrn
    // ntrn - newt
    // newt - ntrn
    // usdc - newt
    // newt - usdc

    // update prices on the oracle

    // init services manager
    // init the rebalancer
    // register the rebalancer to the manager
    // init MM bidder accounts with all tokens to bid that will buy from the auctions

    Ok(())
}
