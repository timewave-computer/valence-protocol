# Lite Processor

This is a simplified version of the [Processor](processor.md) contract, with more limited functionality that is optimized for specific domains where gas costs are critical. This version of the processor is currently available for `EVM` execution environments only.

The main difference between the Lite Processor and the Processor is that the former does not store message batches, but instead executes messages directly when received. The Lite Processor does not handle retries, function callbacks, or queues. More details can be found below.

### Execution

The Lite Processor is not `ticked`, instead it will receive a `MessageBatch` from the Authorization contract and execute it immediately. Therefore, the execution gas cost will be paid by the relayer of the batch instead of the user who ticks the processor.
There might be a case that the `MessageBatch` received is already expired, which can happen if the relayer was not working or was slow to send the batch. In this case, the Processor will discard the batch and return an `Expired(0)` `ExecutionResult` to the Authorization contract.

This processor does not store batches or use any queue, instead it will simply receive the batch, execute it atomically or non-atomically, and send a callback to the Authorization contract with the `ExecutionResult`. The only information stored by this processor is the information of the Authorization contract, the information of the Connector (e.g. Hyperlane Mailbox, origin domain id, ...) and the authorized entities that can also execute batches on it without requiring them to be sent from the main domain.

Since there are no queues, operations like `InsertAt` or `RemoveFrom` queue that the owner of the Authorization Contract may perform on the Processor are not available on the Lite Processor. Therefore the operations that the Lite Processor supports from the Authorization contract are limited to: `Pause`, `Resume` and `SendMsgs`.

In addition to the limitations above, the Lite Processor does not support retries or function callbacks. This means that the `MessageBatch` received will be executed only once and the `NonAtomic` batches can not be confirmed asynchronously because batch execution will be attempted once, non-atomically, the moment it is received.

In addition to executing batches that come from the Authorization contract, the Lite Processor defines a set of authorized addresses that can send batches to it for execution. Since the Processor can execute batches from any address, we only send a callback if the address that sent the batch is a smart contract. Thus the authorized addresses are in charge of the handling/ignoring of these callbacks.
