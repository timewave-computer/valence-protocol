# Skip Swap Valence Strategist

An off-chain component that polls for token deposits and orchestrates Skip API interactions with the Valence Protocol.

## Overview

The Skip Swap Valence Strategist is a long-running process that:

1. Polls Valence input accounts for specific token deposits
2. Queries the Skip Swap Valence library for route parameters
3. Interacts with the Skip API to find optimal routes
4. Submits optimized routes back to the Skip Swap Valence library

**Critical Security Principle**: The Strategist never has custody over any funds. It only returns route data that is validated by the Skip Swap Valence library through its authorization module before any swap is executed.

## Configuration

The Strategist is configured via a TOML file:

```toml
# config.toml
[network]
chain_id = "neutron-1"
rpc_url = "https://rpc-neutron.example.com:26657"
grpc_url = "https://grpc-neutron.example.com:9090"

[library]
contract_address = "neutron1..."
polling_interval = 10  # seconds

[accounts]
strategist_key = "./.keys/strategist.key"
# Or use mnemonic instead of key file
# strategist_mnemonic = "word1 word2 word3..."

[skip_api]
base_url = "https://api.skip.money"
api_key = "your-skip-api-key"  # Strongly recommended - see API Key section below
timeout = 30  # seconds

[monitoring]
log_level = "info"  # debug, info, warn, error
metrics_port = 9100  # Prometheus metrics port
```

## Skip API Keys

### Benefits of Using an API Key

While the Skip API can be used without an API key, there are significant benefits to authenticating with one:

1. **No Rate Limits**: Authenticated integrators are not subject to the restrictive global rate limit shared with unauthenticated users.
2. **Improved Fee Revenue Share**: Authenticated integrators are subject to a 20% revenue share on fee revenue (vs. 25% for unauthenticated).
3. **Access to Premium Features**: API key holders get access to privileged features like gas estimation APIs and universal balance query APIs.
4. **Volume and Revenue Metrics**: Get access to monthly statistics about your swap and transfer volume and earned fee revenue.

### How to Get an API Key

1. Open a support ticket on the [Skip Discord](https://discord.gg/skip) and request an API key
2. When requesting, provide:
   - Your name/contact info
   - Your project name
   - A brief description of your project

**Important**: Store your API key securely as soon as you receive it. Skip does not store the raw API key for security reasons and cannot recover it if lost.

### Security Considerations

- Keep your API key private
- Never commit your API key to version control
- Consider using environment variables or secrets management for the key
- When developing frontend applications, use a backend proxy to add your API key to requests

## Implementation

The Skip Swap Valence Strategist is implemented in Rust and consists of three main components:

### 1. Polling Service

Continuously polls the Valence input account for token deposits. When tokens are detected, it initiates the route optimization process.

### 2. Library Interface

Communicates with the Skip Swap Valence library to:
- Get route parameters using the `GetRouteParameters` query
- Submit optimized routes using the `ExecuteOptimizedRoute` execution message

Note that the Strategist only sends routing information to the library - it never directly interacts with user funds.

### 3. Skip API Client

Interfaces with the Skip API to:
- Query chain information
- Determine optimal routes
- Generate messages for execution

The API client automatically includes the API key (if configured) in the authorization header of all requests to the Skip API.

## Security Architecture

The Strategist follows a strict security model:

1. **No Fund Custody**: The Strategist never has access to or custody of user funds
2. **Route Information Only**: The Strategist only provides routing information
3. **Library Validation**: All routes must be validated by the Skip Swap Valence library's authorization module
4. **Identity Verification**: The library verifies that routes are submitted by the authorized Strategist address
5. **Parameter Validation**: Routes are validated against pre-configured allowed asset pairs, venues, and slippage parameters

This approach minimizes trust requirements and ensures that even if the Strategist is compromised, it cannot:
- Access user funds
- Execute unauthorized swaps
- Use disallowed swap venues
- Exceed slippage parameters

## Running the Strategist

```bash
# Build the Strategist
nix develop
cargo build --release

# Run with a specific config file
./target/release/skip-swap-valence-strategist --config ./config.toml

# Run with environment variables
SKIP_STRATEGIST_CONFIG=./config.toml ./target/release/skip-swap-valence-strategist
```

## Docker Support

The Strategist can be run as a Docker container:

```bash
# Build the Docker image
docker build -t skip-swap-valence-strategist .

# Run the container
docker run -v /path/to/config:/app/config -v /path/to/keys:/app/keys skip-swap-valence-strategist
```

## Security Considerations

The Strategist holds a private key with authorization to submit routes to the Valence Skip Swap Valence library. Ensure:

1. The key is stored securely
2. The Strategist runs in a secure environment
3. Network communications are encrypted
4. Logs don't expose sensitive information

While the Strategist cannot access user funds, protecting its private key is still important as it is authorized to submit routes to the library.

## Monitoring

The Strategist exposes Prometheus metrics on the configured port:

- `skip_swap_valence_strategist_polls_total`: Total number of polls
- `skip_swap_valence_strategist_deposits_detected`: Number of deposits detected
- `skip_swap_valence_strategist_routes_submitted`: Number of routes submitted
- `skip_swap_valence_strategist_api_requests`: Skip API request count
- `skip_swap_valence_strategist_errors`: Error count by type

## Development

### Prerequisites

- Rust (latest stable)
- Cargo
- Nix (for development environment)

### Testing

```bash
cargo test
```

### Integration Testing

```bash
cargo test --features integration
```
