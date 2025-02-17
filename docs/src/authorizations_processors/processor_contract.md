# Processor Contract

The Processor will be a contract on each domain within the program. The Processor handles execution of message batches it receives from the Authorization contract.
Depending on the Processor type in use, its features will vary. There are currently two types of processors: Lite Processor and Processor. The former is a simplified version of the latter. The Lite Processor has limited functionality to optimize for gas-constrained domains.

The Processor will be instantiated in advance with the correct address that can send messages to them, according to the _InstantiationFlow_ described in the [Assumptions](assumptions.md) section.

In the table below we summarize the main characteristics of the processors supported:

|                                                   | Processor                     | Lite Processor                      |
| ------------------------------------------------- | ----------------------------- | ----------------------------------- |
| **Execution Environment**                         | CosmWasm                      | EVM                                 |
| **Stores batches in queues**                      | Yes, FIFO queue with priority | No, executed immediately by relayer |
| **Needs to be ticked**                            | Yes, permissionlessly         | No                                  |
| **Messages can be retried**                       | Yes                           | No                                  |
| **Can confirm non-atomic function with callback** | Yes                           | No                                  |
| **Supports Pause operation**                      | Yes                           | Yes                                 |
| **Supports Resume operation**                     | Yes                           | Yes                                 |
| **Supports SendMsgs operation**                   | Yes                           | Yes                                 |
| **Supports InsertMsgs operation**                 | Yes                           | No, no queues to insert in          |
| **Supports EvictMsgs operation**                  | Yes                           | No, no queues to remove from        |
