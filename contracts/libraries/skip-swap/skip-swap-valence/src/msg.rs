/*
 * Message definitions for the Skip Swap Valence contract.
 * Defines all message structures used for contract communication:
 * - InstantiateMsg for contract initialization
 * - ExecuteMsg for contract operations (swaps, route execution, config updates)
 * - QueryMsg for data retrieval operations
 * - Response structures for query results
 */

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
    /// Gets a list of pending simulation requests that need to be fulfilled by the strategist
    GetPendingSimulationRequests {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Gets a specific simulation request by ID
    GetSimulationRequest {
        request_id: u64,
    },
    /// Gets a simulation response by request ID
    GetSimulationResponse {
        request_id: u64,
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
    /// Request a route simulation from the strategist
    /// This creates a simulation request that the strategist can fulfill
    RequestRouteSimulation {
        input_denom: String,
        input_amount: Uint128,
        output_denom: String,
        max_slippage: Option<String>,
    },
    /// Submit a route simulation result (strategist only)
    /// This fulfills a pending simulation request with the optimized route
    SubmitRouteSimulation {
        request_id: u64,
        route: SkipRouteResponse,
    },
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

/// Response type for simulation request
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SimulationRequestResponse {
    pub request_id: u64,
    pub requester: String,
    pub input_denom: String,
    pub input_amount: Uint128,
    pub output_denom: String,
    pub max_slippage: String,
    pub timestamp: u64,
    pub fulfilled: bool,
}

/// Response type for pending simulation requests
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct PendingSimulationRequestsResponse {
    pub requests: Vec<SimulationRequestResponse>,
} 