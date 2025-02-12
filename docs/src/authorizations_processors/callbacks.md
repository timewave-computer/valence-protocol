# Callbacks

There are different types of callbacks in our application. Each of them have a specific function and are used in different parts of the application.

## Function Callbacks

For the execution of `NonAtomic` batches, each function in the batch can optionally be confirmed with a callback from a specific address. When the processor reaches a function that requires a callback, it will inject the execution_id of the batch into the message that is going to be executed on the library, which means that the library needs to be ready to receive that execution_id and know what the expected callback is and from where it has to come from to confirm that function, otherwise that function will stay unconfirmed and the batch will not move to the next function. The callback will be sent to the processor with the execution_id so that the processor can know what function is being confirmed. The processor will then validate that the correct callback was received from the correct address.

If the processor receives the expected callback from the correct address, the batch will move to the next function. If it receives a different callback than expected from that address, the execution of that function is considered to have failed and it will be retried (if applicable). In either case, a callback must be received to determine if the function was successful or not.

Note: This functionality is not available on the Lite Processor, as this version of the processor is not able to receive asynchronous callbacks from libraries.

## Processor Callbacks

Once a Processor batch is executed or it fails and there are no more retries available, the Processor will send a callback to the Authorizations contract with the execution_id of the batch and the result of the execution. All this information will be stored in the Authorization contract state so the history of all executions can be queried from it. This is how a `ProcessorCallback` looks like:

```rust
pub struct ProcessorCallbackInfo {
    // Execution ID that the callback was for
    pub execution_id: u64,
    // Who started this operation, used for tokenfactory actions
    pub initiator: OperationInitiator,
    // Address that can send a bridge timeout or success for the message (if applied)
    pub bridge_callback_address: Option<Addr>,
    // Address that will send the callback for the processor
    pub processor_callback_address: Addr,
    // Domain that the callback came from
    pub domain: Domain,
    // Label of the authorization
    pub label: String,
    // Messages that were sent to the processor
    pub messages: Vec<ProcessorMessage>,
    // Optional ttl for re-sending in case of bridged timeouts
    pub ttl: Option<Expiration>,
    // Result of the execution
    pub execution_result: ExecutionResult,
}

pub enum ExecutionResult {
    InProcess,
    // Everthing executed successfully
    Success,
    // Execution was rejected, and the reason
    Rejected(String),
    // Partially executed, for non-atomic function batches
    // Indicates how many functions were executed and the reason the next function was not executed
    PartiallyExecuted(usize, String),
    // Removed by Owner - happens when, from the authorization contract, a remove item from queue is sent
    RemovedByOwner,
    // Timeout - happens when the bridged message times out
    // We'll use a flag to indicate if the timeout is retriable or not
    // true - retriable
    // false - not retriable
    Timeout(bool),
    // Unexpected error that should never happen but we'll store it here if it ever does
    UnexpectedError(String),
}
```

The key information from here is the `label`, to identify the authorization that was executed; the `messages`, to identify what the user sent; and the `execution_result`, to know if the execution was successful, partially successful or rejected.

## Bridge Callbacks

When messages need to be sent through bridges because we are executing batches on external domains, we need to know if, for example, a timeout happened and keep track of it. For this reason we have callbacks per bridge that we support and specific logic that will be executed if they are received. For `Polytone` timeouts, we will check if the `ttl` field has not expired and allow permissionless retries if it's still valid. In case the `ttl` has expired, we will set the ExecutionResult to timeout and not retriable, then send the authorization token back to the user if the user sent it to execute the authorization.
