/*
 * Skip Protocol API integration for the Valence strategist.
 * Provides clients and utilities for interacting with the Skip Protocol API,
 * including route optimization, swap simulation, and message construction.
 * Supports both synchronous and asynchronous operations.
 */
use cosmwasm_std::{Coin, CosmosMsg, Decimal, StdError, StdResult, Uint128, WasmMsg, to_json_binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use thiserror::Error;
use std::{sync::Arc, str::FromStr};

/// Skip Swap library ExecuteMsg
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
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

/// Skip API client interface for interacting with the Skip Protocol API
pub trait SkipApiClient {
    /// Query the Skip API for the optimal route
    fn query_optimal_route(
        &self,
        input_denom: &str,
        output_denom: &str,
        amount: Uint128,
        allowed_venues: &[String],
        max_slippage: Decimal,
    ) -> StdResult<SkipRouteResponse>;
}

/// Errors that can occur when interacting with the Skip API
#[derive(Error, Debug)]
pub enum SkipApiError {
    #[error("HTTP error: {0}")]
    HttpError(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("API error: {0}")]
    ApiError(String),
}

/// Trait defining the async interface for interacting with the Skip API
#[async_trait]
pub trait SkipAsync: Send + Sync {
    /// Get the optimal route for a swap between two assets
    async fn get_optimal_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponseAsync, SkipApiError>;
}

/// Represents a swap operation in a route (async version)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapOperationAsync {
    /// Chain ID where the swap occurs
    pub chain_id: String,
    
    /// Type of operation (e.g., "swap")
    pub operation_type: String,
    
    /// Venue for the swap (e.g., "astroport")
    pub swap_venue: Option<String>,
    
    /// Details for the swap operation
    pub swap_details: Option<SwapDetails>,
    
    /// Details for any transfer operation
    pub transfer_details: Option<TransferDetails>,
}

/// Details specific to a swap operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapDetails {
    /// Input token denom
    pub input_denom: String,
    
    /// Output token denom
    pub output_denom: String,
    
    /// Pool ID for the swap
    pub pool_id: Option<String>,
}

/// Details specific to a transfer operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransferDetails {
    /// Source chain ID
    pub source_chain_id: String,
    
    /// Destination chain ID
    pub dest_chain_id: String,
    
    /// Port on the source chain
    pub source_port: String,
    
    /// Channel on the source chain
    pub source_channel: String,
}

/// Response from the Skip API containing route information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkipRouteResponseAsync {
    /// Chain ID of the source token
    pub source_chain_id: String,
    
    /// Denom of the source token
    pub source_asset_denom: String,
    
    /// Chain ID of the destination token
    pub dest_chain_id: String,
    
    /// Denom of the destination token
    pub dest_asset_denom: String,
    
    /// Amount to swap
    pub amount: Uint128,
    
    /// Operations to perform in sequence
    pub operations: Vec<SwapOperationAsync>,
    
    /// Expected output amount
    pub expected_output: Uint128,
    
    /// Slippage tolerance as a percentage
    pub slippage_tolerance_percent: Decimal,
}

/// Mock Skip API client implementation for testing
pub struct MockSkipApiClient {
    /// Custom operations to return
    operations: Option<Vec<SwapOperation>>,
    /// Custom expected output to return
    expected_output: Option<Uint128>,
    /// Custom timeout timestamp
    timeout_timestamp: Option<u64>,
    /// Custom swap venue to return
    swap_venue: Option<String>,
}

impl MockSkipApiClient {
    /// Create a new mock client with default settings
    pub fn new() -> Self {
        Self {
            operations: None,
            expected_output: None,
            timeout_timestamp: None,
            swap_venue: None,
        }
    }
    
    /// Create a new mock client with predefined operations
    pub fn with_operations(operations: Vec<SwapOperation>) -> Self {
        Self {
            operations: Some(operations),
            expected_output: None,
            timeout_timestamp: None,
            swap_venue: None,
        }
    }
    
    /// Create a new mock client with a predefined expected output
    pub fn with_expected_output(expected_output: Uint128) -> Self {
        Self {
            operations: None,
            expected_output: Some(expected_output),
            timeout_timestamp: None,
            swap_venue: None,
        }
    }
    
    /// Create a new mock client with a predefined swap venue
    pub fn with_swap_venue(swap_venue: String) -> Self {
        Self {
            operations: None,
            expected_output: None,
            timeout_timestamp: None,
            swap_venue: Some(swap_venue),
        }
    }
    
    /// Builder method to set operations
    pub fn operations(mut self, operations: Vec<SwapOperation>) -> Self {
        self.operations = Some(operations);
        self
    }
    
    /// Builder method to set expected output
    pub fn expected_output(mut self, expected_output: Uint128) -> Self {
        self.expected_output = Some(expected_output);
        self
    }
    
    /// Builder method to set timeout timestamp
    pub fn timeout_timestamp(mut self, timeout_timestamp: u64) -> Self {
        self.timeout_timestamp = Some(timeout_timestamp);
        self
    }
    
    /// Builder method to set swap venue
    pub fn swap_venue(mut self, swap_venue: String) -> Self {
        self.swap_venue = Some(swap_venue);
        self
    }
}

impl SkipApiClient for MockSkipApiClient {
    fn query_optimal_route(
        &self,
        input_denom: &str,
        output_denom: &str,
        amount: Uint128,
        allowed_venues: &[String],
        _max_slippage: Decimal,
    ) -> StdResult<SkipRouteResponse> {
        // Use custom operations if provided, otherwise create a reasonable default
        let operations = match &self.operations {
            Some(ops) => ops.clone(),
            None => vec![SwapOperation {
                pool_id: "mock-pool-1".to_string(),
                denom_in: input_denom.to_string(),
                denom_out: output_denom.to_string(),
            }],
        };
        
        // Use custom expected output if provided, otherwise use a reasonable default
        let expected_output = self.expected_output.unwrap_or_else(|| {
            // Default behavior: apply a 1% slippage to the input amount
            amount.multiply_ratio(99u128, 100u128)
        });
        
        // Use custom timeout timestamp if provided, otherwise use current time + 5 minutes
        let timeout_timestamp = self.timeout_timestamp.unwrap_or_else(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            now + 300 // 5 minutes
        });
        
        // Use custom swap venue if provided, otherwise use the first allowed venue or a default
        let swap_venue = self.swap_venue.clone().unwrap_or_else(|| {
            allowed_venues.first().cloned().unwrap_or_else(|| "astroport".to_string())
        });
        
        Ok(SkipRouteResponse {
            operations,
            expected_output,
            timeout_timestamp,
            swap_venue,
        })
    }
}

/// Implementation of Skip API client
pub struct SkipApi {
    /// Base URL for the Skip API
    pub base_url: String,
    /// Optional API key for authentication
    /// When provided, this will be sent in the Authorization header
    /// for requests to the Skip API, which provides benefits such as:
    /// - No rate limits
    /// - Improved fee revenue share (20% vs 25%)
    /// - Access to premium features
    /// - Volume and revenue metrics
    pub api_key: Option<String>,
    
    /// Chain ID to use for API requests (e.g., "neutron-1")
    pub chain_id: String,
    
    /// HTTP client for making requests
    #[cfg(feature = "runtime")]
    client: reqwest::Client,
}

impl SkipApi {
    /// Create a new Skip API client
    /// 
    /// # Arguments
    /// 
    /// * `base_url` - Base URL for the Skip API (e.g., "https://api.skip.money")
    /// * `api_key` - Optional API key for authentication. When provided, this will be
    ///               sent in the Authorization header for all requests to the Skip API.
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        #[cfg(feature = "runtime")]
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
            
        Self {
            base_url: base_url.to_string(),
            api_key,
            chain_id: "neutron-1".to_string(), // Default chain ID
            #[cfg(feature = "runtime")]
            client,
        }
    }
    
    /// Create a new Skip API client with a specific chain ID
    /// 
    /// # Arguments
    /// 
    /// * `base_url` - Base URL for the Skip API (e.g., "https://api.skip.money")
    /// * `api_key` - Optional API key for authentication
    /// * `chain_id` - Chain ID to use for API requests (e.g., "neutron-1")
    pub fn new_with_chain(base_url: &str, api_key: Option<String>, chain_id: &str) -> Self {
        #[cfg(feature = "runtime")]
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
            
        Self {
            base_url: base_url.to_string(),
            api_key,
            chain_id: chain_id.to_string(),
            #[cfg(feature = "runtime")]
            client,
        }
    }
}

impl SkipApiClient for SkipApi {
    fn query_optimal_route(
        &self,
        input_denom: &str,
        output_denom: &str,
        amount: Uint128,
        allowed_venues: &[String],
        max_slippage: Decimal,
    ) -> StdResult<SkipRouteResponse> {
        #[cfg(feature = "runtime")]
        {
            // Create a runtime to execute async code
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                StdError::generic_err(format!("Failed to create tokio runtime: {}", e))
            })?;
            
            // Execute the async query in the runtime
            rt.block_on(async {
                self.query_optimal_route_async(input_denom, output_denom, amount, allowed_venues, max_slippage).await
            })
        }
        
        #[cfg(not(feature = "runtime"))]
        {
            // Fallback mock implementation for environments without runtime support
        Ok(SkipRouteResponse {
            operations: vec![
                SwapOperation {
                    pool_id: "1".to_string(),
                    denom_in: input_denom.to_string(),
                    denom_out: output_denom.to_string(),
                },
            ],
            expected_output: amount,
            timeout_timestamp: 1634567890,
            swap_venue: allowed_venues.first().cloned().unwrap_or_default(),
        })
        }
    }
}

#[cfg(feature = "runtime")]
impl SkipApi {
    /// Async implementation of query_optimal_route
    async fn query_optimal_route_async(
        &self,
        input_denom: &str,
        output_denom: &str,
        amount: Uint128,
        allowed_venues: &[String],
        max_slippage: Decimal,
    ) -> StdResult<SkipRouteResponse> {
        // Construct the API URL for the route endpoint
        let route_url = format!("{}/v2/fungible/route", self.base_url);
        
        // Build the request payload
        let payload = serde_json::json!({
            "source_chain_id": self.chain_id,
            "destination_chain_id": self.chain_id,
            "source_asset_denom": input_denom,
            "destination_asset_denom": output_denom,
            "amount": amount.to_string(),
            "slippage_tolerance_percent": max_slippage.to_string(),
            "allowed_swap_venues": allowed_venues,
            "allowed_bridges": []
        });
        
        // Create a request builder with the appropriate headers
        let mut request_builder = self.client.post(&route_url)
            .header("Content-Type", "application/json");
            
        // Add API key header if available
        if let Some(api_key) = &self.api_key {
            request_builder = request_builder.header("Authorization", api_key);
        }
        
        // Send the request with the payload
        let response = request_builder
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                StdError::generic_err(format!("HTTP error querying Skip API: {}", e))
            })?;
        
        // Check for rate limiting (HTTP 429)
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(StdError::generic_err("Rate limit exceeded for Skip API. Consider using an API key."));
        }
        
        // Check for any other HTTP errors
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(StdError::generic_err(format!("Error from Skip API: {}", error_text)));
        }
        
        // Parse the successful response
        let api_response: serde_json::Value = response.json().await.map_err(|e| {
            StdError::generic_err(format!("Error parsing Skip API response: {}", e))
        })?;
        
        // Extract the required data from the response
        let operations = extract_operations_from_response(&api_response, input_denom, output_denom)?;
        
        // Get the expected output and calculate a timeout timestamp (now + 5 minutes)
        let expected_output_str = api_response["amount_out"]
            .as_str()
            .ok_or_else(|| StdError::generic_err("Missing expected output in Skip API response"))?;
            
        let parsed_value = u128::from_str(expected_output_str).map_err(|_| {
            StdError::generic_err(format!("Invalid expected output format: {}", expected_output_str))
        })?;
        let expected_output = Uint128::from(parsed_value);
        
        // Calculate a timeout timestamp (now + 5 minutes)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let timeout_timestamp = current_time + 300; // 5 minutes
        
        // Determine the swap venue from operations or use the first allowed venue
        let swap_venue = operations
            .first()
            .and_then(|_op| api_response["route"]["compound_operations"][0]["swap_venue"].as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| allowed_venues.first().cloned().unwrap_or_default());
        
        // Construct the SkipRouteResponse
        Ok(SkipRouteResponse {
            operations,
            expected_output,
            timeout_timestamp,
            swap_venue,
        })
    }
}

#[cfg(feature = "runtime")]
fn extract_operations_from_response(
    response: &serde_json::Value,
    input_denom: &str,
    output_denom: &str,
) -> StdResult<Vec<SwapOperation>> {
    // Extract the operations from the API response
    let mut operations = Vec::new();
    
    // Check if we have compound operations in the response
    if let Some(compound_ops) = response["route"]["compound_operations"].as_array() {
        for (i, op) in compound_ops.iter().enumerate() {
            // Extract the pool ID from the operation
            let pool_id = if let Some(pool_id) = op["pool_id"].as_str() {
                pool_id.to_string()
            } else if let Some(pool_id) = op["pool_ids"].as_array().and_then(|ids| ids.first().and_then(|id| id.as_str())) {
                // Some Skip responses use "pool_ids" array instead of "pool_id"
                pool_id.to_string()
            } else if let Some(pool_id) = op["swap_operations"].as_array().and_then(|ops| 
                ops.first().and_then(|swap_op| swap_op["pool_id"].as_str())) {
                // Handle nested swap_operations structure
                pool_id.to_string()
            } else {
                // If no pool ID found, use a unique identifiable ID based on the operation details
                format!("{}-{}-{}", 
                    op["swap_venue"].as_str().unwrap_or("unknown-venue"),
                    op["denom_in"].as_str().unwrap_or(input_denom),
                    op["denom_out"].as_str().unwrap_or(output_denom)
                )
            };
                
            // For the first operation, use the input_denom
            let denom_in = if i == 0 {
                input_denom.to_string()
            } else {
                // For subsequent operations, use the output of the previous operation
                op["denom_in"]
                    .as_str()
                    .unwrap_or(&format!("intermediate-{}", i))
                    .to_string()
            };
            
            // For the last operation, use the output_denom
            let denom_out = if i == compound_ops.len() - 1 {
                output_denom.to_string()
            } else {
                // For intermediate operations, use the specified output
                op["denom_out"]
                    .as_str()
                    .unwrap_or(&format!("intermediate-{}", i + 1))
                    .to_string()
            };
            
            operations.push(SwapOperation {
                pool_id,
                denom_in,
                denom_out,
            });
        }
    } else if let Some(operations_array) = response["route"]["operations"].as_array() {
        // Handle the "operations" array format that some Skip API versions use
        for op in operations_array {
            if op["operation_type"].as_str() == Some("swap") {
                if let Some(swap_details) = op["swap_details"].as_object() {
                    let pool_id = if let Some(pool) = swap_details.get("pool_id").and_then(|p| p.as_str()) {
                        pool.to_string()
                    } else {
                        // Generate a pool ID based on the input/output denoms if not present
                        let input = swap_details.get("input_denom").and_then(|d| d.as_str()).unwrap_or(input_denom);
                        let output = swap_details.get("output_denom").and_then(|d| d.as_str()).unwrap_or(output_denom);
                        format!("pool-{}-{}", input, output)
                    };
                    
                    let denom_in = swap_details.get("input_denom")
                        .and_then(|d| d.as_str())
                        .unwrap_or(input_denom)
                        .to_string();
                        
                    let denom_out = swap_details.get("output_denom")
                        .and_then(|d| d.as_str())
                        .unwrap_or(output_denom)
                        .to_string();
                        
                    operations.push(SwapOperation {
                        pool_id,
                        denom_in,
                        denom_out,
                    });
                }
            }
        }
    } else {
        // If no operations found in the response, create a single direct swap operation
        // using the venue information if available
        let swap_venue = response["route"]["swap_venue"].as_str().unwrap_or("unknown");
        operations.push(SwapOperation {
            pool_id: format!("{}-direct-swap", swap_venue),
            denom_in: input_denom.to_string(),
            denom_out: output_denom.to_string(),
        });
    }
    
    if operations.is_empty() {
        return Err(StdError::generic_err("No valid operations found in Skip API response"));
    }
    
    Ok(operations)
}

/// Skip route response as returned by the Skip API
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SkipRouteResponse {
    pub operations: Vec<SwapOperation>,
    pub expected_output: Uint128,
    pub timeout_timestamp: u64,
    pub swap_venue: String,
}

/// Swap operation within a Skip route
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapOperation {
    pub pool_id: String,
    pub denom_in: String,
    pub denom_out: String,
}

/// Asset information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    /// Native token
    Native { denom: String },
    /// CW20 token
    Token { address: String },
}

/// Asset with amount
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

/// A swap definition for Skip
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Swap {
    /// Name of the DEX to use (e.g., "astroport")
    pub swap_venue_name: String,
    /// Series of swap operations to execute
    pub operations: Vec<SwapOperation>,
}

/// A post-swap action for Skip
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Send tokens to a specific address
    Transfer { to_address: String },
}

/// The swap and action execute message for Skip's entry point
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkipExecuteMsg {
    /// Execute a swap and follow with a post-swap action
    SwapAndAction {
        /// Asset being sent to Skip for swapping
        sent_asset: Option<Asset>,
        /// Definition of the swap to perform
        user_swap: Swap,
        /// Minimum asset expected to receive after swap (for slippage protection)
        min_asset: Asset,
        /// Timestamp after which the swap should be considered expired
        timeout_timestamp: u64,
        /// Action to perform after the swap
        post_swap_action: Action,
        /// Any affiliate fees to apply
        affiliates: Vec<Affiliate>,
    },
}

/// Affiliate fee definition for Skip
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Affiliate {
    /// Fee amount in basis points (1/100 of a percent)
    pub basis_points_fee: Uint128,
    /// Address to receive the fee
    pub address: String,
}

/// Create a swap response that includes details about the executed route
/// for verification in tests
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RouteExecutionResponse {
    /// The input token denom
    pub input_denom: String,
    /// The output token denom
    pub output_denom: String,
    /// The DEX used for the swap
    pub swap_venue: String,
    /// The pool ID used for the swap
    pub pool_id: String,
    /// The address receiving the output tokens
    pub output_address: String,
    /// The amount of input tokens swapped
    pub input_amount: Uint128,
    /// The minimum amount of output tokens expected
    pub min_output_amount: Uint128,
}

/// Route request parameters for Skip API
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteRequest {
    /// Source chain ID (e.g., "neutron-1")
    pub source_chain_id: String,
    /// Destination chain ID (can be same as source for same-chain swaps)
    pub destination_chain_id: String,
    /// Source asset denomination
    pub source_asset_denom: String,
    /// Destination asset denomination
    pub destination_asset_denom: String,
    /// Amount of source asset to swap
    pub amount: String,
    /// Address to receive the swapped tokens
    pub destination_address: String,
    /// Slippage tolerance in percentage points (e.g., "0.5" for 0.5%)
    pub slippage_tolerance_percent: String,
    /// Allowed bridges for cross-chain operations
    pub allowed_bridges: Option<Vec<String>>,
    /// Allowed DEXes for swaps
    pub allowed_swap_venues: Option<Vec<String>>,
}

/// Create a Skip Swap execute message for executing an optimized route
pub fn create_execute_optimized_route_msg(
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
    min_output_amount: Uint128,
    route: SkipRouteResponse,
    contract_address: String,
) -> StdResult<CosmosMsg> {
    let msg = ExecuteMsg::ExecuteOptimizedRoute {
        input_denom: input_denom.clone(),
        input_amount,
        output_denom,
        min_output_amount,
        operations: route.operations,
        timeout_timestamp: route.timeout_timestamp,
        swap_venue: route.swap_venue,
    };
    
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_address,
        msg: to_json_binary(&msg)?,
        funds: vec![Coin {
            denom: input_denom,
            amount: input_amount,
        }],
    }))
}

/// Implementation of the Async Skip API client
pub struct SkipApiClientAsync {
    /// Base URL for the Skip API
    base_url: String,
    
    /// API key for authentication
    api_key: Option<String>,
    
    /// Chain ID for source assets (e.g., "neutron-1")
    chain_id: String,
    
    /// HTTP client for making requests
    #[cfg(feature = "runtime")]
    client: reqwest::Client,
}

impl SkipApiClientAsync {
    /// Create a new Async Skip API client
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        #[cfg(feature = "runtime")]
        let client = reqwest::Client::new();
        
        Self {
            base_url,
            api_key,
            chain_id: "neutron-1".to_string(), // Default chain ID
            #[cfg(feature = "runtime")]
            client,
        }
    }
    
    /// Create a new Async Skip API client with a specific chain ID
    pub fn new_with_chain(base_url: String, api_key: Option<String>, chain_id: String) -> Self {
        #[cfg(feature = "runtime")]
        let client = reqwest::Client::new();
        
        Self {
            base_url,
            api_key,
            chain_id,
            #[cfg(feature = "runtime")]
            client,
        }
    }
    
    /// Get the chain ID for this client
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }
    
    /// Set the chain ID for this client
    pub fn set_chain_id(&mut self, chain_id: String) {
        self.chain_id = chain_id;
    }
}

#[cfg(feature = "runtime")]
#[async_trait]
impl SkipAsync for SkipApiClientAsync {
    async fn get_optimal_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponseAsync, SkipApiError> {
        // Build the API URL
        let url = format!("{}/v1/router/routes", self.base_url);
        
        // Construct the request payload using chain ID from configuration
        let payload = serde_json::json!({
            "source_asset_denom": source_asset_denom,
            "source_asset_chain_id": self.chain_id,
            "dest_asset_denom": dest_asset_denom,
            "dest_asset_chain_id": self.chain_id,
            "amount": amount.to_string(),
            "slippage_tolerance_percent": slippage_tolerance.to_string(),
        });
        
        // Build the request with optional API key
        let mut request_builder = self.client.post(&url).json(&payload);
            if let Some(api_key) = &self.api_key {
            request_builder = request_builder.header("x-skip-api-key", api_key);
            }
            
        // Execute the request
        let response = request_builder
                .send()
                .await
            .map_err(|e| SkipApiError::HttpError(e.to_string()))?;
            
        // Check for rate limiting
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(SkipApiError::RateLimitExceeded);
            }
            
        // Check for other errors
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SkipApiError::ApiError(error_text));
        }
        
        // Parse the response
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SkipApiError::InvalidResponse(e.to_string()))?;
        
        // Extract the route from the response
        let route: SkipRouteResponseAsync = serde_json::from_value(response_json["route"].clone())
            .map_err(|e| SkipApiError::InvalidResponse(e.to_string()))?;
        
        Ok(route)
    }
}

// Non-runtime implementation that returns a mock response
#[cfg(not(feature = "runtime"))]
#[async_trait]
impl SkipAsync for SkipApiClientAsync {
    async fn get_optimal_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponseAsync, SkipApiError> {
        // Create a simple mock route
        Ok(SkipRouteResponseAsync {
            source_chain_id: "neutron".to_string(),
            source_asset_denom: source_asset_denom.to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: dest_asset_denom.to_string(),
            amount,
            operations: vec![SwapOperationAsync {
                chain_id: "neutron".to_string(),
                operation_type: "swap".to_string(),
                swap_venue: Some("astroport".to_string()),
                swap_details: Some(SwapDetails {
                    input_denom: source_asset_denom.to_string(),
                    output_denom: dest_asset_denom.to_string(),
                    pool_id: Some("pool1".to_string()),
                }),
                transfer_details: None,
            }],
            expected_output: amount.multiply_ratio(99u128, 100u128),  // 1% slippage
            slippage_tolerance_percent: slippage_tolerance,
        })
    }
}

/// Async Mock implementation of the Skip API client for testing
pub struct MockSkipApiAsync {
    /// Predefined route to return
    route: Option<Arc<SkipRouteResponseAsync>>,
}

impl MockSkipApiAsync {
    /// Create a new mock client with no predefined route
    pub fn new() -> Self {
        Self { route: None }
    }
    
    /// Create a new mock client with a predefined route
    pub fn with_route(route: Arc<SkipRouteResponseAsync>) -> Self {
        Self { route: Some(route) }
    }
}

#[async_trait]
impl SkipAsync for MockSkipApiAsync {
    async fn get_optimal_route(
            &self,
            source_asset_denom: &str,
        dest_asset_denom: &str,
            amount: Uint128,
            slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponseAsync, SkipApiError> {
        // If a predefined route is set, return it
        if let Some(route) = &self.route {
            return Ok(route.as_ref().clone());
        }
        
        // Otherwise, create a simple route
        Ok(SkipRouteResponseAsync {
            source_chain_id: "neutron".to_string(),
                source_asset_denom: source_asset_denom.to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: dest_asset_denom.to_string(),
            amount,
            operations: vec![SwapOperationAsync {
                chain_id: "neutron".to_string(),
                operation_type: "swap".to_string(),
                swap_venue: Some("astroport".to_string()),
                swap_details: Some(SwapDetails {
                    input_denom: source_asset_denom.to_string(),
                    output_denom: dest_asset_denom.to_string(),
                    pool_id: Some("pool1".to_string()),
                }),
                transfer_details: None,
            }],
            expected_output: amount.multiply_ratio(99u128, 100u128),  // 1% slippage
            slippage_tolerance_percent: slippage_tolerance,
        })
    }
}

#[cfg(feature = "runtime")]
impl From<SkipRouteResponseAsync> for SkipRouteResponse {
    fn from(async_response: SkipRouteResponseAsync) -> Self {
        // Convert operations from async to sync format
        let operations = async_response.operations.clone()
            .into_iter()
            .filter_map(|op| {
                // Only handle swap operations
                if op.operation_type == "swap" && op.swap_details.is_some() {
                    let swap_details = op.swap_details.unwrap();
                    Some(SwapOperation {
                        pool_id: swap_details.pool_id.unwrap_or_else(|| "default-pool".to_string()),
                        denom_in: swap_details.input_denom,
                        denom_out: swap_details.output_denom,
                    })
                } else {
                    None
                }
            })
            .collect();
        
        // Create a timestamp 5 minutes in the future
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let timeout_timestamp = current_time + 300; // 5 minutes
        
        // Extract swap venue from the first operation
        let swap_venue = async_response.operations
            .iter()
            .filter_map(|op| op.swap_venue.clone())
            .next()
            .unwrap_or_else(|| "default".to_string());
            
        Self {
            operations,
            expected_output: async_response.expected_output,
            timeout_timestamp,
            swap_venue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_execute_optimized_route_msg() {
        let input_denom = "uatom".to_string();
        let input_amount = Uint128::new(1000000);
        let output_denom = "uusdc".to_string();
        let min_output_amount = Uint128::new(950000);
        
        let route = SkipRouteResponse {
            operations: vec![
                SwapOperation {
                    pool_id: "1".to_string(),
                    denom_in: input_denom.clone(),
                    denom_out: output_denom.clone(),
                },
            ],
            expected_output: Uint128::new(990000),
            timeout_timestamp: 1634567890,
            swap_venue: "astroport".to_string(),
        };
        
        let msg = create_execute_optimized_route_msg(
            input_denom.clone(),
            input_amount,
            output_denom,
            min_output_amount,
            route,
            "skip_swap_valence".to_string(),
        ).unwrap();
        
        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg: _msg, funds }) => {
                assert_eq!(contract_addr, "skip_swap_valence");
            assert_eq!(funds.len(), 1);
                assert_eq!(funds[0].denom, input_denom);
                assert_eq!(funds[0].amount, input_amount);
            },
            _ => panic!("Expected Wasm Execute message"),
        }
    }
    
    #[tokio::test]
    async fn test_mock_skip_api_async() {
        let mock_client = MockSkipApiAsync::new();
        
        let result = mock_client.get_optimal_route(
            "uusdc",
            "uatom",
            Uint128::new(1000000),
            Decimal::percent(1),
        ).await;
        
        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.source_asset_denom, "uusdc");
        assert_eq!(route.dest_asset_denom, "uatom");
        assert_eq!(route.operations.len(), 1);
    }
} 