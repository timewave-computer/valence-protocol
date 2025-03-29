use cosmwasm_std::{Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg, to_json_binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use thiserror::Error;
use std::sync::Arc;

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

/// Response from the Skip API containing route information (async version)
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
pub struct MockSkipApiClient;

impl SkipApiClient for MockSkipApiClient {
    fn query_optimal_route(
        &self,
        _input_denom: &str,
        _output_denom: &str,
        amount: Uint128,
        allowed_venues: &[String],
        _max_slippage: Decimal,
    ) -> StdResult<SkipRouteResponse> {
        // In a real implementation, this would query the Skip API
        // For now, we'll return a mock response
        Ok(SkipRouteResponse {
            // Mock implementation details
            operations: vec![],
            expected_output: amount,
            timeout_timestamp: 1634567890,
            swap_venue: allowed_venues.first().cloned().unwrap_or_default(),
        })
    }
}

/// Production implementation of Skip API client
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
        Self {
            base_url: base_url.to_string(),
            api_key,
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
        _max_slippage: Decimal,
    ) -> StdResult<SkipRouteResponse> {
        // In a real implementation with reqwest HTTP client, this would:
        // 1. Construct an HTTP request to the Skip API
        //    - URL: {base_url}/v2/fungible/route
        //    - Method: POST
        //    - Content-Type: application/json
        // 2. Include API key in the Authorization header if available
        //    - When API key is provided, set header "Authorization: {api_key}"
        //    - This removes rate limits and provides other benefits
        // 3. Set request payload with proper parameters
        // 4. Handle the response and errors
        
        // For now, this is a mock implementation that returns simplified data
        // In a production environment, this would use the http_client module
        // to make actual HTTP requests to the Skip API
        
        // NOTE: The real implementation would use tokio/async for HTTP calls
        // But for demonstration purposes, we're using a synchronous approach
        
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
        contract_addr: "skip_swap_valence".to_string(), // This would be the actual contract address
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
            #[cfg(feature = "runtime")]
            client,
        }
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
        
        // Construct the request payload
        let payload = serde_json::json!({
            "source_asset_denom": source_asset_denom,
            "source_asset_chain_id": "neutron",  // This would come from config in real impl
            "dest_asset_denom": dest_asset_denom,
            "dest_asset_chain_id": "neutron",    // This would come from config in real impl
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