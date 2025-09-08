# Processor Contract

The Processor contract exists on each domain within a Valence Program and handles execution of message batches received from the Authorization contract. There are currently two main processor implementations with different capabilities and execution models.

The Full Processor (CosmWasm) provides comprehensive message processing with sophisticated queue management. It uses a priority queue system with High and Medium priority FIFO queues where High priority is processed first. The processor uses tick-based execution with a permissionless `tick()` function that processes queued messages. It includes advanced retry logic with function-level and batch-level retry configurations, callback confirmation support where non-atomic functions can require callback confirmations, comprehensive state management with Active/Paused states, Polytone integration for Cosmos cross-chain operations, and support for both atomic and non-atomic execution models with different retry behaviors.

For message processing, the Full Processor enqueues messages with priority and expiration handling. The `tick()` function processes the queue by handling High priority first, then Medium priority. Expired messages are removed and callbacks sent. Retry cooldown is enforced between retry attempts. For atomic execution, all messages execute in a single transaction, while non-atomic execution processes messages sequentially with per-function retry logic.

## Lite Processor

The Lite Processor (EVM) is optimized for gas-constrained environments with immediate execution. It processes messages immediately without a queuing system and includes cross-chain support capabilities. The Lite Processor supports both cross-chain messages and authorized addresses for dual access control. It has limited message types, supporting only Pause, Resume, and SendMsgs operations. Expiration handling validates message expiration before execution, and it includes an automatic callback system for contract senders.

For execution flow, the Lite Processor receives messages via cross-chain handlers or direct calls. It validates sender and origin, checks expiration, immediately executes the subroutine (atomic or non-atomic), and sends callbacks if the sender is a contract.

The table below summarizes the main characteristics of the processors supported:

|                                                   | Full Processor (CosmWasm)     | Lite Processor (EVM)               |
| ------------------------------------------------- | ----------------------------- | ----------------------------------- |
| **Execution Model**                               | Queue-based with tick         | Immediate execution                 |
| **Stores batches in queues**                      | Yes, FIFO queue with priority | No, executed immediately            |
| **Needs to be ticked**                            | Yes, permissionlessly         | No                                  |
| **Messages can be retried**                       | Yes, with complex retry logic | No                                  |
| **Can confirm non-atomic function with callback** | Yes                           | No                                  |
| **Supports Pause operation**                      | Yes                           | Yes                                 |
| **Supports Resume operation**                     | Yes                           | Yes                                 |
| **Supports SendMsgs operation**                   | Yes                           | Yes                                 |
| **Supports InsertMsgs operation**                 | Yes                           | No, no queues to insert in          |
| **Supports EvictMsgs operation**                  | Yes                           | No, no queues to remove from        |

Both processors are instantiated with the correct Authorization contract address and implement robust access control to ensure only authorized messages are processed. The choice between processors depends on the execution environment requirements, with CosmWasm supporting full queue-based processing and EVM optimizing for immediate execution with lower gas costs.