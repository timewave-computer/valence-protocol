# Skip Swap Valence

A CosmWasm contract that enables secure integration between Valence and Skip Protocol for cross-chain swaps.

## Overview

Skip Swap Valence is a contract library that facilitates interaction with the Skip Protocol for cross-chain swaps, while leveraging Valence's authorization and orchestration capabilities. It provides a security layer for swap validation and execution.

## Key Features

- **Authorization Management**: Validate operations against permissioned actors and parameters
- **Swap Execution**: Safely execute swaps through Skip Protocol's entry point
- **Parameter Validation**: Ensure swaps only use approved asset pairs, venues, and slippage settings
- **Asset Routing**: Configure where swapped assets should be sent
- **Route Simulation & Price Oracle**: Request and receive price data and optimized routes through a trust-minimized process

## Route Simulation System

The Skip Swap Valence contract includes a simulation request system that enables Valence programs to request route simulations or price data from strategists. This provides a trust-minimized way to obtain pricing information before committing to swaps.

### Key Components

#### Message Types

- `RequestRouteSimulation`: Request a route simulation with input asset, output asset, and amount parameters
- `SubmitRouteSimulation`: Submit an optimized route as a response to a simulation request
- `GetSimulationResponse`: Query a simulation response by request ID
- `GetPendingSimulationRequests`: Query all pending (unfulfilled) simulation requests

#### Storage

- Simulation requests are stored with unique IDs
- Responses are linked to their original requests
- Pending requests can be easily queried by strategists

### Workflow

1. **Request Phase**: A Valence program calls `RequestRouteSimulation` with parameters
2. **Polling Phase**: Strategists poll for pending requests using `GetPendingSimulationRequests`
3. **Response Phase**: After querying Skip API, strategists submit optimized routes via `SubmitRouteSimulation`
4. **Validation Phase**: The contract validates routes against request parameters and authorization constraints
5. **Usage Phase**: Valence programs query simulation results using `GetSimulationResponse`

### Use Cases

- **Price Oracle**: Get current market prices for assets without executing swaps
- **Swap Optimization**: Find the most efficient route before committing to execution
- **Price-Dependent Logic**: Build conditional logic in Valence programs based on current prices
- **Multi-Step Operations**: Get price information before executing complex operations

### Security Features

- **Request Authorization**: Only authorized actors can create simulation requests
- **Response Validation**: Submitted routes are validated against original request parameters
- **Parameter Constraints**: Routes must conform to authorized asset pairs, venues, and slippage settings
- **Strategist Authentication**: Only authorized strategists can submit route simulations

## Integration

The Skip Swap Valence contract is designed to work within the Valence ecosystem:

1. It integrates with the Valence authorization system for permission management
2. It works with the Skip Swap Valence Strategist to obtain optimized routes
3. It can be used by any Valence program to request price data or execute swaps

## Configuration

The contract is configured with:

- Allowed asset pairs (input and output assets)
- Allowed swap venues (e.g., "astroport", "osmosis")
- Maximum slippage tolerance
- Token destinations (where swapped tokens should be sent)
- Strategist address
- Skip entry point contract address
- Authorization contract address (optional)

## Features

- **Route Validation**: Ensures all swap routes adhere to configured constraints
- **Configurable Asset Pairs**: Define which token pairs can be swapped
- **Venue Restrictions**: Limit which DEXes can be used for swaps
- **Slippage Protection**: Configure maximum allowed slippage
- **Route Tracking**: Maintains a counter for executed routes
- **Flexible Destination Accounts**: Configure where swapped tokens are sent

## Architecture

The library consists of several key modules:

- `contract.rs`: Contains the core contract logic including entry points
- `error.rs`: Defines custom error types
- `msg.rs`: Defines message types for contract interaction
- `state.rs`: Manages on-chain state
- `types.rs`: Defines data structures
- `validation.rs`: Contains validation logic for routes and parameters

## Usage

### Instantiation

The contract is instantiated with a configuration that includes:

```rust
Config {
    strategist_address: Addr,
    skip_entry_point: Addr,
    allowed_asset_pairs: Vec<AssetPair>,
    allowed_venues: Vec<String>,
    max_slippage: Decimal,
    token_destinations: HashMap<String, Addr>,
    intermediate_accounts: HashMap<String, Addr>,
}
```

### Execute Messages

The contract supports the following execute messages:

- `Swap`: Basic swap with default parameters
- `SwapWithParams`: Swap with custom parameters
- `ExecuteOptimizedRoute`: Execute a route provided by the Strategist
- `UpdateConfig`: Update the contract configuration

### Query Messages

The contract supports the following query messages:

- `GetConfig`: Get the current configuration
- `GetRouteParameters`: Get parameters for a specific token
- `SimulateSwap`: Simulate a swap to get expected output

## Integration with Skip Protocol

The library integrates with the Skip Protocol by:

1. Validating routes against configured constraints
2. Constructing and forwarding valid swap messages to the Skip entry point
3. Tracking route execution

## Limitations and Advanced Usage

### Multi-hop Routes

**Important:** Multi-hop routes have not been thoroughly tested in the current implementation.

If you intend to implement multi-hop routes:

1. You **must** provide intermediate account addresses in the configuration
2. These intermediate accounts **must be Valence accounts**
3. The `intermediate_accounts` configuration field maps token denoms to their respective intermediate account addresses

This requirement is an intrinsic part of how the Skip API works for multi-hop routing, as each "hop" requires tokens to be temporarily held in an intermediate account before proceeding to the next swap in the sequence.

Example configuration for multi-hop routes:

```rust
let intermediate_accounts = HashMap::from([
    ("uatom".to_string(), Addr::unchecked("valence_intermediate_atom")),
    ("uusdc".to_string(), Addr::unchecked("valence_intermediate_usdc")),
]);
```

## Security Considerations

- Only the strategist address can execute optimized routes
- Asset pairs are explicitly configured
- Venues (DEXes) are explicitly allowed
- Maximum slippage is enforced
- Destination accounts are preconfigured

## Development

### Testing

Run the tests:

```
cargo test
```

### Building

Build the contract:

```
cargo build
```
