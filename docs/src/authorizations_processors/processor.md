# Processor

This version of the processor is currently available for `CosmWasm` Execution Environment only. It contains all the features and full functionality of the processor as described below.

It handles two execution queues: `High` and `Med`, which allow giving different priorities to message batches. The Authorization contract will send the message batches to the Processor specifying the priority of the queue where they should be enqueued.

The Processor can be `ticked` permissionlessly, which will trigger the execution of the message batches in the queues in a `FIFO` manner. It will handle the `Retry` logic for each batch (if the batch is atomic) or function (if the batch is non-atomic). In the particular case that the current batch at the top of the queue is not retriable yet, the processor will rotate it to the back of the queue. After a `MessageBatch` has been executed successfully or it reached the maximum amount of retries, it will be removed from the execution queue and the Processor will send a callback with the execution information to the Authorization contract.

The Authorization contract will be the only address allowed to add message batches to the execution queues. It will also be allowed to Pause/Resume the Processor or to arbitrarily remove functions from the queues or add certain messages at a specific position in any of them.

### Execution

When a processor is `Ticked`, the first `Message Batch` will be taken from the queue (`High` if there are batches there or `Med` if there aren’t).
After taking the `Message Batch`, the processor will first check if the batch is expired. If that's the case, the processor will discard the batch and return an `Expired(executed_functions)` `ExecutionResult` to the Authorization contract. There might be a case that the batch is `NonAtomic` and it's already partially executed, therefore the processor also returns the number of functions that were executed before the expiration.
If the batch has not expired, the processor will execute the batch according to whether it is `Atomic` or `NonAtomic`.

- For `Atomic` batches, the Processor will execute either all functions or none of them. If execution fails, the batch `RetryLogic` is checked to determine if the match should be re-enqueued. If not, a callback is sent with a `Rejected(error)` status to the Authorization contract.
  If the execution succeeded we will send a callback with `Executed` status to the Authorization contract.

- For `NonAtomic` batches, we will execute the functions one by one and applying the RetryLogic individually to each function if they fail. `NonAtomic` functions might also be confirmed via `CallbackConfirmations` in which case we will keep them in a separate storage location until we receive that specific callback.
  Each time a function is confirmed, we will re-queue the batch and keep track of what function we have to execute next.
  If at some point a function uses up all its retries, the processor will send a callback to the Authorization contract with a `PartiallyExecuted(num_of_functions_executed, execution_error)` execution result if some succeeded or `Rejected(error)` if none did. If all functions are executed successfully, an `Executed` execution result will be sent.
  For `NonAtomic` batches, the processor must be ticked each time the batch is at the top of the queue to continue, so at least as many ticks will be required as the number of functions in the batch.

### Storage

The Processor will receive message batches from the Authorization contract and will enqueue them in a custom storage structure called a `QueueMap`. This structure is a FIFO queue with owner privileges, which allow the owner to insert or remove messages from any position in the queue.
Each “item” stored in the queue is a `MessageBatch` object that has the following structure:

```rust
pub struct MessageBatch {
    pub id: u64,
    pub msgs: Vec<ProcessorMessage>,
    pub subroutine: Subroutine,
    pub priority: Priority,
    pub expiration_time: Option<u64>,
    pub retry: Option<CurrentRetry>,
}
```

- id: represents the global id of the batch. The Authorization contract, to understand the callbacks that it will receive from each processor, identifies each batch with an id. This id is unique for the entire application.
- msgs: the messages the processor needs to execute for this batch (e.g. a CosmWasm ExecuteMsg or MigrateMsg).
- subroutine: This is the config that the authorization table defines for the execution of these functions. With this field we can know if the functions need to be executed atomically or not atomically, for example, and the retry logic for each batch/function depending on the config type.
- priority (for internal use): batches will be queued in different priority queues when they are received from the Authorization contract. We also keep this priority here because they might need to be re-queued after a failed execution and we need to know where to re-queue them.
- expiration_time: optional absolute timestamp after which the batch is considered expired by the Processor. When set and already expired at processing time, the batch yields an Expired result (with the number of functions executed so far for NonAtomic).
- retry (for internal use): we are keeping the current retry we are at (if the execution previously failed) to know when to abort if we exceed the max retry amounts.
