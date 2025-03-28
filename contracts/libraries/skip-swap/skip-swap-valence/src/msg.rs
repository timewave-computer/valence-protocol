use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{AssetPair, Config, SkipRouteResponse};

/// Message type for instantiation
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct InstantiateMsg {
    pub config: Config,
}

/// Message type for queries
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    GetRouteParameters { token: String },
    SimulateSwap {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
    },
}

/// Message type for execute functions
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Swap {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
    },
    SwapWithParams {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
        max_slippage: Option<String>,
        output_address: Option<String>,
    },
    ExecuteOptimizedRoute {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
        min_output_amount: Uint128,
        route: SkipRouteResponse,
        timeout_timestamp: Option<u64>,
        swap_venue: Option<String>,
    },
    UpdateConfig {
        config: Config,
    },
    /// Creates a skip swap authorization in the Valence authorization contract
    CreateSkipSwapAuthorization {},
}

/// Response type for route parameters
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct RouteParametersResponse {
    pub allowed_asset_pairs: Vec<AssetPair>,
    pub allowed_venues: Vec<String>,
    pub max_slippage: String,
    pub token_destinations: Vec<(String, String)>,
}

/// Response type for configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub skip_entry_point: String,
    pub strategist_address: String,
    pub allowed_asset_pairs: Vec<AssetPair>,
    pub allowed_venues: Vec<String>,
    pub max_slippage: String,
    pub authorization_contract: Option<String>,
    pub use_authorization_contract: bool,
    pub swap_authorization_label: String,
}

/// Response type for simulate swap
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SimulateSwapResponse {
    pub expected_output: Uint128,
    pub route_description: String,
} 