# Skip Swap Valence Strategist Testing Documentation

This document outlines the unit testing strategy for the Skip Swap Valence Strategist component.

## Testing Approach

The Skip Swap Valence Strategist requires thorough unit testing across several areas:

1. **Skip API Client**: Tests for the client that interacts with the Skip API
2. **Strategist Logic**: Tests for the core strategist functionality
3. **Chain Client**: Tests for the client that interacts with the blockchain
4. **Configuration Management**: Tests for config parsing and validation
5. **Error Handling**: Tests for proper error handling and recovery

Note: All integration tests that verify the interaction between the Strategist and the Skip Swap Valence library should be placed in the `integration-test` crate, not here.

## Running Tests

All tests should be run from the main `skip-swap-valence-strategist` directory, not from the `tests` subdirectory.

### Using Nix

```bash
# Enter the Nix development shell
nix develop

# Run all tests in the strategist crate
cd contracts/libraries/skip-swap/skip-swap-valence-strategist
cargo test

# Run a specific test file
cargo test --test skipapi_test

# Run a specific test with output
cargo test --test skipapi_test::test_skip_api_client_creation -- --nocapture
```

### Without Nix

If you prefer not to use Nix, you can run the tests directly with Cargo:

```bash
cd contracts/libraries/skip-swap/skip-swap-valence-strategist
cargo test
```

## Test Categories

### Skip API Client Tests

#### `test_skip_api_client_creation`
- **Purpose**: Verify that the Skip API client can be created with various configurations
- **Expected Outcome**: Client is created with the correct API endpoint and optional API key
- **Details**: Tests client creation with and without API keys, with different endpoints

#### `test_skip_api_client_route_query`
- **Purpose**: Verify that the Skip API client can query for routes
- **Expected Outcome**: Client correctly formats requests and handles responses
- **Details**: Tests route queries with different parameters, mock API responses

### Strategist Logic Tests

#### `test_strategist_creation`
- **Purpose**: Verify that the strategist can be created with a valid configuration
- **Expected Outcome**: Strategist is created with the provided configuration
- **Details**: Tests creation with different configurations, validates internal state

#### `test_strategist_route_selection`
- **Purpose**: Verify that the strategist selects optimal routes
- **Expected Outcome**: Strategist chooses the route with the best expected output
- **Details**: Tests selection logic with various mock routes

### Chain Client Tests

#### `test_chain_client_creation`
- **Purpose**: Verify that the chain client can be created
- **Expected Outcome**: Client is created with the correct network configuration
- **Details**: Tests client creation with different networks and endpoints

#### `test_chain_client_strategist_assignment`
- **Purpose**: Verify that the chain client correctly uses the strategist address
- **Expected Outcome**: Client is created with the provided strategist address
- **Details**: Tests client operations with different addresses

### Configuration Tests

#### `test_config_parsing`
- **Purpose**: Verify that configuration can be parsed from TOML
- **Expected Outcome**: Configuration is correctly parsed from TOML strings
- **Details**: Tests parsing of various configuration formats and values

#### `test_config_validation`
- **Purpose**: Verify that configuration validation works correctly
- **Expected Outcome**: Invalid configurations are rejected, valid ones accepted
- **Details**: Tests validation of missing fields, invalid values, etc.

### Error Handling Tests

#### `test_api_error_handling`
- **Purpose**: Verify that API errors are handled gracefully
- **Expected Outcome**: Client properly handles API errors without crashing
- **Details**: Tests various error conditions from the Skip API

#### `test_chain_error_handling`
- **Purpose**: Verify that blockchain errors are handled gracefully
- **Expected Outcome**: Client properly handles chain errors without crashing
- **Details**: Tests various error conditions from the blockchain

## Test Coverage Goals

- 90%+ line coverage
- 85%+ branch coverage
- 100% coverage of critical paths (polling, API integration, transaction submission)

## Adding New Tests

When adding new tests:

1. Place the test in the appropriate file in the `tests` directory
2. Follow the existing naming conventions and documentation style
3. Ensure that the test covers a specific functionality or edge case
4. Add appropriate assertions to verify the expected behavior

For example, to add a new Skip API client test:

```rust
#[tokio::test]
async fn test_skip_api_new_feature() {
    // Test setup
    let client = MockSkipApiClient::new();
    
    // Test the new feature
    let result = client.test_new_feature().await;
    assert!(result.is_ok());
    
    // Test error handling
    let result = client.test_new_feature_with_error().await;
    assert!(matches!(result, Err(SkipApiError::InvalidResponse { .. })));
}
```

## Unit Tests

### Orchestrator Tests (`orchestrator_test.rs`)

#### `test_new_orchestrator`
- **Purpose**: Verify that the orchestrator initializes correctly
- **Expected Outcome**: Orchestrator instance is created with the provided configuration
- **Details**: Tests proper initialization of chain client, Skip API client, and config

#### `test_polling`
- **Purpose**: Verify that the polling logic works correctly
- **Expected Outcome**: Deposits are detected and processed
- **Details**: Tests the detection of token deposits and tracking of balances

#### `test_process_deposit`
- **Purpose**: Verify that deposits are processed correctly
- **Expected Outcome**: Route parameters are queried and submitted correctly
- **Details**: Tests the end-to-end flow from deposit detection to route submission

#### `test_submit_with_retry`
- **Purpose**: Verify that the retry logic works correctly
- **Expected Outcome**: Failed transactions are retried up to the configured limit
- **Details**: Tests retry behavior with simulated failures

### Skip API Client Tests (`skip_test.rs`)

#### `test_skip_api_init`
- **Purpose**: Verify that the Skip API client initializes correctly
- **Expected Outcome**: Client is created with the provided base URL and API key
- **Details**: Tests initialization with and without API key

#### `test_query_optimal_route`
- **Purpose**: Verify that route queries to the Skip API work correctly
- **Expected Outcome**: API client returns a valid route for the provided parameters
- **Details**: Tests route queries with various inputs

#### `test_skip_api_error_handling`
- **Purpose**: Verify that the Skip API client handles errors correctly
- **Expected Outcome**: Client returns appropriate errors for various API failures
- **Details**: Tests error handling for timeouts, bad responses, and network issues

#### `test_api_key_usage`
- **Purpose**: Verify that the API key is used correctly in requests
- **Expected Outcome**: API key is included in the authorization header
- **Details**: Tests request headers with and without API key

### Chain Client Tests (`chain_test.rs`)

#### `test_chain_client_init`
- **Purpose**: Verify that the chain client initializes correctly
- **Expected Outcome**: Client is created with the provided strategist address
- **Details**: Tests initialization with different address configurations

#### `test_query_balance`
- **Purpose**: Verify that balance queries work correctly
- **Expected Outcome**: Client returns the correct balance for the queried account and token
- **Details**: Tests balance queries with various accounts and tokens

#### `test_query_route_parameters`
- **Purpose**: Verify that route parameter queries work correctly
- **Expected Outcome**: Client returns the correct route parameters from the library contract
- **Details**: Tests route parameter queries with various tokens

#### `test_submit_transaction`
- **Purpose**: Verify that transaction submission works correctly
- **Expected Outcome**: Client correctly submits transactions to the blockchain
- **Details**: Tests transaction submission with various messages

#### `test_wait_for_transaction`
- **Purpose**: Verify that transaction confirmation works correctly
- **Expected Outcome**: Client correctly waits for transaction confirmation
- **Details**: Tests confirmation waiting with simulated confirmations and timeouts

### Message Construction Tests (`messages_test.rs`)

#### `test_create_execute_optimized_route_msg`
- **Purpose**: Verify that route messages are constructed correctly
- **Expected Outcome**: Messages include all required fields in the correct format
- **Details**: Tests message construction with various route parameters

#### `test_create_skip_route_msgs`
- **Purpose**: Verify that Skip API route messages are constructed correctly
- **Expected Outcome**: Messages include all required fields in the correct format
- **Details**: Tests Skip API message construction with various parameters

### Configuration Tests (`config_test.rs`)

#### `test_load_config`
- **Purpose**: Verify that configuration loading works correctly
- **Expected Outcome**: Configuration is loaded from the provided file
- **Details**: Tests loading from various file formats and locations

#### `test_validate_config`
- **Purpose**: Verify that configuration validation works correctly
- **Expected Outcome**: Invalid configurations are rejected with appropriate errors
- **Details**: Tests validation with various invalid configurations

#### `test_api_key_loading`
- **Purpose**: Verify that the API key is loaded correctly
- **Expected Outcome**: API key is loaded from the configuration file
- **Details**: Tests API key loading from various configuration formats

## Mock Tests

### Skip API Mock Tests

#### `test_with_mock_skip_api`
- **Purpose**: Test the orchestrator with a mock Skip API
- **Expected Outcome**: Orchestrator interacts correctly with the mock
- **Details**: Uses a mock implementation to test without requiring the actual Skip API

### Chain Client Mock Tests

#### `test_with_mock_chain_client`
- **Purpose**: Test the orchestrator with a mock chain client
- **Expected Outcome**: Orchestrator interacts correctly with the mock
- **Details**: Uses a mock implementation to test without requiring a real blockchain

## HTTP Client Tests

#### `test_http_client_construction`
- **Purpose**: Verify that the HTTP client is constructed correctly
- **Expected Outcome**: Client is initialized with the correct base URL and timeout
- **Details**: Tests various initialization parameters

#### `test_get_chains`
- **Purpose**: Verify that the chains endpoint works correctly
- **Expected Outcome**: Client returns a list of supported chains
- **Details**: Tests construction of request to the `/v2/info/chains` endpoint

#### `test_get_bridges`
- **Purpose**: Verify that the bridges endpoint works correctly
- **Expected Outcome**: Client returns a list of supported bridges
- **Details**: Tests construction of request to the `/v2/info/bridges` endpoint

#### `test_get_route`
- **Purpose**: Verify that the route endpoint works correctly
- **Expected Outcome**: Client returns a valid route for the provided parameters
- **Details**: Tests construction of request to the `/v2/fungible/route` endpoint with various parameters

## Error Handling Tests

#### `test_error_handling`
- **Purpose**: Verify that the application handles errors gracefully
- **Expected Outcome**: Errors are caught, logged, and retried appropriately
- **Details**: Tests various error scenarios across components

#### `test_timeout_handling`
- **Purpose**: Verify that timeouts are handled correctly
- **Expected Outcome**: Timed out operations are retried or reported appropriately
- **Details**: Tests timeout handling in API calls and transactions

## Performance Tests

#### `test_polling_performance`
- **Purpose**: Measure the performance of the polling mechanism
- **Expected Outcome**: Polling completes within acceptable time limits
- **Details**: Tests polling performance with various load scenarios

## Security Tests

#### `test_api_key_security`
- **Purpose**: Verify that the API key is handled securely
- **Expected Outcome**: API key is not logged or exposed in error messages
- **Details**: Tests API key handling in various scenarios
