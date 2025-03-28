# Skip Swap for Valence

A set of components for integrating Skip Swap functionality into Valence vaults.

## Components

The Skip Swap integration consists of three main components:

1. **Skip Swap Valence Library** (in `skip-swap-valence/`): The on-chain library that handles validation, configuration, and execution
2. **Skip Swap Valence Strategist** (in `skip-swap-valence-strategist/`): The off-chain component responsible for monitoring accounts and finding optimal routes via the Skip API
3. **Integration Tests** (in `integration-test/`): Tests that verify the integration between the Valence library and Strategist

This architecture separates concerns, with the library handling secure on-chain validation and execution, while the Strategist handles off-chain API interactions.

## Features

### Library Features

- Stores configuration for allowed asset pairs, venues, and slippage parameters
- Provides message construction for Skip integration
- Validates routes returned by the Strategist
- Executes optimized swap routes through the Skip entry point

### Strategist Actor

The off-chain Strategist:

- Monitors specified accounts for token deposits
- Interacts with the Skip API to find optimal routes
- Submits routes to the library for execution
- Handles authentication and API key management

## Configuration

### Library Configuration

The library is configured with:

- Allowed asset pairs (input and output assets)
- Allowed swap venues (e.g., "astroport", "osmosis")
- Maximum slippage tolerance
- Token destinations (where swapped tokens should be sent)
- Intermediate accounts (for multi-step swaps)
- Strategist address
- Skip entry point contract address

Example configuration:

```rust
Config {
    strategist_address: "neutron...",
    skip_entry_point: "neutron...",
    allowed_asset_pairs: [
        AssetPair { input_asset: "uusdc", output_asset: "uatom" },
        // ...
    ],
    allowed_venues: ["astroport", "osmosis"],
    max_slippage: Decimal::percent(1),
    // ...
}
```

## Workflow

1. Valence input account receives tokens
2. Strategist polls the Valence input account for token deposits
3. When tokens are detected, Strategist calls `GetRouteParameters()`
4. Skip Swap library returns the allowed routes, venues, and slippage
5. Strategist queries Skip API with these parameters to find optimal route
6. Strategist calls `ExecuteOptimizedRoute()` with the optimized route
7. Library validates the route (allowed asset pairs, venues, slippage, and the Strategist's identity)
8. Library forwards the route to the Skip entry point for execution

## Security

The architecture is designed with security in mind:

- Strict validation of all routes before execution
- Only pre-configured asset pairs and venues are allowed
- Maximum slippage enforcement prevents excessive losses
- Only the designated Strategist address can submit routes

## Development

### Prerequisites

- Rust (latest stable)
- Cargo
- Nix (for development environment)

### Building

Build the library:

```bash
cd skip-swap-valence
cargo build --release
```

Build the strategist:

```bash
cd skip-swap-valence-strategist
cargo build --release
```

### Testing

This project uses Nix to provide a consistent development environment for testing. Each component has its own test suite.

#### Running Skip Swap Valence Library Tests

```bash
# Enter the Nix development environment
nix develop

# Run unit tests for the library
cd skip-swap-valence
cargo test

# To run a specific test with output
cargo test test_name -- --nocapture

# To run tests in the tests directory only
cargo test --test '*'
```

#### Running Skip Swap Valence Strategist Tests

```bash
# Enter the Nix development environment
nix develop

# Run unit tests for the strategist
cd skip-swap-valence-strategist
cargo test

# To run tests with async features
cargo test --features runtime
```

#### Running Integration Tests

```bash
# Enter the Nix development environment
nix develop

# Run integration tests
cd integration-test
cargo test

# To run a specific integration test
cargo test test_name
```

#### Test Coverage

To run tests with coverage reporting:

```bash
# Enter the Nix development environment
nix develop

# Install tarpaulin if not already installed
cargo install cargo-tarpaulin

# Run tests with coverage
cd skip-swap-valence  # or other component directory
cargo tarpaulin --out Html
```

This will generate an HTML report in the current directory showing test coverage statistics.
