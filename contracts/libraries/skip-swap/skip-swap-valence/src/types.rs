use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the Skip Swap library
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Address of the contract owner (deployer)
    pub owner: Addr,
    
    /// Address authorized to query Skip API and submit routes
    pub strategist_address: Addr,
    
    /// Skip entry point contract address
    pub skip_entry_point: Addr,
    
    /// Permitted asset pairs that can be swapped
    pub allowed_asset_pairs: Vec<AssetPair>,
    
    /// Permitted venues (DEXes) to use for swaps
    pub allowed_venues: Vec<String>,
    
    /// Maximum slippage allowed (in percentage points)
    pub max_slippage: Decimal,
    
    /// Destination accounts for specific token transfers
    pub token_destinations: HashMap<String, Addr>,
    
    /// Intermediate accounts for multi-hop routes
    pub intermediate_accounts: HashMap<String, Addr>,
    
    /// Valence authorization contract address
    pub authorization_contract: Option<Addr>,
    
    /// Whether to use the authorization contract for validation
    /// If false, the contract will use local validation
    pub use_authorization_contract: bool,
    
    /// The authorization label for swap operations
    pub swap_authorization_label: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            owner: Addr::unchecked(""),
            strategist_address: Addr::unchecked(""),
            skip_entry_point: Addr::unchecked(""),
            allowed_asset_pairs: vec![],
            allowed_venues: vec![],
            max_slippage: Decimal::percent(1),
            token_destinations: HashMap::new(),
            intermediate_accounts: HashMap::new(),
            authorization_contract: None,
            use_authorization_contract: false,
            swap_authorization_label: "skip_swap".to_string(),
        }
    }
}

/// Represents a pair of assets that can be swapped
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetPair {
    pub input_asset: String,
    pub output_asset: String,
}

/// Response from the Skip API containing route information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SkipRouteResponse {
    /// Chain ID of the source chain
    pub source_chain_id: String,
    
    /// Asset denomination on the source chain
    pub source_asset_denom: String,
    
    /// Chain ID of the destination chain
    pub dest_chain_id: String,
    
    /// Asset denomination on the destination chain
    pub dest_asset_denom: String,
    
    /// Amount to be swapped
    pub amount: Uint128,
    
    /// Operations to perform for the swap
    pub operations: Vec<SwapOperation>,
    
    /// Expected output amount
    pub expected_output: Uint128,
    
    /// Slippage tolerance in percentage
    pub slippage_tolerance_percent: Decimal,
}

/// Operation to perform in the swap execution
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapOperation {
    /// Chain ID where the operation is performed
    pub chain_id: String,
    
    /// Type of operation (swap, transfer, etc.)
    pub operation_type: String,
    
    /// The venue (DEX) to use for the operation
    pub swap_venue: Option<String>,
    
    /// Specific swap details if this is a swap operation
    pub swap_details: Option<SwapDetails>,
    
    /// Transfer details if this is a transfer operation
    pub transfer_details: Option<TransferDetails>,
}

/// Details for a swap operation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapDetails {
    /// Input asset denomination
    pub input_denom: String,
    
    /// Output asset denomination
    pub output_denom: String,
    
    /// Pool ID on the specific DEX
    pub pool_id: Option<String>,
}

/// Details for a transfer operation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferDetails {
    /// Source address for transfer
    pub source_address: Option<String>,
    
    /// Destination address for transfer
    pub dest_address: Option<String>,
    
    /// The asset being transferred
    pub asset_denom: String,
} 