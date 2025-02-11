# Encoding

When messages are passed between the `Authorization` contract and a `Processor` contract on a domain that is not using a `CosmWasm ExecutionEnvironment`, we need to encode the messages in a way that the `Processor` contract and the Libraries it calls can understand them. To do this two new contracts were created: `Encoder Broker` and `Encoder`.

## Encoder Broker

The `Encoder Broker` is a very simple contract that will route the messages to the correct `Encoder` contract. It maps from `Encoder Version` to `Encoder Contract Address`. The `Encoder Broker` will be instantiated once on the `Main Domain` with an owner that can add/remove these mappings. An example of Mapping can be `"evm_encoder_v1"` to `<encoder_contract_address_on_neutron>`. The `Encoder Broker` has two queries: `Encode` and `Decode`, which routes the message to encode/decode to the `Encoder Version` specified.

## Encoder

The `Encoder` is the contract that will encode/decode the messages for a specific `ExecutionEnvironment`. It will be instantiated on the `Main Domain` an added to the `Encoder Broker` with a version. `Encoders` are defined for a specific `Execution Environment` and have an `Encode` and `Decode` query where we provide the Message to be encoded/decoded. Here is an example of how these queries are performed:

```rust
fn encode(message: ProcessorMessageToEncode) -> StdResult<Binary> {
    match message {
        ProcessorMessageToEncode::SendMsgs {
            execution_id,
            priority,
            subroutine,
            messages,
        } => send_msgs::encode(execution_id, priority, subroutine, messages),
        ProcessorMessageToEncode::InsertMsgs {
            execution_id,
            queue_position,
            priority,
            subroutine,
            messages,
        } => insert_msgs::encode(execution_id, queue_position, priority, subroutine, messages),
        ProcessorMessageToEncode::EvictMsgs {
            queue_position,
            priority,
        } => evict_msgs::encode(queue_position, priority),
        ProcessorMessageToEncode::Pause {} => Ok(pause::encode()),
        ProcessorMessageToEncode::Resume {} => Ok(resume::encode()),
    }
}

fn decode(message: ProcessorMessageToDecode) -> StdResult<Binary> {
    match message {
        ProcessorMessageToDecode::HyperlaneCallback { callback } => {
            Ok(hyperlane::callback::decode(&callback)?)
        }
    }
}
```

As we can see above, the `Encoder` will have a match statement for each type of message that it can encode/decode. The `Encoder` will be able to encode/decode messages for a specific `ExecutionEnvironment`. In the case of `ProcessorMessages` that include messages for a specific library, these messages will include the Library they are targetting and this way the `Encoder` will apply the encoding/decoding logic for that specific library.
This `Encoder` will be called internally through the `Authorization` contract when the user sends a message to it. Here is an example of how the flow looks like:

1. The owner adds an `ExternalDomain` with an `EVM ExecutionEnvironment` to the Authorization contract, specifying the `Encoder Broker` address and the `Encoder Version` to be used.
2. The owner creates an authorization with a subroutine with an `AtomicFunction` that is of `EvmCall(EncoderInfo, LibraryName)` type.
3. A user executes this authorization passing the message. The `Authorization` contract will route the message to the `Encoder Broker` with the `Encoder Version` specified in `EncoderInfo` and passing the `LibraryName` to be used for the message.
4. The `Encoder Broker` will route the message to the correct `Encoder` contract, which will encode the message for that particular library and return the encoded bytes to the Authorization Contract.
5. The Authorization contract will send the encoded message to the `Processor` contract on the `ExternalDomain`, which will be able to decode and understand the message.

We currently have an `Encoder` for `EVM` messages, but more will be added as we add more compatible `ExecutionEnvironments` to the protocol.
