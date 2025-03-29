use skip_swap_valence_strategist::skip::{
    SkipAsync, SkipApiClientAsync, MockSkipApiAsync, SkipRouteResponseAsync
};
use cosmwasm_std::{Decimal, Uint128};
use std::sync::Arc;

#[tokio::test]
async fn test_mock_skip_api_client() {
    // Test the default mock client
    let client = MockSkipApiAsync::new();
    
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
    assert!(route.expected_output > Uint128::zero());
}

#[tokio::test]
async fn test_mock_skip_api_client_with_route() {
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
    
    // Test the mock client with a predefined route
    let client_with_route = MockSkipApiAsync::with_route(predefined_route.clone());
    
    let result = client_with_route.get_optimal_route(
        "uosmo", // Even though we pass different parameters
        "ujuno",
        Uint128::new(2000000),
        Decimal::percent(5),
    ).await;
    
    assert!(result.is_ok());
    let route = result.unwrap();
    
    // Verify it returns our predefined route regardless of input
    assert_eq!(route.source_asset_denom, "uusdc");
    assert_eq!(route.dest_asset_denom, "uatom");
    assert_eq!(route.amount, Uint128::new(1000000));
    assert_eq!(route.expected_output, Uint128::new(990000));
}

#[tokio::test]
async fn test_skip_api_client_async() {
    // Create a client instance - we're just testing that it can be instantiated
    let _client = SkipApiClientAsync::new(
        "https://api.skip.money".to_string(),
        Some("test-api-key".to_string()),
    );
    
    // This test just verifies that the client can be constructed
    // We don't make actual API calls or check private fields
    assert!(true); // Simple assertion to make the test pass
} 