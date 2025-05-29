# Valence Clearing Queue Library

The **Valence Clearing Queue** library allows **registration and settlement of withdrawal obligations** in a FIFO (First-In-First-Out) manner. It maintains a queue of pending withdrawal obligations, with each obligation containing recipient information, payout amounts, and a unique identifier. When settling obligations, funds are pulled from a **settlement input account** and sent to the specified recipients.

> **Important:**
> This library functions solely as a settlement engine. The settlement account funding (liquidity-management) flow is outside of its scope and is managed by a strategist. This management process likely involves monitoring both the settlement account balance and the obligation queue in order to ensure the settlement account maintains sufficient liquidity for obligation settlements.

## High-level flow

```mermaid
---
title: Clearing Queue Library
---
graph LR
  IA((Settlement<br>Account))
  R((Recipient))
  P[Processor]
  CQ[Clearing<br>Queue<br>Library]
  P -- 1/Register<br>Obligation --> CQ
  CQ -- 2/Store<br>Obligation --> CQ
  P -- 3/Settle<br>Next<br>Obligation --> CQ
  CQ -- 4/Query Settlement<br>Account Balance --> IA
  CQ -- 5/Validate & Execute Transfer --> IA
  IA -- 6/Send Funds --> R
```

## Functions

| Function               | Parameters                                          | Description                                                                                                                                            |
|------------------------|-----------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|
| **RegisterObligation** | `recipient: String`<br>`payout_coins: Vec<Coin>`<br>`id: Uint64` | Registers a new withdrawal obligation in the queue with the specified recipient, payout coins, and unique ID. Each obligation must have non-zero payout amounts. Recipient must be a valid bech32 address. |
| **SettleNextObligation** | -                                                   | Settles the oldest withdrawal obligation in the queue by transferring funds from the **settlement input account** to the specified recipient. Fails if there are no pending obligations or if the input account has insufficient balance. |

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    /// settlement input account which we tap into in order
    /// to settle the obligations
    pub settlement_acc_addr: LibraryAccountType,
}
```

The `settlement_acc_addr` specifies the account from which funds will be pulled to fulfill settlement obligations. The library will check that this account has sufficient balance before attempting to settle each obligation.
