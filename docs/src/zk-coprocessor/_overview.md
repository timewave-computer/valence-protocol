# Valence zk-Coprocessor

> ⚠️ **Note:** Valence's zk-Coprocessor is currently in specification stage and evolving rapidly. This document is shared to give partners a preview of our roadmap in the spirit of building in public.

## Overview

The Valence zk-Coprocessor is a universal DeFi execution engine. It allows developers to compose programs once and deploy them across multiple blockchains. Additionally, the coprocessor facilitates execution of arbitrary cross-chain messages with a focus on synchronizing state between domains. Using Valence, developers get:
1. To build once, deploy everywhere. You write your program code in Rust but execution is settled on EVM, Wasm, Move, and SVM chains.
2. To introduce no additional trust assumptions. You only trust the consensus of the chains you are already building on.

While the actual execution is straightforward, **the challenge lies in encoding the state data** to enable the ZK program, as a pure function, to utilize the existing state as arguments and produce the modified state output.

Initially, we can develop an efficient version of this coprocessor approximately at par with creating the state encoder. However, it is crucial to note that each chain will necessitate a separate encoder implementation. The initial version would necessitate users to deploy their custom verification keys along with the state mutation function within the target blockchain. Although the code required for this purpose would be minimal, users would still need to implement their own verification keys and state mutation functions.

In the long term, we plan to develop a decoder that would automate the state mutation process based on the output of the ZK commitment. This development would require additional engineering resources and is not essential for the initial version launch. Instead, users will be able to perform raw mutations directly, as the correctness of ZK proofs will ensure the validity of messages according to the implemented ZK circuit.

```mermaid
---
title: zk-Coprocessor overview
---
graph TB;
    %% Programs
    subgraph zk-Coprocessor
        P1[zk Program 1]
        P2[zk Program 2]
        P3[zk Program 3]
    end

    %% Chains
    C1[Chain 1]
    C2[Chain 2]
    C3[Chain 3]

    P1 <--> C1
    P2 <--> C2
    P3 <--> C3
```

## zkVM Primer

A zero-knowledge virtual machine (zkVM) is a zero-knowledge proof system that allows developers to prove the execution of arbitrary programs, and in our case these programs are written in Rust. Given a Rust program that can be described as a pure function`f(x) = y`, you can prove the evaluation in the following way:
1. Define `f` using normal Rust code and compile it as an executable binary
2. With this executable binary, set up a proving key `pk` and verifying key `vk`
 3. Generate a proof `p` using the zkVM, by calling `prove(pk, x)`.
Conceptually, you can think of a zkVM as proving the evaluation of a function f(x) = y by following the steps below:
4. Now you can verify the proof `p` by calling `verify(vk, x, y, p)`

## Building the Valence zk-Coprocessor

Let's assume that we have Valence Accounts in each domain. These accounts implement a kv store. 

Every Zero-Knowledge (ZK) computation will follow the format of a pure state transition function; specifically, we input a state `A`, apply the function `f` to it, and produce the resulting state `B` : `f(A) = B` .
For the function `f`, the chosen zero-knowledge Virtual Machine (zkVM) will generate a verifying key `K`, which remains consistent across all state transition functions.

### Encoding the account state: Unary Encoder

To ensure every state transition computed as a Zero-Knowledge (ZK) proof by the coprocessor is a pure state transition function, we require a method to encode the entire account's state into initial and mutated forms, `A` and `B`, respectively, for use in providing the applicable state modifications for the target chain.

In essence, let's consider an account with its state containing a map that assigns a balance (u64 value) to each key. A contract execution transferring 100 tokens from key `m` to `n` can be achieved by invoking `state.transfer(signature, m, n, 100)`. This on-chain transfer function may look something like this:

```rust
fn transfer(&mut self, signature: Signature, from: Address, to: Address, value: u64) {
    assert!(signature.verify(&from));
    assert!(value > 0);

    let balance_from = self.get(&from).unwrap();
    let balance_to = self.get(&from).unwrap_or(0);

    let balance_from = balance_from.checked_sub(value).unwrap();
    let balance_to = balance_to.checked_add(value).unwrap();

    self.insert(from, balance_from);
    self.insert(to, balance_to);
}
```
Here, the pre-transfer state is `A` and after the transfer, the state is `B`.

Let's write a new function called `transfer_trusted` that leaves signature verification to the zk-Coprocessor.

```rust
fn transfer_trusted(&mut self, from: Address, to: Address, value: u64) {
    let balance_from = self.get(&from).unwrap();
    let balance_to = self.get(&to).unwrap_or(0);

    self.insert(from, balance_from - value);
    self.insert(to, balance_to + value);
}
```
In the ZK setting, we execute the `transfer` function within the zkVM. We must input the encoded state the account and receive as output the encoded state of the mutated account.

```rust
fn program(mut state: State, encoder: Encoder, arguments: Arguments) -> Commitment {
    let (signature, from, to, value) = arguments;
    let initial = encoder.commitment(state);

    state.transfer(signature, from, to, value);

	let arguments = encoder.commitment(arugment)
    let finalized = encoder.commitment(state)
    let output = encoder.commitment(initial, arguments, finalized) 

    encoder.commitment(initial, arguments, output)
}
```
Running this program within the zkVM, also allows us to generate a `Proof`.

Upon receiving the `(Proof, Commitment, Arguments)` data on the target chain, it can validate the execution correctness by verifying the proof and commitments, leveraging the ZK property that the proof will be valid if, and only if, the contract's execution was accurate for the given inputs, and the supplied commitments are those generated specifically for this proof.

```rust
fn verify(&self, proof: Proof, arguments: Arguments) {
    let current = self.state.commitment();
    let args = arguments.commitment();
    let (from, to, value) = arguments;

    self.transfer_trusted(from, to, value);

    let mutated = self.state.commitment();
    let commitment = (current, args, mutated).commitment();

    proof.verify(&self.vk, commitment);
}
```
By doing so, we switch from on-chain signature verification to arguments commitment computation, followed by ZK proof verification. Although this is a simplified example, the complexity of signature verification computation can be increased to accommodate any computation supported by zkVMs (namely, any RiscV program). This advantage enables us to process multiple transfers in batches, perform intricate computations, and succinctly verify their correctness.

We refer to this variant as a **"Unary Encoder"** because we compress the two states of the account, 'current' and 'mutated', into a single Zero-Knowledge (ZK) proof.

This component will be the responsible for compressing any chain account state into a compatible commitment for the chosen zkVM. The encoding is a one-way function that allows anyone in possession of its pre-image (i.e. inputs to the encoding function) to reconstruct the commitment. This commitment will be transparent to the target chain, enabling its use in composing the block header for verification purposes. This is the aforementioned `commitment` function.

### Handling state transition dependencies across domains: Merkelized Encoder

Lets assume a hypothetical situation where we aim to achieve decoupled state updates across three distinct chains: Chain1, Chain2, and Chain3. The objective is to generate a unified Zero-Knowledge (ZK) proof that verifies the correctness of the state transitions on all chains.

Specifically, Chain3 will depend on a mutation from Chain1, while Chain2 operates independently of the mutations on both Chain1 and Chain3.

```mermaid
graph TB
    %% Root node
    r[R]
    
    %% Level 1
    m1[M1] --> r
    m2[M2] --> r
    
    %% Level 2
    c1[C1] --> m1
    c2[C2] --> m1
    c3[C3] --> m2
    zero((0)) --> m2
    
    %% Level 3
    chain1[["(S1 --> T1), K1"]] -- Chain1 transition encoding --> c1
    chain2[["(S2 --> T2), K2"]] -- Chain2 transition encoding --> c2
    chain3[["(S3 --> T3), K3"]] -- Chain3 transition encoding --> c3
```

The Merkle Graph above depicts the state transition that can be compressed into a single commitment via Merkelization. Given an encoder with a specialized argument - a Sparse Merkle tree containing encoded state transition values indexed by the program's view key on the target blockchain - we obtain a Merkle Root denoted as `R`.

The ZK coprocessor can execute proof computations either sequentially or in parallel. The parallel computation associated with `C2` operates independently and generates a unary proof of `S2 -> T2`. Conversely, the proof for `C3` requires querying `T1`.

Since `Chain3` has a sequential execution, the coprocessor will first process `C1`, then relay the pre-image of `T1` to the coprocessor responsible for computing `C3`. Due to the deterministic nature of unary encoding, the `Chain3` coprocessor can easily derive `T1` and validate its foreign state while concurrently processing `C3`.

At this point, there is no justification given for Merkelizing the produced proofs - we could as well just hash the entire set of Merkle arguments, and it would work just fine. However, it's worth noting that `Chain2` doesn't require knowledge of the data `(S1, T1, K1, S3, T3, K3)`. Including such information in the verification arguments of `Chain3` would unnecessarily burden its proving process. A Merkle tree is employed here due to its logarithmic verification property: the condensed proof generated for `Chain2` will only require a Merkle Opening to `R`, without the excess state data of side chains. Essentially, when generating the Merkelized proof, the `Chain2` coprocessor, after computing `C2`, will need only `C1` and `M2`, instead of the full Merkle arguments.

Finally, each chain will receive `R`, accompanied by its individual state transition arguments, and the Merkle Path leading to `R` will be proven inside of the circuit.

```mermaid
---
title: On-chain Proof Verification
---
graph TD;
	Coprocessor --(R1, T1)--> Chain1
	Coprocessor --(R2, T2)--> Chain2
	Coprocessor --(R3, T3, R1, T1, C2)--> Chain3
```

`Chain3` will first `verify(R3, T3)`, then `verify(R1, T1)`, then it will `query(T1)`, then compute `C1 := encoding(S1, T1)`, then compute  `C3 := encoding(S3, T3)`, and finally will assert `R == H(H(C1, C2), H(C3, 0))`.