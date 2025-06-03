# EVM One-Way Vault

The `OneWayVault.sol` contract is a specialized, ERC4626-compliant tokenized vault designed for cross-domain asset management within the Valence Protocol. It allows users to deposit an underlying ERC20 token on an EVM-compatible source chain and, in return, receive vault shares. The "one-way" nature refers to its primary design: deposits occur on the EVM chain, while withdrawal requests are initiated on the EVM chain but are intended for fulfillment on a separate destination chain.

## Purpose and Use Case

This vault serves as an entry point for users wishing to participate in strategies where their assets are ultimately utilized or managed on a different blockchain network. It abstracts the complexities of direct bridging by handling deposits and share issuance on the source EVM chain and creating structured withdrawal requests that can be processed by off-chain services or contracts on the destination chain.

## Key Features & Functionality

The `OneWayVault.sol` implements the [EIP-4626](https://eips.ethereum.org/EIPS/eip-4626) standard for tokenized vaults, providing common functions like `deposit`, `mint`, `withdraw`, `redeem`, `totalAssets`, `convertToShares`, and `convertToAssets`.

Regarding deposits, users contribute an underlying ERC20 token into the vault. A configurable `depositFeeBps` (basis points) can be charged, with collected fees (`feesOwedInAsset`) accruing in the vault. Upon deposit, *share issuance* occurs, where depositors receive vault shares representing their portion of the total assets. The number of shares is determined by the deposited asset amount (post-fees) and the current `redemptionRate`.

Users initiate withdrawal requests by calling `withdraw` (specifying asset amount) or `redeem` (specifying share amount). These actions burn the user's vault shares on the EVM chain. A `WithdrawRequest` struct is created and stored, containing a unique `id`, the `owner`, `sharesAmount` burned, the `redemptionRate` at request time, and a `receiver` string (for the recipient address on the destination chain). An event, `WithdrawRequested`, is emitted for off-chain systems to monitor and process the actual asset transfer on the destination chain.

The *redemption rate* dictates the value of vault shares in terms of the underlying asset and is initialized at a starting rate. The `strategist`, a privileged address, can update this `redemptionRate` via the `update()` function, typically reflecting yield or other strategy changes. This `update()` call also triggers *fee distribution* of accumulated deposit fees (`feesOwedInAsset`). These fees are split between a `platformAccount` and a `strategistAccount` based on `strategistRatioBps` (defined in `FeeDistributionConfig`), paid out by minting new vault shares to these accounts.

The vault has distinct Strategist and Owner roles. The Owner has administrative control, including updating vault configuration (`updateConfig`), authorizing contract upgrades (UUPS proxy), and pausing/unpausing the vault. The Strategist is responsible for updating the `redemptionRate` (and thus fee distribution) and can also pause the vault (but not unpause if the owner paused it). *Pausability* allows the owner or strategist to temporarily halt deposits and withdrawal request initiations. A *deposit cap* can be set to limit total assets, and deposited assets are held in a configured *deposit account* (an instance of `BaseAccount.sol` or similar).

## Interaction with `BaseAccount.sol`

The `depositAccount` specified in the `OneWayVaultConfig` is a crucial component. It is an external smart contract (typically `BaseAccount.sol`) that actually holds the underlying ERC20 tokens deposited into the vault. The `OneWayVault` itself acts as the share ledger and the interface for deposits and withdrawal requests, while the `depositAccount` serves as the custodian of the funds on the EVM chain.

## Intended Cross-Chain Flow (Conceptual)

The conceptual cross-chain flow for `OneWayVault.sol` begins when a user deposits Asset X into the vault on EVM Chain A, receiving vault shares in return. These Assets X are then transferred to the `depositAccount` contract on EVM Chain A. 

When the user wishes to withdraw, they call `redeem` on `OneWayVault.sol`, which burns their vault shares. This action causes `OneWayVault.sol` to emit a `WithdrawRequested` event. This event contains details such as the amount of shares, the redemption rate at that time, and the user's intended recipient address on a different Destination Chain B (e.g., a Neutron address).

An off-chain relayer or bridge system monitors this event on EVM Chain A. Upon detecting the event, this system verifies the request. It then facilitates the transfer of the corresponding amount of Asset X (or its equivalent) from a liquidity pool or treasury on Destination Chain B to the user's specified `receiver` address on that chain. The exact mechanism for sourcing these assets on the destination chain will depend on the specific cross-chain strategy being implemented.

The `OneWayVault.sol` thus provides the on-chain EVM primitives for the deposit and *withdrawal initiation* legs of such a cross-chain strategy. 