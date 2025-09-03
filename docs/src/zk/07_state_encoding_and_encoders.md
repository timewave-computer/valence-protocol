# State Encoding and Encoders

This document explains how the Valence ZK Coprocessor handles state encoding for zero-knowledge proofs and cross-chain state synchronization. Understanding these concepts is essential for building applications that work across multiple blockchains.

> **Implementation Status:** The state encoding mechanisms described in this document represent the design goals and architecture for the Valence ZK Coprocessor. While the core coprocessor infrastructure exists (as shown in the [valence-coprocessor-app template](https://github.com/timewave-computer/valence-coprocessor-app)), the full state encoding and cross-chain coordination features are still in active development.

## The State Encoding Challenge

The core challenge in ZK coprocessor design lies in encoding state. ZK applications are pure functions that must utilize existing state as arguments to produce an evaluated output state. This means we need a way to compress blockchain state into a format suitable for zero-knowledge proofs.

For any state transition, we can describe it as a pure function: `f(A) = B`, where `A` is the initial state and `B` is the resulting state after applying function `f`.

## Pure Functions in zkVMs

The Valence ZK Coprocessor leverages zero-knowledge virtual machines (zkVMs) to execute Rust programs and generate proofs of their execution. Specifically, Valence uses a RISC-V zkVM, currently Succinct's SP1. For state encoding purposes, these applications must be structured as pure functions `f(x) = y`.

The zkVM workflow for state transitions follows the following pattern:

1. **Application definition**: The state transition logic is written in Rust as a pure function
2. **Key generation**: The compiled application produces a proving key `pk` and verifying key `vk`
3. **Proof generation**: Given inputs `x`, the zkVM calls `prove(pk, x)` to generate proof `p`
4. **Verification**: The proof is verified by calling `verify(vk, x, y, p)`

This pure function constraint is what necessitates the state encoding mechanisms described in this document - we must compress mutable blockchain state into immutable inputs and outputs suitable for zero-knowledge proving.

## Unary Encoder

The Unary Encoder compresses account state transitions into zero-knowledge proofs. It handles the transformation from on-chain state mutations to ZK-provable computations.

### Basic State Transition Example

Consider an account with a key-value store that maps addresses to balances. A traditional on-chain transfer function might look like:

```rust
fn transfer(&mut self, signature: Signature, from: Address, to: Address, value: u64) {
    assert!(signature.verify(&from));
    assert!(value > 0);
    
    let balance_from = self.get(&from).unwrap();
    let balance_to = self.get(&to).unwrap_or(0);
    
    self.insert(from, balance_from.checked_sub(value).unwrap());
    self.insert(to, balance_to.checked_add(value).unwrap());
}
```

For ZK execution, we can create a trusted version that delegates signature verification to the ZK circuit:

```rust
fn transfer_trusted(&mut self, from: Address, to: Address, value: u64) {
    let balance_from = self.get(&from).unwrap();
    let balance_to = self.get(&to).unwrap_or(0);
    
    self.insert(from, balance_from - value);
    self.insert(to, balance_to + value);
}
```

### ZK Application Structure

In the current Valence Coprocessor template, ZK applications consist of two components: a controller and a circuit. The controller processes inputs and generates witnesses, while the circuit performs the ZK-provable computation.

**Controller (processes JSON inputs and generates witnesses):**
```rust
pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    let (signature, from, to, value) = parse_transfer_args(args);
    
    // Verify signature off-chain and prepare witness data
    signature.verify(&from)?;
    
    let witness_data = TransferWitness {
        from,
        to, 
        value,
        initial_state: get_current_state(),
    };
    
    Ok(vec![Witness::Data(witness_data.encode())])
}
```

**Circuit (performs ZK computation):**
```rust
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    let witness_data = TransferWitness::decode(witnesses[0].as_data().unwrap());
    let mut state = witness_data.initial_state;
    
    // Perform trusted transfer (signature already verified in controller)
    state.transfer_trusted(witness_data.from, witness_data.to, witness_data.value);
    
    // Return state commitment for on-chain verification
    state.commitment().encode()
}
```

> **Note:** The above examples show the conceptual structure for state encoding. The current template implementation uses simpler examples (like incrementing a counter), as the full state encoding mechanisms are still in development.

### On-Chain Verification

When the target chain receives the proof and circuit output, it can verify execution correctness:

```rust
fn verify(&self, proof: Proof, circuit_output: Vec<u8>) {
    let current_commitment = self.state.commitment();
    
    // Extract the new state commitment from circuit output
    let new_commitment = StateCommitment::decode(circuit_output);
    
    // Verify the ZK proof
    proof.verify(&self.vk, &[current_commitment, new_commitment].concat());
    
    // Apply the proven state transition
    self.state.apply_commitment(new_commitment);
}
```

## Merkleized Encoder

For cross-chain applications, the Merkleized Encoder handles state transition dependencies across multiple domains. This enables parallel execution while maintaining correctness for chains that depend on each other's state.

### Cross-Chain State Dependencies

Consider three chains where:
- Chain 1 executes independently 
- Chain 2 executes independently
- Chain 3 depends on the result from Chain 1

The Merklelized Encoder creates a Merkle tree structure:

```text
        R (Root)
       /         \
     M1           M2
    /  \         /  \
   C1   C2      C3   0
   |    |       |
Chain1 Chain2 Chain3
```

Each leaf contains the encoded state transition for its respective chain:
- `C1`: `(S1 → T1), K1` (Chain 1 transition)
- `C2`: `(S2 → T2), K2` (Chain 2 transition) 
- `C3`: `(S3 → T3), K3` (Chain 3 transition, depends on T1)

### Parallel and Sequential Execution

The ZK coprocessor can execute proofs in parallel where possible:

1. **Independent execution**: Chain 1 and Chain 2 can execute in parallel
2. **Sequential dependency**: Chain 3 waits for Chain 1's result `T1`
3. **State sharing**: Chain 3 receives `T1` and validates the foreign state while processing

### Optimized Verification

The Merkle tree structure provides logarithmic verification efficiency. Each chain only needs:

- Its own state transition arguments
- The Merkle path to the root `R`
- Any dependent state from other chains

For example, Chain 2 only needs `C1` and `M2` for its Merkle proof, not the complete state data from Chains 1 and 3.

### On-Chain Proof Distribution

Each chain receives the minimal data needed for verification:

- **Chain 1**: `(R1, T1)`
- **Chain 2**: `(R2, T2)` 
- **Chain 3**: `(R3, T3, R1, T1, C2)`

Chain 3's verification process includes:
1. Verify its own transition: `verify(R3, T3)`
2. Verify the dependency: `verify(R1, T1)`
3. Query the foreign state: `query(T1)`
4. Reconstruct the commitments and validate the Merkle root

This architecture enables the Valence Coprocessor to securely and efficiently coordinate complex cross-chain programs. 
### Domain Implementations (Examples)

Domains are pluggable modules that supply controller logic and circuits for chain‑specific state proofs. Each implementation typically includes:
- A controller (Wasm) that knows how to fetch/structure state inputs
- A circuit (zkVM target) that verifies the state proof and binds it to the Coprocessor root
- Optional services (e.g., light clients)

Example: Ethereum (as one implementation)
- Build storage layouts with a builder (e.g., mapping indices, combined slots, variable‑length values)
- Create `StateProofArgs` for the target account/storage and optional payload
- Produce a `StateProof` witness that the Coprocessor can open to the historical root and verify

New domains can follow the same pattern: define controller APIs that emit domain‑specific `Witness::StateProof` entries, implement a circuit that verifies those proofs, and optionally provide a service component for light‑client or state synthesis. For how these proofs bind to the Coprocessor root via domain and historical openings, see [Domain Proofs](./08_domain_proofs.md).
