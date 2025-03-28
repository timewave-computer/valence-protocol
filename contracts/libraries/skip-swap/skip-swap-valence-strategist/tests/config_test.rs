use skip_swap_valence_strategist::config::{
    load_config, ConfigError, StrategistConfig, NetworkConfig, 
    LibraryConfig, AccountsConfig, SkipApiConfig, MonitoringConfig
};
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
    assert_eq!(config.accounts.strategist_key, Some("./.keys/strategist.key".to_string()));
    assert_eq!(config.skip_api.api_key, Some("test-api-key".to_string()));
    assert_eq!(config.monitoring.as_ref().unwrap().log_level, "info");
    assert_eq!(config.monitoring.as_ref().unwrap().metrics_port, Some(9100));
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

#[test]
fn test_missing_section() {
    let config_content = r#"
    [network]
    chain_id = "neutron-1"
    rpc_url = "https://rpc-neutron.example.com:26657"
    grpc_url = "https://grpc-neutron.example.com:9090"
    
    [accounts]
    strategist_key = "./.keys/strategist.key"
    
    [skip_api]
    base_url = "https://api.skip.money"
    timeout = 30
    "#;
    
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(config_content.as_bytes()).unwrap();
    
    let result = load_config(file.path());
    assert!(result.is_err());
    
    // The error should be a TomlError since a required section is missing
    if let Err(ConfigError::TomlError(_)) = result {
        // Expected error
    } else {
        panic!("Expected TomlError");
    }
}

#[test]
fn test_invalid_toml() {
    let config_content = r#"
    [network
    chain_id = "neutron-1"
    rpc_url = "https://rpc-neutron.example.com:26657"
    grpc_url = "https://grpc-neutron.example.com:9090"
    "#;
    
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(config_content.as_bytes()).unwrap();
    
    let result = load_config(file.path());
    assert!(result.is_err());
    
    if let Err(ConfigError::TomlError(_)) = result {
        // Expected error
    } else {
        panic!("Expected TomlError");
    }
}

#[test]
fn test_optional_fields() {
    let config_content = r#"
    [network]
    chain_id = "neutron-1"
    rpc_url = "https://rpc-neutron.example.com:26657"
    grpc_url = "https://grpc-neutron.example.com:9090"
    
    [library]
    contract_address = "neutron1abc123..."
    polling_interval = 10
    
    [accounts]
    strategist_mnemonic = "word1 word2 word3..."
    
    [skip_api]
    base_url = "https://api.skip.money"
    # API key is optional
    timeout = 30
    
    # Monitoring section is optional
    "#;
    
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(config_content.as_bytes()).unwrap();
    
    let config = load_config(file.path()).unwrap();
    
    assert_eq!(config.accounts.strategist_key, None);
    assert_eq!(config.accounts.strategist_mnemonic, Some("word1 word2 word3...".to_string()));
    assert_eq!(config.skip_api.api_key, None);
    assert!(config.monitoring.is_none());
} 