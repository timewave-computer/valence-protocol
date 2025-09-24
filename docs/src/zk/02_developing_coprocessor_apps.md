# Developing Valence Coprocessor Apps

This guide is designed for developers looking to build Zero-Knowledge (ZK) applications, or "guest programs," for the Valence ZK Coprocessor. It focuses on using the `valence-coprocessor-app` template as a foundation. Before diving in, it is beneficial to have a grasp of the concepts presented in [Introduction to Valence ZK](./_overview.md) and [Valence ZK System Overview](./01_system_overview.md).

The [valence-coprocessor-app template repository](https://github.com/timewave-computer/valence-coprocessor-app) serves as the primary starting point and practical reference for this guide.

### Core Structure of a Coprocessor App

A Valence Coprocessor App (a Guest Program), when based on the template, is primarily structured around two main Rust crates, which compile into the two logical parts of the Guest Program: the Controller and the ZK Circuit.

1. **The `controller` Crate (compiles to the Controller):** This component contains off-chain logic executed as Wasm within the Valence ZK Coprocessor's sandboxed environment. This Controller acts as an intermediary between user inputs and the ZK circuit. Key responsibilities include receiving input arguments (often JSON) for proof requests, processing inputs to generate a "witness" (private and public data the ZK circuit needs), and interacting with the Coprocessor service to initiate proof generation. The Controller handles proof computation results; it has an entrypoint function the Coprocessor calls upon successful proof generation, allowing the Controller to store the proof or log information. The Controller can utilize a virtual filesystem provided by the Coprocessor, which is FAT-16 based (implying constraints like 3-character file extensions and case-insensitive paths), for persistent data storage.

2. **The `circuit` Crate (defines the ZK Circuit):** This crate defines the ZK Circuit itself. The ZK Circuit is the heart of the ZK application, containing the actual computations and assertions whose correctness will be proven. It's typically written using a specialized language or Domain-Specific Language (DSL) that compiles down to a ZK proving system supported by the Coprocessor (for example, SP1). The ZK Circuit receives the witness data prepared by the Controller. It then performs its defined computations and assertions. If all these pass, it produces a public output (as a `Vec<u8>`), which represents the public statement that will be cryptographically verified on-chain. This output forms a crucial part of the "public inputs" of the ZK proof.

While these two crates form the core, the template might also include an optional `./crates/domain` crate. This is generally intended for more advanced scenarios, such as defining how to derive state proofs from JSON arguments or for validating block data that might be incorporated within the Coprocessor's operations, though its direct use can vary significantly depending on the specific application's needs.

### General Development Workflow

Developing a Coprocessor App typically follows a sequence of steps from setup to deployment and testing:

1. **Environment Setup:** The initial step involves preparing your development environment. This requires installing Docker, a recent Rust toolchain, and the Cargo Valence subcommand (the `cargo-valence` CLI included in this repository). You would then clone the `valence-coprocessor-app` template repository to serve as the foundation for your new ZK application. For development, you can either use the public Valence ZK Coprocessor service at `https://service.coprocessor.valence.zone` (default socket) or optionally run a [local instance](https://github.com/timewave-computer/valence-coprocessor#local-execution).

2. **ZK Circuit Development (`./crates/circuit`):** The next phase is to define the logic of your ZK circuit. This involves specifying the exact computations to be performed, the private inputs (the witness) that the circuit will consume, and the public inputs or outputs it will expose. The public output of your ZK circuit (a `Vec<u8>`) is of particular importance, as this is the data that will ultimately be verified on-chain. It's essential to remember that the first 32 bytes of the *full* public inputs (as seen by the on-chain verifier) are reserved by the Coprocessor for its own internal root hash; your application-specific public output data will follow these initial 32 bytes.

3. **Controller Development (`./crates/controller`):** Concurrently, you'll develop the Controller logic within the `controller` crate. This includes implementing the logic to parse incoming JSON arguments that are provided when a proof is requested for your application. You will also need to write the code that transforms these user-provided arguments into the precise witness format required by your ZK circuit. A key part of the Controller is its entrypoint function; this function is called by the Coprocessor service when a proof for your program has been successfully generated and is ready. This entrypoint typically receives the proof itself, the initial arguments that triggered the request, and any logs generated during the process. You must also implement how your Controller should handle this generated proof – a common pattern is to store it to a specific path (e.g., `/var/share/proof.bin`) within its virtual filesystem using a `store` command payload directed to the Coprocessor.

4. **Application Build and Deployment:** Once the ZK Circuit (from `circuit` crate) and Controller (from `controller` crate) are developed, build and deploy your Guest Program using the `cargo-valence` CLI. Example:

   `cargo-valence deploy circuit --controller ./crates/controller --circuit <circuit-crate-project-name>`

   The CLI defaults to `https://service.coprocessor.valence.zone`; specify `--socket <url>` if targeting a different endpoint. This compiles both crates (Controller to Wasm) and submits them to the service. On success, the service returns a controller ID (e.g., `8965...df783`) used in subsequent requests.

5. **Requesting Proof Generation:** With your Guest Program deployed and its Controller ID known, request proving with:

   `cargo-valence prove -j '{"value": 42}' -p /var/share/proof.bin <CONTROLLER_ID>`

   Replace the JSON with the expected controller input. The `-p` path tells the controller where to store the resulting proof within the virtual filesystem. The CLI encapsulates this as a payload `{ cmd: "store", path: "/var/share/proof.bin" }`, which the service passes to the controller entrypoint after proving.

6. **Retrieving Proofs and Public Inputs:** After proving completes and the proof is stored by your controller, retrieve it with:

   `cargo-valence storage -p /var/share/proof.bin <CONTROLLER_ID> | jq -r '.data' | base64 -d | jq`

   To view the public inputs:

   `cargo-valence proof-inputs -p /var/share/proof.bin <CONTROLLER_ID> | jq -r '.inputs' | base64 -d | hexdump -C`

   The first 32 bytes represent the Coprocessor root; your circuit output follows.

This workflow allows for an iterative development process, enabling you to test and refine your ZK guest programs effectively. 

Note: As an alternative to `cargo-valence`, you can use the `valence-coprocessor` binary from the domain clients toolkit to call the same REST API directly. Both approaches interact with the Coprocessor using the endpoints and payload conventions described in Coprocessor Internals → Service API.

### Client Library Usage

The `valence-domain-clients` crate provides a Coprocessor client and helpers that call the REST API, submit proving jobs, and poll the virtual filesystem for generated proofs.

- Default base URL: `https://service.coprocessor.valence.zone`
- REST base path: `/api`
- Typical flow:
  1. Submit a prove request with a “store” payload specifying a virtual filesystem path.
  2. Poll the storage file endpoint until the proof appears.
  3. Decode the proof and extract public inputs for on‑chain submission.

Headers used by clients (see Coprocessor Internals → Service API for details):
- `valence-coprocessor-circuit`: hex controller ID
- `valence-coprocessor-root`: historical root hex to pin requests
- `valence-coprocessor-signature`: optional signature over JSON body

Example (async Rust):

```rust
use serde_json::json;
use valence_domain_clients::clients::coprocessor::CoprocessorClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = CoprocessorClient::default();
    let circuit = "<controller_id_hex>";
    let root = client.root().await?; // optional pin to current

    // Submit prove (with store payload) and poll storage for the proof
    let args = json!({ "value": 42 });
    let proof = client.get_single_proof(circuit, &args, &root).await?;

    // Decode base64 proof and inputs
    let (_proof_bytes, inputs) = proof.decode()?;
    println!("inputs length: {}", inputs.len());
    Ok(())
}
```

### Incorporating Verifiable External State

Guest programs on the Valence Coprocessor can be designed to utilize verifiable state from external blockchains, like Ethereum. This allows ZK applications to react to or incorporate off-chain data in a trust-minimized way. Services such as the state proof service facilitate this by generating state proofs (e.g., Merkle proofs for account balances or storage slots on Ethereum at specific block heights). Currently, this interaction for fetching external state is often achieved via ABI-encoded HTTP calls, though future implementations might support other protocols like WebSockets.

When developing a guest program, you would design its Controller (within the `controller` crate) to accept such state proofs as part of its input. The ZK `circuit` can then use the proven external state in its computations. The resulting ZK proof from the Valence Coprocessor will thus attest to the correctness of operations performed on this externally verified data. More detailed architectural considerations for this pattern, including how the Coprocessor environment might support or interact with such external proofs, are discussed in [ZK Coprocessor Internals](./04_coprocessor_internals.md). 

### Apps ownership

The apps will be owned by a private key, only if their deployment is signed by the client. The Valence Domain client employs an environment variable `VALENCE_SIGNER` for specifying its secret key during signature processes. When a signature becomes available for a deployed app, the controller bytecode, storage, and dedicated prover list will only be modifiable upon signing a request using the provided secret key.

The initial phase involves installing the valence-coprocessor binary:

```sh
cargo install valence-domain-clients \
  --no-default-features \
  --features coprocessor-bin \
  --bin valence-coprocessor
```

You can verify the installation by running:

```sh
valence-coprocessor --version
```

To create the signer key, utilize the [foundry](https://getfoundry.sh/cast/reference/wallet/) tool:

```sh
cast wallet new --account valence
```

The private key can be retrieved as follows:

```sh
cast wallet private-key --account valence
```

An easy way to store it in the appropriate environment variable:

```sh
export VALENCE_SIGNER='{"SecretEccNistP256":"'$(cast wallet private-key --account valence)'"}'
```

This readies the environment for utilizing an EccNistP256 signer. Every invocation of the `valence-coprocessor` binary will leverage such environment variable and sign the requests accordingly.

To view the allocated GPU workers associated with the key:

```sh
$ valence-coprocessor provers get
Using valence signer `EccNistP256(...)`...
Fetching provers...
{"owned":[],"public":["wss://prover.coprocessor.valence.zone"]}
```

The user may assign a specific prover to their app:

```sh
valence-coprocessor provers add 'wss://prover.coprocessor.valence.zone'
```

The co-processor will cycle through the available dedicated GPU provers of the app to generate proofs.
