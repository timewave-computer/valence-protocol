# Skip Swap Valence Strategist

A service that interacts with both the Skip Protocol API and the Skip Swap Valence contract to provide optimized swap routes and pricing information.

## Overview

The Skip Swap Valence Strategist is responsible for:

1. Continuously monitoring the Skip Swap Valence contract for pending simulation requests
2. Querying the Skip Protocol API for optimal swap routes and pricing data
3. Submitting route simulation responses back to the contract
4. Providing trusted price data and optimized routes for Valence programs

## Route Simulation Service

The strategist implements a critical component of the Skip Swap Valence ecosystem by fulfilling route simulation requests with real-time data from the Skip Protocol.

### Key Responsibilities

#### Monitoring Simulation Requests

The strategist periodically polls the Skip Swap Valence contract to discover pending simulation requests using the `GetPendingSimulationRequests` query endpoint. This allows it to stay aware of any new requests that need to be fulfilled.

#### Route Optimization

When a pending request is found, the strategist:
1. Extracts the request parameters (input denom, output denom, amount, slippage)
2. Makes an API call to Skip Protocol to find the most efficient route
3. Processes the response to format it according to the contract's expected format

#### Submitting Route Responses

After obtaining optimized route information, the strategist:
1. Prepares a `SubmitRouteSimulation` transaction with the route details
2. Signs and broadcasts the transaction to the blockchain
3. Confirms the successful submission

#### Security and Authorization

The strategist:
- Only submits routes that have been verified through the Skip Protocol API
- Properly signs all transactions using its authorized key
- Ensures routes meet the requirements specified in the original request
- Validates expected output amounts and slippage parameters

## Configuration

The strategist requires configuration for:
- Connection details for the blockchain node
- The Skip Swap Valence contract address
- API credentials for the Skip Protocol
- Signing keys and permission settings
- Polling interval for checking pending requests

## Example Flow

```
[Skip Swap Contract] → New simulation request created
       |
[Strategist] → Polls for pending requests
       |
       ↓
[Strategist] → Finds request for swapping USDC to ATOM
       |
       ↓
[Strategist] → Calls Skip API for optimal USDC → ATOM route
       |
       ↓
[Strategist] → Receives route with expected output of 2.5 ATOM
       |
       ↓
[Strategist] → Prepares SubmitRouteSimulation transaction
       |
       ↓
[Skip Swap Contract] → Stores route simulation response
       |
       ↓
[Valence Program] → Can now query and use the simulation results
```

By fulfilling route simulation requests, the strategist enables Valence programs to make informed decisions about swaps based on current market conditions without directly integrating with external APIs.

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
