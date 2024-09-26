# Astroport Withdrawer service

The **Valence Astroport Withdrawer service** service allows to **withdraw liquidity** from an Astroport Liquidity Pool from an **input account** an deposit the withdrawed tokens into an **output account**.

## High-level flow

```mermaid
---
title: Astroport Liquidity Withdrawal
---
graph LR
  IA((Input
      Account))
  OA((Output
		  Account))
  P[Processor]
  S[Astroport
      Liquidity
      Withdrawal]
  AP[Astroport
     Pool]
  P -- 1/Withdraw Liquidity --> S
  S -- 2/Query balances --> IA
  S -- 3/Compute amounts --> S
  S -- 4/Do Withdraw Liquidity --> IA
  IA -- 5/Withdraw Liquidity
				  [LP Tokens] --> AP
  AP -- 5'/Transfer assets --> OA
```

## Configuration

The service is configured on instantiation via the `ServiceConfig` type.

```rust
pub struct ServiceConfig {
    // Account from which the funds are LPed
    pub input_addr: String,
    // Account to which the LP tokens are forwarded
    pub output_addr: String,
    // Pool address
    pub pool_addr: String,
    // Liquidity withdrawer configuration
    pub withdrawer_config: LiquidityWithdrawerConfig,
}

pub struct LiquidityWithdrawerConfig {
    // Pool type, old Astroport pools use Cw20 lp tokens and new pools use native tokens, so we specify here what kind of token we are will use.
    // We also provide the PairType structure of the right Astroport version that we are going to use for each scenario
    pub pool_type: PoolType,
}

pub enum PoolType {
    NativeLpToken(astroport::factory::PairType),
    Cw20LpToken(astroport_cw20_lp_token::factory::PairType),
}
```
