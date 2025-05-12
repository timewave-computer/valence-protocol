# Valence Union Transfer library

The **Valence Union Transfer** library allows to transfer funds over [Union](https://union.build/) from an **input account** on a source chain to an **output account** on a destination chain. It is typically used as part of a **Valence Program**. In that context, a **Processor** contract will be the main contract interacting with the Forwarder library.


## High-level flow

```mermaid
---
title: Union Transfer Library
---
graph LR
  IA((Input Account))
  ZG((zkGM))
  R((Recipient))
  P[Processor]
  U[Union Transfer
   Library]
  UTM[Union Token
   Protocol]
  subgraph EVM[ EVM Chain ]
    UTM -- 6/Send tokens --> R
  end
  subgraph CW[ CosmWasm Domain ]
    P -- 1/Call
    transfer(quote_amount) --> U
    U -- 2/Query CW20/Native
     token balance --> IA
    U -- 3/Call approve (if applies) and send --> IA
    IA -- 4/Approve CW20 (if applies) --> ZG
    IA -- 5/Call send with instructions --> ZG
  end
  CW --- EVM
```

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    pub denom: UncheckedUnionDenomConfig,
    pub amount: TransferAmount,
    // Information about the asset to be transferred.
    pub input_asset_name: String,
    pub input_asset_symbol: String,
    pub input_asset_decimals: u8,
    pub input_asset_token_path: Uint256,
    // Information about the asset to be received.
    pub quote_token: String,
    pub quote_amount: Uint256,
    // Information about the remote chain.
    pub channel_id: u64,
    pub transfer_timeout: Option<u64>, // If not provided, a default 3 days will be used (259200 seconds).
    // Information about the protocol
    pub zkgm_contract: String, // The address of the ZKGM contract that we will interact with
    // They are using a batch operation with a transfer (FungibleAssetOrder) operation inside, so we need the version for both instructions.
    // If not provided, we will use the versions currently used by the protocol, but this is meant to be used for future upgrades.
    pub batch_instruction_version: Option<u8>, // The version of the batch instruction to be used. If not provided, the current default version will be used.
    pub transfer_instruction_version: Option<u8>, // The version of the transfer instruction to be used. If not provided, the current default version will be used.
}

pub enum UncheckedUnionDenomConfig {
    /// A native (bank module) asset.
    Native(String),
    /// A cw20 asset along with the token minter address that needs to be approved for spending during transfers.
    Cw20(UncheckedUnionCw20Config),
}

pub struct UncheckedUnionCw20Config {
    pub token: String,
    pub minter: String,
}

pub enum TransferAmount {
    FullAmount,
    FixedAmount(Uint128),
}
```
