use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when loading or parsing the strategist configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Configuration for the Skip Swap Valence Strategist
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategistConfig {
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Library contract configuration
    pub library: LibraryConfig,
    
    /// Account configuration
    pub accounts: AccountsConfig,
    
    /// Skip API configuration
    pub skip_api: SkipApiConfig,
    
    /// Monitored accounts for the orchestrator
    pub monitored_accounts: Option<MonitoredAccountsConfig>,
    
    /// Monitoring configuration
    pub monitoring: Option<MonitoringConfig>,
}

/// Network configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    /// Chain ID (e.g., "neutron-1")
    pub chain_id: String,
    
    /// RPC URL for the chain
    pub rpc_url: String,
    
    /// gRPC URL for the chain
    pub grpc_url: String,
}

/// Library contract configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryConfig {
    /// Address of the Skip Swap Valence library contract
    pub contract_address: String,
    
    /// Polling interval in seconds
    pub polling_interval: u64,
    
    /// Maximum retries for failed transactions
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    
    /// Delay between retries in seconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
}

fn default_max_retries() -> u8 {
    3
}

fn default_retry_delay() -> u64 {
    5
}

/// Account configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountsConfig {
    /// Path to strategist key file
    pub strategist_key: Option<String>,
    
    /// Mnemonic for strategist account
    pub strategist_mnemonic: Option<String>,
}

/// Skip API configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkipApiConfig {
    /// Base URL for the Skip API
    pub base_url: String,
    
    /// Skip API key (optional but recommended)
    pub api_key: Option<String>,
    
    /// Timeout for API requests in seconds
    pub timeout: u64,
}

/// Monitored accounts configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitoredAccountsConfig {
    /// List of accounts to monitor, mapping token denom to account address
    pub accounts: Vec<MonitoredAccount>,
}

/// A single monitored account entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitoredAccount {
    /// Token denomination to monitor (e.g., "uatom", "uusdc")
    pub token_denom: String,
    
    /// Account address to monitor
    pub account_address: String,
}

/// Monitoring configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitoringConfig {
    /// Log level (debug, info, warn, error)
    pub log_level: String,
    
    /// Port for Prometheus metrics
    pub metrics_port: Option<u16>,
}

/// Load configuration from a TOML file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<StrategistConfig, ConfigError> {
    let config_str = fs::read_to_string(path)?;
    let config: StrategistConfig = toml::from_str(&config_str)?;
    
    // Validate required fields
    if config.accounts.strategist_key.is_none() && config.accounts.strategist_mnemonic.is_none() {
        return Err(ConfigError::MissingField(
            "Either strategist_key or strategist_mnemonic must be provided".into()
        ));
    }
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
        [network]
        chain_id = "neutron-1"
        rpc_url = "https://rpc-neutron.example.com:26657"
        grpc_url = "https://grpc-neutron.example.com:9090"
        
        [library]
        contract_address = "neutron1abc123..."
        polling_interval = 10
        
        [accounts]
        strategist_key = "./.keys/strategist.key"
        
        [skip_api]
        base_url = "https://api.skip.money"
        api_key = "test-api-key"
        timeout = 30
        
        [monitoring]
        log_level = "info"
        metrics_port = 9100
        "#;
        
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_content.as_bytes()).unwrap();
        
        let config = load_config(file.path()).unwrap();
        
        assert_eq!(config.network.chain_id, "neutron-1");
        assert_eq!(config.library.contract_address, "neutron1abc123...");
        assert_eq!(config.library.polling_interval, 10);
        assert_eq!(config.accounts.strategist_key, Some("./.keys/strategist.key".into()));
        assert_eq!(config.skip_api.api_key, Some("test-api-key".into()));
        assert_eq!(config.monitoring.unwrap().log_level, "info");
    }
    
    #[test]
    fn test_missing_required_fields() {
        let config_content = r#"
        [network]
        chain_id = "neutron-1"
        rpc_url = "https://rpc-neutron.example.com:26657"
        grpc_url = "https://grpc-neutron.example.com:9090"
        
        [library]
        contract_address = "neutron1abc123..."
        polling_interval = 10
        
        [accounts]
        # Missing both strategist_key and strategist_mnemonic
        
        [skip_api]
        base_url = "https://api.skip.money"
        timeout = 30
        "#;
        
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_content.as_bytes()).unwrap();
        
        let result = load_config(file.path());
        assert!(result.is_err());
        
        if let Err(ConfigError::MissingField(msg)) = result {
            assert!(msg.contains("strategist_key or strategist_mnemonic"));
        } else {
            panic!("Expected MissingField error");
        }
    }
} 