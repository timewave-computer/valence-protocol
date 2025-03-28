use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use cosmwasm_std::Decimal;

/// Route parameters response from the Skip Swap library
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteParameters {
    /// Permitted asset pairs that can be swapped
    pub allowed_asset_pairs: Vec<AssetPair>,
    
    /// Permitted venues (DEXes) to use for swaps
    pub allowed_venues: Vec<String>,
    
    /// Maximum slippage allowed
    pub max_slippage: Decimal,
    
    /// Destination accounts for specific token transfers
    pub token_destinations: HashMap<String, String>,
    
    /// Intermediate accounts for multi-hop routes
    pub intermediate_accounts: HashMap<String, String>,
}

/// Asset pair definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetPair {
    pub input_asset: String,
    pub output_asset: String,
} 