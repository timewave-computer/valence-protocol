# Domains

A **Valence Program** is an instance of the **Valence Protocol**. A Valence program's execution can typically _span multiple Blockchains_. In the Valence Protocol, we refer to the various Blockchains that are supported as **domains**.

A **Domain** is an environment in which the components that form a program (more on these later) can be instantiated (deployed).

**Domains** are defined by **three properties**:
  1. The **Chain**: the Blockchain's name _e.g. Neutron, Osmosis, Ethereum mainnet_.
  2. The **Execution environment**: the environment under which programs (typically smart contracts) can be executed on that particular chain _e.g. CosmWasm, EVM, SVM_.
  3. The type of **Bridge** used from the **Main Domain** to other domains _e.g. Polytone over IBC, Hyperlane_.

Within a particular ecosystem of Blockchains (e.g. Cosmos), the Valence Protocol usually defines one specific domain as the **Main Domain**, on which some supporting infrastructure components are deployed. Think of it as the _home base_ supporting the execution and operations of a Valence programs. This will be further clarified in the [Authorizations & Processors](./authorizations_processors/_overview.md) section.

Below is a simplified representation of a _program transferring tokens_ from a given **input account** on the **Neutron domain**, a CosmWasm-enabled smart contract platform secured by the Cosmos Hub, to a specified **output account** on the **Osmosis domain**, a well-known DeFi platform in the Cosmos ecosystem. 
```mermaid
---
title: Valence cross-domain program
---
graph LR
  IA((Input
      Account))
  OA((Output
		  Account))
  subgraph Neutron
  IA
  end
  subgraph Osmosis
  IA -- 4/Transfer tokens --> OA
  end
```
