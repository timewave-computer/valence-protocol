use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Decimal, Uint128,
};
use std::collections::HashMap;

use skip_swap_valence::contract::{execute, instantiate};
use skip_swap_valence::msg::{ExecuteMsg, InstantiateMsg};
use skip_swap_valence::types::{AssetPair, Config, SkipRouteResponse, SwapOperation, SwapDetails};
use skip_swap_valence::state::{CONFIG, ROUTE_COUNT};

fn create_instantiate_msg() -> InstantiateMsg {
    let mut token_destinations = HashMap::new();
    token_destinations.insert("uusdc".to_string(), Addr::unchecked("usdc_destination"));
    
    let mut intermediate_accounts = HashMap::new();
    intermediate_accounts.insert("ueth".to_string(), Addr::unchecked("eth_address"));
    
    InstantiateMsg {
        config: Config {
            strategist_address: Addr::unchecked("strategist"),
            skip_entry_point: Addr::unchecked("skip_entry"),
            allowed_asset_pairs: vec![
                AssetPair {
                    input_asset: "uatom".to_string(),
                    output_asset: "uusdc".to_string(),
                },
                AssetPair {
                    input_asset: "uluna".to_string(),
                    output_asset: "uusdc".to_string(),
                },
            ],
            allowed_venues: vec!["astroport".to_string(), "osmosis".to_string()],
            max_slippage: Decimal::percent(5),
            token_destinations,
            intermediate_accounts,
        }
    }
}

fn create_test_route() -> SkipRouteResponse {
    SkipRouteResponse {
        source_chain_id: "cosmos-hub-4".to_string(),
        source_asset_denom: "uatom".to_string(),
        dest_chain_id: "cosmos-hub-4".to_string(),
        dest_asset_denom: "uusdc".to_string(),
        amount: Uint128::new(1000000),
        operations: vec![
            SwapOperation {
                chain_id: "cosmos-hub-4".to_string(),
                operation_type: "swap".to_string(),
                swap_venue: Some("astroport".to_string()),
                swap_details: Some(SwapDetails {
                    input_denom: "uatom".to_string(),
                    output_denom: "uusdc".to_string(),
                    pool_id: Some("1".to_string()),
                }),
                transfer_details: None,
            }
        ],
        expected_output: Uint128::new(100),
        slippage_tolerance_percent: Decimal::percent(2),
    }
}

#[test]
fn test_state_initialization() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = create_instantiate_msg();
    
    // Test instantiation
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());
    
    // Verify config is stored correctly
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.strategist_address, Addr::unchecked("strategist"));
    assert_eq!(config.skip_entry_point, Addr::unchecked("skip_entry"));
    assert_eq!(config.allowed_asset_pairs.len(), 2);
    assert_eq!(config.allowed_venues.len(), 2);
    assert_eq!(config.max_slippage, Decimal::percent(5));
    assert_eq!(config.token_destinations.len(), 1);
    assert_eq!(config.token_destinations.get("uusdc").unwrap(), &Addr::unchecked("usdc_destination"));
    assert_eq!(config.intermediate_accounts.len(), 1);
    assert_eq!(config.intermediate_accounts.get("ueth").unwrap(), &Addr::unchecked("eth_address"));
    
    // Verify route counter is initialized to 0
    let route_count = ROUTE_COUNT.load(&deps.storage).unwrap();
    assert_eq!(route_count, 0);
}

#[test]
fn test_increment_route_counter() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = create_instantiate_msg();
    
    // Instantiate
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    
    // Initial route counter should be 0
    let route_count = ROUTE_COUNT.load(&deps.storage).unwrap();
    assert_eq!(route_count, 0);
    
    // Execute an optimized route as strategist
    let route = create_test_route();
    let execute_msg = ExecuteMsg::ExecuteOptimizedRoute {
        input_denom: "uatom".to_string(),
        input_amount: Uint128::new(1000000),
        output_denom: "uusdc".to_string(),
        min_output_amount: Uint128::new(100),
        route: route,
        timeout_timestamp: Some(1634567890),
        swap_venue: Some("astroport".to_string()),
    };
    
    let res = execute(deps.as_mut(), env.clone(), mock_info("strategist", &[]), execute_msg);
    assert!(res.is_ok());
    
    // Route counter should be incremented to 1
    let route_count = ROUTE_COUNT.load(&deps.storage).unwrap();
    assert_eq!(route_count, 1);
    
    // Execute another optimized route to increment again
    let route = create_test_route();
    let execute_msg = ExecuteMsg::ExecuteOptimizedRoute {
        input_denom: "uatom".to_string(),
        input_amount: Uint128::new(1000000),
        output_denom: "uusdc".to_string(),
        min_output_amount: Uint128::new(100),
        route: route,
        timeout_timestamp: Some(1634567890),
        swap_venue: Some("astroport".to_string()),
    };
    
    let res = execute(deps.as_mut(), env.clone(), mock_info("strategist", &[]), execute_msg);
    assert!(res.is_ok());
    
    // Route counter should be incremented to 2
    let route_count = ROUTE_COUNT.load(&deps.storage).unwrap();
    assert_eq!(route_count, 2);
}

#[test]
fn test_state_updates() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = create_instantiate_msg();
    
    // Instantiate
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    
    // Execute an optimized route as strategist
    let route = create_test_route();
    let execute_msg = ExecuteMsg::ExecuteOptimizedRoute {
        input_denom: "uatom".to_string(),
        input_amount: Uint128::new(1000000),
        output_denom: "uusdc".to_string(),
        min_output_amount: Uint128::new(100),
        route: route,
        timeout_timestamp: Some(1634567890),
        swap_venue: Some("astroport".to_string()),
    };
    
    let res = execute(deps.as_mut(), env.clone(), mock_info("strategist", &[]), execute_msg);
    assert!(res.is_ok());
    
    // Update token destinations
    let mut new_token_destinations = HashMap::new();
    new_token_destinations.insert("uusdc".to_string(), Addr::unchecked("new_usdc_destination"));
    new_token_destinations.insert("uluna".to_string(), Addr::unchecked("luna_destination"));
    
    let mut new_config = CONFIG.load(&deps.storage).unwrap();
    new_config.token_destinations = new_token_destinations;
    
    let update_msg = ExecuteMsg::UpdateConfig {
        config: new_config,
    };
    
    let res = execute(deps.as_mut(), env.clone(), mock_info("strategist", &[]), update_msg);
    assert!(res.is_ok());
    
    // Verify config changes were applied
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.token_destinations.len(), 2);
    assert_eq!(config.token_destinations.get("uusdc").unwrap(), &Addr::unchecked("new_usdc_destination"));
    assert_eq!(config.token_destinations.get("uluna").unwrap(), &Addr::unchecked("luna_destination"));
    
    // Verify route counter was preserved during update
    let route_count = ROUTE_COUNT.load(&deps.storage).unwrap();
    assert_eq!(route_count, 1); 
    
    // Update intermediate accounts
    let mut new_intermediate_accounts = HashMap::new();
    new_intermediate_accounts.insert("ueth".to_string(), Addr::unchecked("new_eth_address"));
    new_intermediate_accounts.insert("ubtc".to_string(), Addr::unchecked("btc_address"));
    
    let mut new_config = CONFIG.load(&deps.storage).unwrap();
    new_config.intermediate_accounts = new_intermediate_accounts;
    
    let update_msg = ExecuteMsg::UpdateConfig {
        config: new_config,
    };
    
    let res = execute(deps.as_mut(), env.clone(), mock_info("strategist", &[]), update_msg);
    assert!(res.is_ok());
    
    // Verify intermediate accounts changes were applied
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.intermediate_accounts.len(), 2);
    assert_eq!(config.intermediate_accounts.get("ueth").unwrap(), &Addr::unchecked("new_eth_address"));
    assert_eq!(config.intermediate_accounts.get("ubtc").unwrap(), &Addr::unchecked("btc_address"));
} 