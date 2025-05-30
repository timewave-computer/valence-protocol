// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ProcessorBase} from "../processor/ProcessorBase.sol";
import {IProcessorMessageTypes} from "../processor/interfaces/IProcessorMessageTypes.sol";
import {ProcessorMessageDecoder} from "../processor/libs/ProcessorMessageDecoder.sol";
import {ICallback} from "../processor/interfaces/ICallback.sol";
import {IProcessor} from "../processor/interfaces/IProcessor.sol";
import {VerificationGateway} from "../verification/VerificationGateway.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title Authorization
 * @dev This contract manages authorizations for interactions with a processor contract.
 * It provides mechanisms for both standard address-based authorizations and ZK proof-based authorizations.
 * @notice The Authorization contract acts as a middleware for managing access control
 * to the Processor contract. It controls which addresses can call specific functions
 * on specific contracts in a specific order through the processor.
 * It will receive callbacks from the processor after executing messages and can either store
 * the callback data in its state or just emit events for them.
 */
contract Authorization is Ownable, ICallback, ReentrancyGuard {
    // Address of the processor that we will forward batches to
    ProcessorBase public processor;

    modifier onlyProcessor() {
        if (msg.sender != address(processor)) {
            revert("Only processor can call this function");
        }
        _;
    }

    /**
     * @notice Boolean indicating whether to store callbacks or just emit events for them
     * @dev If true, the contract will store callback data in the contract's state
     */
    bool public storeCallbacks;

    /**
     * @notice Event emitted when a callback is received from the processor
     * @dev This event is emitted when the processor sends a callback after executing a message
     * @param executionId The ID of the executed message
     * @param executionResult The result of the execution (success or failure)
     * @param executedCount The number of successfully executed functions
     * @param data Additional data related to the callback execution
     */
    event CallbackReceived(
        uint64 indexed executionId, IProcessor.ExecutionResult executionResult, uint64 executedCount, bytes data
    );
    /**
     * @notice Event emitted when an admin address is added
     * @dev This event is emitted when a new admin address is added to the list of authorized addresses
     * @dev Only used for Standard authorizations
     * @param admin The address that was added as an admin
     */
    event AdminAddressAdded(address indexed admin);
    /**
     * @notice Event emitted when an admin address is removed
     * @dev This event is emitted when an admin address is removed from the list of authorized addresses
     * @dev Only used for Standard authorizations
     * @param admin The address that was removed from the admin list
     */
    event AdminAddressRemoved(address indexed admin);
    /**
     * @notice Event emitted when an authorization is added
     * @dev This event is emitted when a new authorization with a specific label is added
     * @dev Only used for Standard authorizations
     * @param label The label of the authorization that was added
     */
    event AuthorizationAdded(string label);
    /**
     * @notice Event emitted when an authorization is removed
     * @dev This event is emitted when an authorization with a specific label is removed
     * @dev Only used for Standard authorizations
     * @param label The label of the authorization that was removed
     */
    event AuthorizationRemoved(string label);

    /**
     * @notice Callback data structure for processor callbacks
     * @dev This struct is used to store the callback data received from the processor
     * @param executionResult The result of the execution
     * @param executedCount The number of successfully executed functions
     * @param data Additional data related to the callback execution
     */
    struct ProcessorCallback {
        IProcessor.ExecutionResult executionResult;
        uint64 executedCount;
        bytes data;
    }

    /**
     * @notice Mapping of execution IDs to callback data
     * @dev This mapping stores the callback data for each execution ID
     *     Key: execution ID, Value: Callback information
     */
    mapping(uint64 => ProcessorCallback) public callbacks;

    /**
     * @notice Current execution ID for tracking message execution
     * @dev This ID is incremented with each message processed and helps track message sequence
     */
    uint64 public executionId;

    // ========================= Standard authorizations =========================

    /**
     * @notice Mapping of addresses that are allowed to perform admin operations
     * @dev Admin addresses can perform privileged operations like pausing/unpausing
     */
    mapping(address => bool) public adminAddresses;

    /**
     * @notice Mapping of authorization labels to their associated addresses that can execute them
     * @dev This mapping is used to check if a user is authorized to send a message for a specific label
     * @dev The mapping is structured as follows:
     *     label -> user addresses
     *     If address(0) is used as the user address, it indicates permissionless access
     */
    mapping(string => address[]) public authorizations;

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

    /**
     * @notice Mapping of authorization labels to their associated data
     * @dev This mapping stores the authorization data for each label
     *     Key: label, Value: array of AuthorizationData
     */
    mapping(string => AuthorizationData[]) public authorizationsData;

    // ========================= ZK authorizations =========================

    /**
     * @notice Address of the verification gateway contract used for zero-knowledge proof verification
     * @dev If zero-knowledge proofs are not being used, this can be set to address(0)
     */
    VerificationGateway public verificationGateway;

    /**
     * @notice Structure representing a ZK message that we'll get a proof for
     * @dev This structure contains all the information to know if the sender is authorized to provide this message and to prevent replay attacks
     * @param registry An ID to identify this message, similar to the label on CosmWasm authorizations
     * @param blockNumber The block number when the message was created
     * @param authorizationContract The address of the authorization contract that this message is for. If address(0) is used, then it's valid for any contract
     * @param processorMessage The actual message to be processed and that was proven
     */
    struct ZKMessage {
        uint64 registry;
        uint64 blockNumber;
        address authorizationContract;
        IProcessorMessageTypes.ProcessorMessage processorMessage;
    }

    /**
     * @notice Mapping of what addresses are authorized to send messages for a specific registry ID
     * @dev This mapping is used to check if a user is authorized to send a message for a specific registry ID
     * @dev The mapping is structured as follows:
     *     registry ID -> user addresses
     *     If address(0) is used as the user address, it indicates permissionless access
     */
    mapping(uint64 => address[]) public zkAuthorizations;

    /**
     * @notice Mapping of registry ID to boolean indicating if we need to validate the last block execution
     * @dev This mapping is used to check if we need to validate the last block execution for a specific registry ID
     * @dev The mapping is structured as follows:
     *     registry ID -> boolean indicating if we need to validate the last block execution
     */
    mapping(uint64 => bool) public validateBlockNumberExecution;
    /**
     * @notice Mapping of the last block a proof was executed for
     * @dev This mapping is used to prevent replay attacks by ensuring that proofs that are older or the same than the last executed one cannot be used
     * @dev This is important to ensure that the same or a previous proof cannot be used
     * @dev The mapping is structured as follows:
     *     registry ID -> last block number of the proof executed
     */
    mapping(uint64 => uint64) public zkAuthorizationLastExecutionBlock;

    // ========================= Implementation =========================

    /**
     * @notice Sets up the Authorization contract with initial configuration
     * @dev Initializes the contract with owner, processor, and optional verifier
     * @param _owner Address that will be set as the owner of this contract
     * @param _processor Address of the processor contract that will execute messages
     * @param _verificationGateway Address of the ZK verification gateway contract (can be address(0) if not using ZK proofs)
     * @param _storeCallbacks Boolean indicating whether to store callbacks or just emitting events
     */
    constructor(address _owner, address _processor, address _verificationGateway, bool _storeCallbacks)
        Ownable(_owner)
    {
        if (_processor == address(0)) {
            revert("Processor cannot be zero address");
        }
        processor = ProcessorBase(_processor);
        verificationGateway = VerificationGateway(_verificationGateway);
        executionId = 0;
        storeCallbacks = _storeCallbacks;
    }

    /**
     * @notice Updates the processor contract address
     * @dev Can only be called by the owner
     * @param _processor New processor contract address
     */
    function updateProcessor(address _processor) external onlyOwner {
        if (_processor == address(0)) {
            revert("Processor cannot be zero address");
        }
        processor = ProcessorBase(_processor);
    }

    /**
     * @notice Updates the ZK verification gateway contract address
     * @dev Can only be called by the owner
     * @param _verificationGateway New verificationGateway contract address
     */
    function updateVerificationGateway(address _verificationGateway) external onlyOwner {
        verificationGateway = VerificationGateway(_verificationGateway);
    }

    // ========================= Standard Authorizations =========================

    /**
     * @notice Adds an address to the list of admin addresses
     * @dev Can only be called by the owner
     * @param _admin Address to be granted admin privileges
     */
    function addAdminAddress(address _admin) external onlyOwner {
        adminAddresses[_admin] = true;
        emit AdminAddressAdded(_admin);
    }

    /**
     * @notice Removes an address from the list of admin addresses
     * @dev Can only be called by the owner
     * @param _admin Address to have admin privileges revoked
     */
    function removeAdminAddress(address _admin) external onlyOwner {
        delete adminAddresses[_admin];
        emit AdminAddressRemoved(_admin);
    }

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
    ) external onlyOwner {
        // Check that the arrays are the same length
        // We are allowing adding multiple authorizations at once for gas optimization
        // The arrays must be the same length because for each label we have a list of authorization data
        require(
            _labels.length == _authorizationData.length && _labels.length == _users.length, "Array lengths must match"
        );

        for (uint256 i = 0; i < _labels.length; i++) {
            // Get the label and the authorization data
            string memory label = _labels[i];
            address[] memory users = _users[i];
            // Check that users is not empty
            require(users.length > 0, "Users array cannot be empty");
            AuthorizationData[] memory authorizationData = _authorizationData[i];
            // Check that the authorization data is not empty
            require(
                authorizationData.length > 0, string.concat("Authorization data array cannot be empty for: ", label)
            );

            // Add the label to the mapping
            authorizations[label] = users;
            // Add the authorization data to the mapping
            authorizationsData[label] = authorizationData;

            emit AuthorizationAdded(label);
        }
    }

    /**
     * @notice Removes standard authorizations for a specific set of labels
     * @dev Can only be called by the owner
     * @param _labels Array of labels for the authorizations to be removed
     */
    function removeStandardAuthorizations(string[] memory _labels) external onlyOwner {
        for (uint256 i = 0; i < _labels.length; i++) {
            // Get the label
            string memory label = _labels[i];
            // Remove from state
            delete authorizationsData[label];
            delete authorizations[label];
            emit AuthorizationRemoved(label);
        }
    }

    /**
     * @notice Sends a message to the processor for execution
     * @dev This function is called by authorized addresses to send messages to the processor
     * @param label The label of the authorization that is being used
     * @param _message The encoded message to be sent to the processor
     */
    function sendProcessorMessage(string calldata label, bytes calldata _message) external nonReentrant {
        // Make a copy of the message to apply modifications
        bytes memory message = _message;

        // Decode the message to check authorization and apply modifications
        IProcessorMessageTypes.ProcessorMessage memory decodedMessage = ProcessorMessageDecoder.decode(message);

        // Process message based on type
        if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            message = _handleSendMsgsRequest(label, decodedMessage);
        } else if (decodedMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.InsertMsgs) {
            message = _handleInsertMsgsRequest(decodedMessage);
        } else {
            _requireAdminAccess();
        }

        // Forward the validated and modified message to the processor
        processor.execute(message);

        // Increment the execution ID for the next message
        executionId++;
    }

    /**
     * @notice Handles the InsertMsgs message type
     * @dev This function modifies the InsertMsgs message to set the execution ID and encode it back. This requires admin access.
     * @param decodedMessage The decoded InsertMsgs message
     * @return The encoded processor message with the updated execution ID
     */
    function _handleInsertMsgsRequest(IProcessorMessageTypes.ProcessorMessage memory decodedMessage)
        private
        view
        returns (bytes memory)
    {
        _requireAdminAccess();

        IProcessorMessageTypes.InsertMsgs memory insertMsgs =
            abi.decode(decodedMessage.message, (IProcessorMessageTypes.InsertMsgs));

        // Set the execution ID of the message
        insertMsgs.executionId = executionId;

        // Encode the message back after modification
        decodedMessage.message = abi.encode(insertMsgs);

        // Return the encoded processor message
        return abi.encode(decodedMessage);
    }

    /**
     * @notice Handles the SendMsgs message type
     * @dev This function modifies the SendMsgs message to set the execution ID and encode it back. It also verifies authorizations based on subroutine type.
     * @param label The label of the authorization that is being used
     * @param decodedMessage The decoded SendMsgs message
     * @return The encoded processor message with the updated execution ID
     */
    function _handleSendMsgsRequest(
        string calldata label,
        IProcessorMessageTypes.ProcessorMessage memory decodedMessage
    ) private view returns (bytes memory) {
        // Decode the SendMsgs message
        IProcessorMessageTypes.SendMsgs memory sendMsgs =
            abi.decode(decodedMessage.message, (IProcessorMessageTypes.SendMsgs));

        // Verify authorizations based on subroutine type
        if (sendMsgs.subroutine.subroutineType == IProcessorMessageTypes.SubroutineType.Atomic) {
            _verifyAtomicSubroutineAuthorization(label, sendMsgs);
        } else {
            _verifyNonAtomicSubroutineAuthorization(label, sendMsgs);
        }

        // Apply standard modifications to all SendMsgs
        sendMsgs.priority = IProcessorMessageTypes.Priority.Medium;
        sendMsgs.executionId = executionId;

        // Encode the message back after modifications
        decodedMessage.message = abi.encode(sendMsgs);

        // Return the encoded processor message
        return abi.encode(decodedMessage);
    }

    /**
     * @notice Verifies the authorization for an atomic subroutine
     * @dev This function checks if the sender is authorized to execute the atomic subroutine
     * @param label The label of the authorization that is being used
     * @param sendMsgs The SendMsgs message containing the subroutine to be executed
     */
    function _verifyAtomicSubroutineAuthorization(
        string calldata label,
        IProcessorMessageTypes.SendMsgs memory sendMsgs
    ) private view {
        IProcessorMessageTypes.AtomicSubroutine memory atomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.AtomicSubroutine));

        // Verify message and function array lengths match
        if (atomicSubroutine.functions.length > 0 && atomicSubroutine.functions.length != sendMsgs.messages.length) {
            revert("Subroutine functions length does not match messages length");
        }

        // Create the AuthorizationData array for the atomic subroutine
        AuthorizationData[] memory authorizationData = new AuthorizationData[](atomicSubroutine.functions.length);
        for (uint256 i = 0; i < atomicSubroutine.functions.length; i++) {
            // Get the contract address and function signature hash
            address contractAddress = atomicSubroutine.functions[i].contractAddress;
            bytes4 functionSelector = bytes4(sendMsgs.messages[i]); // Takes first 4 bytes of the message
            bytes32 callHash = keccak256(sendMsgs.messages[i]);

            // Add the authorization data to the array
            authorizationData[i] = AuthorizationData(contractAddress, true, functionSelector, callHash);
        }

        // Check if address is authorized to execute this subroutine
        if (!_checkAddressIsAuthorized(msg.sender, label, authorizationData)) {
            revert("Unauthorized access");
        }
    }

    /**
     * @notice Verifies the authorization for a non-atomic subroutine
     * @dev This function checks if the sender is authorized to execute the non-atomic subroutine
     * @param label The label of the authorization that is being used
     * @param sendMsgs The SendMsgs message containing the subroutine to be executed
     */
    function _verifyNonAtomicSubroutineAuthorization(
        string calldata label,
        IProcessorMessageTypes.SendMsgs memory sendMsgs
    ) private view {
        IProcessorMessageTypes.NonAtomicSubroutine memory nonAtomicSubroutine =
            abi.decode(sendMsgs.subroutine.subroutine, (IProcessorMessageTypes.NonAtomicSubroutine));

        // Verify message and function array lengths match
        if (
            nonAtomicSubroutine.functions.length > 0 && nonAtomicSubroutine.functions.length != sendMsgs.messages.length
        ) {
            revert("Subroutine functions length does not match messages length");
        }

        // Create the AuthorizationData array for the non-atomic subroutine
        AuthorizationData[] memory authorizationData = new AuthorizationData[](nonAtomicSubroutine.functions.length);
        for (uint256 i = 0; i < nonAtomicSubroutine.functions.length; i++) {
            // Get the contract address and function signature hash
            address contractAddress = nonAtomicSubroutine.functions[i].contractAddress;
            bytes4 functionSelector = bytes4(sendMsgs.messages[i]); // Takes first 4 bytes of the message
            bytes32 callHash = keccak256(sendMsgs.messages[i]);

            // Add the authorization data to the array
            authorizationData[i] = AuthorizationData(contractAddress, true, functionSelector, callHash);
        }
        // Check if address is authorized to execute this subroutine
        if (!_checkAddressIsAuthorized(msg.sender, label, authorizationData)) {
            revert("Unauthorized access");
        }
    }

    /**
     * @notice Require that sender has admin access
     * @dev Reverts if sender is not in the adminAddresses mapping
     */
    function _requireAdminAccess() private view {
        if (!adminAddresses[msg.sender]) {
            revert("Unauthorized access");
        }
    }

    /**
     * @notice Checks if the address is authorized to execute a message
     * @dev This function checks if the address is in the list of authorized addresses for this label
     * @param _address The address to check for authorization
     * @param label The label of the authorization that is being used
     * @param _authorizationData The authorization data that needs to be checked
     * @return True if the address is authorized, false otherwise
     */
    function _checkAddressIsAuthorized(
        address _address,
        string calldata label,
        AuthorizationData[] memory _authorizationData
    ) internal view returns (bool) {
        // Check if the address is in the list of authorized addresses for this label
        address[] memory authorizedAddresses = authorizations[label];

        bool isAuthorized = false;
        for (uint256 i = 0; i < authorizedAddresses.length; i++) {
            if (authorizedAddresses[i] == _address || authorizedAddresses[i] == address(0)) {
                isAuthorized = true;
                break;
            }
        }

        // If the address is not authorized, return false
        if (!isAuthorized) {
            return false;
        }

        // Load the authorization data for this label
        AuthorizationData[] memory labelAuthorizationData = authorizationsData[label];

        // Check that the lengths are the same
        if (labelAuthorizationData.length != _authorizationData.length) {
            return false;
        }

        // Check that each element is the same
        for (uint256 i = 0; i < labelAuthorizationData.length; i++) {
            // Check that the contract address is the same
            if (labelAuthorizationData[i].contractAddress != _authorizationData[i].contractAddress) {
                return false;
            }

            // If we need to check the function selector, check that it's the same
            if (labelAuthorizationData[i].useFunctionSelector) {
                if (labelAuthorizationData[i].functionSelector != _authorizationData[i].functionSelector) {
                    return false;
                }
            } else {
                // If we don't need to check the function selector, check that the call hash is the same
                if (labelAuthorizationData[i].callHash != _authorizationData[i].callHash) {
                    return false;
                }
            }
        }

        return true;
    }

    // ========================= ZK authorizations =========================

    /**
     * @notice Adds a new registry with its associated users and verification keys
     * @dev This function allows the owner to add multiple registries and their associated users and verification keys
     * @param registries Array of registry IDs to be added
     * @param users Array of arrays of user addresses associated with each registry
     * @param vks Array of verification keys associated with each registry
     * @param validateBlockNumber Array of booleans indicating if we need to validate the last block execution for each registry
     */
    function addRegistries(
        uint64[] memory registries,
        address[][] memory users,
        bytes32[] calldata vks,
        bool[] memory validateBlockNumber
    ) external onlyOwner {
        // Since we are allowing multiple registries to be added at once, we need to check that the arrays are the same length
        // because for each registry we have a list of users, a verification key and a boolean
        // Allowing multiple to be added is useful for gas optimization
        require(
            users.length == registries.length && users.length == vks.length
                && users.length == validateBlockNumber.length,
            "Array lengths must match"
        );

        for (uint256 i = 0; i < registries.length; i++) {
            // Add the registry to the verification gateway
            verificationGateway.addRegistry(registries[i], vks[i]);
            zkAuthorizations[registries[i]] = users[i];
            // Only store if true because default is false
            if (validateBlockNumber[i]) {
                validateBlockNumberExecution[registries[i]] = true;
            }
        }
    }

    /**
     * @notice Removes a registry and its associated users
     * @dev This function allows the owner to remove a registry and its associated users
     * @param registries Array of registry IDs to be removed
     */
    function removeRegistries(uint64[] memory registries) external onlyOwner {
        for (uint256 i = 0; i < registries.length; i++) {
            // Remove the registry from the verification gateway
            verificationGateway.removeRegistry(registries[i]);
            delete zkAuthorizations[registries[i]];
            // Delete the last execution block for the registry
            delete zkAuthorizationLastExecutionBlock[registries[i]];
            // Delete the validation flag for the registry
            delete validateBlockNumberExecution[registries[i]];
        }
    }

    /**
     * @notice Get all authorized addresses for a specific registry ID
     * @param registryId The registry ID to check
     * @return An array of all authorized addresses for the given registry ID
     * @dev This function returns all addresses that are authorized to send messages for the given registry ID
     * @dev It's useful for checking which addresses have permission to send messages in one go
     */
    function getZkAuthorizationsList(uint64 registryId) public view returns (address[] memory) {
        return zkAuthorizations[registryId];
    }

    /**
     * @notice Executes a ZK message with proof verification
     * @dev This function verifies the proof and executes the message if authorized
     * @dev The proof is verified using the verification gateway before executing the message
     * @param _message Encoded ZK message to be executed
     * @param _proof Proof associated with the ZK message
     */
    function executeZKMessage(bytes calldata _message, bytes calldata _proof) external nonReentrant {
        // Check that the verification gateway is set
        if (address(verificationGateway) == address(0)) {
            revert("Verification gateway not set");
        }

        // Decode the message to check authorization and apply modifications
        // We need to skip the first 32 bytes because this will be the coprocessor root which we don't need to decode
        ZKMessage memory decodedZKMessage = abi.decode(_message[32:], (ZKMessage));

        // Check that the message is valid for this authorization contract
        if (
            decodedZKMessage.authorizationContract != address(0)
                && decodedZKMessage.authorizationContract != address(this)
        ) {
            revert("Invalid authorization contract");
        }

        // Check that sender is authorized to send this message
        address[] memory authorizedAddresses = zkAuthorizations[decodedZKMessage.registry];
        bool isAuthorized = false;
        for (uint256 i = 0; i < authorizedAddresses.length; i++) {
            if (authorizedAddresses[i] == msg.sender || authorizedAddresses[i] == address(0)) {
                isAuthorized = true;
                break;
            }
        }

        if (!isAuthorized) {
            revert("Unauthorized address for this registry");
        }

        // Cache the validate block condition
        bool validateBlockNumberExecCondition = validateBlockNumberExecution[decodedZKMessage.registry];

        // If we need to validate the last block execution, check that the block number is greater than the last one
        if (validateBlockNumberExecCondition) {
            if (decodedZKMessage.blockNumber <= zkAuthorizationLastExecutionBlock[decodedZKMessage.registry]) {
                revert("Proof no longer valid");
            }
        }

        // Verify the proof using the verification gateway
        if (!verificationGateway.verify(decodedZKMessage.registry, _proof, _message)) {
            revert("Proof verification failed");
        }

        // Get the message and update the execution ID if it's a SendMsgs or InsertMsgs message, according to the
        // current execution ID of the contract
        if (decodedZKMessage.processorMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.SendMsgs) {
            IProcessorMessageTypes.SendMsgs memory sendMsgs =
                abi.decode(decodedZKMessage.processorMessage.message, (IProcessorMessageTypes.SendMsgs));
            sendMsgs.executionId = executionId;
            decodedZKMessage.processorMessage.message = abi.encode(sendMsgs);
        } else if (
            decodedZKMessage.processorMessage.messageType == IProcessorMessageTypes.ProcessorMessageType.InsertMsgs
        ) {
            IProcessorMessageTypes.InsertMsgs memory insertMsgs =
                abi.decode(decodedZKMessage.processorMessage.message, (IProcessorMessageTypes.InsertMsgs));
            insertMsgs.executionId = executionId;
            decodedZKMessage.processorMessage.message = abi.encode(insertMsgs);
        }

        // Increment the execution ID for the next message
        executionId++;

        // Update the last execution block for the registry (only if we need to validate the last block execution)
        if (validateBlockNumberExecCondition) {
            zkAuthorizationLastExecutionBlock[decodedZKMessage.registry] = decodedZKMessage.blockNumber;
        }

        // Execute the message using the processor
        processor.execute(abi.encode(decodedZKMessage.processorMessage));
    }

    // ========================= Processor Callbacks =========================

    /**
     * @notice Handles callbacks from the processor after executing messages
     * @dev This function is called by the processor to notify the contract of execution results
     * @param callbackData Encoded callback data containing execution result and other information
     */
    function handleCallback(bytes memory callbackData) external override onlyProcessor {
        // Decode the callback data
        IProcessor.Callback memory callback = abi.decode(callbackData, (IProcessor.Callback));

        // Store the callback data if storeCallbacks is true
        if (storeCallbacks) {
            callbacks[callback.executionId] = ProcessorCallback({
                executionResult: callback.executionResult,
                executedCount: uint64(callback.executedCount),
                data: callback.data
            });
        }

        emit CallbackReceived(
            callback.executionId, callback.executionResult, uint64(callback.executedCount), callback.data
        );
    }
}
