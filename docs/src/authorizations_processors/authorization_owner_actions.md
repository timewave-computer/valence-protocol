# Owner Actions

- `create_authorizations(vec[Authorization])`: provides an authorization list which is the core information of the authorization contract, it will include all the possible set of actions that can be executed. It will contain the following information:

  - Label: unique name of the authorization. This label will be used to identify the authorization and will be used as subdenom of the tokenfactory token in case it is permissioned. Due to tokenfactory module restrictions, the max length of this field is 44 characters.
    Example: If the label is `withdraw` and only address `neutron123` is allowed to execute this authorization, we will create the token `factory/<contract_addr>/withdraw` and mint one to that address. If `withdraw` was permissionless, there is no need for any token, so it's not created.

  - Mode: can either be `Permissioned` or `Permissionless`. If `Permissionless` is chosen, any address can execute this action list. In case of `Permissioned`, we will also say what type of permissioned type we want (with `CallLimit` or without), a list of addresses will be provided for both cases. In case there is a `CallLimit` we will mint a certain amount of tokens for each address that is passed, in case there isn’t we will only mint one token and that token will be used all the time.

  - NotBefore: from what time the authorization can be executed. We can specify a block height or a timestamp.

  - Expiration: until when (what block or timestamp) this authorization is valid.

  - MaxConcurrentExecutions (default 1): to avoid DDoS attacks and to clog the execution queues, we will allow certain authorizations to be present a maximum amount of times (default 1 unless overwritten) in the execution queue.

  - ActionsConfig: set of actions in a specific order to be executed. This config can be of two types: `Atomic` or `NonAtomic`. For the `Atomic` config, we will provide an array of `Atomic` actions and an optional `RetryLogic` for the entire bundle. For the `NonAtomic` config we will provide simply an array of `NonAtomic` actions.

    - `AtomicAction`: each Atomic action has the following parameters:

      - Domain of execution (must be the same for all actions in v1).

      - MessageDetails: type (e.g. CosmWasmExecuteMsg) and message (name of the message in the ExecuteMsg json that can be executed with, if applied, three list of parameters: one for `MustBeIncluded`, one for `CannotBeIncluded` and one for `MustBeValue`. (This gives more control over the authorizations. Example: we want one authorization to provide the message with parameters (admin action for that service) but another authorization for the message without any Parameters (user action for that service).
      - Contract address that will execute it.

    - `NonAtomicAction`: each NonAtomic action has the following parameters:

      - Domain of execution

      - MessageDetails (like above).

      - Contract address that will execute it.

      - RetryLogic (optional, self-explanatory).

      - CallbackConfirmation (optional): This defines if a NonAtomicAction is completed after receiving a callback (Binary) from a specific address instead of when it executes correctly. This is used in case of the correct message execution not being enough to consider the message completed, so it will define what callback we should receive from a specific address to flag that message as completed. For this, the processor will append an `execution_id` to the message which will be also passed in the callback by the service to identify what action this callback is for.

  - Priority (default Med): priority of a set of actions can be set to High. If this is the case, they will go into a preferential execution queue. Messages in the `High` priority queue will be taken over messages in the `Med` priority queue.
    All authorizations will have an initial state of `Enabled` .

  Here is an example of an Authorization table after its creation:

  ![Authorization Table](../img/authorization_table.png)

- `add_external_domains([external_domains])`: if we want to add external domains after instantiation.

- `modify_authorization(label, updated_values)`: can modify certain updatable fields of the authorization: start_time, expiration, max_concurrent_executions and priority.

- `disable_authorization(label)`: puts an Authorization to state `Disabled`. These authorizations can not be run anymore.

- `enable_authorization(label)`: puts an Authorization to state `Enabled` so that they can be run again.

- `mint_authorization(label, vec[(addresses, Optional: amounts)])`: if the authorization is `Permissioned` with `CallLimit : true`, this action will mint the corresponding token amounts of that authorization to the addresses provided. If `CallLimit: false` it will mint 1 token to the new addresses provided.

- `pause_processor(domain)`: pause the processor of the domain.

- `resume_processor(domain)`: resume the processor of the domain.

- `insert_messages(label, queue_position, queue_type, vec[ProcessorMessage])`: adds these set of messages to the queue at a specific position in the queue.

- `evict_messages(label, queue_position, queue_type)`: remove the set of messages from the specific position in a queue.

- `add_sub_owners(vec[addresses])`: add the current addresses as 2nd tier owners. These sub_owners can do everything except adding/removing admins.

- `remove_sub_owners(vec[addresses])`: remove these addresses from the sub_owner list.
