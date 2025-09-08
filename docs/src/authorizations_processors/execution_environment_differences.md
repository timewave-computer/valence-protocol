# Execution Environment Differences

The Valence Protocol supports both CosmWasm and EVM execution environments, each with different processor implementations and behavioral characteristics. This section outlines the key differences between these environments.

## Processor Architecture Differences

The CosmWasm environment provides a Full Processor with queue-based execution using sophisticated FIFO priority queues (High/Medium priority). It requires permissionless `tick()` calls to process queued messages and includes comprehensive retry mechanisms with configurable intervals. Non-atomic functions can require library callback confirmations, and it uses Polytone for Cosmos ecosystem integration with full state tracking for concurrent executions and callbacks.

The EVM environment provides a Lite Processor with immediate execution that processes messages immediately without queuing. It is designed for EVM gas cost constraints and has limited message types, supporting only Pause, Resume, and SendMsgs operations (no InsertMsgs/EvictMsgs). Messages execute once with immediate success/failure and no retry logic. It includes cross-chain messaging capabilities with minimal state tracking focused on immediate execution.

## Execution Success Behavior

In CosmWasm execution, a function fails if the target CosmWasm contract doesn't exist, if the entry point of that contract doesn't exist, if the contract execution fails for any reason, or if contract messages always fail when entry points don't exist (no fallback mechanism).

In EVM execution, a function fails if the contract explicitly fails or reverts, if contract existence checks fail (implemented in EVM Processor), or if Valence Libraries detect execution entering the fallback function (implemented safeguard).

The key difference is that EVM contracts may silently succeed even with non-existent entry points if they have a non-reverting fallback function, while CosmWasm contracts always fail for non-existent entry points.

## Message Processing Models

For atomic subroutines, CosmWasm executes all messages in a single transaction via a self-call pattern, while EVM uses try-catch with external call to maintain atomicity.

For non-atomic subroutines, CosmWasm provides sequential execution with per-function retry logic and callback confirmations, while EVM provides sequential execution until first failure with no retry or callback confirmations.

## Cross-Chain Integration

For Authorization contract routing, CosmWasm domains route messages via Polytone with proxy creation. Both environments support callback mechanisms for execution result reporting.

Polytone provides IBC-based cross-chain communication with timeout handling and retry mechanisms for reliable cross-chain execution.

## Practical Implications

When designing cross-environment programs, developers should account for:

1. Execution Guarantees: CosmWasm provides stronger execution failure guarantees
2. Retry Capabilities: Only available in CosmWasm environment
3. Queue Management: Only CosmWasm supports message prioritization and queue operations
4. Gas Models: EVM optimization focuses on immediate execution vs. CosmWasm's more complex state management
5. Library Integration: Valence Libraries include EVM-specific safeguards but cannot guarantee behavior for arbitrary contracts

Key Consideration: Functions targeting non-Valence contracts in EVM environments may succeed when they should fail if the contract has a non-reverting fallback function, while equivalent CosmWasm executions would properly fail.