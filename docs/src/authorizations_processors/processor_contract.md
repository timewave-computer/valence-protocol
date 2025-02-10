# Processor Contract

The `Processor` will be a contract on each domain of our workflow. It handles the execution of the `Message Batches` that are sent to it by the `Authorization` contract. Depending on the
type of `Processor` being used, the features will vary. We currently have two types of processors: `LiteProcessor` and `Processor`. The former is a simplified version of the latter, with more limited functionality and optimized for specific domains where gas costs are a concern.

The `Processor` will be instantiated in advance with the correct address that can send messages to them, according to the _InstantiationFlow_ described in the [Assumptions](assumptions.md) section.
