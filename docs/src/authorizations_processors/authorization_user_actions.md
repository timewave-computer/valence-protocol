# User Actions

## CosmWasm

- `send_msgs(label, vec[ProcessorMessage])`: users can run an authorization with a specific label. If the authorization is `Permissioned (without limit)`, the Authorization contract will check if their account is allowed to execute by checking that the account holds the token in its wallet. If the authorization is `Permissioned (with limit)` the account must attach the authorization token to the contract execution. Along with the authorization label, the user will provide an array of encoded messages, together with the message type (e.g. `CosmwasmExecuteMsg`, `EvmCall`, etc.) and any other parameters for that specific ProcessorMessage (e.g. for a `CosmwasmMigrateMsg` we need to also pass a code_id). The contract will then check that the messages match those defined in the authorization, that the messages appear in correct order, and that any applied parameter restrictions are correct.

  If all checks are correct, the contract will route the messages to the correct Processor with an `execution_id` for the processor to callback with. This `execution_id` is unique for the entire application.
  If execution of all actions is confirmed via a callback, the authorization token is burned. If execution fails, the token is sent back.
  Here is an example flowchart of how a user interacts with the Authorization contract to execute functions on an external CosmWasm domain that is connected to the main domain with Polytone:

![User flowchart](../img/user_flowchart.png)

## EVM

- `sendProcessorMessage(string label, bytes message)`: users submit an ABI‑encoded `ProcessorMessage` (as defined in `IProcessorMessageTypes`) with the target label. The contract verifies the sender against the label’s allowlist and validates the messages against the subroutine’s function set (contract address + function selector or call hash). For SendMsgs/InsertMsgs, the Authorization injects the current `executionId` before forwarding to the Processor. There is no tokenization on EVM; authorization is address/function constrained.
