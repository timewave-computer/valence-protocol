use crate::config::StrategistConfig;
use crate::skip::{SkipRouteResponseAsync, SkipApiClientAsync, SkipApi, SkipAsync};
use crate::types::{AssetPair, RouteParameters};
use crate::orchestrator::{Orchestrator, OrchestratorConfig};
use crate::chain::ChainClient;
use cosmwasm_std::{Addr, Decimal, Uint128};
use thiserror::Error;
use std::collections::HashMap;
use std::path::Path;
use std::{env, fs, process};

// Other imports are used in the tests module at the end of the file
#[cfg(test)]
use crate::config::{
    NetworkConfig, LibraryConfig, AccountsConfig, 
    SkipApiConfig, MonitoringConfig, MonitoredAccountsConfig,
    MonitoredAccount
};

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
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),
}

/// Route validation parameters for ensuring optimal routes meet requirements
#[derive(Clone, Debug)]
pub struct RouteValidationParameters {
    /// Maximum allowed slippage percentage
    pub max_slippage: Option<Decimal>,
    
    /// List of allowed venues for swaps
    pub allowed_venues: Option<Vec<String>>,
    
    /// Allowed asset pairs and other route parameters
    pub route_parameters: Option<RouteParameters>,
    
    /// Minimum output ratio as a percentage of input (e.g., 50% means output should be at least 50% of input)
    pub min_output_ratio: Decimal,
}

impl Default for RouteValidationParameters {
    fn default() -> Self {
        Self {
            max_slippage: Some(Decimal::percent(5)),  // Default 5% max slippage
            allowed_venues: None,                      // Allow all venues by default
            route_parameters: None,                    // No specific route parameters
            min_output_ratio: Decimal::percent(50),    // Default minimum output should be 50% of input
        }
    }
}

/// Core strategist implementation that manages route discovery and submission
pub struct Strategist<T: SkipAsync> {
    /// Configuration for the strategist
    config: StrategistConfig,
    
    /// Skip API client for route discovery
    skip_api: T,
    
    /// Library contract address
    library_address: Addr,
    
    /// Parameters for validating routes
    validation_params: RouteValidationParameters,
}

impl<T: SkipAsync> Strategist<T> {
    /// Create a new strategist with the given configuration and Skip API client
    pub fn new(config: StrategistConfig, skip_api: T) -> Result<Self, StrategistError> {
        let library_address = Addr::unchecked(&config.library.contract_address);
        
        Ok(Self {
            config,
            skip_api,
            library_address,
            validation_params: RouteValidationParameters::default(),
        })
    }
    
    /// Create a new strategist with custom validation parameters
    pub fn new_with_validation(
        config: StrategistConfig, 
        skip_api: T,
        validation_params: RouteValidationParameters
    ) -> Result<Self, StrategistError> {
        let library_address = Addr::unchecked(&config.library.contract_address);
        
        Ok(Self {
            config,
            skip_api,
            library_address,
            validation_params,
        })
    }
    
    /// Find the optimal route for a given asset pair and amount
    #[cfg(feature = "runtime")]
    pub async fn find_optimal_route(
        &self,
        asset_pair: &AssetPair,
        amount: Uint128,
        slippage_tolerance: Decimal,
    ) -> Result<SkipRouteResponseAsync, StrategistError> {
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
    fn validate_route(&self, route: &SkipRouteResponseAsync) -> Result<(), StrategistError> {
        // 1. Check if assets in the route are valid asset pairs
        if let Some(route_params) = &self.validation_params.route_parameters {
            let valid_asset_pair = route_params.allowed_asset_pairs.iter().any(|pair| 
                pair.input_asset == route.source_asset_denom && 
                pair.output_asset == route.dest_asset_denom
            );
                
            if !valid_asset_pair {
                return Err(StrategistError::ValidationError(format!(
                    "Asset pair {}/{} is not allowed",
                    route.source_asset_denom, route.dest_asset_denom
                )));
            }
        }
        
        // 2. Verify slippage is within acceptable limits
        if let Some(max_slippage) = self.validation_params.max_slippage {
            if route.slippage_tolerance_percent > max_slippage {
                return Err(StrategistError::ValidationError(format!(
                    "Slippage {}% exceeds maximum allowed {}%",
                    route.slippage_tolerance_percent, max_slippage
                )));
            }
        }
        
        // 3. Check if all venues in operations are allowed
        if let Some(allowed_venues) = &self.validation_params.allowed_venues {
            for op in &route.operations {
                if let Some(venue) = &op.swap_venue {
                    if !allowed_venues.contains(venue) {
                        return Err(StrategistError::ValidationError(format!(
                            "Swap venue {} is not allowed", venue
                        )));
                    }
                }
            }
        }
        
        // 4. Verify the expected output is reasonable
        // Check that the output is not suspiciously low compared to input
        let input_amount = route.amount;
        let expected_output = route.expected_output;
        
        let min_output_ratio = self.validation_params.min_output_ratio;
        let actual_ratio = Decimal::from_ratio(expected_output, input_amount);
        
        if actual_ratio < min_output_ratio {
            return Err(StrategistError::ValidationError(format!(
                "Expected output ratio {}% is below minimum threshold {}%",
                actual_ratio.to_string(), min_output_ratio.to_string()
            )));
        }
        
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
    
    /// Get the validation parameters
    pub fn validation_params(&self) -> &RouteValidationParameters {
        &self.validation_params
    }
    
    /// Set new validation parameters
    pub fn set_validation_params(&mut self, params: RouteValidationParameters) {
        self.validation_params = params;
    }
}

// Implementation of the run methods specifically for SkipApiClientAsync
impl Strategist<SkipApiClientAsync> {
    /// Run the strategist with configuration from a file
    /// 
    /// This function is meant to be used as an entry point when running the
    /// strategist as a standalone process. It loads configuration, sets up
    /// the necessary components, and starts the polling loop.
    #[cfg(feature = "runtime")]
    pub async fn run_from_config_file(config_path: Option<String>) -> Result<(), StrategistError> {
        // Load configuration
        let config_path = config_path.unwrap_or_else(|| "config.toml".to_string());
        
        println!("Loading configuration from {}", config_path);
        
        if !Path::new(&config_path).exists() {
            eprintln!("Configuration file not found: {}", config_path);
            process::exit(1);
        }
        
        let config_content = fs::read_to_string(&config_path)?;
        let config: StrategistConfig = toml::from_str(&config_content)
            .map_err(|e| StrategistError::ConfigError(e.to_string()))?;
        
        // Create Skip API client
        let skip_api_client = SkipApiClientAsync::new(
            config.skip_api.base_url.clone(),
            config.skip_api.api_key.clone(),
        );
        
        // Create strategist
        let strategist = Self::new(config.clone(), skip_api_client)?;
        
        // Create monitored accounts map for the orchestrator
        let mut monitored_accounts = HashMap::new();
        
        // Populate monitored accounts from configuration if available
        if let Some(monitored_config) = &config.monitored_accounts {
            for account in &monitored_config.accounts {
                monitored_accounts.insert(
                    account.token_denom.clone(), 
                    Addr::unchecked(&account.account_address)
                );
                println!("Monitoring account {} for token {}", 
                    account.account_address, account.token_denom);
            }
        } else {
            println!("No monitored accounts specified in configuration");
        }
        
        // Create orchestrator config
        let orchestrator_config = OrchestratorConfig {
            library_address: strategist.library_address().clone(),
            monitored_accounts,
            polling_interval: config.library.polling_interval,
            max_retries: config.library.max_retries,
            retry_delay: config.library.retry_delay,
            skip_api_url: config.skip_api.base_url.clone(),
        };
        
        // Create chain client with the strategist address
        // Get strategist address from configuration - this should be based on the key/mnemonic
        // In a real implementation, this would derive the address from a key or mnemonic
        // For now, we'll use a placeholder address but log a warning
        let strategist_address = get_strategist_address(&config)?;
        println!("Using strategist address: {}", strategist_address);
        
        let chain_client = ChainClient::new(strategist_address);
        
        // Create Skip API client for the orchestrator
        let skip_api = SkipApi::new(
            &config.skip_api.base_url,
            config.skip_api.api_key.clone(),
        );
        
        println!("Starting strategist with library address {}", strategist.library_address());
        
        // Create orchestrator
        let mut orchestrator = Orchestrator::new(chain_client, skip_api, orchestrator_config);
        
        println!("Starting polling loop...");
        
        // Start polling
        orchestrator.start_polling().await.map_err(|e| {
            StrategistError::ChainError(format!("Error in polling loop: {}", e))
        })
    }
    
    /// Helper function to run the strategist from command line arguments
    #[cfg(feature = "runtime")]
    pub async fn run_from_args() -> Result<(), StrategistError> {
        let config_path = env::args().nth(1);
        Self::run_from_config_file(config_path).await
    }
}

/// Helper function to get the strategist address from configuration
/// In a real implementation, this would derive the address from the key or mnemonic
#[cfg(feature = "runtime")]
fn get_strategist_address(config: &StrategistConfig) -> Result<Addr, StrategistError> {
    // If we have a key file specified, we would load it and derive the address
    if let Some(key_path) = &config.accounts.strategist_key {
        // In a production implementation, this would:
        // 1. Load the private key from the key file
        // 2. Derive the public key
        // 3. Derive the address from the public key
        
        // For now, we'll log that we would derive from the key file
        println!("In production: Would derive address from key file at {}", key_path);
        
        // Return a placeholder address for demonstration
        return Ok(Addr::unchecked("strategist_from_key_file"));
    }
    
    // If we have a mnemonic, we would derive the address from it
    if let Some(mnemonic) = &config.accounts.strategist_mnemonic {
        // In a production implementation, this would:
        // 1. Derive the private key from the mnemonic
        // 2. Derive the public key
        // 3. Derive the address from the public key
        
        // For now, we'll log that we would derive from the mnemonic
        // Note: Never log the actual mnemonic in a production environment!
        println!("In production: Would derive address from mnemonic (first 3 chars: {}...)",
                &mnemonic[0..3.min(mnemonic.len())]);
        
        // Return a placeholder address for demonstration
        return Ok(Addr::unchecked("strategist_from_mnemonic"));
    }
    
    // If we don't have either, use a default address and warn
    println!("WARNING: No key or mnemonic specified, using default strategist address");
    Ok(Addr::unchecked("default_strategist"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skip::MockSkipApiAsync;
    
    // Create a test configuration
    fn create_test_config() -> StrategistConfig {
        // Create a list of monitored accounts for testing
        let monitored_accounts = vec![
            MonitoredAccount {
                token_denom: "uusdc".to_string(),
                account_address: "test_account1".to_string(),
            },
            MonitoredAccount {
                token_denom: "uatom".to_string(),
                account_address: "test_account2".to_string(),
            },
        ];
        
        StrategistConfig {
            network: NetworkConfig {
                chain_id: "neutron-1".to_string(),
                rpc_url: "https://rpc.example.com".to_string(),
                grpc_url: "https://grpc.example.com".to_string(),
            },
            library: LibraryConfig {
                contract_address: "neutron1abc123".to_string(),
                polling_interval: 10,
                max_retries: 3,
                retry_delay: 5,
            },
            accounts: AccountsConfig {
                strategist_key: Some("test_key".to_string()),
                strategist_mnemonic: None,
            },
            skip_api: SkipApiConfig {
                base_url: "https://api.skip.money".to_string(),
                api_key: Some("test_api_key".to_string()),
                timeout: 30,
            },
            monitored_accounts: Some(MonitoredAccountsConfig {
                accounts: monitored_accounts,
            }),
            monitoring: Some(MonitoringConfig {
                log_level: "info".to_string(),
                metrics_port: Some(9100),
            }),
        }
    }
    
    #[cfg(feature = "runtime")]
    #[tokio::test]
    async fn test_find_optimal_route() {
        let config = create_test_config();
        let mock_skip_api = MockSkipApiAsync::new();
        
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
    }
} 