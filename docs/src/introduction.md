# Introduction

> 🚧 Valence Protocol architecture and developer documentation is still evolving rapidly. Portions of the toolchain have stabilized to build cross-chain vaults, and extending vaults with multi-party agreements. Send us a message on [X](https://x.com/valencezone) if you'd like to get started!

**Valence** is a unified development environment that enables building *trust-minimized cross-chain DeFi applications*, called **Valence Programs**.

Valence Programs are:

- **Easy to understand** and **quick to deploy**: a program can be set up with a configuration file and no code.
- **Extensible**: if we don't yet support a DeFi integration out of the box, new integrations can be written in a matter of hours!

> **Example Use Case**:
>
> A DeFi protocol wants to bridge tokens to another chain and deposit them into a vault. After a certain date, it wants to unwind the position. While the position is active, it may also want to delegate the right to change vault parameters to a designated committee so long as the parameters are within a certain range.
> Without Valence Programs, the protocol would have two choices:  
> 1. Give the tokens to a **multisig** to execute actions on the protocol's behalf  
> 2. Write **custom smart contracts** and deploy them across multiple chains to handle the cross-chain token operations.
>
> **Valence Programs** offer the DeFi protocol a third choice: rapidly configure and deploy a secure solution that meets its needs without trusting a multisig or writing complex smart contracts.
