# Domain Proofs

This document explains how domain proofs are modeled and validated in the Valence Coprocessor and how historical proofs are bound to a domain proof.

Domain proofs bind chain‑specific state (for example, an Ethereum account/storage proof) to a single Coprocessor root. The Coprocessor root is the root of a global Historical tree. Each leaf of that tree is a domain root, and each domain root is the root of a per‑domain sparse Merkle tree that maps block number to state root. Guest programs provide domain‑specific state proofs; the Coprocessor augments them with openings up to the Coprocessor root and proves the combined statement. For API access and client conventions, see [Coprocessor Internals](./04_coprocessor_internals.md) and for on‑chain consumption, see [On‑Chain Integration](./03_onchain_integration.md).

## Structure

Per‑domain, we maintain a sparse Merkle tree keyed by block number whose leaves are state roots. Using the block number as the key improves locality—consecutive blocks tend to share path prefixes—so proof paths are short on average. Globally, we maintain a sparse Merkle tree keyed by `Hash(domain identifier)` whose leaves are the current domain roots. The root of this Historical tree is the Coprocessor root. The Coprocessor places this 32‑byte root at the start of public inputs for every program proof; the remainder of the inputs is the program’s circuit output.

## Binding State to the Coprocessor Root

To bind a domain value to the Coprocessor root, the Coprocessor combines two openings. First, it computes a per‑domain opening from the block number to the state root in the domain tree. Second, it computes a historical opening from `Hash(domain id)` to the domain root in the Historical tree. These openings are combined into a single “state opening” that binds the state root to the Coprocessor root; the Coprocessor enforces that the opening corresponds to the correct domain identifier. Finally, the domain‑specific value proof (for example, an Ethereum MPT proof) is verified against the state root. The result is a proof that the value is included in the domain state committed by the Coprocessor root at the referenced block.

## Adding New Blocks

New blocks are added through the domain’s controller (for example, `POST /api/registry/domain/:domain`). The controller validates the domain‑specific inputs and yields the new `(block number, state root)` pair, and the Coprocessor persists the historical update and proofs. You can query the latest per‑domain information at `/api/registry/domain/:domain/latest`, the current Coprocessor root at `/api/historical`, a specific update at `/api/historical/:root`, or a block proof for a domain at `/api/historical/:domain/:number`.

## Recursive Proofs and Publication

The “state transition” for the Historical tree is modeled via recursive proofs produced by a domain prover service. The service ingests historical updates, computes an inner proof over intervals of updates, and wraps it in a stable “wrapper” proof with a published verifying key (VK). Consumers read the latest state and wrapper VK from the domain prover and can bind their verification logic to that VK and the expected controller ID. Per‑domain block validity is enforced when adding a block to the Coprocessor; the wrapper proof chains these updates. See [Coprocessor Internals](./04_coprocessor_internals.md) for how the domain prover and Coprocessor interact.

## On‑Chain Consumption

On‑chain, program proofs always start with the 32‑byte Coprocessor root in public inputs; the circuit‑defined output follows. Authorization uses a VerificationRouter route to verify proofs against the correct VK and route (for example, a guest program VK or a domain prover wrapper VK). Upon success, Authorization dispatches the validated message to the Processor. There is currently no on‑chain registry of “valid Coprocessor roots”; the domain prover route and VK binding provide the trust anchor. A root registry could be added later if desired.
