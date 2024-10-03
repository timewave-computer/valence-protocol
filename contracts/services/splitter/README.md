# Valence Splitter service

The **Valence Splitter** service allows to **split funds** from **one input account** to **one or more output account(s)**, for **one or more token denom(s)** according to the configured **ratio(s)**. It is typically used as part of a **Valence Workflow**. In that context, a **Processor** contract will be the main contract interacting with the Forwarder service.

## High-level flow

```mermaid
---
title: Splitter Service
---
graph LR
  IA((Input
      Account))
  OA1((Output
		  Account 1))
	OA2((Output
		  Account 2))
  P[Processor]
  S[Splitter
    Service]
  C[Contract]
  P -- 1/Split --> S
  S -- 2/Query balances --> IA
  S -. 3/Query split ratio .-> C
  S -- 4/Do Send funds --> IA
  IA -- 5/Send funds --> OA1
  IA -- 5'/Send funds --> OA2
```

## Configuration

The service is configured on instantiation via the `ServiceConfig` type.
```rust
struct ServiceConfig {
    input_addr: String,                // Address of the input account
    splits: Vec<UncheckedSplitConfig>, // Split configuration per denom
}

// Split config for specified account
struct UncheckedSplitConfig {
  denom: UncheckedDenom,                // Denom for this split configuration (either native or CW20)
  account: String,                      // Address of the output account for this split config
  amount: Option<Uint128>,              // Fixed amount of tokens
  ratio: Option<UncheckedRatioConfig>,  // Amount defined based on a ratio
}

// Ratio configuration, either fixed & dynamically calculated
enum UncheckedRatioConfig {
  FixedRatio(Decimal),  // Fixed ratio e.g. 0.0262 for NTRN/STARS (or could be another arbitrary ratio)
  DynamicRatio {        // Dynamic ratio calculation (delegated to external contract)
	contract_addr: "<TWAP Oracle wrapper contract address>",
    params: "base64-encoded arbitrary payload to send in addition to the denoms"
  }
}

// Standard query & response for contract computing a dynamic ratio
// for the Splitter & Reverse Splitter services.
#[cw_serde]
#[derive(QueryResponses)]
pub enum DynamicRatioQueryMsg {
    #[returns(DynamicRatioResponse)]
    DynamicRatio {
        denoms: Vec<String>,
        params: String,
    }
}

#[cw_serde]
// Response returned by the external contract for a dynamic ratio
struct DynamicRatioResponse {
    pub denom_ratios: HashMap<String, Decimal>,
}
```
