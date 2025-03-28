use cosmwasm_std::{Decimal, Uint128};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::{sync::Arc, collections::HashMap};

/// Represents a swap operation in a route
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapOperation {
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
pub struct SkipRouteResponse {
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
    pub operations: Vec<SwapOperation>,
    
    /// Expected output amount
    pub expected_output: Uint128,
    
    /// Slippage tolerance as a percentage
    pub slippage_tolerance_percent: Decimal,
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

/// Trait defining the interface for interacting with the Skip API
#[async_trait]
pub trait SkipApi: Send + Sync {
    /// Get the optimal route for a swap between two assets
    async fn get_optimal_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponse, SkipApiError>;
}

/// Implementation of the Skip API client
pub struct SkipApiClient {
    /// Base URL for the Skip API
    base_url: String,
    
    /// API key for authentication
    api_key: Option<String>,
    
    /// HTTP client for making requests
    client: reqwest::Client,
}

impl SkipApiClient {
    /// Create a new Skip API client
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SkipApi for SkipApiClient {
    async fn get_optimal_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponse, SkipApiError> {
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
        let route: SkipRouteResponse = serde_json::from_value(response_json["route"].clone())
            .map_err(|e| SkipApiError::InvalidResponse(e.to_string()))?;
        
        Ok(route)
    }
}

/// Mock implementation of the Skip API client for testing
pub struct MockSkipApiClient {
    /// Predefined route to return
    route: Option<Arc<SkipRouteResponse>>,
}

impl MockSkipApiClient {
    /// Create a new mock client with no predefined route
    pub fn new() -> Self {
        Self { route: None }
    }
    
    /// Create a new mock client with a predefined route
    pub fn with_route(route: Arc<SkipRouteResponse>) -> Self {
        Self { route: Some(route) }
    }
}

#[async_trait]
impl SkipApi for MockSkipApiClient {
    async fn get_optimal_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponse, SkipApiError> {
        // If a predefined route is set, return it
        if let Some(route) = &self.route {
            return Ok(route.as_ref().clone());
        }
        
        // Otherwise, create a simple route
        Ok(SkipRouteResponse {
            source_chain_id: "neutron".to_string(),
            source_asset_denom: source_asset_denom.to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: dest_asset_denom.to_string(),
            amount,
            operations: vec![SwapOperation {
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
            expected_output: amount.mul_ceil(Decimal::percent(99)),  // 1% slippage
            slippage_tolerance_percent: slippage_tolerance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_client() {
        let client = MockSkipApiClient::new();
        
        let result = client.get_optimal_route(
            "uusdc",
            "uatom",
            Uint128::new(1000000),
            Decimal::percent(1),
        ).await;
        
        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.source_asset_denom, "uusdc");
        assert_eq!(route.dest_asset_denom, "uatom");
        assert_eq!(route.amount, Uint128::new(1000000));
        assert_eq!(route.operations.len(), 1);
        assert_eq!(route.operations[0].operation_type, "swap");
        assert_eq!(route.operations[0].swap_venue, Some("astroport".to_string()));
    }
    
    #[tokio::test]
    async fn test_mock_client_with_predefined_route() {
        let predefined_route = Arc::new(SkipRouteResponse {
            source_chain_id: "neutron".to_string(),
            source_asset_denom: "predefined_denom".to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: "predefined_dest".to_string(),
            amount: Uint128::new(5000000),
            operations: vec![],
            expected_output: Uint128::new(4950000),
            slippage_tolerance_percent: Decimal::percent(2),
        });
        
        let client = MockSkipApiClient::with_route(predefined_route);
        
        let result = client.get_optimal_route(
            "uusdc",  // These parameters should be ignored
            "uatom",
            Uint128::new(1000000),
            Decimal::percent(1),
        ).await;
        
        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.source_asset_denom, "predefined_denom");
        assert_eq!(route.dest_asset_denom, "predefined_dest");
        assert_eq!(route.amount, Uint128::new(5000000));
        assert_eq!(route.slippage_tolerance_percent, Decimal::percent(2));
    }
} 