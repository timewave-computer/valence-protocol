# Authorization Contract

The Authorization contract serves as the authority and message routing hub for Valence Programs. It supports two distinct authorization mechanisms: standard authorizations for traditional access control and ZK authorizations for zero-knowledge proof–based execution.

A Valence Program has one Authorization contract and one Processor contract per domain. The Authorization contract defines authorizations that control access to library functions within the program. The contract validates user permissions and routes authorized messages to the associated Processor contract for execution.

## Standard Authorizations

Standard authorizations use a label-based system with different authorization modes.

- CosmWasm: Permissionless authorizations allow anyone to execute (default Medium priority). Permissioned authorizations are enforced with per‑label TokenFactory tokens. With call limit, one token is consumed (burned on success, refunded on failure) per execution; without call limit, holding one token suffices. Tokens use `factory/{authorization_contract}/{label}` and enable on-chain transferability.
- EVM: Permissioned access is enforced per label with address allowlists and function‑level constraints. For each label, the contract stores an array of AuthorizationData entries containing the target contract address and either the function selector or a call hash. No tokens are minted; authorization is purely address/function based.

For standard message execution, the contract validates sender permissions and authorization state, ensures the message(s) align with the label’s subroutine configuration, routes the message to the Processor, and processes callbacks. On CosmWasm, token mint/burn/refund applies for call‑limited flows.

## ZK Authorizations

ZK authorizations enable proof‑based execution via a registry‑keyed configuration. Each registry stores allowed execution addresses, a verification key, a verification route (for a VerificationRouter), optional last‑block validation for replay prevention, and a metadata hash linking the VK to the program.

- EVM: Users call `executeZKMessage(bytes inputs, bytes proof, bytes payload)`. The Authorization verifies sender allowance, optional replay protection, then routes to the `VerificationRouter.verify(route, vk, proof, inputs, payload)`. On success, it injects the current `executionId` into SendMsgs/InsertMsgs and forwards to the Processor.
- CosmWasm: Users call `ExecuteZkAuthorization { label, inputs, proof, payload }`. The Authorization verifies sender allowance and optional last‑block execution checks, uses the configured verification route, and forwards the decoded Processor message.

Note: CosmWasm cross‑domain routing uses Polytone (CosmWasm↔CosmWasm). EVM cross‑domain routing uses Hyperlane mailboxes. Both environments support callbacks to the Authorization for execution results.
