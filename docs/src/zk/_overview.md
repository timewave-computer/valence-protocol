# Introduction to Valence ZK

Valence Protocol provides Zero-Knowledge proofs and a dedicated ZK Coprocessor system to enhance its capabilities, particularly in areas requiring complex computation, privacy, and verifiable off-chain operations. This ZK integration allows Valence to bridge the gap between the rich, flexible environment of off-chain processing and the trust-minimized, verifiable nature of blockchain execution.

At a high level, ZK proofs enable one party (the prover, in this case, the ZK Coprocessor) to prove to another party (the verifier, typically on-chain smart contracts) that a certain statement is true, without revealing any information beyond the validity of the statement itself. In Valence, this means that computationally intensive or private tasks can be executed off-chain by a "guest program" running on the ZK Coprocessor. This guest program produces a result along with a cryptographic proof attesting to the correctness of that result according to the program's logic.

This proof, which is relatively small and efficient to check, is then submitted to the Valence smart contracts on-chain. The on-chain contracts only need to verify this succinct proof to be assured that the off-chain computation was performed correctly, rather than having to re-execute the entire complex computation themselves. This model brings several advantages, including reduced gas costs, increased transaction throughput, the ability to handle private data, and the capacity to implement more sophisticated logic than would be feasible purely on-chain.

Key terms you will encounter in this documentation include:

- **ZK Coprocessor:** An off-chain service responsible for running "guest programs" and generating ZK proofs of their execution.
- **Guest Program:** A piece of software designed by developers for off-chain execution on the ZK Coprocessor. It comprises two main parts: the **ZK Circuit** (which defines the core ZK-provable computations) and the **Controller** (Wasm-compiled logic that prepares inputs for the circuit, handles its outputs, and interacts with the Coprocessor environment).
- **zkVM (Zero-Knowledge Virtual Machine):** An environment that can execute arbitrary programs and produce a ZK proof of that execution. The Valence ZK Coprocessor leverages such technology (e.g., SP1) to run guest programs.
- **Encoders:** Systems that compress blockchain state into formats suitable for ZK proofs. The Unary Encoder handles single-chain state transitions, while the Merklelized Encoder manages cross-chain state dependencies.
- **Proof:** A small piece of cryptographic data that demonstrates a computation was performed correctly according to a specific program, without revealing all the details of the computation.
- **Public Inputs/Outputs:** The specific data points that are part of the public statement being proven. The ZK proof attests that the guest program correctly transformed certain public inputs into certain public outputs.
- **Witness:** The complete set of inputs, both public (known to prover and verifier) and private (known only to the prover), required by a ZK circuit to perform its computation and allow the generation of a proof. The ZK proof demonstrates that the computation was performed correctly using this witness, without revealing the private inputs.

This set of documentation will guide you through understanding how this ZK system works within Valence, how to develop your own guest programs for the Coprocessor, and how to integrate these ZK-proven results with the on-chain components of the Valence Protocol. For detailed information on how blockchain state is encoded for ZK proofs and cross-chain coordination, see [State Encoding and Encoders](./07_state_encoding_and_encoders.md).
