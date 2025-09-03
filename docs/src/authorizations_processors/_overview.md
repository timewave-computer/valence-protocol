# Authorization & Processors

The Authorization and Processor contracts are foundational pieces of the Valence Protocol, as they enable execution of Valence Programs and enforce access control to the program's Subroutines via Authorizations.

This section explains the rationale for these contracts and shares insights into their technical implementation, as well as how end-users can interact with Valence Programs via Authorizations.

## Rationale

- To provide users with a single point of entry to interact with the Valence Program through controlled access to library functions.
- To centralize user authorizations and permissions, making it easy to control application access.
- To have a single address (Processor) that will execute the authorized messages. On CosmWasm this uses execution queues and permissionless ticks; on EVM the Lite Processor executes immediately (no queues).
- To create, edit, or remove different application permissions with ease.

Note: Programs can optionally include libraries and accounts deployed across multiple domains for certain multi-chain scenarios.
