use cosmwasm_std::{Coin, CosmosMsg, Decimal, StdError, StdResult, Uint128, WasmMsg, to_json_binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
        max_slippage: Decimal,
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
    pub allowed_routes: Option<Vec<String>>,
}

/// Route response from Skip API
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteResponse {
    /// Chain operations to perform
    pub chain_operations: Vec<ChainOperation>,
    /// Total estimated time in seconds
    pub estimated_time_seconds: u64,
    /// Total estimated fee in USD
    pub estimated_fees_usd: String,
}

/// Chain operation in a route
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChainOperation {
    /// Chain ID
    pub chain_id: String,
    /// Operations to perform
    pub operations: Vec<Operation>,
}

/// Operation type in a chain operation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(tag = "operation_type")]
pub enum Operation {
    /// Swap operation
    Swap {
        /// Swap venue name
        swap_venue: SwapVenue,
        /// Swap in asset
        swap_in: Asset,
        /// Swap operations
        swap_operations: Vec<SwapStep>,
    },
    /// Transfer operation for cross-chain
    Transfer {
        /// Port ID
        port: String,
        /// Channel ID
        channel: String,
        /// Destination chain
        destination_chain: String,
    },
}

/// Swap venue
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapVenue {
    /// Venue name
    pub name: String,
}

/// Swap step
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapStep {
    /// DEX to use
    pub dex: String,
    /// Pool ID
    pub pool_id: String,
    /// Input denomination
    pub input_denom: String,
    /// Output denomination
    pub output_denom: String,
}

/// Validate a route execution response against expected parameters
pub fn validate_route_execution(
    response: &RouteExecutionResponse,
    expected_input_denom: &str,
    expected_output_denom: &str,
    expected_swap_venue: &str,
    expected_pool_id: &str,
    expected_output_address: &str,
) -> bool {
    response.input_denom == expected_input_denom
        && response.output_denom == expected_output_denom
        && response.swap_venue == expected_swap_venue
        && response.pool_id == expected_pool_id
        && response.output_address == expected_output_address
}

/// Extract route details from a Skip execute message for validation
pub fn extract_route_details(msg: &SkipExecuteMsg) -> Option<RouteExecutionResponse> {
    match msg {
        SkipExecuteMsg::SwapAndAction {
            sent_asset,
            user_swap,
            min_asset,
            post_swap_action,
            ..
        } => {
            // Extract input denom and amount
            let (input_denom, input_amount) = if let Some(asset) = sent_asset {
                match &asset.info {
                    AssetInfo::Native { denom } => (denom.clone(), asset.amount),
                    AssetInfo::Token { address } => (address.clone(), asset.amount),
                }
            } else {
                return None;
            };

            // Extract output denom and amount
            let (output_denom, min_output_amount) = match &min_asset.info {
                AssetInfo::Native { denom } => (denom.clone(), min_asset.amount),
                AssetInfo::Token { address } => (address.clone(), min_asset.amount),
            };

            // Extract output address
            let output_address = match post_swap_action {
                Action::Transfer { to_address } => to_address.clone(),
            };

            // Extract pool ID and swap venue
            if user_swap.operations.is_empty() {
                return None;
            }
            
            let first_op = &user_swap.operations[0];
            
            Some(RouteExecutionResponse {
                input_denom,
                output_denom,
                swap_venue: user_swap.swap_venue_name.clone(),
                pool_id: first_op.pool_id.clone(),
                output_address,
                input_amount,
                min_output_amount,
            })
        }
    }
}

/// Creates a message to execute an optimized route
pub fn create_execute_optimized_route_msg(
    _input_denom: String,
    _input_amount: Uint128,
    _output_denom: String,
    _min_output_amount: Uint128,
    _route: SkipRouteResponse,
) -> StdResult<CosmosMsg> {
    // Return a generic error as this is just a placeholder
    Err(StdError::generic_err("Not implemented - requires library address in real implementation"))
}

/// Creates a static USDC to stETH route
pub fn create_static_usdc_to_steth_route(
    skip_entry_point: &str,
    output_address: &str,
    usdc_amount: Uint128,
    min_steth_amount: Uint128,
    timeout_timestamp: u64,
    astroport_pool_id: &str,
    usdc_denom: &str,
    steth_denom: &str,
) -> StdResult<CosmosMsg> {
    // Validate input parameters
    if skip_entry_point.is_empty() {
        return Err(StdError::generic_err("Skip entry point address cannot be empty"));
    }
    
    if output_address.is_empty() {
        return Err(StdError::generic_err("Output address cannot be empty"));
    }
    
    if usdc_amount.is_zero() {
        return Err(StdError::generic_err("USDC amount must be greater than zero"));
    }
    
    if min_steth_amount.is_zero() {
        return Err(StdError::generic_err("Minimum stETH amount must be greater than zero"));
    }
    
    if astroport_pool_id.is_empty() {
        return Err(StdError::generic_err("Astroport pool ID cannot be empty"));
    }
    
    if usdc_denom.is_empty() {
        return Err(StdError::generic_err("USDC denom cannot be empty"));
    }
    
    if steth_denom.is_empty() {
        return Err(StdError::generic_err("stETH denom cannot be empty"));
    }

    // Create the swap operation
    let swap_operation = SwapOperation {
        pool_id: astroport_pool_id.to_string(),
        denom_in: usdc_denom.to_string(),
        denom_out: steth_denom.to_string(),
    };

    // Create asset objects
    let sent_asset = Some(Asset {
        info: AssetInfo::Native {
            denom: usdc_denom.to_string(),
        },
        amount: usdc_amount,
    });

    let min_asset = Asset {
        info: AssetInfo::Native {
            denom: steth_denom.to_string(),
        },
        amount: min_steth_amount,
    };

    // Create the swap message
    let swap_msg = SkipExecuteMsg::SwapAndAction {
        sent_asset,
        user_swap: Swap {
            swap_venue_name: "astroport".to_string(),
            operations: vec![swap_operation],
        },
        min_asset,
        timeout_timestamp,
        post_swap_action: Action::Transfer {
            to_address: output_address.to_string(),
        },
        affiliates: vec![],
    };

    // Create the WasmMsg
    let wasm_msg = WasmMsg::Execute {
        contract_addr: skip_entry_point.to_string(),
        msg: to_json_binary(&swap_msg)
            .map_err(|e| StdError::generic_err(format!("Failed to serialize swap message: {}", e)))?,
        funds: vec![Coin {
            denom: usdc_denom.to_string(),
            amount: usdc_amount,
        }],
    };

    Ok(CosmosMsg::Wasm(wasm_msg))
}

#[cfg(feature = "runtime")]
pub mod http_client {
    use reqwest::{Client, header};
    use crate::skip::{RouteRequest, RouteResponse};
    use cosmwasm_std::{Decimal, Uint128};
    use std::time::Duration;

    pub struct SkipHttpClient {
        client: Client,
        base_url: String,
        api_key: Option<String>,
    }

    impl SkipHttpClient {
        pub fn new(base_url: &str, api_key: Option<String>) -> Self {
            let client = Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default();

            Self {
                client,
                base_url: base_url.to_string(),
                api_key,
            }
        }

        pub async fn get_chains(&self) -> Result<Vec<String>, String> {
            let url = format!("{}/v2/info/chains", self.base_url);
            
            let mut request = self.client.get(&url);
            
            // Add API key if available
            if let Some(api_key) = &self.api_key {
                request = request.header(header::AUTHORIZATION, api_key);
            }
            
            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to send request: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!("Failed to get chains: {}", response.status()));
            }
            
            response.json::<Vec<String>>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))
        }

        pub async fn get_bridges(&self) -> Result<Vec<String>, String> {
            let url = format!("{}/v2/info/bridges", self.base_url);
            
            let mut request = self.client.get(&url);
            
            // Add API key if available
            if let Some(api_key) = &self.api_key {
                request = request.header(header::AUTHORIZATION, api_key);
            }
            
            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to send request: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!("Failed to get bridges: {}", response.status()));
            }
            
            response.json::<Vec<String>>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))
        }

        pub async fn get_route(
            &self,
            source_chain_id: &str,
            destination_chain_id: &str,
            source_asset_denom: &str,
            destination_asset_denom: &str,
            amount: Uint128,
            destination_address: &str,
            slippage_tolerance: Decimal,
            allowed_routes: Option<Vec<String>>,
            allowed_bridges: Option<Vec<String>>,
        ) -> Result<RouteResponse, String> {
            let url = format!("{}/v2/fungible/route", self.base_url);
            
            let route_request = RouteRequest {
                source_chain_id: source_chain_id.to_string(),
                destination_chain_id: destination_chain_id.to_string(),
                source_asset_denom: source_asset_denom.to_string(),
                destination_asset_denom: destination_asset_denom.to_string(),
                amount: amount.to_string(),
                destination_address: destination_address.to_string(),
                slippage_tolerance_percent: slippage_tolerance.to_string(),
                allowed_bridges,
                allowed_routes,
            };
            
            let mut request = self.client.post(&url)
                .json(&route_request);
            
            // Add API key if available
            if let Some(api_key) = &self.api_key {
                request = request.header(header::AUTHORIZATION, api_key);
            }
            
            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to send request: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!("Failed to get route: {}", response.status()));
            }
            
            response.json::<RouteResponse>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_skip_api_client() {
        let client = MockSkipApiClient;
        
        let response = client.query_optimal_route(
            "uusdc",
            "steth",
            Uint128::from(1000000u128),
            &["astroport".to_string()],
            Decimal::percent(1),
        ).unwrap();
        
        assert_eq!(response.expected_output, Uint128::from(1000000u128));
        assert_eq!(response.swap_venue, "astroport");
    }
    
    #[test]
    fn test_skip_api() {
        let client = SkipApi::new(
            "https://api.skip.money",
            Some("api-key".to_string()),
        );
        
        assert_eq!(client.base_url, "https://api.skip.money");
        assert_eq!(client.api_key, Some("api-key".to_string()));
    }
    
    #[test]
    fn test_create_route() {
        // Test parameters
        let skip_entry_point = "neutron1qd7prxvuhdq0p3rjk9xuwz2ph5fvwddqmfsj25qgmw0p2z3g5tscyqccnz";
        let output_address = "neutron1user";
        let usdc_amount = Uint128::from(1000000u128); // 1 USDC
        let min_steth_amount = Uint128::from(500000u128); // 0.5 stETH
        let timeout_timestamp = 1634567890u64;
        let astroport_pool_id = "neutron1astropool";
        let usdc_denom = "ibc/uusdc";
        let steth_denom = "ibc/steth";
        
        // Create the route
        let route_msg = create_static_usdc_to_steth_route(
            skip_entry_point,
            output_address,
            usdc_amount,
            min_steth_amount,
            timeout_timestamp,
            astroport_pool_id,
            usdc_denom,
            steth_denom,
        ).unwrap();
        
        // Extract the message
        if let CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg, funds }) = route_msg {
            // Verify contract address
            assert_eq!(contract_addr, skip_entry_point);
            
            // Verify funds
            assert_eq!(funds.len(), 1);
            assert_eq!(funds[0].denom, usdc_denom);
            assert_eq!(funds[0].amount, usdc_amount);
            
            // Parse the message
            let swap_msg: SkipExecuteMsg = serde_json_wasm::from_slice(&msg.0).unwrap();
            
            // Extract route details
            let route_details = extract_route_details(&swap_msg).unwrap();
            
            // Validate route details
            assert!(validate_route_execution(
                &route_details,
                usdc_denom,
                steth_denom,
                "astroport",
                astroport_pool_id,
                output_address,
            ));
            
            // Additional validations
            assert_eq!(route_details.input_amount, usdc_amount);
            assert_eq!(route_details.min_output_amount, min_steth_amount);
        } else {
            panic!("Expected WasmMsg::Execute");
        }
    }
    
    #[test]
    fn test_validation_errors() {
        // Valid parameters
        let skip_entry_point = "neutron1qd7prxvuhdq0p3rjk9xuwz2ph5fvwddqmfsj25qgmw0p2z3g5tscyqccnz";
        let output_address = "neutron1user";
        let usdc_amount = Uint128::from(1000000u128);
        let min_steth_amount = Uint128::from(500000u128);
        let timeout_timestamp = 1634567890u64;
        let astroport_pool_id = "neutron1astropool";
        let usdc_denom = "ibc/uusdc";
        let steth_denom = "ibc/steth";
        
        // Test empty Skip entry point
        let result = create_static_usdc_to_steth_route(
            "",
            output_address,
            usdc_amount,
            min_steth_amount,
            timeout_timestamp,
            astroport_pool_id,
            usdc_denom,
            steth_denom,
        );
        assert!(result.is_err());
        
        // Test zero USDC amount
        let result = create_static_usdc_to_steth_route(
            skip_entry_point,
            output_address,
            Uint128::zero(),
            min_steth_amount,
            timeout_timestamp,
            astroport_pool_id,
            usdc_denom,
            steth_denom,
        );
        assert!(result.is_err());
        
        // Test empty pool ID
        let result = create_static_usdc_to_steth_route(
            skip_entry_point,
            output_address,
            usdc_amount,
            min_steth_amount,
            timeout_timestamp,
            "",
            usdc_denom,
            steth_denom,
        );
        assert!(result.is_err());
    }
} 