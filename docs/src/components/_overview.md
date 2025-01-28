> 🚧 Valence Protocol architecture and developer documentation is still evolving rapidly. Portions of the toolchain have stabilized to build cross-chain vaults, and extending vaults with multi-party agreements. Send us a message on [X](https://x.com/valencezone) if you'd like to get started!

# Introduction

**Valence** is a unified development environment that enables building *trust-minimized cross-chain DeFi applications*, called **Valence Programs**.

Valence Programs are:

- **Easy to understand** and **quick to deploy**: a program can be set up with a configuration file and no code.
- **Extensible**: if we don't support a DeFi integration out of the box, you can write one yourself in a matter of hours!

> **Example Use-case**:
>
> A DeFi protocol wants to bridge tokens to another chain and deposit them into a vault. After a certain date, it wants to unwind the position. While the position is active, it may also want to delegate the right to change vault parameters to a designated committee, as long as the parameters are within a certain range.

Without Valence Programs, the protocol would have two choices:  
1. Give the tokens to a **multisig** to execute actions on the protocol's behalf  
2. Write **custom smart contracts** and deploy them across multiple chains to handle the cross-chain token operations.

**Valence Programs** offer a third choice: the protocol does not need to trust a multisig, nor does it need to spend resources writing complex cross-chain logic.

*By using Valence, the protocol can rapidly configure and deploy a secure solution that meets its needs.*

## Key components

The rest of this section provides a high-level breakdown of the components that compose a Valence cross-chain program.

- [Domains](./domains.md)
- [Accounts](./accounts.md)
- [Libraries and Functions](./libraries_and_functions.md)
- [Programs and Authorizations](./programs_and_authorizations.md)
- [Middleware](./middleware.md)
- [Storage Accounts](./storage_account.md)
