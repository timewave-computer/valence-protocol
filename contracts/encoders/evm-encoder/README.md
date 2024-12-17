# Valence EVM Encoder

The Valence EVM Encoder is a specialized smart contract responsible for encoding instructions that will be executed by **EVM Valence Processors**. It utilizes the Ethereum Application Binary Interface (ABI) encoding standard to perform two critical encoding functions:

Processor Message Encoding: Transforms high-level instructions into a format that can be interpreted by the **Valence Processor**.
Library Function Encoding: Prepares function calls that will be executed by various **Valence libraries** within the EVM environment.

This encoder ensures that complex, multi-step operations can be properly serialized and executed within the Ethereum Virtual Machine, maintaining type safety and execution integrity throughout the process. By handling both processor-level messages and library function calls, it creates a complete encoded instruction set that defines the entire execution flow.

To do this, it will receive the **Subroutine** information, and the messages that need to be encoded according to that subroutine. The subroutine information will contain the **Library** and **Function** that is being targeted for each message so that the encoder can properly encode the message.
