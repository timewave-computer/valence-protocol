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
- Enables Valence programs to request and receive price data and optimized routes through a trust-minimized process

### Strategist Actor

The off-chain Strategist:

- Monitors specified accounts for token deposits
- Interacts with the Skip API to find optimal routes
- Submits routes to the library for execution
- Handles authentication and API key management
- Processes requests for route simulations and price data by querying Skip API and submitting verified results back to the contract

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

### Basic Swap Workflow

1. Valence input account receives tokens
2. Strategist polls the Valence input account for token deposits
3. When tokens are detected, Strategist calls `GetRouteParameters()`
4. Skip Swap library returns the allowed routes, venues, and slippage
5. Strategist queries Skip API with these parameters to find optimal route
6. Strategist calls `ExecuteOptimizedRoute()` with the optimized route
7. Library validates the route (allowed asset pairs, venues, slippage, and the Strategist's identity)
8. Library forwards the route to the Skip entry point for execution

### Route Simulation & Price Oracle Workflow

1. Valence program calls `RequestRouteSimulation()` with parameters (input asset, output asset, amount)
2. Skip Swap contract records the request with a unique ID
3. Strategist polls for pending simulation requests using `GetPendingSimulationRequests()`
4. Strategist queries Skip API to find the optimal route meeting the request parameters
5. Strategist submits the route data using `SubmitRouteSimulation()`
6. Skip Swap contract verifies the route against request parameters and authorization constraints
7. Valence program can query the simulation result using `GetSimulationResponse()`
8. The route data can be used as price information or executed through the standard swap execution flow

This oracle/simulation system enables Valence programs to obtain current market prices or optimized routes through a trust-minimized process with proper validation and authorization checks. It's particularly useful for:

- Getting current asset prices before making decisions
- Finding optimal swap routes without committing to execution
- Building price-dependent logic into Valence programs
- Creating multi-step operations where pricing information is needed before execution

## Security

The architecture is designed with security in mind:

- Strict validation of all routes before execution
- Only pre-configured asset pairs and venues are allowed
- Maximum slippage enforcement prevents excessive losses
- Only the designated Strategist address can submit routes
- Request parameters are validated against actual returned routes
- Authorization checks are performed at each step of the process

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

# Skip Swap Valence Library

Skip Swap Valence is a contract library that facilitates interaction with the Skip Protocol for cross-chain swaps, while leveraging Valence's authorization and orchestration capabilities.

## Features

- Execute cross-chain token swaps through Skip Protocol
- Validate swap parameters against configurable constraints
- Integrate with Valence authorization contracts for permission management
- Submit optimized swap routes with slippage protection

## Route Simulation System

The Skip Swap Valence contract includes a simulation request system that enables Valence programs to request route simulations or price data from the strategist. This system provides a trust-minimized way to obtain pricing information and optimize routes before executing swaps.

### How It Works

1. **Request Phase**: A Valence program calls the `RequestRouteSimulation` function on the Skip Swap contract with parameters (input asset, output asset, amount, and optional max slippage).

2. **Polling Phase**: The strategist periodically polls the contract using `GetPendingSimulationRequests` to discover new simulation requests that need to be fulfilled.

3. **Response Phase**: After querying the Skip API for the best route, the strategist submits the optimized route back to the contract using `SubmitRouteSimulation`.

4. **Validation Phase**: The contract validates the returned route against the original request parameters and authorization constraints.

5. **Usage Phase**: The simulation result can be queried using `GetSimulationResponse` and used in subsequent swap operations, providing price information or executing the optimized route.

### Key Components

- **Simulation Requests**: Stored on-chain with a unique ID, containing input/output denoms, amount, and slippage preferences
- **Pending Request Queries**: Allow strategists to discover and fulfill pending requests
- **Route Validation**: Ensures routes meet the requirements of the original request and contract authorization rules
- **Integration with Swaps**: Simulation results can be used directly in swap execution

### Example Flow

```
[Valence Program] → RequestRouteSimulation → [Skip Swap Contract]
                                               |
[Strategist] ← GetPendingSimulationRequests ← [Skip Swap Contract]
      |
      ↓
[Skip API] → Get optimized route
      |
      ↓
[Strategist] → SubmitRouteSimulation → [Skip Swap Contract]
                                         |
[Valence Program] ← GetSimulationResponse ← [Skip Swap Contract]
      |
      ↓
[Execute swap using the optimized route]
```

This system enables Valence programs to obtain reliable, validated pricing data or optimized routes with minimal trust assumptions and proper authorization checks.
