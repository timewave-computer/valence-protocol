use skip_swap_valence_strategist::skipapi::{
    SkipApi, SkipApiClient, MockSkipApiClient, SkipApiError, SkipRouteResponse
};
use cosmwasm_std::{Decimal, Uint128};
use std::sync::Arc;

#[tokio::test]
async fn test_mock_skip_api_client_creation() {
    // Test default mock client creation
    let client = MockSkipApiClient::new();
    assert!(client.route.is_none());
    
    // Test mock client with predefined route
    let predefined_route = Arc::new(SkipRouteResponse {
        source_chain_id: "neutron".to_string(),
        source_asset_denom: "uusdc".to_string(),
        dest_chain_id: "neutron".to_string(),
        dest_asset_denom: "uatom".to_string(),
        amount: Uint128::new(1000000),
        operations: vec![],
        expected_output: Uint128::new(990000),
        slippage_tolerance_percent: Decimal::percent(1),
    });
    
    let client_with_route = MockSkipApiClient::with_route(predefined_route.clone());
    assert!(client_with_route.route.is_some());
    
    // Test real client creation
    let real_client = SkipApiClient::new(
        "https://api.skip.money".to_string(),
        Some("test_api_key".to_string())
    );
    
    // We can't easily test the internal state of the real client,
    // but we can verify it was created without errors
}

#[tokio::test]
async fn test_mock_skip_api_route_query() {
    // Test getting a route with default mock client
    let client = MockSkipApiClient::new();
    
    let result = client.get_optimal_route(
        "uusdc",
        "uatom",
        Uint128::new(1000000),
        Decimal::percent(1)
    ).await;
    
    assert!(result.is_ok());
    let route = result.unwrap();
    
    // Verify basic route properties from mock
    assert_eq!(route.source_asset_denom, "uusdc");
    assert_eq!(route.dest_asset_denom, "uatom");
    assert_eq!(route.amount, Uint128::new(1000000));
    assert_eq!(route.slippage_tolerance_percent, Decimal::percent(1));
    
    // Verify that the mock created a default operation with astroport venue
    assert!(!route.operations.is_empty());
    assert_eq!(route.operations[0].operation_type, "swap");
    assert_eq!(route.operations[0].swap_venue, Some("astroport".to_string()));
    
    // Test with predefined route
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
    
    let client_with_route = MockSkipApiClient::with_route(predefined_route);
    
    let result = client_with_route.get_optimal_route(
        "uusdc", // These should be ignored
        "uatom",
        Uint128::new(1000000),
        Decimal::percent(1)
    ).await;
    
    assert!(result.is_ok());
    let route = result.unwrap();
    
    // Verify that we got the predefined route regardless of input parameters
    assert_eq!(route.source_asset_denom, "predefined_denom");
    assert_eq!(route.dest_asset_denom, "predefined_dest");
    assert_eq!(route.amount, Uint128::new(5000000));
    assert_eq!(route.slippage_tolerance_percent, Decimal::percent(2));
}

// Note: Testing the real SkipApiClient would require mocking HTTP responses
// or using an integration test against the real API, which is beyond the
// scope of these unit tests. 