# EigenLayer Documentation Summaries

## Slashing Concepts

### Slashing Concept
EigenLayer's slashing mechanism is an opt-in system where operators must specifically choose to participate in slashable Operator Sets created by AVSs, meaning stakers are not automatically subject to slashing when the feature is released. The system enables AVSs to enforce cryptoeconomic guarantees by punishing operators and their delegated stakers for failing to meet service commitments, completing EigenLayer's transition to a feature-complete restaking protocol.

**References:**
- [EigenLayer Slashing Concept](https://docs.eigenlayer.xyz/eigenlayer/concepts/slashing/slashing-concept)
- [Intro to Slashing on EigenLayer: Stakers' Edition](https://www.blog.eigenlayer.xyz/intro-to-slashing-on-eigenlayer-stakers-edition/)
- [Intro to Slashing on EigenLayer: AVS Edition](https://www.blog.eigenlayer.xyz/intro-to-slashing-on-eigenlayer-avs-edition/)

### Unique Stake
Unique Stake Allocation allows AVSs to slash only the specific portion of stake that operators have designated for that particular AVS, isolating slashing risks between different services. This system enables operators to allocate different amounts of stake to multiple AVSs simultaneously without exposing their entire stake to risks from unrelated tasks or services.

**References:**
- [Unique Stake](https://docs.eigenlayer.xyz/eigenlayer/concepts/slashing/unique-stake)

### Magnitudes When Slashed
The magnitude of slashing is determined by each AVS based on their specific requirements and the severity of the operator's failure to meet commitments. AVSs have flexibility to set custom slashing conditions and penalties that align with their business needs, risk profile, and security requirements for their particular service.

**References:**
- [Slashing Magnitudes](https://docs.eigenlayer.xyz/eigenlayer/concepts/slashing/magnitudes-when-slashed)

### Safety Delays Concept
Safety delays provide a ~17.5 day period when operators first opt into slashing with an AVS's Operator Set, giving stakers time to adjust their positions or withdraw funds if they disagree with the operator's decision. After the initial allocation, operators can control whether future allocations are instant or include additional staker safety delays, providing ongoing protection for delegated stakers.

**References:**
- [Safety Delays Concept](https://docs.eigenlayer.xyz/eigenlayer/concepts/slashing/safety-delays-concept)

### Slashable Stake Risks
Slashable stake risks vary based on the specific conditions set by each AVS and the operator's ability to meet those commitments consistently. Stakers must monitor their operators' allocations to different AVSs and Operator Sets, as changes in slashable stake exposure directly impact all of the operator's delegated stakers' risk profile.

**References:**
- [Slashable Stake Risks](https://docs.eigenlayer.xyz/eigenlayer/concepts/slashing/slashable-stake-risks)

## Rewards Concepts

### Rewards Concept
EigenLayer's rewards system enables AVSs to distribute ERC20 token rewards to operators and stakers based on their participation and stake allocation using both onchain and offchain calculations consolidated through Merkle trees. The system distributes rewards proportionally based on relative stake weight and globally-defined operator commissions, creating direct revenue streams for participants securing AVS services.

**References:**
- [Rewards Concept](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/rewards-concept)
- [EigenLayer AVS Rewards: Risk Considerations - Llama Risk](https://www.llamarisk.com/research/avs-rewards)

### Earners, Claimers, Recipients
Earners are the operators and stakers who participate in securing AVSs and are entitled to rewards based on their stake allocation and participation time. Claimers are authorized addresses (which can be the earner themselves or designated representatives) who can process reward claims using valid Merkle proofs, while recipients are the final addresses that receive the distributed tokens.

**References:**
- [Earners, Claimers, Recipients](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/earners-claimers-recipients)

### Rewards Claiming
Rewards claiming occurs through a Merkle proof-based system where eligible participants submit cryptographic proofs to validate their earnings against stored distribution roots. Claims are processed through smart contracts after predetermined activation delays, with rewards becoming claimable on a weekly basis starting every Tuesday at 19:00 UTC.

**References:**
- [Rewards Claiming](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/rewards-claiming)

### Rewards Split
Rewards are split between operators and stakers based on configurable commission rates, with initial implementations using a fixed 10% operator commission and 90% going to stakers. The system distributes rewards proportionally to stake weight (amount of assets staked multiplied by the time period staked) across all eligible participants.

**References:**
- [Rewards Split](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/rewards-split)

### PI Split (Programmatic Incentives Split)
Programmatic Incentives v1 distributes newly-minted EIGEN tokens with 3% going to ETH and LST stakers/operators (weighted equally) and 1% to EIGEN stakers/operators. The distribution follows a fixed operator commission of 10% (~128,742 EIGEN per week) to operators and 90% (~1,158,678 EIGEN per week) to stakers, though these percentages may change in future updates.

**References:**
- [PI Split](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/pi-split)
- [Introducing Programmatic Incentives v1](https://www.blog.eigenlayer.xyz/introducing-programmatic-incentives-v1/)

### Rewards Submission
AVSs submit rewards through smart contracts that coordinate which addresses have restaked which assets for specific time periods, with submissions processed within a 70-day window. The submission process transfers tokens to the RewardsCoordinator contract and later distributes them based on offchain calculations that are posted onchain as Merkle roots.

**References:**
- [Rewards Submission](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/rewards-submission)

### Rewards Claiming FAQ
Common considerations include maximum waiting periods of 16 days (including calculation delay, root submission cadence, and activation delay) and tax implications that vary by jurisdiction. Participants should consider timing their claims strategically and consult local tax guidance, as rewards generation may have different tax treatment than claiming.

**References:**
- [Rewards Claiming FAQ](https://docs.eigenlayer.xyz/eigenlayer/concepts/rewards/rewards-claiming-faq)

## Operator Sets

### Operator Sets Concept
Operator Sets are a new feature that allows AVSs to manage their operators by setting specific conditions and registration requirements for participation. AVSs create Operator Sets with custom slashing conditions and requirements, while operators can choose to opt into these sets based on whether they can meet the specified conditions and commitments.

**References:**
- [Operator Sets Concept](https://docs.eigenlayer.xyz/eigenlayer/concepts/operator-sets/operator-sets-concept)

### Allocation Deallocation
Operators can allocate specific amounts of stake to different AVSs through Operator Sets, with initial allocations requiring a ~17.5 day period for staker safety. After the initial allocation, operators control whether future allocations are instant or include safety delays, allowing for flexible stake management across multiple services.

**References:**
- [Allocation Deallocation](https://docs.eigenlayer.xyz/eigenlayer/concepts/operator-sets/allocation-deallocation)

### Strategies and Magnitudes
Each Operator Set can define different strategies for stake allocation and specify the magnitude of potential slashing based on the AVS's specific security and operational requirements. AVSs have flexibility to customize these parameters to match their risk tolerance and the level of cryptoeconomic security they require for their services.

**References:**
- [Strategies and Magnitudes](https://docs.eigenlayer.xyz/eigenlayer/concepts/operator-sets/strategies-and-magnitudes)

## User Access Management (UAM)

### User Access Management
EigenLayer's User Access Management system provides role-based access control for various protocol functions and operations. The system enables different permission levels and access controls to ensure proper governance and operational security across the protocol's various components and stakeholders.

**References:**
- [User Access Management](https://docs.eigenlayer.xyz/eigenlayer/concepts/uam/user-access-management)

### UAM Accounts
UAM accounts represent different types of users within the EigenLayer ecosystem, including stakers, operators, and AVS administrators, each with specific permissions and capabilities. These accounts are designed to provide appropriate access levels while maintaining security and operational integrity across the protocol.

**References:**
- [UAM Accounts](https://docs.eigenlayer.xyz/eigenlayer/concepts/uam/uam-accounts)

### UAM Admins
UAM admins have elevated privileges to manage user access, configure protocol parameters, and oversee operational aspects of the EigenLayer system. These administrative roles are crucial for maintaining protocol security, managing upgrades, and ensuring proper governance of the restaking ecosystem.

**References:**
- [UAM Admins](https://docs.eigenlayer.xyz/eigenlayer/concepts/uam/uam-admins)

### UAM Appointees
UAM appointees are designated users who have been granted specific permissions or roles by administrators or through governance processes. These appointees can perform certain administrative or operational functions within their granted scope of authority while maintaining accountability through the UAM system.

**References:**
- [UAM Appointees](https://docs.eigenlayer.xyz/eigenlayer/concepts/uam/uam-appointees)

## Keys and Signatures

### Keys and Signatures
EigenLayer uses dual cryptographic key systems with ECDSA keys for Ethereum transactions and BLS keys for efficient signature aggregation in consensus protocols. The CLI supports multiple key management backends including local keystores, Fireblocks with AWS KMS, and Web3Signer, providing operators with flexible security options for their cryptographic operations.

**References:**
- [Keys and Signatures](https://docs.eigenlayer.xyz/eigenlayer/concepts/keys-and-signatures)
- [EigenLayer CLI Repository](https://github.com/Layr-Labs/eigenlayer-cli)

## Additional Resources

### General Documentation
- [EigenLayer Official Website](https://www.eigenlayer.xyz/)
- [EigenLayer Documentation Portal](https://docs.eigenlayer.xyz/)
- [Eigen Foundation](https://eigenfoundation.org/)
- [Update on the EIGEN Stakedrop - Eigen Foundation](https://blog.eigenfoundation.org/eigen-community-update/)
