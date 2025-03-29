use skip_swap_valence_strategist::{
    config::{StrategistConfig, NetworkConfig, LibraryConfig, AccountsConfig, SkipApiConfig, MonitoringConfig, MonitoredAccountsConfig, MonitoredAccount},
    skip::{MockSkipApiAsync, SkipRouteResponseAsync},
    strategist::Strategist,
    types::AssetPair,
};
use cosmwasm_std::{Decimal, Uint128};
use std::sync::Arc;

#[cfg(feature = "runtime")]
#[tokio::test]
async fn test_strategist_creation() {
    let config = create_test_config();
    let mock_skip_api = MockSkipApiAsync::new();
    
    let strategist = Strategist::new(config.clone(), mock_skip_api);
    assert!(strategist.is_ok());
    
    let strategist = strategist.unwrap();
    assert_eq!(strategist.library_address().to_string(), "neutron1abc123");
    assert_eq!(strategist.config().library.polling_interval, 10);
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

#[cfg(feature = "runtime")]
#[tokio::test]
async fn test_find_optimal_route_with_predefined_route() {
    let config = create_test_config();
    
    // Create a predefined route
    let predefined_route = Arc::new(SkipRouteResponseAsync {
        source_chain_id: "neutron".to_string(),
        source_asset_denom: "uusdc".to_string(),
        dest_chain_id: "neutron".to_string(),
        dest_asset_denom: "uatom".to_string(),
        amount: Uint128::new(1000000),
        operations: vec![],
        expected_output: Uint128::new(990000),
        slippage_tolerance_percent: Decimal::percent(1),
    });
    
    let mock_skip_api = MockSkipApiAsync::with_route(predefined_route);
    
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
    assert_eq!(route.expected_output, Uint128::new(990000));
}

// Helper function to create a test configuration
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