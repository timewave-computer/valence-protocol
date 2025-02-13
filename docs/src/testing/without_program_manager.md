# Example without Program Manager

This example demonstrates how to test your program without the Program Manager after your initial testing set up has been completed as described in the [Initial Testing Set Up](./setup.md) section.

> **Use-case**: In this particular example, we will show you how to create a program that liquid stakes NTRN tokens on a Persistence chain directly from a base account without the need of using libraries. Note that this example is just for demonstrating purposes. In a real-world scenario, you would not liquid stake NTRN as it is not a staking token. We also are not using a liquid staking library for this example, although one could be creating for this purpose.

The full code for this example can be found in the [Persistence Liquid Staking example](https://github.com/timewave-computer/valence-protocol/blob/main/e2e/examples/persistence_ls.rs).

1. Set up the Authorization contract and processor on the `Main Domain` (Neutron).

```rust
    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );

    let (authorization_contract_address, _) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;
```

This code sets up the Authorization contract and processor on Neutron. We use a time based salt to ensure that each test run the generated contract addresses are different. The `set_up_authorization_and_processor` function is a helper function instantiates both the Processor and Authorization contracts on Neutron and provides the contract addresses to interact with both. As you can see, we are not using the Processor on Neutron here, but we are still setting it up.

2. Set up an external domain and create a channel to start relaying messages.

```rust
    let processor_on_persistence = set_up_external_domain_with_polytone(
        &mut test_ctx,
        PERSISTENCE_CHAIN_NAME,
        PERSISTENCE_CHAIN_ID,
        PERSISTENCE_CHAIN_ADMIN_ADDR,
        LOCAL_CODE_ID_CACHE_PATH_PERSISTENCE,
        "neutron-persistence",
        salt,
        &authorization_contract_address,
    )?;
```

This function does the following:
- Instantiates all the Polytone contracts on both the main domain and the new external domain. The information of the external domain is provided in the function arguments.
- Creates a channel between the Polytone contracts that the relayer will use to relay messages between the Authorization contract and the processor.
- Instantiates the Processor contract on the external domain with the correct Polytone information and the Authorization contract address.
- Adds the external domain to Authorization contract with the Polytone information and the processor address on the external domain.

After this is done, we can start creating authorizations for that external domain and when we send messages to the Authorization contract, the relayer will relay the messages to the processor on the external domain and return the callbacks.

3. Create one or more base accounts on a domain.

```rust
    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        PERSISTENCE_CHAIN_NAME,
        base_account_code_id,
        PERSISTENCE_CHAIN_ADMIN_ADDR.to_string(),
        vec![processor_on_persistence.clone()],
        1,
        None,
    );
    let persistence_base_account = base_accounts.first().unwrap();
```

This function creates a base account on the external domain and grants permission to the processor address to execute messages on its behalf. If we were using a library instead, we would be granting permission to the library contract instead of the processor address in the array provided.

4. Create the authorization

```rust
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("execute")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(Domain::External(PERSISTENCE_CHAIN_NAME.to_string()))
                        .with_contract_address(LibraryAccountType::Addr(
                            persistence_base_account.clone(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "execute_msg".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    info!("Creating execute authorization...");
    let create_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations { authorizations },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&create_authorization).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
    info!("Execute authorization created!");
```

In this code snippet, we are creating an authorization to execute a message on the persistence base account. For this particular example, since we are going to execute a `CosmosMsg::Stargate` directly on the account passing the protobuf message, we are not going to set up any param restrictions. If we were using a library, we could potentially set up restrictions for the json message that the library would expect.

5. Send message to the Authorization contract

```rust
info!("Send the messages to the authorization contract...");

    let msg_liquid_stake = MsgLiquidStake {
        amount: Some(Coin {
            denom: neutron_on_persistence.clone(),
            amount: amount_to_liquid_stake.to_string(),
        }),
        delegator_address: persistence_base_account.clone(),
    };
    #[allow(deprecated)]
    let liquid_staking_message = CosmosMsg::Stargate {
        type_url: msg_liquid_stake.to_any().type_url,
        value: Binary::from(msg_liquid_stake.to_proto_bytes()),
    };

    let binary = Binary::from(
        serde_json::to_vec(&valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
            msgs: vec![liquid_staking_message],
        })
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "execute".to_string(),
            messages: vec![message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&send_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
```

In this code snippet, we are sending a message to the Authorization contract to execute the liquid staking message on the base account on Persistence. Note that we are using the same label that we used in the authorization creation. This is important because the Authorization contract will check if the label matches the one in the authorization. If it does not match, the execution will fail. The Authorization contract will send the message to the corresponding Polytone contract that will send it via IBC to the processor on the external domain.

6. Tick the processor

```rust
    tick_processor(
        &mut test_ctx,
        PERSISTENCE_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_on_persistence,
    );
    std::thread::sleep(std::time::Duration::from_secs(3));
```

The message must now be sitting on the processor on Persistence, therefore we need to tick the processor to trigger the execution. This will execute the message and send a callback with the result to the Authorization contract, which completes the full testing cycle.
