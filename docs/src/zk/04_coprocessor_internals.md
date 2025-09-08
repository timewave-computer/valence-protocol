# ZK Coprocessor Internals

This document provides an in-depth look into the internal architecture and operational mechanics of the Valence ZK Coprocessor service. It is intended for those who wish to understand more about the Coprocessor's design beyond the scope of typical application development. Familiarity with [Valence ZK System Overview](./01_system_overview.md) is assumed.

The Valence ZK Coprocessor is designed as a persistent off-chain service that registers and executes Zero-Knowledge (ZK) guest applications.

### Service Architecture

The Coprocessor service consists of several coordinated components that work together to provide a complete ZK execution environment. It's important to note a key architectural separation: the `coprocessor` itself (which handles API requests, controller execution, and virtual filesystem management) is distinct from the `prover`. While they work in tandem, they can be deployed and scaled independently. For instance, Valence runs a dedicated, high-performance prover instance at `prover.timewave.computer:37282`. Coprocessor instances, including those run locally for development, can connect to this remote prover (typically requiring a `VALENCE_PROVER_SECRET` for access). This separation also allows developers to run a local coprocessor instance completely isolated from a real prover, using mocked ZK proofs. This is invaluable for rapid iteration and debugging of controller logic without incurring the overhead of actual proof generation.

The main components of the Coprocessor service include:

The API Layer serves as the primary external interface, exposing REST endpoints (typically on port `37281` for the coprocessor service itself) for core operations. Developers can deploy guest programs by submitting `controller` and `circuit` bundles, they can request proofs for deployed programs, query the status of ongoing tasks, and retrieve data stored in the virtual filesystem such as generated proofs or execution logs.

**Request Management & Database** - This component validates incoming requests and queues them for processing. It maintains persistent storage for deployed guest program details including Controller IDs, circuit specifications, and controller bundles, while also tracking proof generation status and execution metadata.

The **Controller Executor / Sandbox** provides an isolated execution environment for `controller` crate logic. This sandbox runs a WebAssembly runtime for controller code and provides a crucial interface that allows controllers to signal when witness preparation is complete and proof generation should commence. Controllers can also perform filesystem operations through this interface.

**Proving Engine Integration** - Orchestrates the actual ZK proof generation process using underlying zkVM systems like SP1 or Groth16. This component manages prover resources, handles the translation of circuits and witnesses into the required formats for specific proving backends, and processes the resulting proof data and public outputs.

The Virtual Filesystem Manager allocates FAT-16 based virtual filesystems to each guest program, enabling controllers to store proofs and logs through `store` commands. This filesystem has certain limitations on filename length and character sets that developers must consider.

### The Coprocessor Process

**Coprocessor Root Hash** is a notable internal detail where the Coprocessor prepends a 32-byte hash to application-specific public outputs from the ZK circuit. This combined data forms the complete "public inputs" that are cryptographically bound to the proof, ensuring that proofs are tied to the specific Coprocessor instance that produced them. On-chain verifiers must account for this structure when validating proofs.

**Task Lifecycle** involves proof generation requests progressing through several distinct stages: initial queuing, controller execution for witness generation, circuit proving, and finally proof delivery back to the controller entrypoint. The API provides mechanisms to track task status throughout this lifecycle.

**Persistent Job Queues** enable the Coprocessor service to handle multiple concurrent proof requests efficiently and reliably through persistent job queues, and worker nodes for computationally intensive proving tasks.

### Handling Verifiable State Proofs

Guest programs can incorporate state from external blockchains through a structured integration pattern that enhances their capabilities significantly.

External State Proof Services, such as the `eth-state-proof-service`, connect to external chains via RPC, query desired state at specific block heights, and construct Merkle proofs relative to known block hashes. These services play a crucial role in bridging external blockchain data into the ZK environment.

The guest program integration follows a clear pattern. During proof ingestion, the controller receives external state proofs via JSON payloads and extracts state values along with relevant metadata like block hashes. In the witness preparation phase, the controller incorporates this external state into the witness for the ZK circuit. The circuit logic then performs computations using the external state data, with the option to verify external proofs directly within the circuit for stronger security guarantees.

**Trust Model Considerations** - The ZK proof fundamentally attests that given a set of provided inputs (which may include externally proven state at the latest block height), the circuit executed correctly to produce the specified outputs. The Coprocessor provides a state proof interface for each chain that exposes a light client prover wrapped in a recursive circuit. All light client circuits are initialized at a trusted height, where block hash and committee composition are taken as "weakly subjective" public inputs.
### Service API (Access & Discovery)

The Coprocessor serves an OpenAPI/Swagger UI and specification alongside its REST endpoints.

You can programmatically discover available routes by fetching the spec. For example, to list available paths:

```bash
curl -s https://service.coprocessor.valence.zone/spec | jq -r '.paths | keys[]'
# or against local:
curl -s http://127.0.0.1:37281/spec | jq -r '.paths | keys[]'
```

Notes
- Virtual filesystem is FAT‑16 emulated; file extensions must be ≤ 3 characters, paths are case‑insensitive.
- The `payload` in proving requests is commonly `{ "cmd": "store", "path": "/var/share/proof.bin" }` to instruct the controller to store the generated proof.

### Related Services

Domain prover services publish recursive proofs and a stable wrapper VK for domains. For how domain and historical proofs are modeled (and how the domain prover feeds the Coprocessor and on‑chain verification), see [Domain Proofs](./08_domain_proofs.md). For domain implementation patterns, see [State Encoding and Encoders](./07_state_encoding_and_encoders.md#domain-implementations-examples).

### Client Conventions

When calling the Coprocessor, clients use a few standard conventions:

Headers
- `valence-coprocessor-circuit`: hex controller ID (context)
- `valence-coprocessor-root`: historical root hex (pinning to a known SMT root)
- `valence-coprocessor-signature`: optional signature over JSON body (if a signer is configured)

Prove payload
- Include a “store” payload to direct the controller to write the generated proof to the virtual filesystem, for example:
  `{ "args": { … }, "payload": { "cmd": "store", "path": "/var/share/proofs/<id>.bin" } }`

Virtual filesystem
- FAT‑16 emulation with 3‑character file extensions and case‑insensitive paths. A common pattern is to store under `/var/share/proofs/…`.

Public inputs layout
- The public inputs buffer starts with a 32‑byte Coprocessor Root, followed by the circuit‑defined output bytes used on‑chain.
