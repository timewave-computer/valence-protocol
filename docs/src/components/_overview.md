# Valence Programs

There are two ways to execute Valence Programs.

1. **On-chain Execution**:
Valence currently supports CosmWasm and EVM. SVM support coming soon. The rest of this section provides a high-level breakdown of the components that comprise a Valence Program using on-chain coprocessors.
    - [Domains](./domains.md)
    - [Accounts](./accounts.md)
    - [Libraries and Functions](./libraries_and_functions.md)
    - [Programs and Authorizations](./programs_and_authorizations.md)
    - [Middleware](./middleware.md)

2. **Off-chain Execution via ZK Coprocessor**:
Early specifications exist for the [Valence ZK coprocessor] (/zk-coprocessor/overview.md). We aim to move as much computation off-chain as possible since off-chain computation is a more scalable approach to building a cross-chain execution environment.

Unless explicitly mentioned, you may assume that documentation and examples in the remaining sections are written with on-chain execution in mind.
