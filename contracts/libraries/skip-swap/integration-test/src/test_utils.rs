use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use std::collections::HashMap;
use skip_swap_valence::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    types::{AssetPair, Config, SkipRouteResponse, SwapDetails, SwapOperation},
    error::ContractError,
};
use skip_swap_valence_strategist::{config::StrategistConfig, skipapi::SkipApi, strategist::Strategist};

// Creates a test configuration for the Skip Swap contract
pub fn create_test_config(owner: &str) -> Config {
    let mut token_destinations = HashMap::new();
    token_destinations.insert("steth".to_string(), Addr::unchecked("dest_account"));

    let mut intermediate_accounts = HashMap::new();
    intermediate_accounts.insert("uusdc".to_string(), Addr::unchecked("intermediate_account"));

    Config {
        strategist_address: Addr::unchecked(owner),
        skip_entry_point: Addr::unchecked("skip_entry"),
        allowed_asset_pairs: vec![
            AssetPair {
                input_asset: "uusdc".to_string(),
                output_asset: "steth".to_string(),
            },
            AssetPair {
                input_asset: "uatom".to_string(),
                output_asset: "steth".to_string(),
            },
        ],
        allowed_venues: vec!["astroport".to_string(), "osmosis".to_string()],
        max_slippage: Decimal::percent(1),
        token_destinations,
        intermediate_accounts,
    }
}

// Creates a test route response
pub fn create_test_route() -> SkipRouteResponse {
    SkipRouteResponse {
        source_chain_id: "neutron".to_string(),
        source_asset_denom: "uusdc".to_string(),
        dest_chain_id: "neutron".to_string(),
        dest_asset_denom: "steth".to_string(),
        amount: Uint128::new(1000000),
        operations: vec![SwapOperation {
            chain_id: "neutron".to_string(),
            operation_type: "swap".to_string(),
            swap_venue: Some("astroport".to_string()),
            swap_details: Some(SwapDetails {
                input_denom: "uusdc".to_string(),
                output_denom: "steth".to_string(),
                pool_id: Some("pool1".to_string()),
            }),
            transfer_details: None,
        }],
        expected_output: Uint128::new(990000),
        slippage_tolerance_percent: Decimal::percent(1),
    }
}

// Stores the Skip Swap contract in the test app
pub fn store_skip_swap_contract() -> Box<dyn Contract<cosmwasm_std::Empty>> {
    let contract = ContractWrapper::new(
        skip_swap_valence::contract::execute,
        skip_swap_valence::contract::instantiate,
        skip_swap_valence::contract::query,
    );
    Box::new(contract)
}

// Sets up a test environment with the Skip Swap contract
pub fn setup_test_env() -> (App, Addr) {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    
    // Store the contract
    let skip_swap_code_id = app.store_code(store_skip_swap_contract());
    
    // Instantiate the contract
    let config = create_test_config("owner");
    let contract_addr = app
        .instantiate_contract(
            skip_swap_code_id,
            owner.clone(),
            &InstantiateMsg { config },
            &[],
            "Skip Swap Library",
            None,
        )
        .unwrap();
    
    (app, contract_addr)
}

// Creates a mock Skip entry point contract
pub fn store_mock_skip_entry_contract() -> Box<dyn Contract<cosmwasm_std::Empty>> {
    let contract = ContractWrapper::new(
        |_deps, _env, info, _msg: ExecuteMsg| -> Result<cosmwasm_std::Response, ContractError> {
            // This is a mock implementation that just returns success
            Ok(cosmwasm_std::Response::new()
                .add_attribute("action", "mock_skip_entry")
                .add_attribute("sender", info.sender.to_string()))
        },
        |_deps, _env, info, _msg: InstantiateMsg| -> Result<cosmwasm_std::Response, ContractError> {
            // Simple mock instantiate
            Ok(cosmwasm_std::Response::new()
                .add_attribute("action", "instantiate")
                .add_attribute("sender", info.sender.to_string()))
        },
        |_deps, _env, _msg: QueryMsg| -> Result<cosmwasm_std::Binary, ContractError> {
            // Mock query response
            Ok(cosmwasm_std::Binary::default())
        },
    );
    Box::new(contract)
} 