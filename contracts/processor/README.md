# Processor Contract

The `Processor` will be a contract that will be sitting on each domain and, when ticked (entry point that will check the execution queues and see if there is any set of functions pending to be executed), will take the next list of functions in the queue and execute it for the workflow on that domain. It will also be in charge of dealing with the `Retry` logic for each message in the function list. If the list of functions executes successfully or we’ve gone through the entire `Retry` logic for any of the messages and they couldn’t be executed, they will be removed from the corresponding queue.

The processors will be instantiated in advanced with the correct address that can send messages to it according to the flowchart defined in the previous section.

The `Authorization` contract will be the only address allowed to add list of functions to the execution queues. It will also be allowed to Pause/Resume the `Processor` or to arbitrarily remove functions from the queues or add certain messages at a specific position.

The contract ticking is permissionless and will just go over the execution queues in a round-robin fashion.

There will be two execution queues: one `High` and one `Med`. This will allow to give priority to certain set of functions.
