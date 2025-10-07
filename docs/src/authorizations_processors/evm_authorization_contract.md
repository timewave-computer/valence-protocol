# EVM Authorization Contract

If a general message passing protocol like Hyperlane wants to be avoided to not require the deployment of additional infrastructure, we also provide a Solidity version of the Authorization contract with similar functionality than the CosmWasm version.

These are the steps to set up our EVM program using the EVM Authorization contract instead of Hyperlane:

1) Deploy `Authorization.sol` providing the program owner, the lite processor address (previously deployed) and a flag specifying if we want to store the callbacks in the contract state or just emit them as events (less gas consumption):

```solidity
constructor(address _owner, address _processor, bool _storeCallbacks) Ownable(_owner)
```

2) Once it's deployed, we need to set the authorization contract as an authorized address on the processor.

```solidity
function addAuthorizedAddress(address _address)
```

This will allow processing the messages that the newly deployed authorization contract will forward to the processor.

3) Now we can start adding our authorizations:

```solidity
 /**
     * @notice Adds standard authorizations for a specific label
     * @dev Can only be called by the owner
     * @param _labels Array of labels for the authorizations
     * @param _users Array of arrays of user addresses associated with each label
     * @param _authorizationData Array of arrays of authorization data associated with each label
     */
    function addStandardAuthorizations(
        string[] memory _labels,
        address[][] memory _users,
        AuthorizationData[][] memory _authorizationData
    )
```

This method allows adding multiple authorizations at the same time using arrays, to optimize the gas consumption. The most important part here is the `AuthorizationData`, which is defined as following:

```solidity
    /**
     * @notice Structure representing the data for the authorization label
     * @dev This structure contains the contract address and the function signature hash
     * @param contractAddress The address of the contract that is authorized to be called
     * @param useFunctionSelector Boolean indicating if the function selector should be used instead of callHash
     * @param functionSelector The function selector of the function that is authorized to be called
     * @param callHash The function signature hash of the function that is authorized to be called
     */
    struct AuthorizationData {
        address contractAddress;
        bool useFunctionSelector;
        bytes4 functionSelector;
        bytes32 callHash;
    }
```

As explained above, we have two ways of defining our authorization: using the function selector or a callHash. If we use a function selector, the authorized address is allowed to execute that specific function with **ANY** arguments. For example, if the function is `transfer(uint256 amount)` the address can specify any amount value when calling the authorization. On the other hand, if we want to restrict the call to a specific value, we provide the hash call so that only those specific call bytes can be executed. For example, we hash `transfer(1000)` and provide that as the `callHash` and then the authorized address can **ONLY** call this authorization with that specific value. 

As we can see this is less flexible than the CosmWasm version due to the nature of the Solidity language vs Rust but tends to be enough for most of the programs. If more flexibility is required, the option of using a message passing protocol with our encoding/decoding mechanisms or using the ZK Coprocessor is also available.

4) Now that everything is set up, we can execute our authorization like this:

```solidity
function sendProcessorMessage(string calldata label, bytes calldata _message) 
```

We simply need to specify what label we want to execute and the encoded `ProcessorMessage` that will be forwarded to the Processor. This perform all the checks against our AuthorizationData, and if they all pass, the message will be forwarded to the processor, executed, and a callback will be received on the Authorization contract.
