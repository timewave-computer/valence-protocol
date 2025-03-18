# Valence ICA IBC Transfer Library

The **Valence ICA IBC Transfer Library** library allows remotely executing an **IBC transfer** using a **Valence interchain account** on a remote IBC connected domain. It does that by remotely sending a **MsgTransfer** to the ICA created by the **Valence interchain account** on the remote domain. It is typically used as part of a **Valence Program**. In that context, a **Processor** contract will be the main contract interacting with the **Valence ICA IBC Transfer Library**.

## High-level flow

```mermaid
---
title: ICA IBC Transfer Library
---
graph LR
    subgraph Neutron
      P[Processor]
      L[ICA IBC
      Transfer Library]
      I[Input Account]
      P -- 1)Transfer --> L
      L -- 2)Query ICA address --> I
      L -- 3)Do ICA MsgTransfer --> I
    end

    subgraph Remote domain
      ICA[Interchain Account]
      I -- 4)Execute MsgTransfer --> ICA
    end
```

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    // Address of the input account (Valence interchain account)
    pub input_addr: LibraryAccountType,
    // Amount that is going to be transferred
    pub amount: Uint128,
    // Denom that is going to be transferred
    pub denom: String,
    // Receiver on the other chain
    pub receiver: String,
    // Remote chain info
    pub remote_chain_info: RemoteChainInfo,
}

pub struct RemoteChainInfo {
    // Channel ID to be used
    pub channel_id: String,
    // Timeout for the IBC transfer in seconds. If not specified, a default 600 seconds will be used will be used
    pub ibc_transfer_timeout: Option<u64>,
}
```
