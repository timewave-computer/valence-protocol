# Valence Protocol Documentation Summary

This document provides two-sentence summaries of all documentation files in the Valence Protocol documentation.

## Core Documentation

### Introduction
Valence is a unified development environment that enables building trust-minimized cross-chain DeFi applications called Valence Programs, which can be set up with configuration files without requiring custom code. It offers a third alternative to multisigs and custom smart contracts by providing rapid deployment of secure solutions for complex cross-chain DeFi operations like bridging and vault management.

**Reference:** [docs/src/introduction.md](../docs/src/introduction.md)

### Security
Valence Programs have been independently audited with reports available in the GitHub repository. Security issues should be disclosed responsibly by contacting the Timewave team at security@timewave.computer.

**Reference:** [docs/src/security.md](../docs/src/security.md)

## Accounts

### Accounts Overview
Valence Programs perform operations on tokens across multiple domains using a primitive called Valence Accounts to ensure funds remain safe throughout program execution. These accounts can also store data not directly related to tokens and come in different types with specific purposes.

**Reference:** [docs/src/accounts/_overview.md](../docs/src/accounts/_overview.md)

### Base Accounts
Base accounts serve as the fundamental account type in the Valence system, providing core functionality for token storage and management. They form the foundation upon which other specialized account types are built.

**Reference:** [docs/src/accounts/base_accounts.md](../docs/src/accounts/base_accounts.md)

### Interchain Accounts
Interchain accounts enable cross-chain operations by allowing accounts on one blockchain to control accounts and execute transactions on other blockchains. This functionality is crucial for Valence Programs that need to operate across multiple domains and chains.

**Reference:** [docs/src/accounts/interchain_accounts.md](../docs/src/accounts/interchain_accounts.md)

### Storage Accounts
Storage accounts are specialized accounts designed to hold and manage data that is not directly related to token operations. They provide a secure way to store program state and configuration data across the Valence system.

**Reference:** [docs/src/accounts/storage_accounts.md](../docs/src/accounts/storage_accounts.md)

## Components

### Components Overview
Valence Programs can be executed in two ways: on-chain execution supporting CosmWasm and EVM (with SVM support coming soon), and off-chain execution via ZK Coprocessor for enhanced scalability. The on-chain approach uses domains, accounts, libraries, programs/authorizations, and middleware as core components.

**Reference:** [docs/src/components/_overview.md](../docs/src/components/_overview.md)

### Domains
A domain is an environment defined by three properties: the blockchain (e.g., Neutron, Osmosis, Ethereum), the execution environment (e.g., CosmWasm, EVM, SVM), and the type of bridge used to connect to other domains. The Valence Protocol typically defines one domain as the "main domain" that serves as the home base for supporting infrastructure components.

**Reference:** [docs/src/components/domains.md](../docs/src/components/domains.md)

### Libraries and Functions
Libraries provide reusable functionality for common DeFi operations and integrations that can be composed into Valence Programs. Functions within these libraries encapsulate specific operations like token swaps, lending, or cross-chain transfers.

**Reference:** [docs/src/components/libraries_and_functions.md](../docs/src/components/libraries_and_functions.md)

### Middleware
Middleware components provide additional functionality and processing capabilities that can be inserted into the execution flow of Valence Programs. They enable features like validation, transformation, and routing of messages between different components.

**Reference:** [docs/src/components/middleware.md](../docs/src/components/middleware.md)

### Programs and Authorizations
Programs define the logic and workflow of Valence applications, while authorizations control access and permissions for program execution. Together they form the core execution framework that determines what operations can be performed and by whom.

**Reference:** [docs/src/components/programs_and_authorizations.md](../docs/src/components/programs_and_authorizations.md)

## Authorizations & Processors

### Authorization & Processors Overview
The Authorization and Processor contracts are foundational pieces that enable on-chain and cross-chain execution of Valence Programs while enforcing access control via authorizations. They provide a single point of entry for users to interact with programs that may have components deployed across multiple chains.

**Reference:** [docs/src/authorizations_processors/_overview.md](../docs/src/authorizations_processors/_overview.md)

### Assumptions
The authorization and processor system operates under specific assumptions about network behavior, consensus mechanisms, and cross-chain communication reliability. Understanding these assumptions is crucial for properly implementing and using Valence Programs in production environments.

**Reference:** [docs/src/authorizations_processors/assumptions.md](../docs/src/authorizations_processors/assumptions.md)

### Authorization Contract
The authorization contract manages permissions and access control for Valence Programs, determining who can execute specific operations. It serves as the gatekeeper that enforces security policies and user permissions across the entire program.

**Reference:** [docs/src/authorizations_processors/authorization_contract.md](../docs/src/authorizations_processors/authorization_contract.md)

### Authorization Instantiation
Authorization instantiation covers the process of deploying and setting up authorization contracts with initial configurations and permissions. This process establishes the security framework that will govern program execution throughout its lifecycle.

**Reference:** [docs/src/authorizations_processors/authorization_instantiation.md](../docs/src/authorizations_processors/authorization_instantiation.md)

### Authorization Owner Actions
Authorization owners have special privileges to modify program configurations, update permissions, and manage the overall governance of Valence Programs. These actions include adding or removing users, changing access levels, and updating program parameters.

**Reference:** [docs/src/authorizations_processors/authorization_owner_actions.md](../docs/src/authorizations_processors/authorization_owner_actions.md)

### Authorization User Actions
Regular users can interact with Valence Programs through a defined set of actions permitted by the authorization system. These actions include executing program functions, querying program state, and performing operations within their granted permissions.

**Reference:** [docs/src/authorizations_processors/authorization_user_actions.md](../docs/src/authorizations_processors/authorization_user_actions.md)

### Callbacks
Callbacks provide a mechanism for handling responses from cross-chain operations and enabling asynchronous communication between different components. They are essential for coordinating complex multi-step operations that span multiple blockchains.

**Reference:** [docs/src/authorizations_processors/callbacks.md](../docs/src/authorizations_processors/callbacks.md)

### Connectors
Connectors facilitate communication and interoperability between different blockchain networks and execution environments. They abstract the complexity of cross-chain protocols and provide standardized interfaces for cross-domain operations.

**Reference:** [docs/src/authorizations_processors/connectors.md](../docs/src/authorizations_processors/connectors.md)

### Encoding
Encoding handles the serialization and deserialization of data as it moves between different chains and execution environments. Proper encoding ensures data integrity and compatibility across the diverse ecosystem of blockchains supported by Valence.

**Reference:** [docs/src/authorizations_processors/encoding.md](../docs/src/authorizations_processors/encoding.md)

### EVM Architecture
The EVM architecture documentation explains how Valence Programs are implemented and executed on Ethereum Virtual Machine compatible blockchains. It covers the specific design patterns and smart contract structures used to support Valence functionality on EVM chains.

**Reference:** [docs/src/authorizations_processors/evm_architecture.md](../docs/src/authorizations_processors/evm_architecture.md)

### Execution Environment Differences
Different blockchain execution environments (CosmWasm, EVM, SVM) have unique characteristics and limitations that affect how Valence Programs operate. Understanding these differences is crucial for designing programs that work effectively across multiple chains.

**Reference:** [docs/src/authorizations_processors/execution_environment_differences.md](../docs/src/authorizations_processors/execution_environment_differences.md)

### Lite Processor
The lite processor is a simplified version of the full processor designed for specific use cases where full processor functionality is not required. It provides a more efficient and cost-effective option for simpler Valence Programs.

**Reference:** [docs/src/authorizations_processors/lite_processor.md](../docs/src/authorizations_processors/lite_processor.md)

### Processor
The processor is a core component that executes messages and operations for Valence Programs using execution queues to route and process requests. It serves as the execution engine that coordinates operations across multiple domains and ensures proper sequencing of program operations.

**Reference:** [docs/src/authorizations_processors/processor.md](../docs/src/authorizations_processors/processor.md)

### Processor Contract
The processor contract implements the on-chain logic for processing and executing Valence Program operations. It manages execution queues, handles cross-chain communication, and ensures proper execution of program logic according to authorization permissions.

**Reference:** [docs/src/authorizations_processors/processor_contract.md](../docs/src/authorizations_processors/processor_contract.md)

## Program Manager

### Program Manager Overview
The Program Manager provides tools and utilities for managing the lifecycle of Valence Programs, from initial configuration to deployment and ongoing maintenance. It simplifies the process of creating and managing complex multi-chain applications.

**Reference:** [docs/src/program_manager/_overview.md](../docs/src/program_manager/_overview.md)

### Build Program Config
The build program configuration process involves defining the structure, parameters, and dependencies of a Valence Program before deployment. This includes specifying which libraries to use, how components should be connected, and what permissions should be granted.

**Reference:** [docs/src/program_manager/build_program_config.md](../docs/src/program_manager/build_program_config.md)

### How To
The how-to guide provides step-by-step instructions for common Program Manager tasks and workflows. It serves as a practical guide for developers looking to quickly get started with creating and deploying Valence Programs.

**Reference:** [docs/src/program_manager/how_to.md](../docs/src/program_manager/how_to.md)

### Library Account Type
Library account types define the different categories of accounts that can be used with various libraries in the Valence ecosystem. Each type has specific characteristics and capabilities that determine how it can be used within different program contexts.

**Reference:** [docs/src/program_manager/library_account_type.md](../docs/src/program_manager/library_account_type.md)

### Manager Config
The manager configuration defines the overall settings and parameters for the Program Manager itself. This includes global settings that affect how programs are built, deployed, and managed across the Valence ecosystem.

**Reference:** [docs/src/program_manager/manager_config.md](../docs/src/program_manager/manager_config.md)

### Program Configs - Instantiate
The instantiate configuration defines how a Valence Program should be initially deployed and set up on target blockchains. It specifies the initial state, parameters, and account configurations that will be established when the program first starts running.

**Reference:** [docs/src/program_manager/program_configs/instantiate.md](../docs/src/program_manager/program_configs/instantiate.md)

### Program Configs - Migrate
The migrate configuration handles the process of upgrading or moving Valence Programs from one version to another or between different environments. It ensures that program state and functionality are preserved during transitions while enabling necessary updates.

**Reference:** [docs/src/program_manager/program_configs/migrate.md](../docs/src/program_manager/program_configs/migrate.md)

### Program Configs - Update
The update configuration manages how Valence Programs can be modified and updated after deployment. It defines which parameters can be changed, who has permission to make updates, and how updates are applied safely without disrupting ongoing operations.

**Reference:** [docs/src/program_manager/program_configs/update.md](../docs/src/program_manager/program_configs/update.md)

## Middleware

### Middleware Overview
Middleware components provide additional processing capabilities that can be inserted into Valence Program execution flows. They enable features like message validation, transformation, routing, and other cross-cutting concerns.

**Reference:** [docs/src/middleware/_overview.md](../docs/src/middleware/_overview.md)

### Broker
The broker middleware manages message routing and delivery between different components and domains in Valence Programs. It handles the complexity of cross-chain communication and ensures messages reach their intended destinations reliably.

**Reference:** [docs/src/middleware/broker.md](../docs/src/middleware/broker.md)

### Type Registry
The type registry maintains a catalog of all message types and data structures used throughout the Valence ecosystem. It enables proper serialization, deserialization, and validation of data as it moves between different components and chains.

**Reference:** [docs/src/middleware/type_registry.md](../docs/src/middleware/type_registry.md)

### Valence Asserter
The Valence Asserter middleware provides validation and assertion capabilities to ensure program operations meet specified conditions before execution. It acts as a safety mechanism to prevent invalid or potentially harmful operations from being executed.

**Reference:** [docs/src/middleware/valence_asserter.md](../docs/src/middleware/valence_asserter.md)

### Valence Types
Valence Types define the core data structures and message formats used throughout the Valence Protocol. These standardized types ensure compatibility and proper communication between different components and chains.

**Reference:** [docs/src/middleware/valence_types.md](../docs/src/middleware/valence_types.md)

## Testing

### Testing Overview
The testing framework provides tools and utilities for testing Valence Programs in controlled environments before production deployment. It supports both unit testing of individual components and integration testing of complete cross-chain workflows.

**Reference:** [docs/src/testing/_overview.md](../docs/src/testing/_overview.md)

### Setup
The testing setup documentation covers how to configure testing environments, prepare test data, and establish the necessary infrastructure for testing Valence Programs. It includes both local testing setups and testnet configurations.

**Reference:** [docs/src/testing/setup.md](../docs/src/testing/setup.md)

### Testing with Program Manager
Testing with the Program Manager involves using the Program Manager's built-in testing capabilities to validate program configurations and deployments. This approach provides comprehensive testing of the entire program lifecycle from configuration to execution.

**Reference:** [docs/src/testing/with_program_manager.md](../docs/src/testing/with_program_manager.md)

### Testing without Program Manager
Testing without the Program Manager covers manual testing approaches and direct interaction with Valence components. This method provides more granular control over testing scenarios but requires more setup and configuration.

**Reference:** [docs/src/testing/without_program_manager.md](../docs/src/testing/without_program_manager.md)

## Libraries

### Libraries Overview
Libraries provide pre-built functionality for common DeFi operations and integrations that can be composed into Valence Programs. They are organized by execution environment (CosmWasm, EVM) and cover various protocols and use cases.

**Reference:** [docs/src/libraries/_overview.md](../docs/src/libraries/_overview.md)

## CosmWasm Libraries

### CosmWasm Libraries Overview
CosmWasm libraries provide functionality specific to the CosmWasm execution environment, particularly for Cosmos ecosystem integrations. These libraries enable seamless interaction with Cosmos-based DeFi protocols and cross-chain operations via IBC.

**Reference:** [docs/src/libraries/cosmwasm/_overview.md](../docs/src/libraries/cosmwasm/_overview.md)

### Astroport LPer
The Astroport LPer library enables liquidity provision to Astroport decentralized exchange pools on Terra and other Cosmos chains. It handles the complexities of pool interactions, token swaps, and liquidity management within Valence Programs.

**Reference:** [docs/src/libraries/cosmwasm/astroport_lper.md](../docs/src/libraries/cosmwasm/astroport_lper.md)

### Astroport Withdrawer
The Astroport Withdrawer library manages the withdrawal of liquidity from Astroport pools and the claiming of rewards. It provides functions for removing liquidity positions and converting them back to underlying tokens.

**Reference:** [docs/src/libraries/cosmwasm/astroport_withdrawer.md](../docs/src/libraries/cosmwasm/astroport_withdrawer.md)

### Drop Liquid Staker
The Drop Liquid Staker library enables staking operations with liquid staking protocols in the Cosmos ecosystem. It allows Valence Programs to stake tokens while maintaining liquidity through receipt tokens that can be used in other DeFi operations.

**Reference:** [docs/src/libraries/cosmwasm/drop_liquid_staker.md](../docs/src/libraries/cosmwasm/drop_liquid_staker.md)

### Drop Liquid Unstaker
The Drop Liquid Unstaker library handles the unstaking process for liquid staking positions, managing the conversion from liquid staking tokens back to underlying staked assets. It handles unbonding periods and the complexities of the unstaking workflow.

**Reference:** [docs/src/libraries/cosmwasm/drop_liquid_unstaker.md](../docs/src/libraries/cosmwasm/drop_liquid_unstaker.md)

### Forwarder
The Forwarder library provides message forwarding capabilities for routing operations between different components and chains. It acts as an intermediary that can transform, validate, and redirect messages according to specified rules.

**Reference:** [docs/src/libraries/cosmwasm/forwarder.md](../docs/src/libraries/cosmwasm/forwarder.md)

### Generic IBC Transfer
The Generic IBC Transfer library provides standardized functionality for transferring tokens between different chains using the Inter-Blockchain Communication (IBC) protocol. It abstracts the complexity of IBC transfers and provides a simple interface for cross-chain token movements.

**Reference:** [docs/src/libraries/cosmwasm/generic_ibc_transfer.md](../docs/src/libraries/cosmwasm/generic_ibc_transfer.md)

### ICA CCTP Transfer
The ICA CCTP Transfer library combines Interchain Account (ICA) functionality with Circle's Cross-Chain Transfer Protocol (CCTP) for efficient cross-chain transfers. It enables automated cross-chain operations while leveraging CCTP's native bridge capabilities for supported assets.

**Reference:** [docs/src/libraries/cosmwasm/ica_cctp_transfer.md](../docs/src/libraries/cosmwasm/ica_cctp_transfer.md)

### ICA IBC Transfer
The ICA IBC Transfer library uses Interchain Accounts to perform IBC transfers on behalf of Valence Programs. It enables programmatic control of accounts on remote chains for executing complex cross-chain workflows.

**Reference:** [docs/src/libraries/cosmwasm/ica_ibc_transfer.md](../docs/src/libraries/cosmwasm/ica_ibc_transfer.md)

### Neutron IBC Transfer
The Neutron IBC Transfer library provides Neutron-specific optimizations and features for IBC token transfers. It leverages Neutron's unique capabilities as a consumer chain to provide enhanced cross-chain functionality.

**Reference:** [docs/src/libraries/cosmwasm/neutron_ibc_transfer.md](../docs/src/libraries/cosmwasm/neutron_ibc_transfer.md)

### Neutron IC Querier
The Neutron IC Querier library enables querying of remote chain state using Neutron's Interchain Queries feature. It allows Valence Programs to access real-time data from other blockchains to make informed decisions without requiring additional infrastructure.

**Reference:** [docs/src/libraries/cosmwasm/neutron_ic_querier.md](../docs/src/libraries/cosmwasm/neutron_ic_querier.md)

### Osmosis CL LPer
The Osmosis CL LPer library provides functionality for adding liquidity to Osmosis Concentrated Liquidity pools. It handles the complexities of concentrated liquidity positions, including tick range management and position optimization.

**Reference:** [docs/src/libraries/cosmwasm/osmosis_cl_lper.md](../docs/src/libraries/cosmwasm/osmosis_cl_lper.md)

### Osmosis CL Withdrawer
The Osmosis CL Withdrawer library manages the withdrawal of liquidity from Osmosis Concentrated Liquidity positions. It handles position closure, fee collection, and the conversion of concentrated liquidity positions back to underlying tokens.

**Reference:** [docs/src/libraries/cosmwasm/osmosis_cl_withdrawer.md](../docs/src/libraries/cosmwasm/osmosis_cl_withdrawer.md)

### Osmosis GAMM LPer
The Osmosis GAMM LPer library enables liquidity provision to Osmosis's traditional automated market maker (GAMM) pools. It provides functionality for joining pools, managing positions, and optimizing liquidity allocation across different pool types.

**Reference:** [docs/src/libraries/cosmwasm/osmosis_gamm_lper.md](../docs/src/libraries/cosmwasm/osmosis_gamm_lper.md)

### Osmosis GAMM Withdrawer
The Osmosis GAMM Withdrawer library handles the withdrawal of liquidity from Osmosis GAMM pools and the collection of trading fees and rewards. It manages the process of exiting liquidity positions and converting them back to underlying assets.

**Reference:** [docs/src/libraries/cosmwasm/osmosis_gamm_withdrawer.md](../docs/src/libraries/cosmwasm/osmosis_gamm_withdrawer.md)

### Reverse Splitter
The Reverse Splitter library aggregates multiple token streams or positions back into a single output, essentially performing the opposite operation of a splitter. It's useful for consolidating positions or combining multiple income streams into a single destination.

**Reference:** [docs/src/libraries/cosmwasm/reverse_splitter.md](../docs/src/libraries/cosmwasm/reverse_splitter.md)

### Splitter
The Splitter library divides incoming tokens or positions across multiple destinations according to specified ratios. It enables the distribution of assets or yields to multiple recipients while maintaining proportional allocations.

**Reference:** [docs/src/libraries/cosmwasm/splitter.md](../docs/src/libraries/cosmwasm/splitter.md)

### Supervaults LPer
The Supervaults LPer library provides liquidity provision functionality for Supervaults, a yield farming protocol in the Cosmos ecosystem. It handles the complexities of vault interactions and optimizes liquidity allocation for maximum returns.

**Reference:** [docs/src/libraries/cosmwasm/supervaults_lper.md](../docs/src/libraries/cosmwasm/supervaults_lper.md)

### Supervaults Withdrawer
The Supervaults Withdrawer library manages the withdrawal process from Supervaults positions, including the collection of earned rewards and fees. It handles the conversion of vault shares back to underlying assets and the claiming of accumulated yields.

**Reference:** [docs/src/libraries/cosmwasm/supervaults_withdrawer.md](../docs/src/libraries/cosmwasm/supervaults_withdrawer.md)

## EVM Libraries

### EVM Libraries Overview
EVM libraries provide functionality specific to Ethereum Virtual Machine compatible blockchains, enabling integration with Ethereum DeFi protocols. These libraries support operations on Ethereum mainnet and various Layer 2 solutions.

**Reference:** [docs/src/libraries/evm/_overview.md](../docs/src/libraries/evm/_overview.md)

### Aave Position Manager
The Aave Position Manager library enables interaction with the Aave lending protocol for supplying, borrowing, and managing collateralized debt positions. It provides functions for optimizing lending yields and managing liquidation risks within Valence Programs.

**Reference:** [docs/src/libraries/evm/aave_position_manager.md](../docs/src/libraries/evm/aave_position_manager.md)

### Balancer V2 Swap
The Balancer V2 Swap library provides functionality for executing token swaps through Balancer V2 decentralized exchange pools. It handles the complexities of multi-hop routing and pool selection to optimize trade execution and minimize slippage.

**Reference:** [docs/src/libraries/evm/balancer_v2_swap.md](../docs/src/libraries/evm/balancer_v2_swap.md)

### CCTP Transfer
The CCTP Transfer library enables cross-chain transfers using Circle's Cross-Chain Transfer Protocol for USDC and other supported assets. It provides native bridging capabilities with enhanced security and reduced settlement times compared to traditional bridges.

**Reference:** [docs/src/libraries/evm/cctp_transfer.md](../docs/src/libraries/evm/cctp_transfer.md)

### Forwarder
The EVM Forwarder library provides message forwarding and routing capabilities specific to EVM-compatible chains. It enables the creation of proxy contracts and message relay systems for complex multi-step operations.

**Reference:** [docs/src/libraries/evm/forwarder.md](../docs/src/libraries/evm/forwarder.md)

### IBC Eureka Transfer
The IBC Eureka Transfer library implements experimental IBC functionality for Ethereum and other EVM chains through the Eureka protocol. It enables IBC-style communication and token transfers between Cosmos and EVM ecosystems.

**Reference:** [docs/src/libraries/evm/ibc_eureka_transfer.md](../docs/src/libraries/evm/ibc_eureka_transfer.md)

### PancakeSwap V3 Position Manager
The PancakeSwap V3 Position Manager library provides functionality for managing concentrated liquidity positions on PancakeSwap V3. It handles position creation, optimization, fee collection, and liquidity range adjustments for maximizing returns.

**Reference:** [docs/src/libraries/evm/pancakeswap_v3_position_manager.md](../docs/src/libraries/evm/pancakeswap_v3_position_manager.md)

### Standard Bridge Transfer
The Standard Bridge Transfer library provides functionality for using standard token bridges between different EVM-compatible chains. It abstracts the complexity of various bridge protocols and provides a unified interface for cross-chain transfers.

**Reference:** [docs/src/libraries/evm/standard_bridge_transfer.md](../docs/src/libraries/evm/standard_bridge_transfer.md)

### Stargate Transfer
The Stargate Transfer library enables cross-chain transfers using the Stargate protocol, which provides unified liquidity across multiple chains. It allows for efficient cross-chain swaps and transfers with minimized slippage and gas costs.

**Reference:** [docs/src/libraries/evm/stargate_transfer.md](../docs/src/libraries/evm/stargate_transfer.md)

## Examples

### Token Swap
The token swap example demonstrates how to create a simple Valence Program that executes token swaps across different decentralized exchanges. It showcases basic program configuration and the use of swap libraries to perform automated trading operations.

**Reference:** [docs/src/examples/token_swap.md](../docs/src/examples/token_swap.md)

### Crosschain Vaults
The crosschain vaults example illustrates how to build sophisticated vault strategies that operate across multiple blockchains. It demonstrates advanced features like cross-chain liquidity management, yield farming, and automated rebalancing strategies.

**Reference:** [docs/src/examples/crosschain_vaults.md](../docs/src/examples/crosschain_vaults.md)

### EVM Oneway Vault
The EVM oneway vault example shows how to create a vault that moves assets from EVM chains to other destinations for yield generation. It demonstrates one-way asset flows and the integration of EVM-specific functionality with cross-chain operations.

**Reference:** [docs/src/examples/evm_oneway_vault.md](../docs/src/examples/evm_oneway_vault.md)

### Vault Strategist
The vault strategist example presents a comprehensive vault management system with multiple strategies and automated decision-making capabilities. It showcases advanced features like dynamic strategy selection, risk management, and performance optimization across multiple chains.

**Reference:** [docs/src/examples/vault_strategist.md](../docs/src/examples/vault_strategist.md)

## Deployment

### Deployment Overview
The deployment section covers the various approaches and tools available for deploying Valence Programs to different environments. It includes both local development setups and production deployment strategies.

**Reference:** [docs/src/deployment/_overview.md](../docs/src/deployment/_overview.md)

### Local Deployment
Local deployment documentation provides step-by-step instructions for setting up and running Valence Programs in local development environments. It covers the necessary tools, configuration files, and testing procedures for local development workflows.

**Reference:** [docs/src/deployment/local.md](../docs/src/deployment/local.md)

## ZK System

### ZK System Overview
The Valence ZK system provides Zero-Knowledge proofs and a dedicated ZK Coprocessor to enhance protocol capabilities in areas requiring complex computation, privacy, and verifiable off-chain operations. It enables computationally intensive tasks to be executed off-chain with cryptographic proofs of correctness submitted on-chain.

**Reference:** [docs/src/zk/_overview.md](../docs/src/zk/_overview.md)

### System Overview
The ZK system overview provides a high-level architecture description of how the Valence ZK Coprocessor integrates with the broader protocol. It explains the relationship between guest programs, the zkVM, encoders, and on-chain verification components.

**Reference:** [docs/src/zk/01_system_overview.md](../docs/src/zk/01_system_overview.md)

### Developing Coprocessor Apps
The guide for developing coprocessor applications covers how to create guest programs that run on the ZK Coprocessor. It provides practical instructions for writing ZK circuits, implementing controllers, and integrating with the Valence ecosystem.

**Reference:** [docs/src/zk/02_developing_coprocessor_apps.md](../docs/src/zk/02_developing_coprocessor_apps.md)

### Onchain Integration
The onchain integration documentation explains how ZK-proven results from the coprocessor are integrated with on-chain Valence components. It covers proof verification, state updates, and the interaction between off-chain computation and on-chain execution.

**Reference:** [docs/src/zk/03_onchain_integration.md](../docs/src/zk/03_onchain_integration.md)

### Coprocessor Internals
The coprocessor internals documentation provides detailed technical information about the internal architecture and operation of the ZK Coprocessor. It covers the technical implementation details necessary for advanced users and contributors to the system.

**Reference:** [docs/src/zk/04_coprocessor_internals.md](../docs/src/zk/04_coprocessor_internals.md)

### Sparse Merkle Trees
The sparse Merkle trees documentation explains how the ZK system uses sparse Merkle trees for efficient state representation and proof generation. It covers the data structures and algorithms used to enable scalable ZK computation over blockchain state.

**Reference:** [docs/src/zk/05_sparse_merkle_trees.md](../docs/src/zk/05_sparse_merkle_trees.md)

### Guest Environment
The guest environment documentation describes the runtime environment available to guest programs running on the ZK Coprocessor. It covers available APIs, data sources, and constraints that govern off-chain computation within the ZK system.

**Reference:** [docs/src/zk/06_guest_environment.md](../docs/src/zk/06_guest_environment.md)

### State Encoding and Encoders
The state encoding documentation explains how blockchain state is compressed and encoded for efficient ZK proof generation. It covers both the Unary Encoder for single-chain operations and the Merkleized Encoder for cross-chain state dependencies.

**Reference:** [docs/src/zk/07_state_encoding_and_encoders.md](../docs/src/zk/07_state_encoding_and_encoders.md) 