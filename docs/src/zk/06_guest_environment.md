# Valence ZK Guest Environment

This document describes the specific execution environment provided by the Valence Coprocessor for "guest applications." Understanding this environment is crucial for developers building robust and efficient ZK applications. It complements the information found in [Developing Valence Coprocessor Apps](./02_developing_coprocessor_apps.md).

When a guest program's `controller` crate logic is executed by the Valence ZK Coprocessor, it runs within a specialized, sandboxed environment. This environment imposes certain characteristics and provides specific interfaces for interaction.

### Execution Sandbox

The primary purpose of the sandbox is to securely execute the guest program's Rust code (often compiled to WebAssembly or a similar intermediate representation) that is responsible for generating the witness for the ZK circuit. This isolation prevents a guest program from interfering with the Coprocessor service itself or other concurrently running guest programs.

While the exact nature of the sandbox can evolve, developers should assume an environment with constrained resources. This means that overly complex or long-running computations within the `controller` crate (before handing off to the ZK circuit for proving) should be approached with caution. The main computationally intensive work should ideally be designed into the ZK circuit itself, as that is what the proving system is optimized for.

### Virtual Filesystem

Each deployed guest program is provided with its own private virtual filesystem by the Coprocessor. This filesystem is essential for storing intermediate data, logs, and most importantly, the generated ZK proofs.

Key characteristics and limitations of this virtual filesystem, as indicated by the `valence-coprocessor-app` template examples, include:

- **FAT-16 Basis:** The underlying structure often emulates a FAT-16 filesystem. This implies certain legacy constraints that developers must be aware of.
- **Extension Length:** File extensions are typically limited to a maximum of three characters (e.g., `.bin`, `.txt`, `.log`).
- **Case Insensitivity:** File and directory names are generally treated as case-insensitive (e.g., `Proof.bin` and `proof.bin` would refer to the same file).
- **Path Structure:** Paths are typically Unix-like (e.g., `/var/share/my_proof.bin`).
- **Interaction:** The `controller` crate interacts with this filesystem by sending specific commands to the Coprocessor service rather than through direct OS-level file I/O calls. For example, to store a generated proof, the `controller` constructs a `store` command with the target path and data, which the Coprocessor then writes to the program's virtual disk image.

Developers should design their `controller` logic to work within these constraints, particularly when choosing filenames for storing proofs or other outputs.

### Interfacing with the Coprocessor Service

From within its sandboxed execution, the `controller` crate logic needs to communicate with the host Coprocessor service for several key operations:

- **Signaling Witness Readiness:** After processing inputs and preparing the witness for the ZK circuit, the `controller` must inform the Coprocessor that it is ready for the proving phase to begin.
- **Receiving Proof Results:** The Coprocessor calls a designated entrypoint function within the `controller` crate upon completion of a proof generation task (successful or failed). This entrypoint receives the proof data, initial arguments, and any logs.
- **Filesystem Operations:** As mentioned above, storing data (like the received proof) or logging information involves sending structured requests to the Coprocessor to perform actions on the program's virtual filesystem.

The exact mechanism for this interaction (e.g., specific function calls, message passing, predefined environment variables or handles) is defined by the Coprocessor's execution environment for guest programs.

### Resource Constraints

Guest applications run with finite system resources including limited memory, CPU time, and storage space. Developers should aim for efficiency in their `controller` crate logic, focusing on input processing, witness generation, and handling results rather than performing heavy computations that are better suited for the ZK circuit itself.

Understanding these environment constraints enables developers to build ZK applications that run efficiently on the Valence Coprocessor. 