# ZK Coprocessor Internals

This document provides a deeper look into the internal architecture and operational mechanics of the Valence ZK Coprocessor service. It is intended for those who wish to understand more about the Coprocessor's design beyond the scope of typical application development. Familiarity with [Valence ZK System Overview](./01_system_overview.md) is assumed.

The Valence ZK Coprocessor is designed as a persistent off-chain service that registers and execute Zero-Knowledge (ZK) guest applications.

### Service Architecture

The Coprocessor service consists of several coordinated components that work together to provide a complete ZK execution environment:

The **API Layer** serves as the primary external interface, exposing REST endpoints (typically on port `37281`) for core operations. Developers can deploy guest programs by submitting `controller` and `circuit` bundles, they can request proofs for deployed programs, query the status of ongoing tasks, and retrieve data stored in the virtual filesystem such as generated proofs or execution logs.

**Request Management & Database** - This component validates incoming requests and queues them for processing. It maintains persistent storage for deployed guest program details including Controller IDs, circuit specifications, and controller bundles, while also tracking proof generation status and execution metadata.

The **Controller Executor / Sandbox** provides an isolated execution environment for `controller` crate logic. This sandbox runs a WebAssembly runtime for controller code and provides a crucial interface that allows controllers to signal when witness preparation is complete and proof generation should commence. Controllers can also perform filesystem operations through this interface.

**Proving Engine Integration** - Orchestrates the actual ZK proof generation process using underlying zkVM systems like SP1 or Groth16. This component manages prover resources, handles the translation of circuits and witnesses into the required formats for specific proving backends, and processes the resulting proof data and public outputs.

The **Virtual Filesystem Manager** allocates FAT-16 based virtual filesystems to each guest program, enabling controllers to store proofs and logs through `store` commands. This filesystem has certain limitations on filename length and character sets that developers must consider.

### The Coprocessor Process

**Coprocessor Root Hash** is a notable internal detail where the Coprocessor prepends a 32-byte hash to application-specific public outputs from the ZK circuit. This combined data forms the complete "public inputs" that are cryptographically bound to the proof, ensuring that proofs are tied to the specific Coprocessor instance that produced them. On-chain verifiers must account for this structure when validating proofs.

**Task Lifecycle** involves proof generation requests progressing through several distinct stages: initial queuing, controller execution for witness generation, circuit proving, and finally proof delivery back to the controller entrypoint. The API provides mechanisms to track task status throughout this lifecycle.

**Persistent Job Queues** enable the Coprocessor service to handle multiple concurrent proof requests efficiently and reliably through persistent job queues, and worker nodes for computationally intensive proving tasks.

### Handling Verifiable State Proofs

Guest programs can incorporate state from external blockchains through a structured integration pattern that enhances their capabilities significantly.

External State Proof Services, such as the `eth-state-proof-service`, connect to external chains via RPC, query desired state at specific block heights, and construct Merkle proofs relative to known block hashes. These services play a crucial role in bridging external blockchain data into the ZK environment.

The guest program integration follows a clear pattern. During **Proof Ingestion**, the controller receives external state proofs via JSON payloads and extracts state values along with relevant metadata like block hashes. In the **Witness Preparation** phase, the controller incorporates this external state into the witness for the ZK circuit. The **Circuit Logic** then performs computations using the external state data, with the option to verify external proofs directly within the circuit for stronger security guarantees.

**Trust Model Considerations** - The ZK proof fundamentally attests that given a set of provided inputs (which may include externally proven state at the latest block height), the circuit executed correctly to produce the specified outputs. The Coprocessor provides a state proof interface for each chain that exposes a light client prover wrapped in a recursive circuit. All light client circuits are initialized at a trusted height, where block hash and committee composition are taken as "weakly subjective" public inputs.
