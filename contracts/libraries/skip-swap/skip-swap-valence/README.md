# Skip Swap Valence

The Skip Swap Valence library is a CosmWasm smart contract that facilitates token swaps through the Skip Protocol. It's designed to be integrated with the Valence Protocol and provides a secure and configurable interface for executing token swaps via the Skip entry point.

## Overview

This library serves as the on-chain component of the Skip Swap system, working in conjunction with the off-chain Strategist actor. The library provides validation, configuration, and execution capabilities for token swaps, while relying on the Strategist for routing logic and Skip API interaction.

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
