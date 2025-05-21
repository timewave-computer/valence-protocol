# Supervaults LPer library

The **Valence Supervaults LPer** library allows to **provide liquidity** into **Neutron Supervaults** from an **input account** and deposit the resulting **LP tokens** into an **output account**.

## High-level flow

```mermaid
---
title: Supervaults Liquidity Provider
---
graph LR
  IA((Input Account))
  OA((Output Account))
  P[Processor]
  S[Supervaults<br>Liquidity<br>Provider]
  SV[Supervault]
  P -- 1/Provide Liquidity --> S
  S -- 2/Query balances --> IA
  S -- 3/Try Provide Liquidity --> IA
  IA -- 4/Provide Liquidity [Tokens] --> SV
  IA -- 4'/Transfer LP Tokens --> OA
```

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    // Account from which the funds are LPed
    pub input_addr: LibraryAccountType,
    // Account to which the LP tokens are forwarded
    pub output_addr: LibraryAccountType,
    // Supervault address
    pub vault_addr: String,
    // LP configuration
    pub lp_config: LiquidityProviderConfig,
}

pub struct LiquidityProviderConfig {
    // Denoms of both native assets we are going to provide liquidity for
    pub asset_data: AssetData,
    // LP token denom for the supervault
    pub lp_denom: String,
}

pub struct PrecDecimalRange {
    pub min: PrecDec,
    pub max: PrecDec,
}

pub struct AssetData {
    pub asset1: String,
    pub asset2: String,
}
```
