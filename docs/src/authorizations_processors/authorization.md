# Authorization

The authorization contract will be a single contract deployed on the main domain and that will define the authorizations of the top-level application, which can include libraries in different domains (chains). For each domain, there will be one Processor (with its corresponding execution queues). The `Authorization` contract will connect to all of the `Processors` using a connector (e.g. Polytone, Hyperlane…) and will route the `Message Batches` to be executed to the right domain. At the same time, for each external domain, we will have a proxy contract in the main domain which will receive the callbacks sent from the processor on the external domain with the `ExecutionResult` of the `Message Batch`.

The contract will be instantiated once at the very beginning and will be used during the entire top-level application lifetime. Users will never interact with the individual Smart Contracts of each workflow, but with the Authorization contract directly.
