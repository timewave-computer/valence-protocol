# Duality Lper library

The **Valence Duality LPer library** allows users to **provide liquidity** into a Duality Liquidity Pool from an **input account** and deposit the LP token into an **output account**.

## High-level flow

```mermaid
---
title: Duality Liquidity Provider
---
graph LR
  IA((Input
      Account))
  OA((Output
          Account))
  P[Processor]
  S[Duality
      Liquidity
      Provider]
  DP[Duality
     Pool]
  P -- 1/Provide Liquidity --> S
  S -- 2/Query balances --> IA
  S -- 3/Do Provide Liquidity --> IA
  IA -- 4/Provide Liquidity
                  [Tokens] --> DP
  DP -- 4'/Mint LP Tokens --> OA

```

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    /// Address of the input account 
    pub input_addr: LibraryAccountType,
    /// Address of the output account 
    pub output_addr: LibraryAccountType,
    /// Configuration for the liquidity provider
    /// This includes the pool address and asset data
    pub lp_config: LiquidityProviderConfig,
}

pub struct LiquidityProviderConfig {
    /// Address of the pool we are going to provide liquidity for
    pub pool_addr: String,
    /// Denoms of both assets we are going to provide liquidity for
    pub asset_data: AssetData,
}
```
