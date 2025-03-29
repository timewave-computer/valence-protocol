/*
 * State management for Skip Swap Valence contract.
 * Defines the storage schema and access patterns for contract state,
 * including configuration and routing information.
 */

use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, Uint128};

use crate::types::Config;

// Storage key for the configuration
pub const CONFIG: Item<Config> = Item::new("config");

// Storage key for the current route count
pub const ROUTE_COUNT: Item<u64> = Item::new("route_count");

// Structure to track active routes
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteInfo {
    pub route_id: u64,
    pub input_denom: String,
    pub output_denom: String,
    pub input_amount: String,
    pub expected_output: String,
    pub timestamp: u64,
    pub completed: bool,
}

// Structure to track route simulation requests
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteSimulationRequest {
    pub request_id: u64,
    pub requester: Addr,
    pub input_denom: String,
    pub input_amount: Uint128,
    pub output_denom: String,
    pub max_slippage: String,
    pub timestamp: u64,
    pub fulfilled: bool,
}

// Storage for simulation requests, indexed by request_id
pub const SIMULATION_REQUESTS: Map<u64, RouteSimulationRequest> = Map::new("simulation_requests");

// Storage for the current simulation request count
pub const SIMULATION_REQUEST_COUNT: Item<u64> = Item::new("simulation_request_count");

// Storage for simulation responses, indexed by request_id
pub const SIMULATION_RESPONSES: Map<u64, crate::types::SkipRouteResponse> = Map::new("simulation_responses"); 