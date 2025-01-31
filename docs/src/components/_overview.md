# Valence Programs

Valence Programs can be executed in two distinct ways.
1. **On-chain Execution**:
Extensive support exists for CosmWasm and some for EVM. The rest of this section provides a high-level breakdown of the components that compose a Valence cross-chain program using on-chain coprocessors.
    - [Domains](./domains.md)
    - [Accounts](./accounts.md)
    - [Libraries and Functions](./libraries_and_functions.md)
    - [Programs and Authorizations](./programs_and_authorizations.md)
    - [Middleware](./middleware.md)
2. **zk-Coprocessor**:
Early specifications exist for the [Valence zk-Coprocessor](../zk-coprocessor/_overview.md). Our current focus is to move computation off-chain as this is a more scalable approach to building a cross-chain execution environment.

Unless explicitly mentioned, you may assume that documentation and examples in the remaining sections are written with on-chain execution in mind.