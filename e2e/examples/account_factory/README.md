# Account Factory E2E Tests

This directory contains comprehensive end-to-end tests for the Valence Account Factory system, covering both EVM and CosmWasm environments with ZK proof integration and **Ferry Service architecture**.

## Overview

The test suite validates the complete account factory workflow including:
- **Ferry Service Architecture**: Tests the complete flow: App → Ferry → ZK → Gateway → Factory
- **Historical Block Entropy**: Validates temporal constraints and entropy-based addressing
- **contract compilation and deployment** (where dependencies are available)
- Account creation and management
- ZK proof generation and verification
- Cross-chain compatibility
- Security and performance scenarios

## Ferry Service Architecture

The e2e tests now implement the ferry service architecture shown in the account factory documentation:

```
Application → Ferry Service → ZK Coprocessor → Verification Gateway → Account Factory
```

### Key Components

1. **FerryService**: Coordinates the entire account creation flow
   - Queues account creation requests from applications
   - Adds historical block numbers for entropy validation
   - Manages batch processing and optimization
   - Coordinates with ZK coprocessor for proof generation
   - Submits verified proofs to verification gateways
   - Creates accounts through factory contracts

2. **Historical Block Validation**: 
   - Ferry service automatically adds recent historical block numbers
   - Contracts validate block age (≤ 200 blocks)
   - Entropy includes historical block hash for security

3. **Multi-Chain Support**:
   - Single ferry service supports multiple chains (Ethereum, Neutron)
   - Chain-specific routing and validation
   - Cross-chain consistency with deterministic addressing

## Test Architecture

### Test Components

1. **FerryService** - Full architecture coordination including ZK proof workflow
2. **EthereumClient** - RPC calls to Anvil for EVM contract deployment
3. **CosmWasmClient** - WASM compilation with mock deployment
4. **CoprocessorClient** - Mock ZK proof generation (connects to service when available)

## Test Scenarios

### Ferry Service Tests

1. **ferry_service_batch** - Batch processing through ferry service architecture
2. **ferry_service_architecture** - Complete architecture flow validation  
3. **historical_block_validation** - Temporal entropy validation

### Core Functionality Tests

4. **basic_account_creation** - Simple account creation on both chains
5. **deterministic_addressing** - Address prediction and validation
6. **atomic_operations** - Atomic account creation with validation
7. **cross_chain_consistency** - Deterministic behavior across chains

### Security & Performance Tests

8. **security_scenarios** - Replay protection and validation
9. **performance_benchmarks** - Gas usage and timing analysis
10. **zk_proof_verification** - Proof generation and validation

### Integration Tests

11. **zk_proof_submission_evm** - ZK proof submission to EVM contracts
12. **zk_proof_submission_cosmwasm** - ZK proof submission to CosmWasm contracts
13. **e2e_account_creation_evm** - Complete EVM workflow with ZK
14. **e2e_account_creation_cosmwasm** - Complete CosmWasm workflow with ZK

## Historical Block Entropy

### Temporal Validation

The ferry service implements historical block entropy validation:

```rust
// Ferry service adds recent historical block
request.historical_block_number = get_recent_historical_block(chain).await?;

// Contracts validate block age
if current_block - historical_block > MAX_BLOCK_AGE {
    return Err("Historical block too old");
}

// Entropy includes historical data
let salt = hash(
    controller,
    program_id,
    account_request_id,
    account_type,
    historical_block_hash,
    historical_block_number
);
```

### Security Properties

- **200 Block Limit**: Prevents stale entropy usage
- **Deterministic Addressing**: Same inputs → same addresses
- **Replay Protection**: Account request IDs prevent duplicates
- **Entropy Freshness**: Recent blocks prevent precomputation attacks

## Running Tests

### Prerequisites

```bash
# Install dependencies
cargo build --release

# Start local blockchain (Anvil)
anvil --port 8545 --chain-id 31337

# Optional: Start ZK coprocessor service
# (tests will use mock if not available)
```

### Execute Tests

```bash
# Run all tests
cargo run --bin account_factory_e2e

# Run with verbose output
RUST_LOG=debug cargo run --bin account_factory_e2e

# Test specific scenarios
cargo test ferry_service_architecture
cargo test historical_block_validation
```

## Architecture Flow Validation

The tests validate each step of the ferry service architecture:

1. **Application Request**: App submits request to ferry service
2. **Historical Block Addition**: Ferry adds recent block number
3. **ZK Proof Generation**: Ferry coordinates with ZK coprocessor
4. **Proof Verification**: Ferry submits proofs to verification gateway
5. **Account Creation**: Ferry triggers factory account creation
6. **Result Validation**: Verify accounts created with correct entropy

This comprehensive testing ensures the account factory system works correctly in realistic production scenarios with third-party actors and historical block entropy validation.

## Contract Compilation Pipeline

### EVM Contracts (Solidity)

**Compilation Process:**
```bash
# Core contracts with available dependencies
forge build src/accounts/JitAccount.sol src/accounts/AccountFactory.sol --force

# Complex contracts (attempt compilation, fallback to mock if dependencies missing)
forge build src/authorization/Authorization.sol --force
forge build src/processor/Processor.sol --force
forge build src/verification/VerificationGateway.sol --force
```

**Artifact Management:**
- Artifacts copied to safe location: `./artifacts/`
- Prevents cleanup between compilation and deployment
- Fallback to original locations if safe artifacts unavailable

### CosmWasm Contracts (Rust/WASM)

**Compilation Process:**
```bash
# From workspace root to ensure proper target directory
cargo build --release --target wasm32-unknown-unknown \
    -p valence-account-factory \
    -p valence-jit-account
```

**Bytecode Extraction:**
- WASM bytecode from `target/wasm32-unknown-unknown/release/`
- Actual file size validation and reporting
- Ready for upload to CosmWasm chains

## Test Suite

1. **environment_setup** - Service connectivity and infrastructure
2. **contract_deployment** - contract compilation and deployment
3. **basic_account_creation** - Core account creation functionality
4. **deterministic_addressing** - Address computation verification
5. **zk_proof_verification** - ZK proof generation and validation
6. **atomic_operations** - Atomic account creation with requests
7. **ferry_service_batch** - Batch processing capabilities
8. **cross_chain_consistency** - EVM/CosmWasm compatibility
9. **security_scenarios** - Replay protection and security
10. **performance_benchmarks** - Timing and efficiency metrics
11. **zk_proof_submission_evm** - EVM ZK proof submission
12. **zk_proof_submission_cosmwasm** - CosmWasm ZK proof submission
13. **e2e_account_creation_evm** - Complete EVM workflow
14. **e2e_account_creation_cosmwasm** - Complete CosmWasm workflow
