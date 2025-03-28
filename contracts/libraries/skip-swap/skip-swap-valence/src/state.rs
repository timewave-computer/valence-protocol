use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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