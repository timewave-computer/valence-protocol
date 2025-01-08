# Crosschain Vaults

**Note:** _This example is still in design and includes new or experimental features of Valence Programs that may not be supported in current production releases._

## Overview

You can use Valence Programs to create crosschain vaults. Users interact with a vault on one chain while the hokens are held on another chain where yield is generated.

In this example, we have made the following assumptions:
- Users can deposit tokens into a standard ERC4626 vault on Ethereum
- ERC20 shares are issued to users on Ethereum
- If a user wishes to redeem their tokens, they can issue a withdrawal request which will burn the user's shares when tokens are redeemed
- The redemption rate that tells us how many tokens can be redeemed per shares is given by: \\( R = \frac{TotalAssets}{TotalIssuedShares} = \frac{TotalInVault + TotalInTransit + TotalInPostion}{TotalIssuedShares}\\)
- A permissioned actor called the "Strategist" is authorized to transport funds from Ethereum to Neutron where they are locked in some DeFi protocol. And vice-versa, the Strategist can withdraw from the position so the funds are redeemable on Ethereum. The redemption rate must be adjusted by the Strategist accordingly

```mermaid 
---
title: Crosschain Vaults Overview
---
graph LR
	User
	EV(Ethereum Vault)
	NP(Neutron Position)

	User -- Tokens --> EV
	EV -- Shares --> User
	EV -- Strategist Transport --> NP
	NP -- Strategist Transport --> EV
```

While we have chosen Ethereum and Neutron as examples here, one could similarly construct such vaults between any two chains as long as they are supported by Valence Programs.

## Implementing Crosschain Vaults as a Valence Program

Recall that Valence Programs are comprised of Libraries and Accounts. Libraries are a collection of Functions that perform token oprations on the Accounts. Since there are two chains here, Libraries and Accounts will exist on both chains.

Since gas is cheaper on Neutron than on Ethereum, computationally expensive operations, such as constraining the Strategist actions will be done on Neutron. Authorized messages will then be executed by each chain's Processor. Hyperlane is used to pass messages from the Authorization contract on Neutron to the Processor on Ethereum.

```mermaid
---
title: Program Control
---
graph LR
	Strategist
	subgraph Ethereum
		EP(Processor)
		EHM(Hyperlane Mailbox)
		EL(Ethereum Libraries)
		EVA(Valence Accounts)

	end
	subgraph Neutron
		A(Authorizations)
		NP(Processor)
		NHM(Hyperlane Mailbox)
		NL(Neutron Libraries)
		NVA(Valence Accounts)
	end

	Strategist --> A
	A --> NHM -- Relayer --> EHM --> EP --> EL --> EVA
	A --> NP --> NL--> NVA
```

### Libraries and Accounts needed

On Ethereum, we'll need Accounts for:
- **Deposit**: To hold user deposited tokens. Tokens from this pool can be then transported to Neutron.
- **Withdraw**: Told hold tokens received from Neutron. Tokens from this pool can then be 

On Neutron, we'll need Accounts for:
- **Deposit**: To hold tokens bridged from Ethereum. Tokens from this pool can be used to enter into the position on Neutron.
- **Position Holder**: Will hold the vouchers or shares associated with the position on Neutron
- **Withdraw**: To hold the tokens that are withdrawn from the position. Tokens from this pool can be bridged back to Ethereum.

We'll need the following Libraries on Ethereum:

We'll need the following Libraries on Neutron:

The Vault contract is a special contract on Ethereum that does the following:


#### Allowing users to deposit and withdraw tokens

```mermaid 
---
title: User Deposit Flow
---
graph LR
	User
	subgraph Ethereum
		direction LR
		EV(Vault)
		ED((Deposit))
	end
	
	User -- 1/ Deposit Tokens --> EV
	EV -- 2/ Send Shares --> User
	EV -- 3/ Send Tokens --> ED
```

```mermaid 
---
title: User Withdraw Flow
---
graph RL
	subgraph Ethereum
		direction RL
		EV(Vault)
		EW((Withdraw))
	end
	EW -- 2/ Send Tokens --> EV -- 3/ Send Tokens --> User
	User -- 1/ Deposit Shares --> EV


```

#### Allowing the Strategist to transport funds


```mermaid
---
title: From Ethereum Deposit Account to Neutron Position Account
---
graph LR
	subgraph Ethereum
		ED((Deposit))
		ET(Bridge Transfer)
	end
	subgraph Neutron
		NPH((Position Holder))
		NPD(Position Depositor)
		ND((Deposit))
	end

	ED --> ET --> ND --> NPD --> NPH
```

```mermaid
---
title: From Neutron Position Account to Ethereum Withdraw Account
---
graph RL
	subgraph Ethereum
		EW((Withdraw))
	end
	subgraph Neutron
		NPH((Position Holder))
		NW((Widthdraw))
		NT(Bridge Transfer)
		NPW(Position Withdrawer)
	end

	NPH --> NPW --> NW --> NT --> EW

```

```mermaid
---
title: Between Ethereum Deposit and Ethereum Withdraw Accounts
---
graph
	subgraph Ethereum
		ED((Deposit))
		EW((Withdraw))
		FDW(Forwarder)
	end
	ED --> FDW --> EW
```


