## Background on Timewave Labs

Timewave is the software development company leading the development building the Valence Protocol.

Timewave co-founders have been working full-time in crypto since 2017 and have been working together on Valence since 2022. We spent 2022 researching cross-chain automation and identifying an initial use case, spent 2023-24 building cross-chain applications for WASM- and EVM-based chains, and have so far spent 2025 adding zero-knowledge capabilities and supporting additional DeFi protocols. We have worked with major blockchains to enable them to lend protocol-owned liquidity, manage treasury assets, and offer competitive risk-adjusted yields. 

**Max Einhorn, Co-Founder**

Former management consultant at Oliver Wyman serving institutional financial clients reporting to the Federal Reserve. Co-Founded 4K, a protocol that brings physical assets on-chain. Magna cum laude graduate of Wharton and lifelong entrepreneur.

**Udit Vira, Co-Founder**

Former Product Development Engineer at PMC-Sierra, led the design and verification of ASICs for packet switching networks. Built low power, wide area mesh networking software deployed internationally. Co-Founder of Hypha Labs.

**Sam Hart, Co-Founder**

Former Skip Head of Product and Strategy (acquired). Former ICF Board of Management and Cosmos Stack Product Lead. Co-developed the earliest liquid staking, restaking, shared sequencing, app-specific sequencing, and ZK accounts products.


---


# Valence History

## Formation

It all began in August of 2022 when Sam, Udit, and Max co-authored the [Cosmos Hub white paper](https://gateway.pinata.cloud/ipfs/QmdC3YuZBUq5b9mEr3bKTDRq4XLcxafe3LHqDNFUgUoa61). In that paper, we invented the Interchain Allocator, an internet-native financial institution capable of cross-chain capital allocation. We saw so much potential for the Allocator that we were not satisfied by simply writing about it — we were ready to commercialize it so we founded [Timewave](https://x.com/TimewaveLabs), the software development company that is leading the development of [Valence](https://x.com/ValenceZone) Protocol. 

Timewave’s first customer was [Stride](https://stride.zone/). They needed a cross-chain workflow that enabled the Cosmos Hub to deploy 450,000 ATOM in protocol-owned liquidity (POL) toward bootstrapping the ATOM:stATOM pool on Astroport on Neutron. We successfully implemented a cross-chain workflow that handled that specific deal and called it Covenant v1.

Shortly thereafter, the Cosmos Hub passed several additional POL deals so [AADAO](https://www.atomaccelerator.com/) paid us to develop a solution that would work for these new deals. We successfully built that solution and called it Covenant v2. We also used Covenant v2 to not only handle the Hub’s deals, but to also implement four bilateral POL deals between Neutron and Nolus, Stargaze, Mars, and Shade. 

In parallel, we closed a partnership with [Neutron](https://x.com/neutron_org) where Timewave receives 0.5% of the total NTRN token supply over 2 years in return for making Neutron Valence’s home chain. Timewave built the Rebalancer, a treasury management solution for internet-native organizations, and deployed it on Neutron. The Rebalancer runs regular auctions on Neutron while still being able to arbitrage against pools on other chains. 

Up to this point (May 2024), Valence didn’t yet exist in the public domain — it was a brand that we were sitting on until the time was right. That time came when we began integrating Covenants and Rebalancer into a single unified whole. 

## Integration

As we built Covenants and the Rebalancer, various people came to us asking for new flows with new assets to new chains in new ecosystems. We quickly realized that in order to keep up with demand, we needed to build infrastructure that is more scalable. Rather than build more bespoke cross-chain workflows, we built infrastructure that made it easy to deploy new workflows. That infrastructure is Valence.

Valence is a cross-chain virtual machine. It is a developer platform that enables anyone to develop cross-chain programs that execute automatically and indefinitely across an unlimited number of chains. 

Neutron was the first to use Valence. Neutron deployed $20M worth of NTRN toward a Valence flow that liquid staked some of the NTRN to create dNTRN, paired some of the dNTRN with NTRN to bootstrap liquidity for the dNTRN:NTRN pair on Astroport, and use the remaining dNTRN to pair with Neutron’s USDC to bootstrap dNTRN:USDC liquidity. For more detail, check out our Xeet on it [here](https://x.com/ValenceZone/status/1913231282630258710). 

Hydro was next to use Valence. The Hub realized that it was haphazardly deploying POL so it clawed the POL back and redeployed it via Hydro, a gauge system that enables protocols to bid on POL, effectively providing the Hub with interest payments for the POL it lends. Hydro uses Valence under the hood to transport assets to/from chains. See [here](https://x.com/HydroTeam_/status/1919423595014422598) for Hydro’s latest Valence shoutout. 

## Expansion

Having saturated the Cosmos market, we have been expanding into the EVM ecosystem ever since late last year.

The expansion began when we integrated Ethereum mainnet with the rest of Valence’s infrastructure, which included integrating Hyperlane so that we’d also gain compatibility with every nearly EVM chain.

In parallel with the buildout, we spoke with multiple potential design partners to identify the highest value cross-chain flows to develop. Our primary design partner with this expansion into EVM has been one of our investors, Lucas from Senotra (formerly Into the Block).

Sentora helps connect TradFi investors with DeFi yields. Sentora runs yield strategies on multiple chains, but are not able to run cross-chain strategies because they lack the infrastructure to do so automatically and were limited by their compliance department from doing so manually. These limitation reduce the risk-adjusted returns Sentora is able to offer their clients because the chains that offer the best leverage are rarely the chains that offer the best yields.

This brings us to the state of Valence today.

# State of Valence

## Team

Everything we do at Valence starts with our people. 

The most meaningful team upgrade to happen since Udit Vira joined us full time at the start of 2024 was Sam Hart joining us full time at the start of 2025. All three co-founders are now all in, which has resulted in a significant acceleration in our development speed and our rate of product iteration.

We recently made the most difficult decision we have ever made, which was to let one of our earliest teammates go due to performance. As hard as that decision was, the increase in overall performance level of the team is already apparent.

In better news, we have continued to have our pick of the litter when it comes to choosing which A+ players we recruit onto our team. In addition to our established teammates [Ben](https://github.com/bekauz) (smart contract engineer), [Keyne](https://github.com/keyleu) (smart contract engineer), [Lena](https://github.com/elenamik) (front end engineer) who have been delivering excellent results for Valence, we brought on three additional superb teammates in 2025:

- [Victor Lopez](https://github.com/vlopes11?tab=repositories) - Core maintainer of Arkworks and formerly of Polygon, Fuel, Sovereign Labs. Victor is our most senior engineer and has significantly up-leveled our degree of engineering excellence.
- [Jonas Pauli](https://github.com/jonas089) - Formerly of Casper and Chainsafe. He is our resident ZK boy genius.
- [Parthiv Seetharaman](https://github.com/Pacman99) - Formerly of Cardano. He is a Nix wizard who completely overhauled our automated deployment infrastructure in a matter of weeks.

## Compatibility

Today, Valence is compatible with EVM- and WASM-based chains. Currently, Valence can bridge assets using LayerZero, Hyperlane, and IBC. Valence also has last-mile integrations into Aave, Astroport, Osmosis, and Pancakeswap. Integrating new chains, bridges, or DeFi protocols is as simple as crafting a new library, something that can be accomplished in a matter of hours. Once a chain, bridge, or protocol has a Valence library, it is immediately compatible with everything else that Valence is compatible with.

For technical information on how Valence works, we encourage you to check out Valence’s highly detailed docs [here](https://docs.valence.zone/introduction.html) and GitHub [here](https://github.com/timewave-computer). 

## Architecture

Even though the version of Valence deployed today is significantly more scalable than the cross-chain application architecture that preceded it, we are always on the lookout for ways to improve. 

To that end, we’ve been working on a substantial revision of the protocol that moves most of the execution off-chain into a ZK execution environment. Over the course of the last 4 months we’ve been spec’ing out and building a multi-chain co-processor that uses light client proofs to anchor into any chain. This greatly extends the capabilities of our system, while creating a more uniform experience for developers building on us.

One of the biggest pain points we keep seeing from the developers of cross-chain applications is the need to write programs using multiple languages (e.g., Rust and Solidity for WASM- and EVM-based chain, respectively). Soon, developers will be able to use our coprocessor to write programs entirely in Rust and compile down to RISC-V. A “Valence Program” then becomes a series of verification keys that we write to each chain which will authorize specific state transition behavior at a defined point in the cross-chain program control flow.

The move to ZK execution isn’t just DevEx win, it also opens up new possibilities with what is possible to build on chain. Our off-chain architecture radically lowers execution costs, while at the same time creating a verifiable graph that encompasses all blockchains. This makes more DeFi endeavors/strategies more economically feasible and easier to pursue.

For an early peak at how we plan to utilize ZK, you’re welcome to check out the ZK co-processor portion of our docs [here](https://docs.valence.zone/zk/_overview.html).

## Go-to-Market

We are balancing keeping our existing users (i.e., Hydro, Neutron) happy while also expanding to attract new users. To attract new users, we are pursuing several related opportunities:

### 1. Cross-Chain Leveraged Yield Vaults

**Problem**: the chain that offers the best leverage on blue-chip assets is rarely the same chain that offers the best yields. Strategists must custody assets in order to get leverage on one chain and pursue yield on another, which reduces the pool of eligible capital allocators.

**Solution**: Valence vault that enables strategists to pursue cross-chain leveraged yield strategies without having to custody assets. We just completed a demo vault whereby a user can deposit wETH on Ethereum mainnet and have Valence automatically route 2/3 of the wETH to Aave to borrow USDC at 50% LTV, route the borrowed USDC and the remaining 1/3 of wETH to Base, and double-sided LP the assets into Pancakeswap. Furthermore, this demo vault automatically borrows more USDC as ETH values rise to maximize yield and recollateralizes the debt position with USDC as the value of ETH falls to decrease the risk of liquidation.

### 2. Cross-Chain Storefronts

**Problem:** new DeFi protocols are emerging that offer higher risk-adjusted returns, better execution quality, and various other improvements over incumbents, but using these new DeFi protocols requires users to change their behavior (e.g., download a new wallet, use a new bridge)

**Solution:** Valence vault that enables a user to deposit assets via a flow that they are already familiar with and have Valence automatically route the asset to the desired DeFi protocol to get that protocol’s benefits. We also have a working demo for this flow whereby a user can deposit USDC on Ethereum mainnet and Valence automatically routes the USDC to an LP position on Astroport on Neutron.

### 3. Other potential directions

We are in early-stage talks with AI Agent protocols, neo banks, and other organizations that may find value in a cross-chain program. Our architecture is incredibly flexible so we want to expose it to as many use cases as possible to surface unexpected value.


---

### Thesis

Timewave’s thesis has always been that crypto suffers from fragmented liquidity and users. While each protocol today operates like an isolated island, the endgame is a networked economy where protocols have seamless access to liquidity and users from everywhere. Solving this fragmentation is key to unlocking the full potential of decentralized finance.

### Early Evidence

We saw evidence of this problem firsthand. In the Cosmos ecosystem—a network of app-specific chains—new DeFi protocols often face intense competition for liquidity and users. To address these challenges, we started with two initial products:

1. **Covenants**
    
    Covenants was our first attempt to tackle liquidity fragmentation. This product enabled trustless, cross-chain agreements between protocols. For example, two protocols could swap tokens or establish a shared liquidity position on a decentralized exchange (DEX).
    
2. **Rebalancer**
    
    Recognizing the need for automated treasury management, we simultaneously launched the Rebalancer. This product automated portfolio management for protocol treasuries, making liquidity allocation easier.
    

Both these products were developed in CosmWasm for the Cosmos ecosystem and are now being phased out.

### Learnings

Building and deploying these products taught us a lot about the complexities of cross-chain infrastructure:

- **Asynchronous Execution**: Handling latency and error recovery is non-trivial.
- **Security**: Each protocol operates with its own security assumptions, making trust boundaries hard to navigate.
- **Integration Costs**: Cross-chain integrations are complex and costly.
- **Versioning**: Dependency upgrades require constant maintenance.
- **Testing**: Simulating multiple networks at once adds significant overhead to development and QA.

At the same time, we saw evidence of demand. Protocols have increasingly started to take more sophisticated cross-chain actions:

- **Liquid Staking and Re-Staking Protocols**: E.g., **Drop**. are issuing tokens on one chain that represent staked positions on a second chain.
- **Liquidity Sharing Agreements**: E.g., **Hydro**. is enabling ATOM to lend its liquidity to other chains for fixed time periods.
- **Cross-Chain Liquidity Management**: E.g., **Spark** has deployments of it's lending protocol on multiple chains and is rebalancing liquidity between these chains to harmonize interest rates and therefore the user experience.

Given our own insights about cross-chain application development and the demand-side evidence, we realized we could significantly accelerate the growth of many application developers that were building cross-chain protocols for the first time.

### Pivoting to Valence Programs

We shifted our focus to creating a generalized system for building cross-chain DeFi applications across Cosmos chains (and soon other ecosystems). We call this system Valence Programs ([Learn more](https://docs.valence.zone/)) . This MVP, which we’re now expanding to minimally support EVM, lays the groundwork for a unified development platform for cross-chain DeFi from anywhere to anywhere.

The current status is that we're using Neutron’s CosmWasm as a **coprocessor** and exploring the potential of zero-knowledge VM’s for future upgrades. This will involve accessing state from chains and within a ZK VM, verifying that state, computing transactions, and submitting proofs for execution across chains where tokens are escrowed in Program-owned accounts.

1. **Access State**: Fetch on-chain state from multiple networks.
2. **Verify State**: Ensure data integrity and security across trust boundaries.
3. **Compute Transactions**: Generate transactions based on program requirements
4. **Post Transactions + Proofs**: Submit proofs and transactions back to the chain for execution.
5. **Verify on Chain**: Validate proofs and execute transactions on-chain.
### **What’s Next**

With the foundation we’ve built, we’re ready to shape the next generation of cross-chain applications. However, we're very early and there is a lot of work to be done. Key challenges ahead include:

1. Supporting new execution domains (Solana, Move)
2. Refining integration points to decrease versioning challenges
3. Significantly improving application development experience in using Valence Programs (ergonomic system to create programs, tools for debugging and monitoring running programs)
4. Transitioning to ZK coprocessing
5. Building the best cross-chain test suite to ensure the correctness of Valence Programs.