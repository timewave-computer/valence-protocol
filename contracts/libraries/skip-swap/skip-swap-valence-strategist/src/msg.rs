use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::skip::SwapOperation;
use crate::types::{AssetPair, RouteParameters};

/// Query messages for the Skip Swap library
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the current configuration
    GetConfig {},
    
    /// Get route parameters for a specific token
    GetRouteParameters {
        input_denom: String,
        input_amount: Uint128,
    },
    
    /// Simulate a swap to get the expected output
    SimulateSwap {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
    },
}

/// Execute messages for the Skip Swap library
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Execute a swap with default parameters
    Swap {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
        min_output_amount: Uint128,
    },
    
    /// Execute a swap with specific parameters
    SwapWithParams {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
        min_output_amount: Uint128,
        max_slippage: String,
        output_address: Option<String>,
    },
    
    /// Execute an optimized route provided by the strategist
    ExecuteOptimizedRoute {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
        min_output_amount: Uint128,
        operations: Vec<SwapOperation>,
        timeout_timestamp: u64,
        swap_venue: String,
    },
}

/// Response for the route parameters query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteParametersResponse {
    pub parameters: RouteParameters,
}

/// Response for the configuration query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub skip_entry_point: String,
    pub strategist_address: String,
    pub allowed_asset_pairs: Vec<AssetPair>,
    pub allowed_venues: Vec<String>,
    pub max_slippage: String,
}

/// Response for the simulate swap query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SimulateSwapResponse {
    pub expected_output: Uint128,
    pub route_description: String,
} 