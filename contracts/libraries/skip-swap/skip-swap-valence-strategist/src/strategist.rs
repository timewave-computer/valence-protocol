use crate::config::StrategistConfig;
use crate::skipapi::{SkipApi, SkipRouteResponse};
use crate::types::AssetPair;
use cosmwasm_std::{Addr, Decimal, Uint128};
use thiserror::Error;

/// Errors that can occur during strategist operations
#[derive(Error, Debug)]
pub enum StrategistError {
    #[error("Skip API error: {0}")]
    SkipApiError(String),
    
    #[error("Chain client error: {0}")]
    ChainError(String),
    
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
    
    #[error("Route validation error: {0}")]
    ValidationError(String),
}

/// Core strategist implementation that manages route discovery and submission
pub struct Strategist {
    /// Configuration for the strategist
    config: StrategistConfig,
    
    /// Skip API client for route discovery
    skip_api: Box<dyn SkipApi>,
    
    /// Library contract address
    library_address: Addr,
}

impl Strategist {
    /// Create a new strategist with the given configuration and Skip API client
    pub fn new(config: StrategistConfig, skip_api: Box<dyn SkipApi>) -> Result<Self, StrategistError> {
        let library_address = Addr::unchecked(&config.library.contract_address);
        
        Ok(Self {
            config,
            skip_api,
            library_address,
        })
    }
    
    /// Find the optimal route for a given asset pair and amount
    pub async fn find_optimal_route(
        &self,
        asset_pair: &AssetPair,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponse, StrategistError> {
        // Query the Skip API for the optimal route
        let route = self.skip_api
            .get_optimal_route(
                &asset_pair.input_asset,
                &asset_pair.output_asset,
                amount,
                slippage_tolerance,
            )
            .await
            .map_err(|e| StrategistError::SkipApiError(e.to_string()))?;
        
        // Validate the returned route
        self.validate_route(&route)?;
        
        Ok(route)
    }
    
    /// Validate a route against strategist configuration
    fn validate_route(&self, route: &SkipRouteResponse) -> Result<(), StrategistError> {
        // Validation logic would go here, checking against allowed venues, slippage, etc.
        // This is a simplified implementation
        
        // Real implementation would need to check:
        // 1. All venues in the route are allowed
        // 2. Slippage is within acceptable limits
        // 3. Asset pair is allowed
        // 4. Expected output is reasonable
        
        Ok(())
    }
    
    /// Get the library contract address
    pub fn library_address(&self) -> &Addr {
        &self.library_address
    }
    
    /// Get the strategist configuration
    pub fn config(&self) -> &StrategistConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skipapi::MockSkipApiClient;
    use std::sync::Arc;
    use std::collections::HashMap;
    
    // Create a test configuration
    fn create_test_config() -> StrategistConfig {
        StrategistConfig {
            network: crate::config::NetworkConfig {
                chain_id: "neutron-1".to_string(),
                rpc_url: "https://rpc.example.com".to_string(),
                grpc_url: "https://grpc.example.com".to_string(),
            },
            library: crate::config::LibraryConfig {
                contract_address: "neutron1abc123".to_string(),
                polling_interval: 10,
            },
            accounts: crate::config::AccountsConfig {
                strategist_key: Some("test_key".to_string()),
                strategist_mnemonic: None,
            },
            skip_api: crate::config::SkipApiConfig {
                base_url: "https://api.skip.money".to_string(),
                api_key: Some("test_api_key".to_string()),
                timeout: 30,
            },
            monitoring: Some(crate::config::MonitoringConfig {
                log_level: "info".to_string(),
                metrics_port: Some(9100),
            }),
        }
    }
    
    // Create a test route response
    fn create_test_route() -> SkipRouteResponse {
        SkipRouteResponse {
            source_chain_id: "neutron".to_string(),
            source_asset_denom: "uusdc".to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: "uatom".to_string(),
            amount: Uint128::new(1000000),
            operations: vec![],
            expected_output: Uint128::new(990000),
            slippage_tolerance_percent: Decimal::percent(1),
        }
    }
    
    #[test]
    fn test_strategist_creation() {
        let config = create_test_config();
        let mock_skip_api = Box::new(MockSkipApiClient::new());
        
        let strategist = Strategist::new(config.clone(), mock_skip_api);
        assert!(strategist.is_ok());
        
        let strategist = strategist.unwrap();
        assert_eq!(strategist.library_address, Addr::unchecked("neutron1abc123"));
        assert_eq!(strategist.config.library.polling_interval, 10);
    }
    
    #[tokio::test]
    async fn test_find_optimal_route() {
        let config = create_test_config();
        let test_route = Arc::new(create_test_route());
        let mock_skip_api = Box::new(MockSkipApiClient::with_route(test_route.clone()));
        
        let strategist = Strategist::new(config, mock_skip_api).unwrap();
        
        let asset_pair = AssetPair {
            input_asset: "uusdc".to_string(),
            output_asset: "uatom".to_string(),
        };
        
        let result = strategist.find_optimal_route(
            &asset_pair,
            Uint128::new(1000000),
            Decimal::percent(1),
        ).await;
        
        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.source_asset_denom, "uusdc");
        assert_eq!(route.dest_asset_denom, "uatom");
        assert_eq!(route.amount, Uint128::new(1000000));
    }
} 