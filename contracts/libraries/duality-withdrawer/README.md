# Duality Withdrawer library

The **Valence Duality Withdrawer library** allows users to **withdraw liquidity** from a Duality Liquidity Pool from an **input account** and deposit the withdrawn tokens into an **output account**.

## High-level flow

```mermaid
---
title: Duality Liquidity Withdrawal
---
graph LR
  IA((Input
      Account))
  OA((Output
          Account))
  P[Processor]
  S[Duality
      Liquidity
      Withdrawal]
  DP[Duality
     Pool]
  P -- 1/Withdraw Liquidity --> S
  S -- 2/Query balances --> IA
  S -- 3/Do Withdraw Liquidity --> IA
  IA -- 4/Withdraw Liquidity
                  [LP Tokens] --> DP
  DP -- 4'/Transfer assets --> OA
```

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    // Address of the input account 
    pub input_addr: LibraryAccountType,
    // Address of the output account 
    pub output_addr: LibraryAccountType,
    // Address of the pool we are going to withdraw liquidity from 
    pub pool_addr: String,
}
```
