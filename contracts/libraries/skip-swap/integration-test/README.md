# Skip Swap Integration Tests

This crate contains integration tests for validating the interaction between the Skip Swap Valence library contract and the Skip Swap Valence Strategist.

**Note: These tests are currently standalone and should eventually be integrated into the Valence interchain test framework for more comprehensive end-to-end testing, or tested on mainnet.**

## Purpose

The integration tests verify that the Skip Swap Valence contract and Strategist work together correctly. This includes:

1. **End-to-End Flow**: Testing the complete flow from token deposit detection to route execution
2. **Component Interaction**: Testing the communication between the Strategist and Valence library
3. **Error Handling**: Verifying proper error handling across component boundaries

## Test Types

### Mock Tests

These tests use mock implementations of external dependencies:

- Mock Skip API responses
- Mock blockchain interactions
- In-memory contract execution via `cw-multi-test`

These tests focus on the interaction logic between components without requiring actual blockchain deployments.

### External Integration (Future Work)

These tests would be integrated into the Valence interchain test framework and would:

- Deploy actual contracts to test networks
- Use real Skip API responses
- Execute full end-to-end flows
- Test cross-chain interactions

## Running Tests

### Using Nix

```bash
# Load the Nix development environment
nix develop

# Run the integration tests
cd contracts/libraries/skip-swap/integration-test
cargo test
```

### Without Nix

```bash
cd contracts/libraries/skip-swap/integration-test
cargo test
```

## Dependencies

The integration tests depend on both the Skip Swap Valence library and the Skip Swap Valence Strategist crates. The tests use:

- `cw-multi-test` for in-memory contract execution
- `tokio` for asynchronous tests involving the Strategist
- `rstest` for parameterized test cases

## Future Improvements

- Integration with the Valence interchain test framework
- More comprehensive testing of edge cases
- More comprehensive testing of the Strategist's interaction with the contract
- Testing with various network configurations and asset pairs
- Performance and stress testing 