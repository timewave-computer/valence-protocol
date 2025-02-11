# Owner Functions

- `create_authorizations(vec[Authorization])`: provides an authorization list which is the core information of the authorization contract, it will include all the possible set of functions that can be executed. It will contain the following information:

  - Label: unique name of the authorization. This label will be used to identify the authorization and will be used as subdenom of the tokenfactory token in case it is permissioned. Due to tokenfactory module restrictions, the max length of this field is 44 characters.
    Example: If the label is `withdraw` and only address `neutron123` is allowed to execute this authorization, we will create the token `factory/<contract_addr>/withdraw` and mint one to that address. If `withdraw` was permissionless, there is no need for any token, so it's not created.

  - Mode: can either be `Permissioned` or `Permissionless`. If `Permissionless` is chosen, any address can execute this function list. In case of `Permissioned`, we will also say what type of permissioned type we want (with `CallLimit` or without), a list of addresses will be provided for both cases. In case there is a `CallLimit` we will mint a certain amount of tokens for each address that is passed, in case there isn’t we will only mint one token and that token will be used all the time.

  - NotBefore: from what time the authorization can be executed. We can specify a block height or a timestamp.

  - Expiration: until when (what block or timestamp) this authorization is valid.

  - MaxConcurrentExecutions (default 1): to avoid DDoS attacks and to clog the execution queues, we will allow certain authorizations subroutines to be present a maximum amount of times (default 1 unless overwritten) in the execution queue.

  - Subroutine: set of functions in a specific order to be executed. Subroutines can be of two types: `Atomic` or `NonAtomic`. For the `Atomic` subroutines, we will provide an array of `Atomic` functions and an optional `RetryLogic` for the entire subroutine. For the `NonAtomic` subroutines we will just provide an array of `NonAtomic` functions.

    - `AtomicFunction`: each Atomic function has the following parameters:

      - Domain of execution (must be the same for all functions in v1).

      - MessageDetails: type (e.g. CosmwasmExecuteMsg, EvmCall ...) and message information. Depending on the type of the message that is being sent, we might need to provide additional values and/or only some specific `ParamRestrictions` can be applied:
        - If we are sending messages that are not for a `CosmWasm ExecutionEnvironment` and the message passed doesn't contain Raw bytes for that particular VM (e.g. `EvmRawCall`), we need to provide the `Encoder` information for that message along with the name of the library that the `Encoder` will use to encode that message. For example, if we are sending a message for an `EvmCall` on an EVM domain, we need to provide the address of the `Encoder Broker` and the `version` of the `Encoder` that the broker needs to route the message to along with the name of the library that the `Encoder` will use to encode that message (e.g. `forwarder`).
        - For all messages that are not raw bytes (`json` formatted), we can apply any of the following `ParamRestrictions`:
          - `MustBeIncluded`: the parameter must be included in the message.
          - `CannotBeIncluded`: the parameter cannot be included in the message.
          - `MustBeValue`: the parameter must have a specific value.
        - For all messages that are raw bytes, we can only apply the `MustBeBytes` restriction, which matches that the bytes sent are the same as the ones provided in restriction, limiting the authorization execution to only one specific message.

      - Contract address that will execute it.

    - `NonAtomicFunction`: each NonAtomic function has the following parameters:

      - Domain of execution

      - MessageDetails (same as above).

      - Contract address that will execute it.

      - RetryLogic (optional, self-explanatory).

      - CallbackConfirmation (optional): This defines if a `NonAtomicFunction` is completed after receiving a callback (Binary) from a specific address instead of after a correct execution. This is used in case of the correct message execution not being enough to consider the message completed, so it will define what callback we should receive from a specific address to flag that message as completed. For this, the processor will append an `execution_id` to the message which will be also passed in the callback by the service to identify what function this callback is for.

  - Priority (default Med): priority of a set of functions can be set to High. If this is the case, they will go into a preferential execution queue. Messages in the `High` priority queue will be taken over messages in the `Med` priority queue.
    All authorizations will have an initial state of `Enabled` .

  Here is an example of an Authorization table after its creation:

  ![Authorization Table](../img/authorization_table.png)

- `add_external_domains([external_domains])`: to add an `ExternalDomain` to the authorization contract, the owner will specify what type of `ExecutionEnvironment` it has (e.g. `CosmWasm`, `Evm`...) and all the information required for each type of `ExecutionEnvironment`. For example, if we are adding a domain that uses `CosmWasm` as ExecutionEnvironment, we need to provide all the Polytone information; if we are adding a domain that uses `EVM` as ExecutionEnvironment, we need to provide all the Hyperlane information and the `Encoder` to be used for encoding the messages.

- `modify_authorization(label, updated_values)`: can modify certain updatable fields of the authorization: start_time, expiration, max_concurrent_executions and priority.

- `disable_authorization(label)`: puts an Authorization to state `Disabled`. These authorizations can not be run anymore.

- `enable_authorization(label)`: puts an Authorization to state `Enabled` so that they can be run again.

- `mint_authorization(label, vec[(addresses, Optional: amounts)])`: if the authorization is `Permissioned` with `CallLimit: true`, this function will mint the corresponding token amounts of that authorization to the addresses provided. If `CallLimit: false` it will mint 1 token to the new addresses provided.

- `pause_processor(domain)`: pause the processor of the domain.

- `resume_processor(domain)`: resume the processor of the domain.

- `insert_messages(label, queue_position, queue_type, vec[ProcessorMessage])`: adds these set of messages to the queue at a specific position in the queue.

- `evict_messages(label, queue_position, queue_type)`: remove the set of messages from the specific position in a queue.

- `add_sub_owners(vec[addresses])`: add the current addresses as 2nd tier owners. These sub_owners can do everything except adding/removing admins.

- `remove_sub_owners(vec[addresses])`: remove these addresses from the sub_owner list.
