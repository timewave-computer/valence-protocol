# Reward Protocol for Rollup BSNs

### Formalisation, Reference Designs & Integration Guidance

*Source: Runchao Han, updated 30 May 2025*&#x20;

## 1  Overview

Rollup block-space networks (BSNs) can “rent” economic security from Bitcoin by paying rewards to BTC stakers through the Babylon protocol.
This document formalises the reward problem, proposes two reference designs, and evaluates them on security, UX, and engineering complexity.

*Key trade-off:* **Design 1** is highly extensible but more code-heavy; **Design 2** is minimal but cannot support one-click, cross-chain withdrawals or precise APR dashboards.&#x20;

---

## 2  Stakeholders & Roles

| Group                          | On-chain location | Role                                   |   |
| ------------------------------ | ----------------- | -------------------------------------- | - |
| **Babylon validators**         | Babylon Genesis   | Produce Babylon blocks                 |   |
| **BABY stakers**               | Babylon Genesis   | Bond BABY to validators                |   |
| **Babylon FPs**                | Babylon Genesis   | Sign finality over Babylon blocks      |   |
| **BTC stakers (Babylon side)** | Babylon Genesis   | Delegate BTC to Babylon FPs            |   |
| **BSN FPs**                    | Rollup BSN        | Sign finality over BSN blocks          |   |
| **BTC stakers (BSN side)**     | Rollup BSN        | Delegate BTC to BSN FPs                |   |
| **Sequencer**                  | Rollup BSN        | Generates rollup blocks & pays rewards |   |

*Reward flow:* sequencer → all other stakeholders.

---

## 3  User Stories

1. **Reward generation** – sequencer earmarks a slice of protocol revenue.
2. **Reward distribution** – sequencer splits that slice across stakeholder classes (Babylon vs BSN).
3. **Reward withdrawal** – every stakeholder eventually pulls (or receives) their share, on Babylon and/or the BSN depending on design.&#x20;

---

## 4  Security Requirements

* **Correctness** – honest BSN + Babylon ⇒ payments match spec.
* **Babylon safety** – BSN must pay Babylon-side stakeholders or integration can be halted via governance.
* **BSN safety** – if Babylon fails to deliver needed features, sequencer may stop paying.
* **Fairness** – rewards proportional to each actor’s contribution inside its class; BSNs may fine-tune (e.g., top-N, jailing).&#x20;

---

## 5  Design-Space Considerations

*Engineering cost, runtime overhead, extensibility, and dashboard integration* are evaluated for every design variant. Desired extensions include:

* Retroactive reward drops
* Single-transaction withdrawals (BSN → Babylon, Babylon → BSN, or Babylon → *all* BSNs)
* Standardised registry for dashboards (à la DefiLlama adapters)&#x20;

---

## 6  Reference Design 1 — *Gauge-Based Smart-Contract Model*

### 6.1 Components

* **Reward contract (BSN, EVM)** with

  * `distribute_reward(list[(BTC_PK, fraction)])`
  * `withdraw_reward(sig(BTC_SK, recipient_addr))`
* **CLI** that the sequencer runs after every interval; it queries Babylon, builds the table, and calls the contract.&#x20;

### 6.2 Reward Flow

| Step             | Action                                                                                                                               |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **Generation**   | Every *reward\_time\_interval*, sequencer reserves `btc_staking_fraction` of revenue and deposits into contract.                     |
| **Distribution** | Tokens are split `fraction_babylon` / `fraction_bsn`. Babylon share is bridged to `fee_collector`; BSN share is recorded in gauges.  |
| **Withdrawal**   | • Babylon-side: follow Babylon’s native flow. • BSN-side: sign BSN address with BTC key, call `withdraw_reward`.                     |

### 6.3 Built-in Extensions

* Retroactive rewards (change CLI params).
* **1-BSN-tx → Babylon**: add `withdraw_reward_to_babylon`.
* **1-Babylon-tx → {Babylon, BSN}**: Babylon contract relays via bridge (e.g., Axelar, Union, IBC).
* **1-Babylon-tx → all BSNs**: same, loop over registry of BSNs.&#x20;

### 6.4 Dashboard Integration

Accurate gauges enable real-time reward + APR display. Withdrawal UX is one click if extensions enabled.&#x20;

---

## 7  Reference Design 2 — *Direct Payout Model*

### 7.1 What changes?

* Reward *distribution = withdrawal*; sequencer sends BSN share directly to stakeholder EOAs.
* Reward contract reduced to a **BTC\_PK → BSN\_addr registry** (for first-time mapping). CLI fires raw transfers.&#x20;

### 7.2 Consequences

| Dimension         | Impact                                                                             |
| ----------------- | ---------------------------------------------------------------------------------- |
| **Engineering**   | Simpler (no gauge math).                                                           |
| **Performance**   | Lowest overhead (plain transfers).                                                 |
| **Extensibility** | No contract-mediated withdrawals, so cross-chain one-click flows **not possible**. |
| **Dashboards**    | Cannot compute precise “accumulated reward”; APR display must be coarse.           |

### 7.3 Optional tweaks

Retroactive drops still trivial; users may self-bridge to Babylon manually.&#x20;

---

## 8  Head-to-Head Comparison (high-level)

| Aspect                 | Design 1                                  | Design 2                |   |
| ---------------------- | ----------------------------------------- | ----------------------- | - |
| **Code to write**      | Gauge contract + CLI (+ optional bridges) | Registry contract + CLI |   |
| **Runtime cost**       | Contract state updates                    | Plain token transfers   |   |
| **UX extensions**      | All listed extensions feasible            | Only manual bridging    |   |
| **Dashboard fidelity** | Exact reward & APR                        | Approximate only        |   |

---

## 9  APR Formula

```
APR = (1 / Total_BTC_stake)
      * BSN_annualised_protocol_revenue
      * btc_staking_fraction
      * fraction_bsn
      * BTC-BSN_exchange_rate
```

> *Data sources*: `Total_BTC_stake` from Babylon; revenue from BSN analytics; FX rate from public APIs (e.g., CoinGecko).

---

## 10  Future Work

1. **Registry standard** for BSNs to expose reward/withdrawal endpoints (reduces per-BSN dashboard work). :contentReference[oaicite:36]{index=36}  
2. Explore non-EVM rollup edge cases (e.g., Move-based chains) where gauge contract may not be feasible.  
3. Governance guard-rails for halting mis-behaving BSNs or Babylon.  
4. Gas-optimised batch distribution in Design 1 to cut per-interval costs.

---

## 11  Takeaways

*Design 1* is the “full-service” option for UX-centric rollups that want one-click, cross-chain withdrawals and accurate analytics.  
*Design 2* is attractive for MVP launches where immediate payout suffices and dashboards can live with approximations.

Both satisfy the core security model; choice hinges on **UX ambition versus engineering appetite**.
