# Processor Contract

The Processor will be a contract on each domain within the program. The Processor handles execution of `Message Batches` it receives from the Authorization contract.
Depending on the Processor type in use, its features will vary. There are currently two types of processors: Lite Processor and Processor. The former is a simplified version of the latter. The Lite Processor has limited functionality to optimize for gas-constrained domains.

The Processor will be instantiated in advance with the correct address that can send messages to them, according to the _InstantiationFlow_ described in the [Assumptions](assumptions.md) section.
