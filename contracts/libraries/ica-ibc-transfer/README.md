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
    // Memo to be passed in the IBC transfer message.
    pub memo: String,
    // Remote chain info
    pub remote_chain_info: RemoteChainInfo,
    // Denom map for the Packet-Forwarding Middleware, to perform a multi-hop transfer.
    pub denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
    // Configuration used for IBC Eureka transfers
    pub eureka_config: Option<EurekaConfig>,
}

pub struct RemoteChainInfo {
    // Channel ID to be used
    pub channel_id: String,
    // Timeout for the IBC transfer in seconds. If not specified, a default 600 seconds will be used will be used
    pub ibc_transfer_timeout: Option<u64>,
}

// Configuration for a multi-hop transfer using the Packet Forwarding Middleware
struct PacketForwardMiddlewareConfig {
  // Channel ID from the source chain to the intermediate chain
  local_to_hop_chain_channel_id: String,
  // Channel ID from the intermediate to the destination chain
  hop_to_destination_chain_channel_id: String,
  // Temporary receiver address on the intermediate chain. Typically this is set to an invalid address so the entire transaction will revert if the forwarding fails. If not 
  // provided it's set to "pfm"
  hop_chain_receiver_address: Option<String>,
}

// Configuration for IBC Eureka transfers
pub struct EurekaConfig {
    /// The address of the contract on intermediate chain that will receive the callback.
    pub callback_contract: String,
    /// The address of the contract on intermediate chain that will trigger the actions, in this case the Eureka transfer.
    pub action_contract: String,
    /// Recover address on intermediate chain in case the transfer fails
    pub recover_address: String,
    /// Source channel on the intermediate chain (e.g. "08-wasm-1369")
    pub source_channel: String,
    /// Optional memo for the Eureka transfer triggered by the contract. Not used right now but could eventually be used.
    pub memo: Option<String>,
    /// Timeout in seconds to be used for the Eureka transfer. For reference, Skip Go uses 12 hours (43200). If not passed we will use that default value
    pub timeout: Option<u64>,
}
```
