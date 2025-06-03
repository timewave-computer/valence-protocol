# Valence Protocol Summary

Valence Protocol provides a unified environment for building trust-minimised cross-chain DeFi applications, called Valence Programs. By abstracting over heterogeneous domains and bridge protocols, Valence enables developers to query and update state on multiple chains and orchestrate distributed control logic within a single logical program.

Valence Programs for common workflows can be created using a simple configuration template—or developers can write completely custom programs, leveraging existing libraries to accelerate development.

Valence provides a cross-chain account and authorization system, a processor for local execution. These core contracts provide the basic machinery for managing user interactions, enforcing permissions, handling message execution, batching, retries, and callbacks. From there, an extensible library pattern, allows for integration with external DeFi protocols, or third party contracts.

Valence supports on-chain execution, as well as off-chain excution via specialized ZK Coprocessor. CosmWasm and EVM target environments are available today, with plans for SVM and MoveVM in the coming months. Libraries tailored to each environment facilitate seamless integration with a diversity DeFi protocols and bridges.

Valence provides a Zero-Knowledge Coprocessor system to significantly enhance its capabilities and abstract complex computations from the on-chain layer. The off-chain Coprocessor system runs guest applications within a zkVM and generates cryptographic proofs verifying execution correctness. By offloading intensive tasks and only verifying succinct proofs on-chain, ZK applications increase throughput, and enable more complex cross-chain logic. A common RISC-V compilation target lets developers work in rust, write once and deploy anywhere.

This hybrid architecture combines the trust-minimisation of on-chain verification with the scalability and flexibility of off-chain computation, allowing developers to leverage the strengths of both to create more powerful and efficient cross-chain programs. Valence's flexibility and ease of use enable rapid development and iteration of cross-chain protocols, while its trust minimization makes it essential for building robust cross-chain protocols.