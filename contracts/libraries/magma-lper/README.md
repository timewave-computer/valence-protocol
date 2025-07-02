# Magma lper library

The **Valence Magma LPer library** allows users to **deposit** into a Magma Vault Pool from an **input account** and receive shares into an **output account**.

## High-level flow

```mermaid
---
title: Magma lper
---
graph LR
  IA((Input
      Account))
  OA((Output
          Account))
  P[Processor]
  S[Magma
      Liquidity
      Provider]
  M[Magma Vault]
  P -- 1/Provide Liquidity --> S
  S -- 2/Query balances --> IA
  S -- 3/Do Provide Liquidity --> IA
  IA -- 4/Provide Liquidity
                  [Tokens] --> M
  M -- 4'/Mint Shares --> OA

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
    /// Address of the vault we are going to deposit into
    pub vault_addr: String,
    /// Denoms of both assets we are going to provide liquidity for
    pub asset_data: AssetData,
}
```
