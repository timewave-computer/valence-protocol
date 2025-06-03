# Valence Protocol EVM + Neutron Vault Audit

## Overview of Valence Protocol for EVM 

Valence Protocol is designed to facilitate trust-minimized cross-chain DeFi applications. The system coordinates its actions through an Authorization contract, which manages permissions and routes execution instructions to Processor contracts deployed on EVM-compatible chains. These Processors, including a gas-optimized Lite Processor variant for EVM, execute pre-defined subroutines by interacting with various on-chain components. Communication between the main Authorization contract and EVM-based Processors is typically handled by connectors like Hyperlane, with specialized Encoder contracts ensuring messages are correctly formatted. This orchestrated system sees Valence Accounts holding assets or state, while abstract Libraries provide the DeFi logic (such as token exchanges, lending, or bridging) that operate on these accounts. The Authorization contract ensures only permitted subroutines, composed of these library functions, are dispatched to the EVM Processor for execution. The integration of Zero-Knowledge (ZK) verification further enhances this system by enabling complex operations to be proven correct off-chain, with the ZK proof facilitating secure and efficient state updates on the EVM chain.

More information about the Valence protocol architecture, details about the CosmWasm and EVM implementations, as well as context on libraries and configuration can be found in the [Valence docs](https://docs.valence.zone/).

## Cross-Chain Vaults and the One-Way Vault

Cross-chain vaults are smart contracts that allow users to deposit assets on one blockchain and utilize or earn yield with them on another, without manually bridging assets. They abstract away the complexities of inter-chain communication and asset transfers, often relying on messaging protocols like Hyperlane to coordinate actions between networks. These vaults typically issue tokenized shares (like ERC4626 tokens) representing the depositor's stake, which can then be redeemed for the underlying assets, potentially plus accrued yield.

The `OneWayVault.sol` contract within the Valence Protocol implements a specific type of cross-chain vault. It facilitates deposits of an underlying asset on an EVM-compatible source chain. Users receive ERC4626-compliant vault shares in return. The core "one-way" functionality means that while deposits happen on the source EVM chain, withdrawal requests initiated on this chain are intended to be processed and fulfilled on a separate destination chain (e.g., a Cosmos-based chain like Neutron). This design is particularly useful for strategies where assets are deployed or managed on a different blockchain ecosystem, with the `OneWayVault` acting as the entry point and share ledger on the EVM side.

## Source Code

Within the [valence-protocol](https://github.com/timewave-computer/valence-protocol) repo the EVM contracts that should be included in the audit scope can be found at the following locations. The number of lines of code, with and without comments, is provided for convenience.

solidity/
└── src/                                      LOC  | LOC + comments
    ├── accounts/                             -----|-----
    │   ├── Account.sol                       34   | 78
    │   └── BaseAccount.sol                   5    | 10
    ├── authorization/
    │   └── Authorization.sol                 284  | 558
    ├── processor/
    │   ├── ProcessorBase.sol                 169  | 299
    │   ├── LiteProcessor.sol                 70   | 117
    │   ├── libs/
    │   │   ├── ProcessorErrors.sol           11   | 12
    │   │   ├── ProcessorEvents.sol           9    | 28
    │   │   └── ProcessorMessageDecoder.sol   35   | 64
    │   └── interfaces/
    │       ├── ICallback.sol                 4    | 11
    │       ├── IProcessor.sol                31   | 69
    │       └── IProcessorMessageTypes.sol    85   | 192
    ├── vaults/
    │   └── OneWayVault.sol                   311  | 557
    └── verification/
        ├── SP1VerificationGateway.sol        23   | 50
        └── VerificationGateway.sol           31   | 74
                                              -----|-----
                                       Total: 1102 | 2119

## Tests

The coorosponding solidity tests for those contracts can be found at the following file locations:

solidity/
└── test/
    ├── accounts/
    │   └── BaseAccount.t.sol
    ├── authorization/
    │   ├── AuthorizationStandard.t.sol
    │   └── AuthorizationZK.t.sol
    ├── processor/
    │   ├── LiteProcessor.t.sol
    │   └── ProcessorMessageDecoder.t.sol
    └── vaults/
        └── OneWayVault.t.sol

## Contract Descriptions

### `solidity/src/accounts/Account.sol`
This contract serves as the base for user accounts within the Valence system, managing ownership and approved library interactions. It allows the owner to authorize specific library contracts that can then execute arbitrary calls on behalf of the account. This mechanism enables modularity and upgradability of account functionality by delegating logic to external libraries.

### `solidity/src/accounts/BaseAccount.sol`
This provides a minimal, concrete implementation of the `Account` contract. It directly inherits all functionality from `Account` without adding any new features or overriding existing ones. This contract acts as a simple, ready-to-deploy account that can be owned and can interact with approved libraries.

### `solidity/src/authorization/Authorization.sol`
This contract is a central component for managing permissions and routing execution instructions to Processor contracts. It supports both standard address-based authorizations and Zero-Knowledge proof-based authorizations for enhanced security and flexibility. As a middleware, it controls which users or ZK proofs can trigger specific operations through the connected Processor, and it handles callbacks after execution.

### `solidity/src/processor/ProcessorBase.sol`
This abstract contract provides the foundational logic for all Processor contracts in the Valence EVM system. It handles core functionalities like pausing/resuming operations, managing authorized addresses that can interact with it directly, and processing atomic and non-atomic subroutines. It also includes logic for sending callbacks, potentially via Hyperlane, to an `authorizationContract` on a different domain.

### `solidity/src/processor/LiteProcessor.sol`
This contract is a gas-optimized version of a Processor, inheriting from `ProcessorBase` but without message queuing features. It directly handles incoming messages, either from Hyperlane or authorized addresses, and executes them based on their type (e.g., Pause, Resume, SendMsgs). This streamlined design is intended for scenarios where the overhead of queue management is not required, providing a more lightweight execution environment.

### `solidity/src/processor/libs/ProcessorErrors.sol`
This library contract defines custom error types used throughout the Processor contracts and their associated components. These custom errors allow for more gas-efficient error handling and provide clearer reasons for transaction reversions. They help in identifying specific failure conditions like unauthorized access, processor being paused, or invalid operations.

### `solidity/src/processor/libs/ProcessorEvents.sol`
This library defines a set of events emitted by the Processor contracts to log significant actions and state changes. These events include notifications for when the processor is paused or resumed, when authorized addresses are added or removed, and when a callback is sent. These events are crucial for off-chain monitoring and an auditable trail of the processor's activity.

### `solidity/src/processor/libs/ProcessorMessageDecoder.sol`
This library provides utility functions for decoding various types of messages that the Processor contracts can handle. It centralizes the logic for parsing byte arrays into structured message types defined in `IProcessorMessageTypes`. This ensures consistent message handling and simplifies the Processor contracts' logic.

### `solidity/src/processor/interfaces/ICallback.sol`
This interface defines a standard for contracts that wish to receive and handle callbacks from a Processor. It mandates the implementation of a `handleCallback` function, which will be invoked by a Processor after executing a message. This allows other contracts, typically an Authorization contract, to react to the outcome of processed operations.

### `solidity/src/processor/interfaces/IProcessor.sol`
This interface defines the structures and enums related to the results of subroutine executions performed by Processor contracts. It includes types for `SubroutineResult`, `Callback`, and `ExecutionResult`, standardizing how outcomes like success, rejection, or partial execution are reported. This allows for clear communication of execution status between the Processor and other interacting contracts.

### `solidity/src/processor/interfaces/IProcessorMessageTypes.sol`
This interface centralizes definitions for all message structures and enums used by Processor contracts. It specifies different message types (e.g., Pause, SendMsgs), priorities, subroutine types (Atomic, NonAtomic), and the detailed structures for these messages. This ensures a consistent and well-defined format for all communications with and within the Processor components.

### `solidity/src/vaults/OneWayVault.sol`
This contract implements a one-way, tokenized vault adhering to the ERC-4626 standard, designed for cross-domain operations. It allows users to deposit assets on one chain, with withdrawals processed as requests to a different chain, and includes features like fee collection, a strategist-controlled redemption rate, and deposit caps. The vault also manages fee distribution between a designated platform and strategist, and supports pausability for operational control.

### `solidity/src/verification/SP1VerificationGateway.sol`
This contract is a specific implementation of the `VerificationGateway` tailored for the SP1 ZK-proving system by Succinct. It allows users to register SP1 verification keys (VKs) and provides a `verify` function that uses an `ISP1Verifier` to validate proofs against these registered VKs. This enables the Valence system to integrate with SP1 for ZK-proof-based authorizations and operations.

### `solidity/src/verification/VerificationGateway.sol`
This abstract, upgradeable contract serves as a generic base for different ZK verification systems within Valence Protocol. It manages the registration of program verification keys (VKs) by developers for specific registries and defines a standard interface for proof verification. Concrete implementations, like `SP1VerificationGateway`, will provide the specific logic for interacting with a particular ZK verifier.

## Auditor Areas of Focus

While automated testing is valuable, we encourage auditors to give particular attention to the following areas where subtle logic flaws, complex interactions, or specific economic attack vectors may be less likely to be discovered by such tools:

1.  Complex financial logic and economic exploits in `OneWayVault.sol`:
    *   Subtle fee and share value miscalculations: Analyze the interactions between deposit/mint fees (`calculateDepositFee`, `calculateMintFee`), `feesOwedInAsset` accumulation, the `_distributeFees` mechanism, and `redemptionRate` changes. Focus on scenarios involving multiple sequential operations (deposits, rate updates, fee distributions) to ensure no unintended value leakage, unfairness to users, or potential for economic manipulation (e.g., front-running rate updates).
    *   Order of operations and atomicity: For example, the `update` function distributing fees *before* setting a new `redemptionRate`. Verify its atomicity and that this specific ordering doesn't create exploitable interim states.
    *   Precision and rounding issues: While fuzzers might catch high level errors, manually review `_convertToAssets` and `_convertToShares` for subtle precision loss or rounding behaviors that could be exploited over time or with specific transaction amounts, especially in conjunction with `ONE_SHARE` and dynamic `redemptionRate`.

2.  ZK Proof System Integration and Logic (`Authorization.sol`, `VerificationGateway.sol`, `SP1VerificationGateway.sol`):
    *   Semantic correctness of ZK Authorization: Look at the logic integrating ZK proofs into the authorization flow. This includes ensuring the correct verification key (`vk`) is fetched and used for the given `msg.sender` and `registry`.
    *   Robustness of replay protection: Scrutinize the ZK proof replay prevention mechanisms (`validateBlockNumberExecution`, `zkAuthorizationLastExecutionBlock`). Consider scenarios where an attacker might try to reuse proofs across different contexts or if block number handling has edge cases.

3.  Sophisticated reentrancy and cross-contract interaction vulnerabilities:
    *   Multi-step/cross-contract reentrancy: While standard reentrancy guards are in place, investigate potential for more complex reentrancy patterns. This includes calls originating from `Account.sol#execute`, callbacks to `Authorization.sol#handleCallback`, or interactions during the `OneWayVault.sol`'s deposit/withdrawal processes, especially if any of these interact with untrusted or complex external contracts.

4.  Access control logic under complex conditions:
    *   Nuances in `Authorization.sol`: Examine scenarios where combinations of standard authorizations, ZK authorizations, and admin roles might lead to unintended effective permissions or bypasses that are not obvious from individual function checks.

5.  Strategic Gas Manipulation and Denial of Service:
    *   Economically unviable operations: Look for patterns where an attacker could manipulate state (e.g., by creating many small withdrawal requests or specific queue states) to make certain legitimate operations for other users prohibitively expensive, without necessarily causing an immediate out-of-gas revert that a fuzzer might catch.