# Valence Drop Liquid Unstaker library

The **Valence Drop Liquid Unstaker** library allows to **liquid unstake** an asset from an **input account** from the [Drop protocol](https://docs.drop.money/) and to **withdraw** the claim once it's withdrawable and deposit the asset into the **output account**. It is typically used as part of a **Valence Program**. In that context, a **Processor** contract will be the main contract interacting with the Forwarder library.

## High-level flow

```mermaid
---
title: Drop Liquid Unstaker Library - LiquidUnstake Flow
---
graph LR
    IA((Input Account))
    CC((Drop Core Contract))
    P2[Processor]
    S2[Drop Liquid 
    Unstaker Library]
    P2 -- "1/Liquid Unstake" --> S2
    S2 -- "2/Query balance" --> IA
    S2 -- "3/Do Liquid Unstake funds" --> IA
    IA -- "4/Liquid Unstake funds" --> CC
    CC -- "5/Send NFT voucher" --> IA
```

```mermaid
---
title: Drop Liquid Unstaker Library - Claim Flow
---
graph LR
    IA((Input Account))
    WW((Withdrawal Manager
    Contract))
    P1[Processor]
    S1[Drop Liquid 
    Unstaker Library]
    OA((Output Account))
    P1 -- "1/Claim (token_id)" --> S1
    S1 -- "2/Check ownership" --> IA
    S1 -- "3/Do Claim" --> IA
    IA -- "4/Send NFT voucher with
    ReceiveMsg" --> WW
    WW -- "5/Send unstaked funds" --> OA
```

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    // Address of the liquid unstaker contract (drop core contract)
    pub liquid_unstaker_addr: String,
    // Address of the claimer contract (drop withdrawal manager)
    pub claimer_addr: String,
    // Address of the voucher NFT contract that we get after unstaking and we use for the claim
    pub voucher_addr: String,
    // Denom of the asset we are going to liquid unstake
    pub denom: String,
}
```
