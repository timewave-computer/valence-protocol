# Skip Swap Integration Testing Documentation

This document outlines the comprehensive integration testing strategy for the Skip Swap system, focusing on the interaction between the Skip Swap Valence library and the Strategist component.

## Overview

The integration tests verify that the Skip Swap Valence contract and Strategist work together correctly. This includes:

1. **End-to-End Workflows**: Testing the complete token swap flow from deposit to execution
2. **Component Interaction**: Testing the communication between the Strategist and Valence library
3. **Error Handling**: Testing error conditions and recovery mechanisms
4. **Multi-Hop Routes**: Testing complex swap routes with intermediate accounts
5. **Security Boundaries**: Testing access control mechanisms across components

## Running Tests

### Using Nix

The integration tests can be run using Nix, which provides a consistent development environment:

```bash
# Enter the development shell
cd /path/to/valence-protocol
nix develop

# Run all integration tests
cd contracts/libraries/skip-swap/integration-test
cargo test -- --nocapture

# Run a specific test
cargo test test_end_to_end_swap -- --nocapture

# Run tests with code coverage
cargo tarpaulin --out Html
```

### Without Nix

If you prefer not to use Nix:

```bash
cd contracts/libraries/skip-swap/integration-test
cargo test
```

## Integration Tests

### End-to-End Flow Tests (`end_to_end_test.rs`)

#### `test_end_to_end_swap`
- **Purpose**: Verify the complete swap flow from deposit detection to swap execution
- **Expected Outcome**: Tokens are detected, route is calculated, and swap is executed
- **Details**: Tests the interaction between all components with mocked external dependencies

#### `test_end_to_end_swap_with_params`
- **Purpose**: Verify the complete swap flow with custom swap parameters
- **Expected Outcome**: Custom parameters are properly passed and respected throughout the flow
- **Details**: Tests custom slippage and destination address parameters

### Multi-Hop Route Tests (`multi_hop_test.rs`)

#### `test_multi_hop_route_with_intermediate_accounts`
- **Purpose**: Verify that multi-hop routes work with proper intermediate accounts
- **Expected Outcome**: Route is executed correctly with tokens flowing through intermediate accounts
- **Details**: Tests setting up intermediate accounts and executing a multi-hop route

#### `test_multi_hop_route_without_intermediate_accounts`
- **Purpose**: Verify that multi-hop routes fail without intermediate accounts
- **Expected Outcome**: System returns an appropriate error about missing intermediate accounts
- **Details**: Tests error handling for missing intermediate accounts configuration

### Skip API Integration Tests (`skip_api_test.rs`)

#### `test_skip_api_integration`
- **Purpose**: Verify that the Strategist correctly processes Skip API responses
- **Expected Outcome**: Skip API responses are correctly transformed into execution messages
- **Details**: Tests various Skip API response formats and edge cases

#### `test_skip_api_error_handling`
- **Purpose**: Verify that the system handles Skip API errors correctly
- **Expected Outcome**: Errors are propagated and reported appropriately
- **Details**: Tests various error scenarios from the Skip API

### System Configuration Tests (`config_test.rs`)

#### `test_system_configuration`
- **Purpose**: Verify that the system components work with various configurations
- **Expected Outcome**: Components correctly use their respective configurations
- **Details**: Tests different configuration combinations and their effects

### Security Tests (`security_test.rs`)

#### `test_strategist_authentication`
- **Purpose**: Verify that only the authorized strategist can execute optimized routes
- **Expected Outcome**: Unauthorized attempts are rejected
- **Details**: Tests various authentication scenarios and edge cases

#### `test_asset_pair_validation`
- **Purpose**: Verify that asset pair validation works across components
- **Expected Outcome**: Only allowed asset pairs can be swapped
- **Details**: Tests various asset pair combinations and validation rules

#### `test_venue_validation`
- **Purpose**: Verify that venue validation works across components
- **Expected Outcome**: Only allowed venues can be used for swaps
- **Details**: Tests various venue combinations and validation rules

### Error Handling Tests (`error_test.rs`)

#### `test_deposit_detection_errors`
- **Purpose**: Verify that deposit detection errors are handled correctly
- **Expected Outcome**: Errors are logged and the system recovers appropriately
- **Details**: Tests various error scenarios in the deposit detection flow

#### `test_route_execution_errors`
- **Purpose**: Verify that route execution errors are handled correctly
- **Expected Outcome**: Errors are logged and the system recovers appropriately
- **Details**: Tests various error scenarios in the route execution flow

## Test Fixtures and Utilities

The integration tests rely on several test fixtures and utilities:

### Mock Skip Entry Point

A mock implementation of the Skip entry point contract for testing swap execution without requiring the actual Skip Protocol.

### Mock Chain Client

A mock implementation of the chain client for testing blockchain interactions without requiring a real blockchain.

### Test Utilities (`test_utils.rs`)

Contains shared test utilities:
- Functions for creating test configurations
- Functions for setting up test environments
- Generators for test data
- Helper functions for asserting test results

## Test Coverage Goals

- 90%+ line coverage across component boundaries
- 85%+ branch coverage for integration scenarios
- 100% coverage of critical inter-component communication paths

## Continuous Integration

For CI/CD pipelines, tests should be run using:

```bash
nix develop -c cargo test
```

This ensures consistent environment across developers and CI systems. 