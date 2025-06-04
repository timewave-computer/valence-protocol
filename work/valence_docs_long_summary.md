# Valence Protocol Documentation Summary

This document provides 6-sentence summaries of each markdown document in the Valence Protocol documentation.

## Main Documentation

### Introduction
Valence is a unified development environment that enables building trust-minimized cross-chain DeFi applications called Valence Programs. These programs are easy to understand and quick to deploy, often requiring only a configuration file with no custom code. Valence Programs are extensible, allowing new DeFi integrations to be written in hours if not supported out of the box. The protocol offers DeFi protocols a third choice beyond using multisigs or writing custom smart contracts for cross-chain operations. Programs can be configured to bridge tokens, deposit into vaults, unwind positions after certain dates, and delegate parameter changes to committees within specified ranges. This provides a secure solution that meets protocol needs without trusting multisigs or writing complex smart contracts.

[docs/src/introduction.md](docs/src/introduction.md)

### Security
Valence Programs have been independently audited with audit reports available in the GitHub repository. Security-related issues should be disclosed responsibly by contacting the Timewave team. The protocol maintains transparency by making audit reports publicly accessible. Users can find comprehensive security documentation in the main audits directory. The team follows responsible disclosure practices for vulnerability reporting. Contact information is provided for security researchers who discover potential issues.

[docs/src/security.md](docs/src/security.md)

## Component Overview

### Valence Programs Overview
There are two ways to execute Valence Programs: on-chain execution and off-chain execution via ZK coprocessor. On-chain execution currently supports CosmWasm and EVM with SVM support coming soon. The on-chain execution model includes domains, accounts, libraries and functions, programs and authorizations, and middleware components. Off-chain execution utilizes the Valence ZK System for more scalable cross-chain computation. The ZK approach aims to move as much computation off-chain as possible since it provides better scalability. The documentation covers both execution models with detailed breakdowns of their respective components.

[docs/src/components/_overview.md](docs/src/components/_overview.md)

### Domains
A domain is an environment where program components can be instantiated and deployed. Domains are defined by three properties: the blockchain name (e.g., Neutron, Osmosis, Ethereum), the execution environment (e.g., CosmWasm, EVM, SVM), and the bridge type used from the main domain to other domains (e.g., Polytone over IBC, Hyperlane). Within a blockchain ecosystem, Valence Protocol typically defines one specific domain as the main domain where supporting infrastructure components are deployed. This main domain serves as the home base for supporting execution and operations of Valence Programs. Cross-domain programs can transfer tokens between different blockchain networks through these domain definitions.

[docs/src/components/domains.md](docs/src/components/domains.md)

## Account Types

### Accounts Overview
Valence Programs perform operations on tokens across multiple domains, requiring Valence Accounts to ensure funds remain safe throughout program execution. Valence Accounts are primitives that can hold and manage tokens across different blockchain networks. These accounts can also store data that is not directly related to tokens. The section introduces different types of Valence Accounts including base accounts, storage accounts, and interchain accounts. Each account type serves specific purposes within the broader Valence Program architecture. Understanding account types is fundamental to designing effective cross-chain token operations.

[docs/src/accounts/_overview.md](docs/src/accounts/_overview.md)

### Base Accounts
A Valence Base Account is an escrow contract that holds balances for various supported token types and ensures only restricted operations can be performed on held tokens. These accounts are created on specific domains and bound to specific Valence Programs, with programs typically using multiple accounts during their lifecycle. Base Accounts are generic by nature and their use in forming a program is entirely up to the program creator. In a token swap example, programs create input accounts, transfer accounts, and output accounts across different domains like Neutron and Osmosis. Base Accounts do not perform operations by themselves; instead, operations are performed by Valence Libraries. The accounts serve as secure escrow mechanisms that maintain fund safety throughout complex cross-chain workflows.

[docs/src/accounts/base_accounts.md](docs/src/accounts/base_accounts.md)

### Storage Accounts
The Valence Storage Account is a type of Valence account that can store Valence Type data objects rather than fungible tokens. Like other accounts, Storage Accounts follow the same pattern of approving and revoking authorized libraries from posting Valence Types into the account. While regular Valence Base accounts store fungible tokens, Storage accounts are specifically designed for non-fungible objects. The account exposes execute methods for library approval/removal and storing ValenceType variants under storage keys. Storage works in an overriding manner, meaning posting data for an existing key will update its previous value. Query methods allow retrieval of stored ValenceType variants and listing of approved libraries.

[docs/src/accounts/storage_accounts.md](docs/src/accounts/storage_accounts.md)

### Interchain Accounts
A Valence Interchain Account creates an ICS-27 Interchain Account over IBC on a different domain and sends protobuf messages for remote execution. It's specifically designed to interact with other Cosmos ecosystem chains, particularly those that don't support smart contracts. These accounts are instantiated on Neutron and bound to specific Valence Programs for triggering remote message execution on other domains. The remote chain must have ICA host functionality enabled and should have an allowlist including the messages being executed. For example, a program bridging USDC from Cosmos to Ethereum via Noble Chain would use an Interchain Account to create an ICA on Noble and send messages to interact with Noble's native modules. The account provides APIs for instantiation, execution methods (including ICA message execution), and query methods for checking ICA state and remote domain information.

[docs/src/accounts/interchain_accounts.md](docs/src/accounts/interchain_accounts.md)

## Core Components

### Libraries and Functions
Valence Libraries contain business logic that can be applied to funds held by Valence Base Accounts, typically performing operations like splitting, routing, or providing liquidity on DEXes. A Valence Base Account must first approve a Valence Library before it can perform operations on the account's balances. Libraries expose Functions that are called by external parties to trigger operations on linked accounts during program execution. Valence Programs are composed of graphs of Base Accounts and Libraries to form sophisticated cross-chain workflows. Libraries play a critical role in integrating Valence Programs with existing decentralized apps and services across blockchain ecosystems. A typical pattern involves input accounts, output accounts, and libraries that facilitate token operations between them, with the example of token swaps demonstrating how multiple libraries coordinate to complete complex cross-chain transactions.

[docs/src/components/libraries_and_functions.md](docs/src/components/libraries_and_functions.md)

### Programs and Authorizations
A Valence Program is an instance of the Valence Protocol consisting of a particular arrangement and configuration of accounts and libraries across multiple domains. Programs are associated with executable Subroutines, which are vectors of Functions that can call one or more functions from single or multiple libraries within one execution domain. Subroutines can be either Non Atomic (execute functions sequentially with individual failure handling) or Atomic (execute all functions or revert all steps). The Authorizations module provides fine-grained access control, supporting schemes like permissioned actors, time-based restrictions, tokenized authorizations, expiration, and parameter constraints. The protocol provides Authorizations Contract as the user entry point for verification and message batch construction, and Processor Contract for executing message batches through execution queues. This architecture enables complex cross-chain workflows with sophisticated access control and execution guarantees.

[docs/src/components/programs_and_authorizations.md](docs/src/components/programs_and_authorizations.md)

### Middleware
The Valence Middleware provides a unified interface for the Valence Type system through brokers, type registries, and Valence types. Middleware brokers manage the lifecycle of middleware instances and their associated types. Middleware Type Registries unify sets of foreign types for use in Valence Programs. Valence Types serve as canonical representations of various external domain implementations. Valence Asserter enables programs to assert specific predicates during runtime for conditional execution of functions based on predicate evaluation. The middleware system is designed to be modifiable though the documentation notes this section is still work in progress.

[docs/src/components/middleware.md](docs/src/components/middleware.md)

## Valence ZK System

### ZK System Overview
The Valence Zero-Knowledge system facilitates execution of complex or private computations off-chain with correctness verified on-chain through cryptographic proofs. The system integrates an off-chain ZK Coprocessor Service with on-chain smart contracts, primarily Authorization and VerificationGateway contracts. ZK proofs enable one party to prove statement validity without revealing information beyond the statement's correctness, allowing computationally intensive tasks to be executed off-chain by guest programs. The ZK Coprocessor runs guest programs and generates cryptographic proofs attesting to execution correctness, which are then submitted to on-chain contracts for verification. This model brings advantages including reduced gas costs, increased transaction throughput, private data handling capability, and support for more sophisticated logic than feasible purely on-chain. Key technical challenges include encoding blockchain state into formats suitable for zero-knowledge proofs and managing cross-chain state dependencies.

[docs/src/zk/_overview.md](docs/src/zk/_overview.md)

### ZK System Architecture
The Valence ZK system comprises several key components with distinct responsibilities for off-chain computation and on-chain verification. The ZK Coprocessor Service operates off-chain as a persistent service managing guest programs, executing them with specific inputs, and generating proofs using underlying zkVMs like SP1. Guest Programs consist of Controller (Wasm-compiled Rust code) and ZK Circuit components, where Controllers process input data and coordinate proof generation while Circuits perform core computations and produce public outputs. The Authorization Contract serves as the entry point for submitting ZK proofs for verification, handling authorization logic and replay protection. The VerificationGateway Contract performs cryptographic verification using stored Verification Keys for registered guest programs. The system follows deployment flows where developers build and register programs, and runtime flows where strategists request proof generation and submit verified proofs to trigger on-chain execution. Operations proceed through development/deployment, proof request/generation, proof submission/verification, and final on-chain processing stages.

[docs/src/zk/01_system_overview.md](docs/src/zk/01_system_overview.md)

### Developing Coprocessor Apps
Developing Valence Coprocessor Apps involves creating Zero-Knowledge applications based on the valence-coprocessor-app template with two main components: Controller and ZK Circuit. The Controller crate compiles to Wasm and runs in the Coprocessor's sandboxed environment, handling input arguments, processing them into witness data for the ZK circuit, and managing proof results through entrypoint functions. The Circuit crate defines the ZK Circuit containing computations and assertions whose correctness will be proven, receiving witness data from the Controller and producing public output. Development workflow includes environment setup with Docker and Rust toolchain, ZK Circuit development defining computations and public outputs, Controller development for parsing inputs and witness generation, application build and deployment using cargo-valence CLI, proof generation requests with JSON inputs, and proof retrieval from the program's virtual filesystem. Guest programs can incorporate verifiable external state from blockchains like Ethereum through state proof services, allowing ZK applications to react to off-chain data in a trust-minimized way. The development process enables iterative testing and refinement of ZK guest programs with comprehensive tooling support.

[docs/src/zk/02_developing_coprocessor_apps.md](docs/src/zk/02_developing_coprocessor_apps.md)

### On-Chain Integration
ZK proof integration with on-chain contracts involves submitting ZK proofs and associated public data to the Authorization.sol contract, which collaborates with VerificationGateway.sol for cryptographic verification. Key data for on-chain interaction includes the ZK proof (cryptographic proof data from the Coprocessor) and the circuit's public output (Vec<u8> representing the proven statement for the Processor contract). Off-chain systems must construct a ZKMessage containing registry ID, block number for replay protection, authorization contract address, and processor message derived from circuit output. On-chain verification follows a sequence: Authorization.sol performs initial checks for sender authorization and replay protection, delegates verification to VerificationGateway.sol which retrieves the appropriate Verification Key and performs cryptographic verification using the ZK proof and public inputs. Upon successful verification, Authorization.sol dispatches the processorMessage to the Processor.sol contract for execution. The public inputs include a 32-byte Coprocessor Root hash (containing integrity commitments for all relevant domain state) followed by circuit-specific output data. This integration pathway ensures off-chain computations can be securely acted upon by on-chain contracts once proven correct.

[docs/src/zk/03_onchain_integration.md](docs/src/zk/03_onchain_integration.md)

### ZK Coprocessor Internals
The Valence ZK Coprocessor is designed as a persistent off-chain service with an architectural separation between the coprocessor (handling API requests, controller execution, and virtual filesystem management) and the prover (dedicated high-performance proof generation). The main components include an API Layer exposing REST endpoints for deploying programs and requesting proofs, Request Management & Database for validating requests and maintaining persistent storage, Controller Executor providing isolated WebAssembly runtime for controller code, Proving Engine Integration orchestrating ZK proof generation using zkVM systems, and Virtual Filesystem Manager allocating FAT-16 based filesystems to guest programs. The Coprocessor prepends a 32-byte hash to application-specific public outputs forming complete "public inputs" cryptographically bound to proofs. Task lifecycle involves proof generation requests progressing through queuing, controller execution, circuit proving, and proof delivery stages with persistent job queues enabling efficient concurrent request handling. Guest programs can incorporate external blockchain state through structured integration patterns where external state proof services query desired state and construct Merkle proofs, with Controllers incorporating this external state into witness preparation for Circuit logic. The trust model attests that given provided inputs (including externally proven state), the circuit executed correctly to produce specified outputs.

[docs/src/zk/04_coprocessor_internals.md](docs/src/zk/04_coprocessor_internals.md)

### Sparse Merkle Trees
A sparse Merkle tree (SMT) is a specialized Merkle tree with leaf indices defined by an injective function from predefined arguments, where verification keys of ZK circuits serve as indices for available programs. SMTs provide efficient data structures for validating membership of leaf nodes within sets in logarithmic time, making them well-suited for large sets and random insertion patterns. Merkle proofs consist of sibling node arrays outlining paths to commitment roots, allowing verifiers to validate membership without trusting the source through single hash commitments. Sparse data structures use deterministic leaf indices making them agnostic to input order, forming unordered sets with consistent equivalence regardless of item insertion sequence and supporting both membership and non-membership proofs. The Valence SMT uses hashes of verifying keys as indices with tree depth adapting to represent the smallest value required for leaf-to-root traversal given the number of elements. Precomputed empty subtrees optimize performance by avoiding recomputation of known constant values for empty positions, enabling efficient management and verification of large collections of authenticated data including ZK proofs and program state commitments.

[docs/src/zk/05_sparse_merkle_trees.md](docs/src/zk/05_sparse_merkle_trees.md)

## Authorizations & Processors

### Authorizations & Processors Overview
The Authorization and Processor contracts are foundational pieces of the Valence Protocol that enable on-chain and cross-chain execution of Valence Programs while enforcing access control through Authorizations. The rationale includes providing users with a single point of entry to interact with Valence Programs that can have libraries and accounts deployed on multiple chains. All user authorizations for multiple domains are centralized in a single place for easy application control. A single Processor address executes messages for all contracts in a domain using execution queues, requiring only ticking of the Processor contract to route and execute messages through queues. The system enables easy creation, editing, and removal of different application permissions. This architecture simplifies management of complex multi-chain applications while maintaining security and access control.

[docs/src/authorizations_processors/_overview.md](docs/src/authorizations_processors/_overview.md)

### Authorization Contract
The Authorization contract is a single contract deployed on the main domain that defines authorizations for the top-level application, which can include libraries in different domains (chains). For each domain, there is one Processor responsible for executing functions on libraries, with the Authorization contract connecting to all Processor contracts using connectors like Polytone Note or Hyperlane Mailbox. The Authorization contract routes message batches to the appropriate domain for execution. For each external domain, a proxy contract in the main domain receives callbacks sent from the processor with ExecutionResult information for MessageBatch operations. The contract is instantiated once at the beginning and used throughout the entire top-level application lifetime. Users never interact directly with individual Smart Contracts of each program, but exclusively with the Authorization contract.

[docs/src/authorizations_processors/authorization_contract.md](docs/src/authorizations_processors/authorization_contract.md)

### Processor
The Processor is currently available for CosmWasm Execution Environment and contains full functionality for handling two execution queues: High and Med, allowing different priorities for message batches. The Authorization contract sends message batches to the Processor specifying the priority queue for enqueuing. The Processor can be ticked permissionlessly, triggering execution of message batches in FIFO manner with retry logic for each batch (atomic) or function (non-atomic). When the current batch at the queue top is not yet retriable, the processor rotates it to the queue back, and after successful execution or maximum retries, batches are removed with callbacks sent to the Authorization contract. The Authorization contract is the only address allowed to add message batches to execution queues and can Pause/Resume the Processor or arbitrarily remove functions or add messages at specific positions. For Atomic batches, the Processor executes all functions or none, checking RetryLogic for re-enqueuing on failure, while NonAtomic batches execute functions individually with separate retry logic and callback confirmations stored until received.

[docs/src/authorizations_processors/processor.md](docs/src/authorizations_processors/processor.md)

### Lite Processor
The Lite Processor is a simplified version optimized for EVM execution environments where gas costs are critical, executing messages directly when received rather than storing message batches. The main differences from the full Processor include no message batch storage, no retries, no function callbacks, and no queues, with immediate execution upon receiving MessageBatch from Authorization contract. The Lite Processor is not ticked but executes batches immediately, with gas costs paid by the relayer rather than the user ticking the processor. Operations like InsertAt or RemoveFrom queue are not available, limiting supported operations to Pause, Resume, and SendMsgs from the Authorization contract. The processor does not support retries or function callbacks, meaning MessageBatch execution occurs only once with NonAtomic batches unable to be confirmed asynchronously. In addition to executing batches from the Authorization contract, the Lite Processor defines authorized addresses that can send batches for execution, sending callbacks only if the sending address is a smart contract.

[docs/src/authorizations_processors/lite_processor.md](docs/src/authorizations_processors/lite_processor.md)

## Examples

### Examples Overview
The examples section provides practical demonstrations of Valence Programs that users can reference to get started with the protocol. The section includes implementations of token swap programs and crosschain vaults with their respective strategist components. Each example showcases different aspects of Valence Program capabilities from simple atomic swaps to complex cross-chain vault management. The examples serve as both educational resources and templates for developers building their own Valence Programs. The documentation provides detailed explanations of program components, flow diagrams, and implementation details for each example use case.

[docs/src/examples/_overview.md](docs/src/examples/_overview.md)

### Token Swap
The token swap example demonstrates a simple program where two parties exchange specific amounts of different tokens they each hold at a previously agreed rate. The program ensures the swap happens atomically so neither party can withdraw without completing the trade using splitter libraries and base accounts. Program components include Party A and Party B deposit accounts, splitter libraries for each party, and corresponding withdraw accounts that can be Valence Base accounts or regular chain accounts. The atomic exchange requirement is fulfilled by implementing an atomic subroutine composed of both splitter functions that must either both succeed or both fail. The Authorizations component ensures either both token transfers execute successfully or none are executed, maintaining fund safety at all times. The program demonstrates the fundamental pattern of using multiple accounts and libraries to create secure cross-chain financial operations.

[docs/src/examples/token_swap.md](docs/src/examples/token_swap.md)

### Crosschain Vaults
Crosschain vaults allow users to interact with a vault on one chain while tokens are held on another chain where yield is generated, with the initial implementation using Neutron for co-processing and Hyperlane for message passing. The example assumes users can deposit tokens into a standard ERC-4626 vault on Ethereum, receive ERC-20 shares, and issue withdrawal requests that burn shares when tokens are redeemed based on a calculated redemption rate. A permissioned Strategist is authorized to transport funds between Ethereum and Neutron where they are locked in DeFi protocols, with the redemption rate adjusted accordingly. The implementation requires libraries and accounts on both chains including deposit/withdraw accounts, bridge transfer libraries, position depositor/withdrawer libraries, and forwarder libraries for fund management. The vault contract provides an ERC-4626 interface with user methods for depositing funds and minting shares, withdrawal methods that create queued records processed at epochs, and strategist methods for updating redemption rates and managing vault operations. Program subroutines authorize the Strategist to transport funds between various accounts and update vault parameters while maintaining security through authorization constraints.

[docs/src/examples/crosschain_vaults.md](docs/src/examples/crosschain_vaults.md)

## Libraries

### Libraries Overview
The libraries section contains detailed descriptions of various libraries that can be used to rapidly build Valence cross-chain programs for each execution environment. Libraries serve as the building blocks that contain business logic for performing operations on tokens held by Valence accounts. The section is organized by execution environment, with separate documentation for CosmWasm and EVM libraries that cover different blockchain ecosystems. Each library provides specific functionality for integrating with existing decentralized applications and services. The libraries enable developers to compose sophisticated cross-chain workflows without writing custom smart contracts from scratch. The comprehensive library ecosystem allows for rapid development of complex DeFi operations across multiple blockchain networks.

[docs/src/libraries/_overview.md](docs/src/libraries/_overview.md)

### CosmWasm Libraries Overview
The CosmWasm libraries section provides detailed descriptions of all libraries available for use in CosmWasm execution environments. These libraries enable integration with Cosmos ecosystem protocols and provide essential functionality for cross-chain operations within the Cosmos network. Libraries cover a wide range of operations including liquidity provision, token swapping, bridging, staking, and position management across various Cosmos-based protocols. Each library is designed to work seamlessly with Valence accounts and can be composed together to create complex cross-chain workflows. The CosmWasm libraries support integration with major Cosmos protocols like Astroport, Osmosis, Neutron, and others through standardized interfaces. These libraries form the foundation for building sophisticated DeFi applications across the Cosmos ecosystem using Valence Programs.

[docs/src/libraries/cosmwasm/_overview.md](docs/src/libraries/cosmwasm/_overview.md)

### EVM Libraries Overview
The EVM libraries section contains detailed descriptions of all libraries available for use in EVM execution environments. These libraries enable integration with Ethereum-based protocols and provide functionality for operations on EVM-compatible chains. Libraries cover essential operations like bridging, swapping, position management, and interaction with major DeFi protocols on Ethereum and other EVM chains. Each library follows standardized interfaces that work with Valence accounts and can be composed to create sophisticated cross-chain workflows. The EVM libraries support integration with protocols like AAVE, PancakeSwap, Balancer, and various bridging solutions including CCTP and Stargate. These libraries enable developers to build complex DeFi applications that span both EVM and non-EVM chains through the Valence Protocol framework.

[docs/src/libraries/evm/_overview.md](docs/src/libraries/evm/_overview.md)

## Program Manager

### Program Manager Overview
The program manager is an off-chain tool that helps instantiate, update, and migrate programs through a comprehensive set of functions and configurations. The tool provides guides for usage including how-to documentation and build program config instructions for developers. Program manager components include manager config for tool setup, program config for instantiation, program config update for modifying existing programs, program config migrate for transitioning between programs, and library account type definitions. The manager abstracts complex functionality and allows creating programs with much less code while providing fine-grained control when needed. The tool works with the local-interchain testing framework and provides helper functions for program management throughout their lifecycle. Users can choose to use the Program Manager for simplified program creation or work directly with lower-level functions for more granular control.

[docs/src/program_manager/_overview.md](docs/src/program_manager/_overview.md)

### Program Manager How-To
The program manager requires a manager config with chain information and a funded wallet (specified via MANAGER_MNEMONIC environment variable) to perform on-chain actions. The manager is a library that can be used as a dependency in any Rust project and provides three main functions: init_program for instantiating new programs, update_program for modifying existing programs, and migrate_program for transitioning to new programs. The init_program function takes a program config to instantiate and mutates it with the instantiated program config details. The update_program function takes update instructions and returns messages that must be executed by the program owner to batch update library configs and authorizations. The migrate_program function allows disabling an old program and moving all funds to a new program by returning messages for fund transfer and program pausing that must be executed by the owner. The wallet used should not be the program owner but rather a helper wallet funded with sufficient funds to perform management actions.

[docs/src/program_manager/how_to.md](docs/src/program_manager/how_to.md)

## Testing

### Testing Overview
The testing infrastructure is built on local-interchain (a component of interchaintest) and localic-utils (a Rust library providing convenient interfaces) to create a comprehensive local testing environment. The core testing framework allows deploying and running chains locally, providing a controlled testing space for blockchain applications. The Program Manager tool helps manage programs with abstractions and helper functions for efficient program creation, though its use is optional for developers wanting fine-grained control. Testing can be done with or without the Program Manager, with examples provided for both approaches in the e2e folder. The framework provides all necessary abstractions to create programs efficiently together with local-interchain infrastructure. Multiple examples are available for different use cases, all located in the examples folder of the e2e directory.

[docs/src/testing/_overview.md](docs/src/testing/_overview.md)

### Testing Setup
Testing setup requires establishing a TestContext using TestContextBuilder with chain configurations, API URLs, artifacts directories, and transfer channels between chains. The TestContext represents the interchain environment where programs run, configured with specific chains like Neutron and Osmosis connected via IBC transfer channels. Chain configurations use ConfigChainBuilder with default configurations for supported chains, though custom configurations can be created for specific requirements. Some chains require additional setup such as registering and activating host zones for liquid staking chains like Persistence, deploying Astroport contracts, or creating Osmosis pools. Helper functions are provided for most common chain-specific setup operations with examples available in the examples folder. The setup process ensures all necessary infrastructure is in place for comprehensive testing of cross-chain programs in a local environment.

[docs/src/testing/setup.md](docs/src/testing/setup.md)

## Deployment

### Deployment Overview
The deployment section contains detailed explanations of how to deploy programs on different environments with current focus on local deployment scenarios. The documentation provides environment-specific guidance for setting up and deploying Valence Programs in various contexts. Local deployment is currently the primary documented environment with comprehensive instructions for developers. The section serves as a reference for understanding deployment processes and requirements across different target environments. Future expansion may include additional deployment environments beyond local setups. The deployment documentation complements the testing framework by providing production deployment guidance.

[docs/src/deployment/_overview.md](docs/src/deployment/_overview.md)

## Additional ZK System Components

### ZK Guest Environment
The Valence ZK Guest Environment describes the specialized, sandboxed execution environment provided by the Coprocessor for guest applications, running within constrained resources to prevent interference with the service or other programs. Each deployed guest program receives its own private virtual filesystem based on FAT-16 structure with specific limitations including three-character file extensions, case-insensitive file names, Unix-like paths, and interaction through Coprocessor service commands rather than direct OS-level I/O. The controller crate interfaces with the Coprocessor service through specific operations including signaling witness readiness after preparing ZK circuit data, receiving proof results through designated entrypoint functions, and performing filesystem operations through structured requests to store data and logs. Guest applications run with finite system resources including limited memory, CPU time, and storage space, requiring developers to focus on efficiency in controller logic for input processing and witness generation. The environment is optimized for preparing witnesses and handling results rather than performing heavy computations that are better suited for ZK circuits. Understanding these constraints enables developers to build efficient ZK applications that run effectively on the Valence Coprocessor infrastructure.

[docs/src/zk/06_guest_environment.md](docs/src/zk/06_guest_environment.md)

### State Encoding and Encoders
State encoding addresses the core challenge of compressing blockchain state into formats suitable for zero-knowledge proofs, as ZK applications must be pure functions utilizing existing state as arguments to produce evaluated output state. The Valence ZK Coprocessor leverages RISC-V zkVMs (currently SP1) to execute Rust programs and generate proofs following a workflow of application definition, key generation, proof generation, and verification with pure function constraints. The Unary Encoder compresses account state transitions into zero-knowledge proofs, transforming on-chain state mutations to ZK-provable computations with ZK applications structured as controller and circuit components where controllers process inputs and generate witnesses while circuits perform ZK-provable computations. The Merkleized Encoder handles cross-chain state dependencies using Merkle tree structures enabling parallel execution while maintaining correctness for chains with interdependencies, creating optimized verification with logarithmic efficiency where each chain needs only its state transition arguments and Merkle path to root. On-chain proof distribution provides minimal data for verification with cross-chain coordination supporting complex multi-domain programs through secure and efficient state synchronization mechanisms. The implementation represents design goals with core infrastructure existing while full state encoding and cross-chain coordination features remain in active development.

[docs/src/zk/07_state_encoding_and_encoders.md](docs/src/zk/07_state_encoding_and_encoders.md)

## Middleware System

### Middleware Overview
The Valence Middleware provides a unified interface for the Valence Type system through brokers, type registries, and Valence types. Middleware brokers manage the lifecycle of middleware instances and their associated types. Middleware Type Registries unify sets of foreign types for use in Valence Programs. Valence Types serve as canonical representations of various external domain implementations. Valence Asserter enables programs to assert specific predicates during runtime for conditional execution of functions based on predicate evaluation. The middleware system is designed to be modifiable though the documentation notes this section is still work in progress.

[docs/src/middleware/_overview.md](docs/src/middleware/_overview.md)

### Middleware Broker
Middleware brokers act as app-level integration gateways in Valence Programs, remaining agnostic to primitives including data types, functions, encoding schemes, and distributed system building blocks that may be implemented differently across domains. The problem arises because Valence Programs span multiple domains for indefinite durations while domains remain sovereign and evolve independently, requiring seamless primitive conversions with no downtime and easy developer updates. Brokers provide graceful recovery mechanisms when remote domains perform upgrades that extend types with additional fields, allowing programs to synchronize and continue operating correctly. Brokers are singleton components instantiated before program start time and referenced by addresses, enabling the same broker instance to be used across many Valence Programs with type registries indexed by semver for runtime additions. The broker interface exposes a single query with registry version and query message parameters, relaying contained RegistryQueryMsg to correct type registries and returning results to callers. This architecture reduces maintenance work by allowing one broker update to immediately become available for all Valence Programs using it.

[docs/src/middleware/broker.md](docs/src/middleware/broker.md)

## Additional Authorization & Processor Components

### Authorization Assumptions
The authorization and processor system operates under several key assumptions for fund handling, bridging, instantiation, relaying, and domain limitations. Funds cannot be sent with messages, and bidirectional message communication between domains is assumed with Authorization contracts communicating with processors and receiving execution confirmation callbacks. All contracts can be instantiated beforehand off-chain with predictable addresses following a specific flow using systems like Polytone for address prediction and contract setup. Relayers are expected to run continuously once everything is instantiated to facilitate cross-domain communication. The main domain requires tokenfactory module with no token creation fee for minting nonfungible authorization tokens at no additional cost. Actions in each authorization are currently limited to a single domain in the current version implementation. These assumptions establish the operational framework and constraints within which the authorization and processor system functions effectively.

[docs/src/authorizations_processors/assumptions.md](docs/src/authorizations_processors/assumptions.md)

## Summary Document
This comprehensive summary covers all major sections of the Valence Protocol documentation including core concepts, account types, ZK system components, authorization and processor architecture, examples, libraries, program management tools, testing framework, and deployment guidance. The documentation demonstrates Valence as a sophisticated protocol for building trust-minimized cross-chain DeFi applications through configurable programs that can operate across multiple blockchain networks with various execution environments. The protocol combines on-chain execution capabilities with advanced ZK coprocessor technology to enable scalable cross-chain operations while maintaining security and decentralization principles throughout the system architecture. 