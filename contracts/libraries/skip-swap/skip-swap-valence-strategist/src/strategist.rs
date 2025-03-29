/*
 * Core strategist implementation for Skip Swap Valence.
 * Provides high-level strategy logic for route optimization and execution,
 * managing the interaction between the Skip API, blockchain, and Valence contract.
 */
use crate::config::StrategistConfig;
use crate::skip::{SkipAsync, SkipRouteResponse, SkipApi};
use crate::types::{AssetPair, RouteParameters};
use crate::orchestrator::{Orchestrator, OrchestratorConfig};
use crate::chain::ChainClient;
use cosmwasm_std::{Addr, Decimal, Uint128};
use thiserror::Error;
use std::collections::HashMap;
use std::path::Path;
use std::{env, fs, process};
use std::option::Option;
use log;

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
    
    #[error("Mnemonic error: {0}")]
    MnemonicError(String),
    
    #[error("Crypto error: {0}")]
    CryptoError(String),
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
pub struct Strategist<T> {
    /// Configuration for the strategist
    config: StrategistConfig,
    
    /// Skip API client for route discovery
    skip_api: T,
    
    /// Library contract address
    library_address: Addr,
    
    /// Parameters for validating routes
    validation_params: RouteValidationParameters,
}

// Generic implementation for types that implement SkipAsync
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
    ) -> Result<SkipRouteResponse, StrategistError> {
        // Query the Skip API for the optimal route
        let async_route = self.skip_api
            .get_optimal_route(
                &asset_pair.input_asset,
                &asset_pair.output_asset,
                amount,
                slippage_tolerance,
            )
            .await
            .map_err(|e| StrategistError::SkipApiError(e.to_string()))?;
        
        // Convert from async response to synchronous response
        let route: SkipRouteResponse = async_route.into();
        
        // Validate the returned route
        self.validate_route(&route)?;
        
        Ok(route)
    }
}

// Implementations common to all Strategist types
impl<T> Strategist<T> {
    /// Validate a route against strategist configuration
    fn validate_route(&self, route: &SkipRouteResponse) -> Result<(), StrategistError> {
        // 1. Check operations and venues
        if let Some(allowed_venues) = &self.validation_params.allowed_venues {
            // swap_venue is a String, not an Option<String>
            if !allowed_venues.contains(&route.swap_venue) {
                return Err(StrategistError::ValidationError(format!(
                    "Swap venue {} is not allowed", route.swap_venue
                )));
            }
        }
        
        // 2. Verify the expected output is reasonable
        let _min_output_ratio = self.validation_params.min_output_ratio;
        let expected_output = route.expected_output;
        
        // Since we don't have input_amount in the SkipRouteResponse, 
        // we'll assume input validation happens elsewhere or we use a fixed ratio
        
        // For now, just validate that expected_output is positive
        if expected_output.is_zero() {
            return Err(StrategistError::ValidationError(
                "Expected output is zero".to_string()
            ));
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

// Implementation for SkipApi specifically
impl Strategist<SkipApi> {
    // Similar to the generic new, but specific for SkipApi
    pub fn new_with_skip_api(config: StrategistConfig, skip_api: SkipApi) -> Result<Self, StrategistError> {
        let library_address = Addr::unchecked(&config.library.contract_address);
        
        Ok(Self {
            config,
            skip_api,
            library_address,
            validation_params: RouteValidationParameters::default(),
        })
    }
    
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
        let skip_api = SkipApi::new(
            &config.skip_api.base_url,
            config.skip_api.api_key.clone(),
        );
        
        // Create strategist - use the specific constructor for SkipApi
        let strategist = Self::new_with_skip_api(config.clone(), skip_api)?;
        
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
            max_retries: config.library.max_retries as u32,
            retry_delay: config.library.retry_delay,
            skip_api_url: config.skip_api.base_url.clone(),
            contract_address: config.library.contract_address.clone(),
        };
        
        // Create chain client with the strategist address
        let strategist_address = get_strategist_address(&config)?;
        println!("Using strategist address: {}", strategist_address);
        
        let chain_client = ChainClient::new(strategist_address);
        
        // Create Skip API client for the orchestrator
        let orchestrator_skip_api = SkipApi::new(
            &config.skip_api.base_url,
            config.skip_api.api_key.clone(),
        );
        
        println!("Starting strategist with library address {}", strategist.library_address());
        
        // Create orchestrator
        let mut orchestrator = Orchestrator::new(chain_client, orchestrator_skip_api, orchestrator_config);
        
        println!("Starting polling loop...");
        
        // Start polling
        orchestrator.start_polling().map_err(|e| {
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

/// Get the strategist address from the configuration
fn get_strategist_address(config: &StrategistConfig) -> Result<Addr, StrategistError> {
    // Try to read the address from the accounts section
    // Try to derive from mnemonic
    #[cfg(feature = "runtime")]
    if let Some(mnemonic) = &config.accounts.strategist_mnemonic {
        let addr_str = derive_address_from_mnemonic(mnemonic)?;
        return Ok(Addr::unchecked(addr_str));
    }
    
    // Try to derive from private key
    #[cfg(feature = "runtime")]
    if let Some(private_key_hex) = &config.accounts.strategist_key {
        // Decode the private key from hex
        let private_key = hex::decode(private_key_hex)
            .map_err(|e| StrategistError::ConfigError(format!("Invalid private key hex: {}", e)))?;
            
        let addr_str = derive_address_from_private_key(&private_key)?;
        return Ok(Addr::unchecked(addr_str));
    }
    
    // If we couldn't find an address, use a default for testing
    Ok(Addr::unchecked("neutron1placeholder"))
}

#[cfg(feature = "runtime")]
pub fn derive_address_from_private_key(private_key: &[u8]) -> Result<String, StrategistError> {
    // Simplified implementation that creates a deterministic address from private key
    log::info!("Using simplified address derivation (not for production)");
    
    // Calculate a simple hash to make a deterministic, but not secure address
    let mut simple_addr = String::from("neutron1");
    
    // Take first 16 bytes of private key or use full private key if shorter 
    let key_prefix = &private_key[0..private_key.len().min(16)];
    
    // Create a simplified address - in production would use proper crypto
    simple_addr.push_str(&hex::encode(key_prefix));
    
    Ok(simple_addr)
}

#[cfg(feature = "runtime")]
pub fn derive_address_from_mnemonic(mnemonic: &str) -> Result<String, StrategistError> {
    // Simplified implementation - returns a deterministic address based on the mnemonic
    log::info!("Using simplified mnemonic derivation (not for production)");
    
    // Calculate a simple hash of the mnemonic
    let mut simple_addr = String::from("neutron1");
    
    // Use first 3 words of mnemonic to generate deterministic address
    let first_words: Vec<&str> = mnemonic.split_whitespace().take(3).collect();
    let combined = first_words.join("");
    
    // Add mnemonic-based characters to address (simplified)
    simple_addr.push_str(&hex::encode(combined.as_bytes())[0..32]);
    
    Ok(simple_addr)
}

#[cfg(feature = "runtime")]
fn decrypt_keystore(keystore_path: &str, password: &str) -> Result<Vec<u8>, StrategistError> {
    log::warn!("Using simplified keystore decryption (not for production)");
    
    // Read the keystore file to check if it exists
    let file_content = fs::read_to_string(keystore_path)?;
    
    // Check if file has Web3 or Cosmos format (simplified)
    if file_content.contains("\"Crypto\"") {
        return decrypt_web3_keystore(keystore_path, password);
    } else if file_content.contains("\"crypto\"") {
        return decrypt_cosmos_keystore(keystore_path, password);
    }
    
    Err(StrategistError::ConfigError(
        "Unsupported keystore format".to_string()
    ))
}

#[cfg(feature = "runtime")]
fn decrypt_web3_keystore(keystore_path: &str, password: &str) -> Result<Vec<u8>, StrategistError> {
    log::warn!("Using simplified Web3 keystore decryption (not for production)");
    
    // In a real implementation, we would:
    // 1. Parse the JSON keystore
    // 2. Extract crypto params (cipher, KDF, etc.)
    // 3. Derive the decryption key
    // 4. Verify the MAC
    // 5. Decrypt the private key
    
    // For this example, we'll create a simplified "private key" from password 
    // This is NOT how real decryption works - just for development/testing
    let mut pseudo_key = Vec::with_capacity(32);
    
    // Create a pseudo-key based on password bytes and file path
    let pass_bytes = password.as_bytes();
    for i in 0..32 {
        let byte_val = if i < pass_bytes.len() {
            pass_bytes[i]
        } else {
            // Use characters from the path if password is short
            let path_bytes = keystore_path.as_bytes();
            let path_idx = i % path_bytes.len();
            path_bytes[path_idx]
        };
        
        pseudo_key.push(byte_val);
    }
    
    Ok(pseudo_key)
}

#[cfg(feature = "runtime")]
fn decrypt_cosmos_keystore(keystore_path: &str, password: &str) -> Result<Vec<u8>, StrategistError> {
    log::warn!("Using simplified Cosmos keystore decryption (not for production)");
    
    // In a real implementation, we would:
    // 1. Parse the JSON keystore
    // 2. Extract crypto params
    // 3. Derive the decryption key
    // 4. Decrypt the private key
    
    // For this example, we'll create a simplified "private key" from password
    // This is NOT how real decryption works - just for development/testing
    let mut pseudo_key = Vec::with_capacity(32);
    
    // Cosmos-style prefix (just for differentiation in this example)
    let cosmos_prefix = [0xC0, 0x5A, 0x05];
    pseudo_key.extend_from_slice(&cosmos_prefix);
    
    // Fill remaining bytes from password or path
    let pass_bytes = password.as_bytes();
    for i in cosmos_prefix.len()..32 {
        let byte_val = if i < pass_bytes.len() {
            pass_bytes[i]
        } else {
            // Use characters from the path if password is short
            let path_bytes = keystore_path.as_bytes();
            let path_idx = i % path_bytes.len();
            path_bytes[path_idx]
        };
        
        pseudo_key.push(byte_val);
    }
    
    Ok(pseudo_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
    
    #[test]
    fn test_create_config() {
        // This just tests that we can create a config without errors
        let config = create_test_config();
        assert_eq!(config.network.chain_id, "neutron-1");
        assert_eq!(config.library.contract_address, "neutron1abc123");
        assert!(config.accounts.strategist_key.is_some());
    }
} 