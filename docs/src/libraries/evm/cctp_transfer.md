# Valence CCTP Transfer library

The **Valence CCTP Transfer** library allows to **transfer funds** from an **input account** to a **mint recipient** using the [Cross-Chain Transfer Protocol (CCTP) v1](https://developers.circle.com/stablecoins/cctp-getting-started). It is typically used as part of a **Valence Program**. In that context, a **Processor** contract will be the main contract interacting with the CCTP Transfer library.

## High-level flow

```mermaid
---
title: CCTP Transfer Library
---
graph LR
  IA((Input Account))
  CCTPR((CCTP Relayer))
  MR((Mint Recipient))
  TM((CCTP Token Messenger))
  P[Processor]
  S[CCTP Transfer Library]

  subgraph DEST[ Destination Domain ]
    CCTPR -- 7/Mint tokens --> MR
  end

  subgraph EVM[ EVM Domain ]
    P -- 1/Forward --> S
    S -- 2/Query balances --> IA
    S -- 3/Do approve and call depositForBurn --> IA
    IA -- 4/ERC-20 approve --> TM
    IA -- 5/Call depositForBurn --> TM
    TM -- 6/Burn tokens and emit event --> TM
  end

  EVM --- DEST
```

## Functions

| Function     | Parameters | Description                                                                                                   |
| ------------ | ---------- | ------------------------------------------------------------------------------------------------------------- |
| **Transfer** | -          | Transfer funds from the configured **input account** to the **mint recipient** on the **destination domain**. |

## Configuration

The library is configured on deployment using the `CCTPTransferConfig` type. A list of the supported CCTP destination domains that can be used in the `destinationDomain` field can be found [here](https://developers.circle.com/stablecoins/supported-domains).

```solidity
    /**
     * @dev Configuration struct for token transfer parameters.
     * @param amount The number of tokens to transfer. If set to 0, the entire balance is transferred.
     * @param mintRecipient The recipient address (in bytes32 format) on the destination chain where tokens will be minted.
     * @param inputAccount The account from which tokens will be debited.
     * @param destinationDomain The domain identifier for the destination chain.
     * @param cctpTokenMessenger The CCTP Token Messenger contract.
     * @param transferToken The ERC20 token address that will be transferred.
     */
    struct CCTPTransferConfig {
        uint256 amount; // If we want to transfer all tokens, we can set this to 0.
        bytes32 mintRecipient;
        Account inputAccount;
        uint32 destinationDomain;
        ITokenMessenger cctpTokenMessenger;
        address transferToken;
    }
```
