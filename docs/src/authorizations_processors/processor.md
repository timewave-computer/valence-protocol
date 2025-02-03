# Processor

The `Processor` will be a contract on each domain of our workflow. It handles the execution queues which contain `Message Batches`. The `Processor` can be `ticked` permissionlessly, which will execute the next `Message Batch` in the queue if this one is executable or rotate it to the back of the queue if it isn't executable yet. The processor will also handle the `Retry` logic for each batch (if the batch is atomic) or function (if the batch is non atomic). After a `Message Batch` has been executed successfully or it reached the maximum amount of retries, it will be removed from the execution queue and the `Processor` will send a callback with the execution information to the `Authorization` contract.

The processors will be instantiated in advance with the correct address that can send messages to them, according to the _InstantiationFlow_ described in the [Assumptions](assumptions.md) section.

The `Authorization` contract will be the only address allowed to add list of functions to the execution queues. It will also be allowed to Pause/Resume the `Processor` or to arbitrarily remove functions from the queues or add certain messages at a specific position.

There will be two execution queues: one `High` and one `Med`. This will allow giving different priorities to `Message`.

### Execution

When a processor is `Ticked` we will take the first `MessageBatch` from the queue (`High` if there are batches there or `Med` if there aren’t).
After taking them, we will execute them in different ways depending if the batch is `Atomic` or `NonAtomic`.

- For `Atomic` batches, the `Processor` will execute them by sending them to itself and trying to execute them in a `Fire and Forget` manner. If this execution fails, we will check the `RetryLogic` of the batch to decide if they are to be re-queued or not (if not, we will send a callback with `Rejected` status to the authorization contract).
  If they succeeded we will send a callback with `Executed` status to the Authorization contract.
- For `NonAtomic` batches, we will execute the functions one by one and applying the RetryLogic individually to each function if they fail. `NonAtomic` functions might also be confirmed via `CallbackConfirmations` in which case we will keep them in a separate Map until we receive that specific callback.
  Each time a function is confirmed, we will re-queue the batch and keep track of what function we have to execute next.
  If at some point a function uses up all its retries, we will send a callback to the Authorization contract with a `PartiallyExecuted(num_of_functions_executed)` status. If all of them succeed it will be `Executed` and if none of them were it will be `Rejected`.
  For `NonAtomic` batches, we need to tick the processor each time the batch is at the top of the queue to continue, so we will need at least as many ticks as number of functions we have in the batch, and each function has to wait for its turn.

### Storage

The `Processor` will receive batches of messages from the authorization contract and will enqueue them in a custom storage structure we designed for this purpose, called a `QueueMap`. This structure is a FIFO queue with owner privileges (allows the owner to insert or remove from any position in the queue).
Each “item” stored in the queue is an object `MessageBatch` that looks like this:

```rust
pub struct MessageBatch {
    pub id: u64,
    pub msgs: Vec<ProcessorMessage>,
    pub subroutine: Subroutine,
    pub priority: Priority,
    pub retry: Option<CurrentRetry>,
}
```

- id: represents the global id of the batch. The `Authorization` contract, to understand the callbacks that it will receive from each processor, identifies each batch with an id. This id is unique for the entire application.
- msgs: the messages the processor needs to execute for this batch (e.g. a CosmWasm ExecuteMsg or MigrateMsg).
- subroutine: This is the config that the authorization table defines for the execution of these functions. With this field we can know if the functions need to be executed atomically or not atomically, for example, and the retry logic for each batch/function depending on the config type.
- priority (for internal use): batches will be queued in different priority queues when they are received from the authorization contract. We also keep this priority here because they might need to be re-queued after a failed execution and we need to know where to re-queue them.
- retry (for internal use): we are keeping the current retry we are at (if the execution previously failed) to know when to abort if we exceed the max retry amounts.
