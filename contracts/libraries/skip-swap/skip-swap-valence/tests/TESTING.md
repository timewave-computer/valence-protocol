# Skip Swap Valence Testing Documentation

This document outlines the unit testing strategy for the Skip Swap Valence library.

## Testing Approach

The Skip Swap Valence library requires thorough unit testing across several areas:

1. **Contract Logic**: Tests for the core contract functions including initialization, execution, and queries
2. **Validation Logic**: Tests for the validation functions that check route parameters and venues
3. **Error Handling**: Tests for the error conditions and responses
4. **State Management**: Tests for the state storage and retrieval functions

Note: All integration tests that verify the interaction between the Skip Swap Valence library and the Strategist should be placed in the `integration-test` crate, not here.

## Running Tests

All tests should be run from the main `skip-swap-valence` directory, not from the `tests` subdirectory.

### Using Nix

```bash
# Enter the Nix development shell
nix develop

# Run all tests in the library crate
cd contracts/libraries/skip-swap/skip-swap-valence
cargo test

# Run a specific test file
cargo test --test validation_test

# Run a specific test with output
cargo test --test validation_test::test_validate_venues -- --nocapture
```

### Without Nix

If you prefer not to use Nix, you can run the tests directly with Cargo:

```bash
cd contracts/libraries/skip-swap/skip-swap-valence
cargo test
```

## Test Organization

Tests are organized in the `tests` directory by functionality:

- `contract_test.rs`: Tests for contract initialization, execution, and migration
- `query_test.rs`: Tests for query functionality
- `validation_test.rs`: Tests for validation functions
- `error_test.rs`: Tests for error conditions and responses
- `state_test.rs`: Tests for state management

## Test Categories

### Contract Tests

#### `test_instantiate`
- **Purpose**: Verify that the contract can be instantiated with valid parameters
- **Expected Outcome**: Contract instantiates and stores the configuration correctly
- **Details**: Tests initialization with various configuration parameters

#### `test_update_config`
- **Purpose**: Verify that the contract configuration can be updated
- **Expected Outcome**: Configuration is updated correctly
- **Details**: Tests updating various configuration parameters

#### `test_execute_optimized_route`
- **Purpose**: Verify that an optimized route can be executed
- **Expected Outcome**: Route execution message is forwarded to the Skip entry point
- **Details**: Tests route execution with various route parameters

### Query Tests

#### `test_query_config`
- **Purpose**: Verify that the contract configuration can be queried
- **Expected Outcome**: Configuration is returned correctly
- **Details**: Tests querying configuration after initialization and updates

#### `test_query_route_parameters`
- **Purpose**: Verify that route parameters can be queried
- **Expected Outcome**: Route parameters are returned for valid asset pairs
- **Details**: Tests querying route parameters for various asset pairs

### Validation Tests

#### `test_validate_venues`
- **Purpose**: Verify that venue validation works correctly
- **Expected Outcome**: Validation passes for allowed venues and fails for disallowed ones
- **Details**: Tests venue validation with various venues

#### `test_validate_asset_pair`
- **Purpose**: Verify that asset pair validation works correctly
- **Expected Outcome**: Validation passes for allowed pairs and fails for disallowed ones
- **Details**: Tests asset pair validation with various asset pairs

#### `test_validate_slippage`
- **Purpose**: Verify that slippage validation works correctly
- **Expected Outcome**: Validation passes for acceptable slippage and fails for excessive slippage
- **Details**: Tests slippage validation with various slippage values

### Error Tests

#### `test_instantiate_errors`
- **Purpose**: Verify that contract instantiation fails with invalid parameters
- **Expected Outcome**: Contract returns the appropriate error
- **Details**: Tests instantiation with missing or invalid parameters

#### `test_execution_errors`
- **Purpose**: Verify that execution fails with invalid parameters or permissions
- **Expected Outcome**: Contract returns the appropriate error
- **Details**: Tests execution with unauthorized senders, invalid routes, etc.

#### `test_query_errors`
- **Purpose**: Verify that queries fail with invalid parameters
- **Expected Outcome**: Contract returns the appropriate error
- **Details**: Tests queries with invalid parameters

### State Tests

#### `test_config_storage`
- **Purpose**: Verify that the configuration is stored correctly
- **Expected Outcome**: Configuration can be retrieved and matches the expected values
- **Details**: Tests storage and retrieval of various configuration values

## Test Coverage Goals

- 90%+ line coverage
- 85%+ branch coverage
- 100% coverage of critical paths (validation, route execution)

## Adding New Tests

When adding new tests:

1. Place the test in the appropriate file in the `tests` directory
2. Follow the existing naming conventions and documentation style
3. Ensure that the test covers a specific functionality or edge case
4. Add appropriate assertions to verify the expected behavior

For example, to add a new validation test:

```rust
#[test]
fn test_validate_new_feature() {
    // Test setup
    let deps = mock_dependencies();
    let env = mock_env();
    
    // Configuration with feature enabled/disabled
    let config = Config {
        // ...
    };
    
    // Test validation with feature enabled
    let result = validate_new_feature(&deps.as_ref(), &env, &config, params);
    assert!(result.is_ok());
    
    // Test validation with feature disabled or invalid params
    let result = validate_new_feature(&deps.as_ref(), &env, &config, invalid_params);
    assert!(matches!(result, Err(ContractError::InvalidFeature { .. })));
} 