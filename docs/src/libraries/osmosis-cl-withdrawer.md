# Osmosis CL liquidity withdrawer library

The **Valence Osmosis CL Withdrawer library** library allows to **withdraw a concentrated liquidity
positions** off an **Osmosis** pool from an **input account**, and transfer the resulting tokens to an **output account**.

## High-level flow

```mermaid
---
title: Osmosis CL Liquidity Withdrawal
---
graph LR
  IA((Input
      Account))
  OA((Output
		  Account))
  P[Processor]
  S[Osmosis
      Liquidity
      Withdrawal]
  AP[Osmosis CL
     Pool]
  P -- 1/Withdraw Liquidity --> S
  S -- 2/Query balances --> IA
  S -- 3/Compute amount --> S
  S -- 4/Do Withdraw Liquidity --> IA
  IA -- 5/Withdraw Liquidity
				  [LP Position] --> AP
  AP -- 5'/Transfer assets --> OA
```

## Functions

| Function    | Parameters | Description |
|-------------|------------|-------------|
| **WithdrawLiquidity** | `position_id: Uint64`<br>`liquidity_amount: String` |  Withdraw liquidity from the configured **Osmosis Pool** from the **input account**, according to the given parameters, and transfer the withdrawned tokens to the configured **output account** |

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    // Account from which the funds are LPed
    pub input_addr: LibraryAccountType,
    // Account to which the LP tokens are forwarded
    pub output_addr: LibraryAccountType,
    // ID of the pool
    pub pool_id: Uint64,
}
```
